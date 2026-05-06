// Graph DB: relations between files via wiki-links, mentions, tags,
// and headings. Backed by sqlite (rusqlite, bundled feature).
//
// Schema (applied on first open via PRAGMA user_version migration):
//
//   nodes(rel_path TEXT PRIMARY KEY,
//         kind     TEXT NOT NULL,    -- "file" | "tag" | "heading"
//         mtime    INTEGER)          -- Unix seconds, NULL for tags
//
//   edges(src      TEXT NOT NULL,    -- node rel_path
//         dst      TEXT NOT NULL,    -- node rel_path
//         kind     TEXT NOT NULL,    -- "link" | "mention" | "tag"
//         anchor   TEXT,             -- optional heading anchor on dst
//         PRIMARY KEY (src, dst, kind))
//
//   headings(rel_path TEXT NOT NULL,
//            level    INTEGER NOT NULL,
//            text     TEXT NOT NULL,
//            anchor   TEXT NOT NULL,
//            ord      INTEGER NOT NULL,
//            PRIMARY KEY (rel_path, ord))
//
// `Drive::graph()` constructs a `GraphView` against the per-drive
// sqlite handle. Reads (neighbors / backlinks / tags / files_with_tag
// / headings_of / files) and writes (replace_file / forget_file /
// clear) are both wired; `Drive::reindex` calls `clear` then
// `replace_file` per file as it walks the tree.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::markdown;

/// Edge kind. Mirrors the wiki-link / mention / tag distinction
/// that the editor already exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeKind {
    /// `[[wiki-link]]` or markdown `[label](path)` from src to dst.
    /// Anchor is the optional `#section` fragment on dst.
    Link,
    /// `@@mention`. dst is `@@name` until the mention resolves to
    /// a real file.
    Mention,
    /// `#tag` applied to src. dst is `#name`.
    Tag,
}

impl EdgeKind {
    fn as_str(self) -> &'static str {
        match self {
            EdgeKind::Link => "link",
            EdgeKind::Mention => "mention",
            EdgeKind::Tag => "tag",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "link" => Some(EdgeKind::Link),
            "mention" => Some(EdgeKind::Mention),
            "tag" => Some(EdgeKind::Tag),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub src: String,
    pub dst: String,
    pub kind: EdgeKind,
    pub anchor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub count: u32,
}

/// Owns the sqlite connection and exposes the read API. Construct
/// via `Drive::graph()`. The connection is behind a Mutex so the
/// view is Sync (rusqlite::Connection is Send but not Sync).
pub struct GraphView {
    conn: Mutex<Connection>,
}

impl GraphView {
    /// Open or create the graph DB for this drive.
    pub fn open(graph_db_path: &Path) -> Result<Self> {
        if let Some(parent) = graph_db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(graph_db_path)?;
        Self::migrate(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn migrate(conn: &Connection) -> Result<()> {
        let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if v < 1 {
            conn.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS nodes (
                    rel_path TEXT PRIMARY KEY,
                    kind     TEXT NOT NULL,
                    mtime    INTEGER
                );
                CREATE TABLE IF NOT EXISTS edges (
                    src    TEXT NOT NULL,
                    dst    TEXT NOT NULL,
                    kind   TEXT NOT NULL,
                    anchor TEXT,
                    PRIMARY KEY (src, dst, kind)
                );
                CREATE INDEX IF NOT EXISTS edges_dst_idx ON edges(dst);
                CREATE TABLE IF NOT EXISTS headings (
                    rel_path TEXT NOT NULL,
                    level    INTEGER NOT NULL,
                    text     TEXT NOT NULL,
                    anchor   TEXT NOT NULL,
                    ord      INTEGER NOT NULL,
                    PRIMARY KEY (rel_path, ord)
                );
                PRAGMA user_version = 1;
                "#,
            )?;
        }
        Ok(())
    }

    /// Outgoing edges from `rel`. Document order is not preserved;
    /// callers that need stable order should sort by (kind, dst).
    pub fn neighbors(&self, rel: &str) -> Result<Vec<Edge>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT dst, kind, anchor FROM edges WHERE src = ? ORDER BY kind, dst",
        )?;
        let rows = stmt.query_map(params![rel], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (dst, kind_str, anchor) = row?;
            if let Some(kind) = EdgeKind::from_str(&kind_str) {
                out.push(Edge {
                    src: rel.to_string(),
                    dst,
                    kind,
                    anchor,
                });
            }
        }
        Ok(out)
    }

    /// Incoming edges into `rel` (other files that link to it).
    pub fn backlinks(&self, rel: &str) -> Result<Vec<Edge>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT src, kind, anchor FROM edges WHERE dst = ? AND kind = 'link' ORDER BY src",
        )?;
        let rows = stmt.query_map(params![rel], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (src, kind_str, anchor) = row?;
            if let Some(kind) = EdgeKind::from_str(&kind_str) {
                out.push(Edge {
                    src,
                    dst: rel.to_string(),
                    kind,
                    anchor,
                });
            }
        }
        Ok(out)
    }

    /// All tags in the drive with their reference counts. Tag dst
    /// nodes are stored as `#name`; we strip the prefix in the result.
    pub fn tags(&self) -> Result<Vec<Tag>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT dst, COUNT(*) FROM edges WHERE kind = 'tag' GROUP BY dst ORDER BY 2 DESC, 1",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u32))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (dst, count) = row?;
            let name = dst.strip_prefix('#').unwrap_or(&dst).to_string();
            out.push(Tag { name, count });
        }
        Ok(out)
    }

    /// Files tagged with `tag` (without the leading `#`).
    pub fn files_with_tag(&self, tag: &str) -> Result<Vec<String>> {
        let dst = format!("#{tag}");
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT DISTINCT src FROM edges WHERE kind = 'tag' AND dst = ? ORDER BY src",
        )?;
        let rows = stmt.query_map(params![dst], |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Replace the graph data for one file: removes existing
    /// edges/headings owned by `rel` and inserts the supplied ones
    /// in a single transaction.
    pub fn replace_file(
        &self,
        rel: &str,
        mtime: Option<i64>,
        outgoing: &[Edge],
        headings: &[markdown::Heading],
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT OR REPLACE INTO nodes(rel_path, kind, mtime) VALUES (?, 'file', ?)",
            params![rel, mtime],
        )?;
        tx.execute("DELETE FROM edges WHERE src = ?", params![rel])?;
        tx.execute("DELETE FROM headings WHERE rel_path = ?", params![rel])?;
        {
            let mut ins_edge = tx.prepare_cached(
                "INSERT OR IGNORE INTO edges(src, dst, kind, anchor) VALUES (?, ?, ?, ?)",
            )?;
            for e in outgoing {
                ins_edge.execute(params![rel, e.dst, e.kind.as_str(), e.anchor])?;
            }
            let mut ins_heading = tx.prepare_cached(
                "INSERT INTO headings(rel_path, level, text, anchor, ord) VALUES (?, ?, ?, ?, ?)",
            )?;
            for h in headings {
                let anchor = markdown::heading_anchor(&h.text);
                ins_heading.execute(params![rel, h.level as i64, h.text, anchor, h.ord as i64])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Drop a file from the graph entirely. Edges with `rel` as
    /// either endpoint go too; no dangling references.
    pub fn forget_file(&self, rel: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM edges WHERE src = ? OR dst = ?",
            params![rel, rel],
        )?;
        tx.execute("DELETE FROM headings WHERE rel_path = ?", params![rel])?;
        tx.execute("DELETE FROM nodes WHERE rel_path = ?", params![rel])?;
        tx.commit()?;
        Ok(())
    }

    /// Wipe every file, edge, and heading. Used by `Drive::reindex`
    /// before rebuilding from scratch.
    pub fn clear(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        tx.execute("DELETE FROM edges", [])?;
        tx.execute("DELETE FROM headings", [])?;
        tx.execute("DELETE FROM nodes", [])?;
        tx.commit()?;
        Ok(())
    }

    /// All files known to the graph, sorted by path.
    pub fn files(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare_cached("SELECT rel_path FROM nodes WHERE kind = 'file' ORDER BY rel_path")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Headings of one file in document order.
    pub fn headings_of(&self, rel: &str) -> Result<Vec<HeadingRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT level, text, anchor, ord FROM headings \
             WHERE rel_path = ? ORDER BY ord",
        )?;
        let rows = stmt.query_map(params![rel], |row| {
            Ok(HeadingRow {
                level: row.get::<_, i64>(0)? as u8,
                text: row.get::<_, String>(1)?,
                anchor: row.get::<_, String>(2)?,
                ord: row.get::<_, i64>(3)? as u32,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

/// A heading as stored in the graph DB. Differs from
/// `markdown::Heading` by carrying the computed anchor and not the
/// source line number (we don't need it for graph queries).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadingRow {
    pub level: u8,
    pub text: String,
    pub anchor: String,
    pub ord: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn count(g: &GraphView, sql: &str) -> i64 {
        let conn = g.conn.lock().unwrap();
        conn.query_row(sql, [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn open_creates_schema() {
        let tmp = TempDir::new().unwrap();
        let db = tmp.path().join("graph.sqlite");
        let g = GraphView::open(&db).unwrap();
        assert_eq!(count(&g, "PRAGMA user_version"), 1);
    }

    #[test]
    fn replace_then_forget_round_trips() {
        let tmp = TempDir::new().unwrap();
        let db = tmp.path().join("graph.sqlite");
        let g = GraphView::open(&db).unwrap();
        g.replace_file("notes/a.md", Some(1000), &[], &[]).unwrap();
        assert_eq!(
            count(
                &g,
                "SELECT COUNT(*) FROM nodes WHERE rel_path = 'notes/a.md'"
            ),
            1
        );
        g.forget_file("notes/a.md").unwrap();
        assert_eq!(
            count(
                &g,
                "SELECT COUNT(*) FROM nodes WHERE rel_path = 'notes/a.md'"
            ),
            0
        );
    }
}
