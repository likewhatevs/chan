// Graph DB: relations between files via wiki-links, mentions, tags,
// and headings. Backed by sqlite (rusqlite, bundled feature).
//
// Schema (applied on first open via PRAGMA user_version migration):
//
//   nodes(rel_path TEXT PRIMARY KEY,
//         kind     TEXT NOT NULL,    -- "file" | "tag" | "heading"
//         mtime    INTEGER,          -- Unix seconds, NULL for tags
//         title    TEXT,             -- file's display title (h1 or
//                                       frontmatter `title`); NULL
//                                       for non-file rows or until
//                                       the next index_file pass
//         basename TEXT)             -- file_name() of rel_path,
//                                       used by link_targets prefix
//                                       lookup. NULL for non-file
//                                       rows.
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
        if v < 2 {
            // v2: add title + basename to nodes for link_targets.
            // basename is derived from rel_path (no file IO needed)
            // and backfilled here. title stays NULL until the next
            // index_file / reindex pass repopulates it from content.
            conn.execute_batch(
                r#"
                ALTER TABLE nodes ADD COLUMN title TEXT;
                ALTER TABLE nodes ADD COLUMN basename TEXT;
                CREATE INDEX IF NOT EXISTS nodes_basename_idx ON nodes(basename);
                PRAGMA user_version = 2;
                "#,
            )?;
            let paths: Vec<String> = {
                let mut stmt = conn.prepare("SELECT rel_path FROM nodes WHERE kind = 'file'")?;
                let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
                let mut out = Vec::new();
                for row in rows {
                    out.push(row?);
                }
                out
            };
            let tx = conn.unchecked_transaction()?;
            {
                let mut stmt = tx.prepare("UPDATE nodes SET basename = ?2 WHERE rel_path = ?1")?;
                for p in &paths {
                    let bn = std::path::Path::new(p)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(p)
                        .to_string();
                    stmt.execute(params![p, bn])?;
                }
            }
            tx.commit()?;
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
    /// in a single transaction. `title` is the file's display title
    /// (h1 or frontmatter `title`) and is stored on the node for
    /// the link-autocomplete query (`link_targets`).
    pub fn replace_file(
        &self,
        rel: &str,
        title: Option<&str>,
        mtime: Option<i64>,
        outgoing: &[Edge],
        headings: &[markdown::Heading],
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let basename = std::path::Path::new(rel)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(rel);
        tx.execute(
            "INSERT OR REPLACE INTO nodes(rel_path, kind, mtime, title, basename) \
             VALUES (?, 'file', ?, ?, ?)",
            params![rel, mtime, title, basename],
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

    /// Link-autocomplete lookup. Drives the `[[` typeahead in the
    /// editor: the user types a fragment and gets back files and
    /// headings to link to.
    ///
    /// Empty `q`: most-recently-edited files first, up to `limit`.
    /// Useful as the initial state of the picker before any keystroke.
    ///
    /// Non-empty `q`: four-tier ASCII case-insensitive match.
    ///
    ///   rank 1  basename starts with q   (e.g. "carb" -> "carbonara.md")
    ///   rank 2  basename contains q      (e.g. "bona" -> "carbonara.md")
    ///   rank 3  title contains q         (e.g. "pasta" -> "2026-05-06.md"
    ///                                     when its h1 is "Pasta night")
    ///   rank 4  heading text contains q  (anchor target inside a file)
    ///
    /// Within a rank, files sort by `mtime DESC NULLS LAST, rel_path
    /// ASC`; headings sort by `rel_path, ord`. Heading hits are
    /// capped at half the limit so a single noisy file with many
    /// heading hits cannot drown out file matches.
    ///
    /// LIKE wildcards (`%`, `_`) and the escape char (`\`) in `q`
    /// are escaped so a filename "100%off.md" is never matched by a
    /// raw "%" query. Case-folding is ASCII-only (SQLite's `LOWER`);
    /// non-ASCII queries match case-sensitively. Sufficient for v1
    /// English/Western notes; revisit when a real Unicode-aware
    /// backend becomes a priority.
    pub fn link_targets(&self, q: &str, limit: u32) -> Result<Vec<LinkTarget>> {
        let limit = if limit == 0 { 50 } else { limit } as i64;
        let conn = self.conn.lock().unwrap();
        if q.is_empty() {
            return recent_files(&conn, limit);
        }
        let escaped = like_escape(q);
        let prefix_pat = format!("{escaped}%");
        let contains_pat = format!("%{escaped}%");

        let mut out = Vec::new();
        // Files: ranks 1-3.
        let mut stmt = conn.prepare_cached(
            "SELECT rel_path, title, mtime, rank FROM ( \
                SELECT rel_path, title, mtime, \
                    CASE \
                        WHEN LOWER(IFNULL(basename, rel_path)) LIKE LOWER(?1) ESCAPE '\\' THEN 1 \
                        WHEN LOWER(IFNULL(basename, rel_path)) LIKE LOWER(?2) ESCAPE '\\' THEN 2 \
                        WHEN title IS NOT NULL AND LOWER(title) LIKE LOWER(?2) ESCAPE '\\' THEN 3 \
                        ELSE 99 \
                    END AS rank \
                FROM nodes WHERE kind = 'file' \
             ) WHERE rank < 99 \
             ORDER BY rank ASC, mtime DESC, rel_path ASC \
             LIMIT ?3",
        )?;
        let rows = stmt.query_map(params![prefix_pat, contains_pat, limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<i64>>(2)?,
            ))
        })?;
        for row in rows {
            let (rel_path, title, mtime) = row?;
            out.push(LinkTarget {
                kind: LinkTargetKind::File,
                path: rel_path,
                title,
                heading: None,
                anchor: None,
                level: None,
                mtime,
            });
        }

        // Headings: rank 4. Capped at limit/2 so a single TOC-heavy
        // file doesn't drown out file matches.
        let heading_cap = (limit / 2).max(1);
        let mut stmt = conn.prepare_cached(
            "SELECT rel_path, level, text, anchor \
             FROM headings \
             WHERE LOWER(text) LIKE LOWER(?1) ESCAPE '\\' \
             ORDER BY rel_path ASC, ord ASC \
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![contains_pat, heading_cap], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)? as u8,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        for row in rows {
            let (rel_path, level, text, anchor) = row?;
            out.push(LinkTarget {
                kind: LinkTargetKind::Heading,
                path: rel_path,
                title: None,
                heading: Some(text),
                anchor: Some(anchor),
                level: Some(level),
                mtime: None,
            });
        }
        if out.len() > limit as usize {
            out.truncate(limit as usize);
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

/// Whether a `LinkTarget` row represents a file or a heading inside
/// a file. The fields populated on `LinkTarget` depend on this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkTargetKind {
    /// A file. `path` + `title` + `mtime` are populated; the
    /// heading-specific fields are None.
    File,
    /// A heading inside a file. `path` is the file's rel_path;
    /// `heading` / `anchor` / `level` describe the heading.
    Heading,
}

/// One row in `link_targets` output. Owned strings + primitives so
/// the type round-trips cleanly through uniffi later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkTarget {
    pub kind: LinkTargetKind,
    /// Rel path of the file (for both kinds).
    pub path: String,
    /// File's display title (h1 / frontmatter `title`). None for
    /// heading rows or for files whose title hasn't been indexed yet
    /// (the v2 graph migration leaves title NULL until the next
    /// index_file pass).
    pub title: Option<String>,
    /// Heading text. None for file rows.
    pub heading: Option<String>,
    /// Heading anchor (the URL fragment after `#`). None for files.
    pub anchor: Option<String>,
    /// Heading depth (1..=6). None for files.
    pub level: Option<u8>,
    /// File mtime as Unix seconds. None for headings.
    pub mtime: Option<i64>,
}

/// Most-recently-edited files first, up to `limit`. Used as the
/// empty-query state of the link picker.
fn recent_files(conn: &Connection, limit: i64) -> Result<Vec<LinkTarget>> {
    let mut stmt = conn.prepare_cached(
        "SELECT rel_path, title, mtime FROM nodes \
         WHERE kind = 'file' \
         ORDER BY mtime DESC, rel_path ASC \
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<i64>>(2)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (rel_path, title, mtime) = row?;
        out.push(LinkTarget {
            kind: LinkTargetKind::File,
            path: rel_path,
            title,
            heading: None,
            anchor: None,
            level: None,
            mtime,
        });
    }
    Ok(out)
}

/// Escape SQL LIKE wildcards (`%`, `_`) and the escape character
/// (`\`) so user-typed queries are matched literally. Pair with
/// `ESCAPE '\\'` in the SQL.
fn like_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if matches!(ch, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
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
        assert_eq!(count(&g, "PRAGMA user_version"), 2);
    }

    #[test]
    fn replace_then_forget_round_trips() {
        let tmp = TempDir::new().unwrap();
        let db = tmp.path().join("graph.sqlite");
        let g = GraphView::open(&db).unwrap();
        g.replace_file("notes/a.md", Some("Hello"), Some(1000), &[], &[])
            .unwrap();
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

    fn populate(g: &GraphView, files: &[(&str, Option<&str>, Option<i64>)]) {
        for (rel, title, mtime) in files {
            g.replace_file(rel, *title, *mtime, &[], &[]).unwrap();
        }
    }

    #[test]
    fn link_targets_ranks_prefix_above_substring() {
        let tmp = TempDir::new().unwrap();
        let g = GraphView::open(&tmp.path().join("g.sqlite")).unwrap();
        populate(
            &g,
            &[
                ("recipes/carbonara.md", Some("Carbonara"), Some(2000)),
                ("notes/oscarboxing.md", Some("Oscar Boxing"), Some(3000)),
                ("notes/title-only.md", Some("Carbon dating"), Some(4000)),
            ],
        );
        let hits = g.link_targets("carb", 10).unwrap();
        let names: Vec<_> = hits.iter().map(|h| h.path.as_str()).collect();
        // rank-1 (basename starts with "carb") must come before
        // rank-2 (basename contains "carb") which comes before
        // rank-3 (title contains "carb").
        assert_eq!(
            names,
            vec![
                "recipes/carbonara.md",
                "notes/oscarboxing.md",
                "notes/title-only.md",
            ]
        );
    }

    #[test]
    fn link_targets_empty_query_returns_recent() {
        let tmp = TempDir::new().unwrap();
        let g = GraphView::open(&tmp.path().join("g.sqlite")).unwrap();
        populate(
            &g,
            &[
                ("a.md", None, Some(1000)),
                ("b.md", None, Some(3000)),
                ("c.md", None, Some(2000)),
            ],
        );
        let hits = g.link_targets("", 10).unwrap();
        let names: Vec<_> = hits.iter().map(|h| h.path.as_str()).collect();
        assert_eq!(names, vec!["b.md", "c.md", "a.md"]);
    }

    #[test]
    fn link_targets_finds_headings() {
        let tmp = TempDir::new().unwrap();
        let g = GraphView::open(&tmp.path().join("g.sqlite")).unwrap();
        let headings = vec![
            markdown::Heading {
                ord: 0,
                line: 0,
                level: 1,
                text: "Pasta night".into(),
            },
            markdown::Heading {
                ord: 1,
                line: 5,
                level: 2,
                text: "Carbonara variant".into(),
            },
        ];
        g.replace_file(
            "notes/2026-05-06.md",
            Some("Pasta night"),
            Some(1),
            &[],
            &headings,
        )
        .unwrap();
        let hits = g.link_targets("variant", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].kind, LinkTargetKind::Heading);
        assert_eq!(hits[0].heading.as_deref(), Some("Carbonara variant"));
        // Anchor is the slug of the heading text.
        assert_eq!(hits[0].anchor.as_deref(), Some("carbonara-variant"));
        assert_eq!(hits[0].level, Some(2));
    }

    #[test]
    fn link_targets_is_ascii_case_insensitive() {
        let tmp = TempDir::new().unwrap();
        let g = GraphView::open(&tmp.path().join("g.sqlite")).unwrap();
        populate(&g, &[("Recipes/Carbonara.md", Some("Carbonara"), Some(1))]);
        let hits = g.link_targets("CARB", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, "Recipes/Carbonara.md");
    }

    #[test]
    fn link_targets_escapes_like_wildcards() {
        let tmp = TempDir::new().unwrap();
        let g = GraphView::open(&tmp.path().join("g.sqlite")).unwrap();
        populate(
            &g,
            &[("100off.md", None, Some(2)), ("100%off.md", None, Some(1))],
        );
        // A bare "%" in the query must match literally, not act as
        // a SQL wildcard. Only "100%off.md" should match.
        let hits = g.link_targets("%", 10).unwrap();
        let names: Vec<_> = hits.iter().map(|h| h.path.as_str()).collect();
        assert_eq!(names, vec!["100%off.md"]);
    }

    #[test]
    fn link_targets_caps_heading_results_at_half_limit() {
        let tmp = TempDir::new().unwrap();
        let g = GraphView::open(&tmp.path().join("g.sqlite")).unwrap();
        let mut hs = Vec::new();
        for i in 0..20 {
            hs.push(markdown::Heading {
                ord: i,
                line: i,
                level: 2,
                text: format!("note-{i}"),
            });
        }
        g.replace_file("toc-heavy.md", None, Some(1), &[], &hs)
            .unwrap();
        let hits = g.link_targets("note", 10).unwrap();
        // Cap at limit/2 = 5 heading hits, regardless of how many
        // headings actually match.
        let heading_hits = hits
            .iter()
            .filter(|h| h.kind == LinkTargetKind::Heading)
            .count();
        assert_eq!(heading_hits, 5);
    }

    #[test]
    fn migration_v2_backfills_basename_for_existing_nodes() {
        // Open at v1, insert a row by hand (simulating an old DB),
        // then re-open: migration must populate basename without
        // touching title (which we don't have at migration time).
        let tmp = TempDir::new().unwrap();
        let db = tmp.path().join("g.sqlite");
        {
            let conn = rusqlite::Connection::open(&db).unwrap();
            conn.execute_batch(
                r#"
                CREATE TABLE nodes (
                    rel_path TEXT PRIMARY KEY,
                    kind     TEXT NOT NULL,
                    mtime    INTEGER
                );
                CREATE TABLE edges (
                    src    TEXT NOT NULL,
                    dst    TEXT NOT NULL,
                    kind   TEXT NOT NULL,
                    anchor TEXT,
                    PRIMARY KEY (src, dst, kind)
                );
                CREATE TABLE headings (
                    rel_path TEXT NOT NULL,
                    level    INTEGER NOT NULL,
                    text     TEXT NOT NULL,
                    anchor   TEXT NOT NULL,
                    ord      INTEGER NOT NULL,
                    PRIMARY KEY (rel_path, ord)
                );
                INSERT INTO nodes(rel_path, kind, mtime)
                VALUES ('legacy/note.md', 'file', 1000);
                PRAGMA user_version = 1;
                "#,
            )
            .unwrap();
        }
        let g = GraphView::open(&db).unwrap();
        let conn = g.conn.lock().unwrap();
        let bn: Option<String> = conn
            .query_row(
                "SELECT basename FROM nodes WHERE rel_path='legacy/note.md'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(bn.as_deref(), Some("note.md"));
        let title: Option<String> = conn
            .query_row(
                "SELECT title FROM nodes WHERE rel_path='legacy/note.md'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            title.is_none(),
            "title backfill needs file content; stays NULL"
        );
        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 2);
    }
}
