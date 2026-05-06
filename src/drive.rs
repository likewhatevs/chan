// Drive: a registered directory exposed as a sandboxed filesystem
// plus search and graph. All I/O routes through `resolve_safe` and
// the editable-text gate. Per-drive state (index, graph, sessions,
// assistant history) lives outside the user's notes tree, keyed by
// the canonical drive path.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::{ChanError, Result};
use crate::fs_ops;
use crate::graph::GraphView;
use crate::lock::DriveLock;
use crate::markdown;
use crate::paths::{drive_paths, DrivePaths};
use crate::registry::KnownDrive;
use crate::search::{Index, IndexDoc, SearchOpts, SearchResults};
use crate::watch::{WatchCallback, WatchHandle};

pub use fs_ops::TreeEntry;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStat {
    pub size: u64,
    pub mtime: Option<i64>,
    pub is_dir: bool,
}

/// One open drive. Holds the writer lock for as long as it lives,
/// so two processes can't both write the same drive's index/graph.
/// Cheap reads are unlocked; writes go through the locked handle.
pub struct Drive {
    entry: KnownDrive,
    paths: DrivePaths,
    /// Held for the lifetime of the Drive. Released on drop.
    _lock: DriveLock,
    /// Lazily constructed; held in an Option so the field can be
    /// observed via `index()` / `graph()` accessors that initialize
    /// on first call.
    index: std::sync::OnceLock<Index>,
    graph: std::sync::OnceLock<GraphView>,
}

impl std::fmt::Debug for Drive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Drive")
            .field("root", &self.entry.path)
            .field("name", &self.entry.name)
            .finish()
    }
}

impl Drive {
    pub(crate) fn open(entry: KnownDrive) -> Result<Arc<Self>> {
        if !entry.path.exists() {
            return Err(ChanError::DriveRootMissing(entry.path.clone()));
        }
        let paths = drive_paths(&entry.path);
        let lock = DriveLock::acquire(&paths.lock)?;
        Ok(Arc::new(Self {
            entry,
            paths,
            _lock: lock,
            index: std::sync::OnceLock::new(),
            graph: std::sync::OnceLock::new(),
        }))
    }

    pub fn root(&self) -> &std::path::Path {
        &self.entry.path
    }

    pub fn name(&self) -> Option<&str> {
        self.entry.name.as_deref()
    }

    /// Per-drive paths (sessions, assistant history, index dir,
    /// graph DB, lock). Exposed for apps that want to put their
    /// own state alongside chan-core's.
    pub fn paths(&self) -> &DrivePaths {
        &self.paths
    }

    // ---- filesystem primitives (path-based, rel-only) ----

    /// Read raw bytes from a file relative to the drive root. No
    /// editable-text gate: callers like image previews need binary
    /// reads.
    pub fn read(&self, rel: &str) -> Result<Vec<u8>> {
        let abs = fs_ops::resolve_safe(self.root(), rel)?;
        Ok(std::fs::read(&abs)?)
    }

    /// Read UTF-8 text. Errors if the file isn't on the editable-
    /// text whitelist.
    pub fn read_text(&self, rel: &str) -> Result<String> {
        if !fs_ops::is_editable_text(rel) {
            return Err(ChanError::NotEditableText(rel.to_string()));
        }
        let abs = fs_ops::resolve_safe(self.root(), rel)?;
        Ok(std::fs::read_to_string(&abs)?)
    }

    /// Atomically write UTF-8 text. Editable-text gate applies.
    pub fn write_text(&self, rel: &str, content: &str) -> Result<()> {
        if !fs_ops::is_editable_text(rel) {
            return Err(ChanError::NotEditableText(rel.to_string()));
        }
        let abs = fs_ops::resolve_safe(self.root(), rel)?;
        fs_ops::atomic_write(&abs, content.as_bytes())
    }

    /// Atomically write raw bytes. NOT gated by editable-text;
    /// used by attachments and the future media browser. Callers
    /// that surface this to the editor must apply their own gate.
    pub fn write_bytes(&self, rel: &str, content: &[u8]) -> Result<()> {
        let abs = fs_ops::resolve_safe(self.root(), rel)?;
        fs_ops::atomic_write(&abs, content)
    }

    pub fn exists(&self, rel: &str) -> bool {
        fs_ops::resolve_safe(self.root(), rel)
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    pub fn stat(&self, rel: &str) -> Result<FileStat> {
        let abs = fs_ops::resolve_safe(self.root(), rel)?;
        let meta = std::fs::metadata(&abs)?;
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64);
        Ok(FileStat {
            size: if meta.is_dir() { 0 } else { meta.len() },
            mtime,
            is_dir: meta.is_dir(),
        })
    }

    /// One-level directory listing. Use `list_tree` for the
    /// recursive variant.
    pub fn list(&self, rel: &str) -> Result<Vec<DirEntry>> {
        let abs = if rel.is_empty() || rel == "." || rel == "/" {
            self.root().to_path_buf()
        } else {
            fs_ops::resolve_safe(self.root(), rel)?
        };
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&abs)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            // Filter at the top level of the drive only; deeper
            // listings return whatever's there.
            if abs == self.root() && (name == ".chan" || name == ".git") {
                continue;
            }
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            out.push(DirEntry { name, is_dir });
        }
        Ok(out)
    }

    pub fn list_tree(&self) -> Result<Vec<TreeEntry>> {
        fs_ops::list_tree(self.root())
    }

    pub fn create_dir(&self, rel: &str) -> Result<()> {
        let abs = fs_ops::resolve_safe(self.root(), rel)?;
        std::fs::create_dir_all(&abs)?;
        Ok(())
    }

    /// Remove a file or empty directory. For non-empty directory
    /// removal, callers must walk and delete explicitly; chan-core
    /// won't recursive-delete on behalf of the user (foot-gun guard).
    pub fn remove(&self, rel: &str) -> Result<()> {
        let abs = fs_ops::resolve_safe(self.root(), rel)?;
        let meta = std::fs::metadata(&abs)?;
        if meta.is_dir() {
            std::fs::remove_dir(&abs)?;
        } else {
            std::fs::remove_file(&abs)?;
        }
        Ok(())
    }

    pub fn rename(&self, from: &str, to: &str) -> Result<()> {
        let from_abs = fs_ops::resolve_safe(self.root(), from)?;
        let to_abs = fs_ops::resolve_safe(self.root(), to)?;
        if let Some(parent) = to_abs.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(&from_abs, &to_abs)?;
        Ok(())
    }

    // ---- search ----

    /// Run a search query against this drive. The first call
    /// initializes the index (creating it if needed); subsequent
    /// calls reuse the same handle.
    pub fn search(&self, query: &str, opts: &SearchOpts) -> Result<SearchResults> {
        self.index()?.search(query, opts)
    }

    /// Re-index the whole drive from scratch: walks the tree,
    /// parses every editable-text file, and rebuilds both the
    /// search index and the graph DB. Synchronous and blocking;
    /// the caller decides whether to spawn a worker.
    pub fn reindex(&self) -> Result<crate::search::IndexStats> {
        let entries = self.list_tree()?;
        let mut docs = Vec::new();
        let mut skipped = 0u32;
        let graph = self.graph()?;
        // Wipe the graph for a clean rebuild. We rely on the per-
        // drive lock + the Mutex<Connection> inside GraphView to
        // serialize; no other writer can race us.
        graph.clear()?;
        for e in &entries {
            if e.is_dir {
                continue;
            }
            if !fs_ops::is_editable_text(&e.path) {
                skipped += 1;
                continue;
            }
            let content = match self.read_text(&e.path) {
                Ok(s) => s,
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            };
            let (doc, headings, edges) = parse_for_index(&e.path, &content, e.mtime);
            docs.push(doc);
            graph.replace_file(&e.path, e.mtime, &edges, &headings)?;
        }
        let mut stats = self.index()?.reindex_iter(docs)?;
        stats.files_skipped = skipped;
        Ok(stats)
    }

    /// Re-index a single file. Reads, parses, updates the search
    /// index and graph for just this path. Used by the watcher
    /// consumer when a file changes.
    pub fn index_file(&self, rel: &str) -> Result<()> {
        if !fs_ops::is_editable_text(rel) {
            return Ok(());
        }
        let content = self.read_text(rel)?;
        let mtime = self.stat(rel).ok().and_then(|s| s.mtime);
        let (doc, headings, edges) = parse_for_index(rel, &content, mtime);
        self.index()?.upsert(&doc)?;
        self.graph()?.replace_file(rel, mtime, &edges, &headings)?;
        Ok(())
    }

    /// Drop a single file from the search index and graph. Used
    /// when the watcher reports a deletion.
    pub fn forget_file(&self, rel: &str) -> Result<()> {
        self.index()?.remove(rel)?;
        self.graph()?.forget_file(rel)?;
        Ok(())
    }

    fn index(&self) -> Result<&Index> {
        if let Some(idx) = self.index.get() {
            return Ok(idx);
        }
        let idx = Index::open(self.root(), &self.paths.index)?;
        let _ = self.index.set(idx);
        Ok(self.index.get().unwrap())
    }

    // ---- graph ----

    /// View into the drive's graph DB.
    pub fn graph(&self) -> Result<&GraphView> {
        if let Some(g) = self.graph.get() {
            return Ok(g);
        }
        let g = GraphView::open(&self.paths.graph_db)?;
        let _ = self.graph.set(g);
        Ok(self.graph.get().unwrap())
    }

    // ---- watch ----

    /// Start a recursive filesystem watcher on the drive. Drop
    /// the returned `WatchHandle` to stop. Events for `.chan/`
    /// and `.git/` are filtered out.
    pub fn watch(self: &Arc<Self>, cb: Arc<dyn WatchCallback>) -> Result<WatchHandle> {
        WatchHandle::start(self.root(), cb)
    }
}

/// Parse a file's content into the structures the search index
/// and graph need: an `IndexDoc` (for tantivy), the heading list
/// (for graph::headings), and the outgoing edges (links + tokens).
fn parse_for_index(
    rel: &str,
    raw: &str,
    mtime: Option<i64>,
) -> (IndexDoc, Vec<markdown::Heading>, Vec<crate::graph::Edge>) {
    let fm = markdown::parse_frontmatter(raw);
    let body_src = &raw[fm.body_offset..];
    let headings = markdown::parse_headings(body_src);
    let title = fm
        .data
        .get("title")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .or_else(|| {
            headings
                .iter()
                .find(|h| h.level == 1)
                .map(|h| h.text.clone())
        });
    let links = markdown::extract_links(body_src);
    let tokens = markdown::extract_tokens(body_src);
    let edges = build_edges(rel, &links, &tokens);

    let doc = IndexDoc {
        path: rel.to_string(),
        title,
        body: body_src.to_string(),
        mtime,
    };
    (doc, headings, edges)
}

/// Convert links + tokens into graph edges. Wiki links and
/// internal markdown links produce `Link` edges; tokens produce
/// `Tag` / `Mention` edges. External links (http://, mailto:) are
/// dropped because they don't connect to anything else in the
/// drive's graph.
fn build_edges(
    src: &str,
    links: &[markdown::Link],
    tokens: &[markdown::Token],
) -> Vec<crate::graph::Edge> {
    use crate::graph::{Edge, EdgeKind};
    let mut out = Vec::new();
    for l in links {
        if !l.is_internal() {
            continue;
        }
        let (target, anchor) = split_anchor(&l.target);
        out.push(Edge {
            src: src.to_string(),
            dst: target,
            kind: EdgeKind::Link,
            anchor,
        });
    }
    for t in tokens {
        match t {
            markdown::Token::Tag { name } => out.push(Edge {
                src: src.to_string(),
                dst: format!("#{name}"),
                kind: EdgeKind::Tag,
                anchor: None,
            }),
            markdown::Token::Mention { name } => out.push(Edge {
                src: src.to_string(),
                dst: format!("@@{name}"),
                kind: EdgeKind::Mention,
                anchor: None,
            }),
            // Dates aren't graph edges yet; the graph view groups
            // files by date through a future query rather than a
            // stored edge. Skip for now.
            markdown::Token::Date { .. } => {}
        }
    }
    out
}

/// Split a link target into (path, anchor). `path#section` becomes
/// `("path", Some("section"))`; a target without `#` returns
/// `(target, None)`.
fn split_anchor(target: &str) -> (String, Option<String>) {
    match target.split_once('#') {
        Some((p, a)) if !a.is_empty() => (p.to_string(), Some(a.to_string())),
        _ => (target.to_string(), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Library;
    use tempfile::TempDir;

    fn fixture() -> (TempDir, TempDir, Arc<Drive>) {
        let cfg = TempDir::new().unwrap();
        let drive_dir = TempDir::new().unwrap();
        let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_drive(drive_dir.path(), Some("Test".into()))
            .unwrap();
        let drive = lib.open_drive(drive_dir.path()).unwrap();
        (cfg, drive_dir, drive)
    }

    #[test]
    fn write_then_read_text_round_trips() {
        let (_cfg, _root, drive) = fixture();
        drive.write_text("notes/a.md", "hello").unwrap();
        assert_eq!(drive.read_text("notes/a.md").unwrap(), "hello");
    }

    #[test]
    fn write_text_rejects_non_text_extensions() {
        let (_cfg, _root, drive) = fixture();
        let err = drive.write_text("img.png", "x").unwrap_err();
        assert!(matches!(err, ChanError::NotEditableText(_)));
    }

    #[test]
    fn write_bytes_allows_binary() {
        let (_cfg, _root, drive) = fixture();
        drive.write_bytes("img.png", &[0xff, 0xd8, 0xff]).unwrap();
        assert_eq!(drive.read("img.png").unwrap(), vec![0xff, 0xd8, 0xff]);
    }

    #[test]
    fn list_skips_chan_and_git_at_top_level() {
        let (_cfg, root, drive) = fixture();
        std::fs::create_dir_all(root.path().join(".chan")).unwrap();
        std::fs::create_dir_all(root.path().join(".git")).unwrap();
        std::fs::write(root.path().join("note.md"), "hi").unwrap();
        let entries = drive.list("").unwrap();
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"note.md"));
        assert!(!names.contains(&".chan"));
        assert!(!names.contains(&".git"));
    }

    #[test]
    fn rename_moves_file() {
        let (_cfg, _root, drive) = fixture();
        drive.write_text("a.md", "x").unwrap();
        drive.rename("a.md", "b/c.md").unwrap();
        assert!(!drive.exists("a.md"));
        assert!(drive.exists("b/c.md"));
    }

    #[test]
    fn second_open_blocks_on_writer_lock() {
        let (cfg, root, _drive) = fixture();
        let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
        let err = lib.open_drive(root.path()).unwrap_err();
        assert!(matches!(err, ChanError::DriveLocked));
    }

    #[test]
    fn graph_opens_lazily() {
        let (_cfg, _root, drive) = fixture();
        // Calling graph() twice returns the same handle; this is
        // the contract the editor relies on for incremental
        // updates from the watcher.
        let _g1 = drive.graph().unwrap();
        let _g2 = drive.graph().unwrap();
    }
}
