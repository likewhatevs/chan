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
// All public methods are stubs at this point; the API shape is
// what we're committing to. The first real implementation lives
// behind `Drive::graph()` which constructs a `GraphView` against
// the per-drive sqlite handle.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Edge kind. Mirrors the wiki-link / mention / tag distinction
/// that the editor already exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeKind {
    /// `[[wiki-link]]` from src to dst. Includes optional anchor.
    Link,
    /// `@mention` of a person / project / topic. Mentions resolve
    /// to a target file when one exists; otherwise to a placeholder
    /// node so the graph stays connected.
    Mention,
    /// `#tag` applied to src. dst is the tag node.
    Tag,
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

    /// Outgoing edges from `rel`.
    pub fn neighbors(&self, _rel: &str) -> Result<Vec<Edge>> {
        Ok(Vec::new())
    }

    /// Incoming edges into `rel`.
    pub fn backlinks(&self, _rel: &str) -> Result<Vec<Edge>> {
        Ok(Vec::new())
    }

    /// All tags in the drive with their reference counts.
    pub fn tags(&self) -> Result<Vec<Tag>> {
        Ok(Vec::new())
    }

    /// Files tagged with `tag`.
    pub fn files_with_tag(&self, _tag: &str) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    /// Replace the graph data for one file: removes existing
    /// edges/headings owned by `rel` and inserts the supplied ones.
    /// The full-update semantic keeps incremental indexing simple.
    pub fn replace_file(
        &self,
        rel: &str,
        mtime: Option<i64>,
        _outgoing: &[Edge],
        _headings: &[Heading],
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT OR REPLACE INTO nodes(rel_path, kind, mtime) VALUES (?, 'file', ?)",
            params![rel, mtime],
        )?;
        tx.execute("DELETE FROM edges WHERE src = ?", params![rel])?;
        tx.execute("DELETE FROM headings WHERE rel_path = ?", params![rel])?;
        // TODO: insert _outgoing and _headings once chunking lands.
        tx.commit()?;
        Ok(())
    }

    /// Drop a file from the graph entirely.
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
}

/// One ATX heading inside a file. Order is the document order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heading {
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
