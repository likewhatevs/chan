// Drive: a registered directory exposed as a sandboxed filesystem
// plus search and graph. All I/O routes through `resolve_safe` and
// the editable-text gate. Per-drive state (index, graph, sessions,
// assistant history) lives outside the user's notes tree, keyed by
// the canonical drive path.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::{ChanError, Result};
use crate::fs_ops;
use crate::graph::GraphView;
use crate::index::{BuildOptions, BuildSummary, Index, Mode as SearchMode, SearchResult};
use crate::lock::DriveLock;
use crate::markdown;
use crate::paths::{drive_paths, DrivePaths};
use crate::registry::KnownDrive;
use crate::trash::{self, TrashEntry, TRASH_RETENTION_SECS};
use crate::watch::{WatchCallback, WatchHandle};

/// User-facing search knobs. The mode defaults to Hybrid (BM25 +
/// dense, RRF-fused) when the binary is built with `embeddings`,
/// otherwise the facade falls back to BM25 with `ready: false`.
#[derive(Debug, Clone, Default)]
pub struct SearchOpts {
    pub mode: SearchMode,
    /// Hard cap on results returned. Defaults to 50 when 0.
    pub limit: u32,
    /// Optional subdir scope (relative to drive root). When set,
    /// only paths under this prefix are returned. None = whole
    /// drive. Filtering is post-rank: the index doesn't track
    /// scope, the Drive does.
    pub scope: Option<String>,
}

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

/// A wiki-link resolved to an actual drive file. `path` is the
/// POSIX rel path of the file that exists today; `anchor` is the
/// `#section` fragment from the original target, passed through
/// unchanged. See `Drive::resolve_link` for the resolution rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedLink {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
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
        // Lazy GC: reclaim expired trash entries on every open. No
        // background thread, matches the codebase's sync-only rule.
        // Errors are swallowed: a corrupt trash dir must never block
        // a legitimate drive open.
        let _ = trash::sweep_expired(&paths.trash, TRASH_RETENTION_SECS);
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
    /// own state alongside chan-drive's.
    pub fn paths(&self) -> &DrivePaths {
        &self.paths
    }

    // ---- filesystem primitives (path-based, rel-only) ----
    //
    // Every entry point here goes through `resolve_safe_strict`
    // (lexical sandbox + canonical-form check that the deepest
    // existing ancestor stays under the drive root). Reads
    // additionally call `ensure_regular_file` (lstat-based) so we
    // never block on a FIFO, drain a device, or follow a symlink
    // off the drive. Writes that target an existing path do the
    // same check; writes to a fresh path skip it because there's
    // nothing to inspect yet (the strict resolve already guarded
    // the parent).

    /// Read raw bytes from a file relative to the drive root. No
    /// editable-text gate: callers like image previews need binary
    /// reads. The path must resolve to a regular file under the
    /// drive root; symlinks, FIFOs, sockets, and devices are
    /// rejected.
    pub fn read(&self, rel: &str) -> Result<Vec<u8>> {
        let abs = fs_ops::resolve_safe_strict(self.root(), rel)?;
        fs_ops::ensure_regular_file(&abs)?;
        Ok(std::fs::read(&abs)?)
    }

    /// Read UTF-8 text. Errors if the file isn't on the editable-
    /// text whitelist or isn't a regular file.
    pub fn read_text(&self, rel: &str) -> Result<String> {
        if !fs_ops::is_editable_text(rel) {
            return Err(ChanError::NotEditableText(rel.to_string()));
        }
        let abs = fs_ops::resolve_safe_strict(self.root(), rel)?;
        fs_ops::ensure_regular_file(&abs)?;
        Ok(std::fs::read_to_string(&abs)?)
    }

    /// Read UTF-8 text and return the file's stat alongside the
    /// content. The stat is taken from the open file handle (fstat)
    /// so the returned mtime corresponds to the bytes returned, with
    /// no second-syscall race window. Pair with `write_text_if_unchanged`
    /// for optimistic-concurrency editor saves.
    pub fn read_text_with_stat(&self, rel: &str) -> Result<(String, FileStat)> {
        use std::io::Read;
        if !fs_ops::is_editable_text(rel) {
            return Err(ChanError::NotEditableText(rel.to_string()));
        }
        let abs = fs_ops::resolve_safe_strict(self.root(), rel)?;
        // lstat-gate before opening: File::open follows symlinks, so
        // refuse if the path is a symlink / FIFO / device.
        fs_ops::ensure_regular_file(&abs)?;
        let mut f = std::fs::File::open(&abs)?;
        let meta = f.metadata()?;
        let mut content = String::new();
        f.read_to_string(&mut content)?;
        let stat = FileStat {
            size: meta.len(),
            mtime: mtime_secs(&meta),
            is_dir: false,
        };
        Ok((content, stat))
    }

    /// Atomically write UTF-8 text. Editable-text gate applies.
    /// Refuses to write through a path whose final component is a
    /// non-regular file (symlink, device, FIFO, socket); the user
    /// must remove the existing entry first if they intend to
    /// replace it.
    pub fn write_text(&self, rel: &str, content: &str) -> Result<()> {
        if !fs_ops::is_editable_text(rel) {
            return Err(ChanError::NotEditableText(rel.to_string()));
        }
        let abs = fs_ops::resolve_safe_strict(self.root(), rel)?;
        ensure_writable(&abs)?;
        fs_ops::atomic_write(&abs, content.as_bytes())
    }

    /// Optimistic-concurrency write: succeeds only when the file's
    /// current mtime matches `expected_mtime`. The editor pairs this
    /// with `read_text_with_stat`: it reads (content, stat), the user
    /// edits, then it writes back with `expected_mtime = stat.mtime`.
    /// If the file changed under the editor (another process, another
    /// pane), the write fails with `ChanError::WriteConflict` and the
    /// editor can prompt to reload, merge, or overwrite.
    ///
    /// Conventions for `expected_mtime`:
    ///   - `None` + missing file: create.
    ///   - `None` + existing file: `WriteConflict`. The caller did
    ///     not know a file was there; treating that as a silent
    ///     overwrite would be the bug we're trying to prevent.
    ///   - `Some(m)` + current mtime == m: write.
    ///   - any other case: `WriteConflict { current_mtime }`.
    ///
    /// Residual race: between the mtime check and the atomic rename,
    /// another writer can land. The window is small (no syscalls
    /// between the two) and the next watcher event will surface the
    /// foreign change so the editor can re-prompt. Callers that need
    /// stronger semantics must serialize at a higher level.
    pub fn write_text_if_unchanged(
        &self,
        rel: &str,
        expected_mtime: Option<i64>,
        content: &str,
    ) -> Result<()> {
        if !fs_ops::is_editable_text(rel) {
            return Err(ChanError::NotEditableText(rel.to_string()));
        }
        let abs = fs_ops::resolve_safe_strict(self.root(), rel)?;
        ensure_writable(&abs)?;
        let (current, exists) = match std::fs::symlink_metadata(&abs) {
            Ok(meta) => (mtime_secs(&meta), true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (None, false),
            Err(e) => return Err(ChanError::Io(e.to_string())),
        };
        let conflict = match (expected_mtime, exists) {
            (None, false) => false,
            (Some(m), true) => current != Some(m),
            _ => true,
        };
        if conflict {
            return Err(ChanError::WriteConflict {
                current_mtime: current,
            });
        }
        fs_ops::atomic_write(&abs, content.as_bytes())
    }

    /// Atomically write raw bytes. NOT gated by editable-text;
    /// used by attachments and the future media browser. Callers
    /// that surface this to the editor must apply their own gate.
    /// Same special-file refusal as `write_text`.
    pub fn write_bytes(&self, rel: &str, content: &[u8]) -> Result<()> {
        let abs = fs_ops::resolve_safe_strict(self.root(), rel)?;
        ensure_writable(&abs)?;
        fs_ops::atomic_write(&abs, content)
    }

    /// True iff the path resolves under the drive and refers to a
    /// regular file. Matches the gate `read` / `read_text` apply,
    /// so a `true` return is a strong signal that a read will
    /// succeed.
    pub fn exists(&self, rel: &str) -> bool {
        let Ok(abs) = fs_ops::resolve_safe_strict(self.root(), rel) else {
            return false;
        };
        std::fs::symlink_metadata(&abs)
            .map(|m| m.is_file() && !m.file_type().is_symlink())
            .unwrap_or(false)
    }

    /// Stat the path using `lstat` semantics (so a symlink reports
    /// as such, not as its target). Refuses paths that escape the
    /// drive root through a mid-path symlink.
    pub fn stat(&self, rel: &str) -> Result<FileStat> {
        let abs = fs_ops::resolve_safe_strict(self.root(), rel)?;
        let meta = std::fs::symlink_metadata(&abs)?;
        Ok(FileStat {
            size: if meta.is_dir() { 0 } else { meta.len() },
            mtime: mtime_secs(&meta),
            is_dir: meta.is_dir(),
        })
    }

    /// One-level directory listing. Use `list_tree` for the
    /// recursive variant. Skips drive-internal noise (`.chan/`,
    /// `.git/`) at the top level and drops non-regular non-dir
    /// entries (symlinks, FIFOs, sockets, devices) at every level.
    pub fn list(&self, rel: &str) -> Result<Vec<DirEntry>> {
        let abs = if rel.is_empty() || rel == "." || rel == "/" {
            self.root().to_path_buf()
        } else {
            fs_ops::resolve_safe_strict(self.root(), rel)?
        };
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&abs)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if abs == self.root() && (name == ".chan" || name == ".git") {
                continue;
            }
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            // Drop non-regular non-dir entries from the listing.
            // We could instead surface them with a marker, but
            // every consumer today treats them as junk.
            if !(ft.is_dir() || (ft.is_file() && !ft.is_symlink())) {
                continue;
            }
            out.push(DirEntry {
                name,
                is_dir: ft.is_dir(),
            });
        }
        Ok(out)
    }

    pub fn list_tree(&self) -> Result<Vec<TreeEntry>> {
        fs_ops::list_tree(self.root())
    }

    pub fn create_dir(&self, rel: &str) -> Result<()> {
        let abs = fs_ops::resolve_safe_strict(self.root(), rel)?;
        std::fs::create_dir_all(&abs)?;
        Ok(())
    }

    /// Soft-delete a file or directory: move it into the per-drive
    /// trash. `trash_list` / `trash_restore` / `trash_purge` /
    /// `trash_empty` operate on the trash. Expired entries are
    /// GC'd lazily on `Drive::open` and on every `trash_*` call;
    /// retention is `TRASH_RETENTION_SECS` (30 days at v1).
    ///
    /// Accepted: regular files and real directories (recursively;
    /// the foot-gun guard against recursive delete is satisfied by
    /// the soft-delete + restore path). Rejected with `SpecialFile`:
    /// symlinks, FIFOs, sockets, char/block devices. Users who
    /// really want those gone can `rm` them out-of-band.
    pub fn remove(&self, rel: &str) -> Result<()> {
        let abs = fs_ops::resolve_safe_strict(self.root(), rel)?;
        let meta = std::fs::symlink_metadata(&abs)?;
        let ft = meta.file_type();
        let is_dir = ft.is_dir();
        let is_regular_file = ft.is_file() && !ft.is_symlink();
        if !(is_dir || is_regular_file) {
            return Err(ChanError::SpecialFile {
                kind: fs_ops::describe_file_kind(&ft).to_string(),
                path: abs,
            });
        }
        let _ = trash::sweep_expired(&self.paths.trash, TRASH_RETENTION_SECS);
        trash::move_into(&self.paths.trash, &abs, rel, is_dir)
    }

    /// List trashed entries for this drive, most-recent-first.
    pub fn trash_list(&self) -> Result<Vec<TrashEntry>> {
        let _ = trash::sweep_expired(&self.paths.trash, TRASH_RETENTION_SECS);
        trash::list(&self.paths.trash)
    }

    /// Restore a trashed entry to its original path. Errors with
    /// `TrashOccupied` if the destination already exists; the caller
    /// can rename the live entry first or `trash_purge` the trash
    /// entry to give up.
    pub fn trash_restore(&self, id: &str) -> Result<()> {
        let _ = trash::sweep_expired(&self.paths.trash, TRASH_RETENTION_SECS);
        trash::restore(&self.paths.trash, self.root(), id)
    }

    /// Permanently delete a single trash entry.
    pub fn trash_purge(&self, id: &str) -> Result<()> {
        let _ = trash::sweep_expired(&self.paths.trash, TRASH_RETENTION_SECS);
        trash::purge_one(&self.paths.trash, id)
    }

    /// Permanently delete every trash entry for this drive.
    pub fn trash_empty(&self) -> Result<()> {
        trash::purge_all(&self.paths.trash)
    }

    // ---- session blobs ----
    //
    // Per-window opaque JSON owned by the host (window/pane
    // layout, active tabs, scroll positions). chan-drive stores
    // bytes; the host decides the schema. Native shells link these
    // via uniffi and avoid reimplementing the atomic-write story
    // per platform.

    /// Atomically write `content` to the session bucket under
    /// `key`. Bucket dir is created on first call.
    pub fn put_session(&self, key: &str, content: &[u8]) -> Result<()> {
        crate::blob::put(&self.paths.sessions, key, content)
    }

    /// Read a session blob; returns `Ok(None)` when missing.
    pub fn get_session(&self, key: &str) -> Result<Option<Vec<u8>>> {
        crate::blob::get(&self.paths.sessions, key)
    }

    /// Sorted flat session keys for this drive.
    pub fn list_sessions(&self) -> Result<Vec<String>> {
        crate::blob::list(&self.paths.sessions)
    }

    /// Idempotent delete; missing key is `Ok(())`.
    pub fn delete_session(&self, key: &str) -> Result<()> {
        crate::blob::delete(&self.paths.sessions, key)
    }

    // ---- assistant blobs ----
    //
    // Per-conversation opaque JSON (typically keyed by sha256 of
    // the related file's drive-relative path). Same shape as
    // sessions; separate bucket so listing one doesn't bleed the
    // other.

    /// Atomically write an assistant conversation blob.
    pub fn put_assistant(&self, key: &str, content: &[u8]) -> Result<()> {
        crate::blob::put(&self.paths.assistant, key, content)
    }

    pub fn get_assistant(&self, key: &str) -> Result<Option<Vec<u8>>> {
        crate::blob::get(&self.paths.assistant, key)
    }

    pub fn list_assistant(&self) -> Result<Vec<String>> {
        crate::blob::list(&self.paths.assistant)
    }

    pub fn delete_assistant(&self, key: &str) -> Result<()> {
        crate::blob::delete(&self.paths.assistant, key)
    }

    /// Wipe every assistant conversation for this drive (the
    /// `/clear` UX). Does not touch the search index; that comes
    /// when the assistant-content indexing piece lands.
    pub fn clear_assistant(&self) -> Result<()> {
        crate::blob::clear(&self.paths.assistant)
    }

    pub fn rename(&self, from: &str, to: &str) -> Result<()> {
        let from_abs = fs_ops::resolve_safe_strict(self.root(), from)?;
        let to_abs = fs_ops::resolve_safe_strict(self.root(), to)?;
        // Source must exist as a regular file or directory; refuse
        // to move a symlink or special file. (renaming a symlink
        // is well-defined at the syscall level but not something
        // the editor should ever do silently.)
        let src_meta = std::fs::symlink_metadata(&from_abs)?;
        let src_ft = src_meta.file_type();
        if !(src_ft.is_dir() || (src_ft.is_file() && !src_ft.is_symlink())) {
            return Err(ChanError::SpecialFile {
                kind: fs_ops::describe_file_kind(&src_ft).to_string(),
                path: from_abs,
            });
        }
        ensure_writable(&to_abs)?;
        if let Some(parent) = to_abs.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(&from_abs, &to_abs)?;
        Ok(())
    }

    // ---- search ----

    /// Run a search query against this drive. Routes through the
    /// hybrid index facade; opens the index lazily on first call.
    /// Scope filtering is applied post-rank: the index doesn't
    /// track scope, so a buffered top-N is fetched and pruned to
    /// the requested limit after the prefix check.
    pub fn search(&self, query: &str, opts: &SearchOpts) -> Result<SearchResult> {
        let limit = if opts.limit == 0 { 50 } else { opts.limit } as usize;
        let fetch = if opts.scope.is_some() {
            limit * 4
        } else {
            limit
        };
        let mut res = self.index()?.search(query, opts.mode, fetch)?;
        if let Some(scope) = &opts.scope {
            res.hits.retain(|h| path_under(&h.path, scope));
        }
        res.hits.truncate(limit);
        Ok(res)
    }

    /// Re-index the whole drive from scratch: walks the tree,
    /// parses every editable-text file, and rebuilds both the
    /// search index and the graph DB. Synchronous and blocking;
    /// the caller decides whether to spawn a worker. Returns the
    /// search-side build summary; graph-side errors short-circuit
    /// the rebuild.
    ///
    /// `cancel`: if set to true mid-build, the rebuild bails out
    /// with `ChanError::Cancelled`. The graph DB is cleared at the
    /// start so a cancelled rebuild leaves an empty graph (the next
    /// `index_stats()` reports `indexed_docs == 0`, which lets the
    /// server's auto-rebuild trigger re-fire on next boot). The BM25
    /// index is unaffected by cancellation: we never reach the
    /// commit, so tantivy discards every pending write.
    pub fn reindex(&self, cancel: Option<&AtomicBool>) -> Result<BuildSummary> {
        self.reindex_with(cancel, |_| {})
    }

    /// Same as `reindex`, but `on_progress` is called once per file
    /// during the search-side pass. The graph-side pass runs first
    /// without progress reporting (it's seconds even on big drives;
    /// the embedding pass is the long part). The CLI uses this to
    /// print live progress; the server's indexer uses the no-arg
    /// `reindex` because it tracks status separately.
    pub fn reindex_with<F>(
        &self,
        cancel: Option<&AtomicBool>,
        on_progress: F,
    ) -> Result<BuildSummary>
    where
        F: FnMut(crate::index::BuildProgress<'_>),
    {
        // Graph rebuild walks the tree once for headings + edges.
        // The search facade walks again for chunking + embeddings.
        // Two passes is the trade for a clean separation; per-file
        // I/O cost is trivial against the embedding work.
        self.rebuild_graph(cancel)?;
        let summary = self
            .index()?
            .build_all(BuildOptions::default(), on_progress, cancel)
            .map_err(|e| match e {
                crate::index::IndexError::Cancelled => ChanError::Cancelled,
                other => other.into(),
            })?;
        Ok(summary)
    }

    fn rebuild_graph(&self, cancel: Option<&AtomicBool>) -> Result<()> {
        let entries = self.list_tree()?;
        let graph = self.graph()?;
        graph.clear()?;
        for e in &entries {
            if let Some(c) = cancel {
                if c.load(Ordering::Relaxed) {
                    return Err(ChanError::Cancelled);
                }
            }
            if e.is_dir || !fs_ops::is_editable_text(&e.path) {
                continue;
            }
            let content = match self.read_text(&e.path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let (title, headings, edges) = parse_for_graph(&e.path, &content);
            graph.replace_file(&e.path, title.as_deref(), e.mtime, &edges, &headings)?;
        }
        Ok(())
    }

    /// How many BM25 chunks are currently in the search index.
    /// Lazily opens the index on first call.
    pub fn num_indexed(&self) -> Result<u64> {
        Ok(self.index()?.stats().indexed_docs)
    }

    /// Snapshot of the search index. Used by the server's status
    /// endpoint.
    pub fn index_stats(&self) -> Result<crate::index::IndexStats> {
        Ok(self.index()?.stats())
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
        let (title, headings, edges) = parse_for_graph(rel, &content);
        self.index()?.index_one(rel)?;
        self.graph()?
            .replace_file(rel, title.as_deref(), mtime, &edges, &headings)?;
        Ok(())
    }

    /// Link-autocomplete lookup. Pass-through to
    /// `GraphView::link_targets`. The editor's `[[` typeahead binds
    /// to this: an empty `q` returns recent files; a non-empty `q`
    /// returns ranked file + heading matches. See `GraphView::link_targets`
    /// for the ranking and case-folding rules.
    pub fn link_targets(&self, q: &str, limit: u32) -> Result<Vec<crate::graph::LinkTarget>> {
        self.graph()?.link_targets(q, limit)
    }

    /// Resolve a wiki-link target string to an existing drive
    /// file. The graph stores link dst nodes verbatim from
    /// markdown (e.g. `[[recipes/pasta]]` -> `dst="recipes/pasta"`),
    /// so backlinks queries match the stored form. Consumers that
    /// want to navigate to or read the actual file (the editor's
    /// click-on-link, the assistant's `read_file` tool when given
    /// a wiki target) call this to find the real path.
    ///
    /// Algorithm:
    ///   1. Split off `#anchor` (everything after the first `#`).
    ///      An empty anchor (target ends in `#`) becomes None.
    ///   2. Try `path.md`, then `path.txt`, then the exact `path`
    ///      (rare case: a file with no extension that matches by
    ///      name). Return the first hit as a regular file.
    ///   3. None when no candidate exists.
    ///
    /// Anchor strings are passed through unchanged; chan-drive
    /// doesn't validate them against the file's headings (callers
    /// can do that via `GraphView::headings_of`).
    pub fn resolve_link(&self, target: &str) -> Option<ResolvedLink> {
        if target.is_empty() {
            return None;
        }
        let (path, anchor) = match target.split_once('#') {
            Some((p, a)) if !a.is_empty() => (p, Some(a.to_string())),
            // Trailing `#` (empty anchor): strip it; path is the
            // prefix, anchor None.
            Some((p, _)) => (p, None),
            None => (target, None),
        };
        if path.is_empty() {
            return None;
        }
        for candidate in [
            format!("{path}.md"),
            format!("{path}.txt"),
            path.to_string(),
        ] {
            if self.exists(&candidate) {
                return Some(ResolvedLink {
                    path: candidate,
                    anchor,
                });
            }
        }
        None
    }

    /// Drop a single file from the search index and graph. Used
    /// when the watcher reports a deletion.
    pub fn forget_file(&self, rel: &str) -> Result<()> {
        self.index()?.forget(rel)?;
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

fn mtime_secs(meta: &std::fs::Metadata) -> Option<i64> {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
}

/// Refuse to write at `abs` if its final component already exists
/// as something other than a regular file (symlink, FIFO, socket,
/// device, directory). Returning Ok when the path is missing is
/// intentional: a fresh write is always safe; the strict resolve
/// already vetted the parent.
fn ensure_writable(abs: &std::path::Path) -> Result<()> {
    match std::fs::symlink_metadata(abs) {
        Ok(meta) => {
            let ft = meta.file_type();
            if ft.is_file() && !ft.is_symlink() {
                Ok(())
            } else {
                Err(ChanError::SpecialFile {
                    kind: fs_ops::describe_file_kind(&ft).to_string(),
                    path: abs.to_path_buf(),
                })
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(ChanError::Io(e.to_string())),
    }
}

/// Parse a file's content into the graph-side structures: the
/// title (for the graph node), the heading list (for graph
/// headings), and the outgoing edges (links + tokens). The
/// search-side chunking is done separately by the index facade.
fn parse_for_graph(
    rel: &str,
    raw: &str,
) -> (
    Option<String>,
    Vec<markdown::Heading>,
    Vec<crate::graph::Edge>,
) {
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
    (title, headings, edges)
}

/// Whether `path` lies under the `prefix` directory. POSIX
/// separators on both sides; case-sensitive. Used by the post-
/// filter for `SearchOpts::scope`.
fn path_under(path: &str, prefix: &str) -> bool {
    let prefix = prefix.trim_end_matches('/');
    if prefix.is_empty() {
        return true;
    }
    if path == prefix {
        return true;
    }
    let with_slash = format!("{prefix}/");
    path.starts_with(&with_slash)
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
    fn read_text_with_stat_returns_content_and_mtime() {
        let (_cfg, _root, drive) = fixture();
        drive.write_text("a.md", "hello").unwrap();
        let (content, stat) = drive.read_text_with_stat("a.md").unwrap();
        assert_eq!(content, "hello");
        assert_eq!(stat.size, 5);
        assert!(stat.mtime.is_some());
        assert!(!stat.is_dir);
    }

    #[test]
    fn write_text_if_unchanged_creates_when_missing_with_none() {
        let (_cfg, _root, drive) = fixture();
        drive.write_text_if_unchanged("a.md", None, "v1").unwrap();
        assert_eq!(drive.read_text("a.md").unwrap(), "v1");
    }

    #[test]
    fn write_text_if_unchanged_conflicts_when_none_but_file_exists() {
        let (_cfg, _root, drive) = fixture();
        drive.write_text("a.md", "v1").unwrap();
        let err = drive
            .write_text_if_unchanged("a.md", None, "v2")
            .unwrap_err();
        assert!(matches!(
            err,
            ChanError::WriteConflict {
                current_mtime: Some(_)
            }
        ));
        assert_eq!(drive.read_text("a.md").unwrap(), "v1");
    }

    #[test]
    fn write_text_if_unchanged_conflicts_when_expected_but_missing() {
        let (_cfg, _root, drive) = fixture();
        let err = drive
            .write_text_if_unchanged("a.md", Some(0), "v1")
            .unwrap_err();
        assert!(matches!(
            err,
            ChanError::WriteConflict {
                current_mtime: None
            }
        ));
        assert!(!drive.exists("a.md"));
    }

    #[test]
    fn write_text_if_unchanged_succeeds_with_matching_mtime() {
        let (_cfg, _root, drive) = fixture();
        drive.write_text("a.md", "v1").unwrap();
        let (_, stat) = drive.read_text_with_stat("a.md").unwrap();
        drive
            .write_text_if_unchanged("a.md", stat.mtime, "v2")
            .unwrap();
        assert_eq!(drive.read_text("a.md").unwrap(), "v2");
    }

    #[test]
    fn write_text_if_unchanged_conflicts_with_stale_mtime() {
        let (_cfg, _root, drive) = fixture();
        drive.write_text("a.md", "v1").unwrap();
        let stale = Some(0i64);
        let err = drive
            .write_text_if_unchanged("a.md", stale, "v2")
            .unwrap_err();
        match err {
            ChanError::WriteConflict { current_mtime } => {
                assert!(current_mtime.is_some());
                assert_ne!(current_mtime, stale);
            }
            other => panic!("expected WriteConflict, got {other:?}"),
        }
        assert_eq!(drive.read_text("a.md").unwrap(), "v1");
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

    #[cfg(unix)]
    #[test]
    fn read_text_rejects_symlink_target() {
        use std::os::unix::fs::symlink;
        let (_cfg, root, drive) = fixture();
        std::fs::write(root.path().join("real.md"), "hi").unwrap();
        symlink("real.md", root.path().join("alias.md")).unwrap();
        let err = drive.read_text("alias.md").unwrap_err();
        assert!(matches!(err, ChanError::SpecialFile { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn read_rejects_unix_socket() {
        use std::os::unix::net::UnixListener;
        let (_cfg, root, drive) = fixture();
        let _l = UnixListener::bind(root.path().join("s")).unwrap();
        let err = drive.read("s").unwrap_err();
        assert!(matches!(err, ChanError::SpecialFile { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn write_text_refuses_to_clobber_symlink() {
        use std::os::unix::fs::symlink;
        let (_cfg, root, drive) = fixture();
        std::fs::write(root.path().join("target.md"), "v1").unwrap();
        symlink("target.md", root.path().join("today.md")).unwrap();
        let err = drive.write_text("today.md", "v2").unwrap_err();
        assert!(matches!(err, ChanError::SpecialFile { .. }));
        // Both the symlink and its target are intact.
        assert_eq!(
            std::fs::read_to_string(root.path().join("target.md")).unwrap(),
            "v1"
        );
    }

    #[cfg(unix)]
    #[test]
    fn write_refuses_through_midpath_symlink_to_outside() {
        use std::os::unix::fs::symlink;
        let outside = TempDir::new().unwrap();
        let (_cfg, root, drive) = fixture();
        symlink(outside.path(), root.path().join("Backup")).unwrap();
        let err = drive.write_text("Backup/today.md", "x").unwrap_err();
        assert!(matches!(err, ChanError::SymlinkEscape(_)));
        // The escape path was never written.
        assert!(!outside.path().join("today.md").exists());
    }

    #[cfg(unix)]
    #[test]
    fn list_tree_drops_symlinks_and_sockets() {
        use std::os::unix::fs::symlink;
        use std::os::unix::net::UnixListener;
        let (_cfg, root, drive) = fixture();
        std::fs::write(root.path().join("note.md"), "hi").unwrap();
        symlink("note.md", root.path().join("alias.md")).unwrap();
        let _l = UnixListener::bind(root.path().join("sock")).unwrap();
        let entries = drive.list_tree().unwrap();
        let paths: Vec<_> = entries.iter().map(|e| e.path.clone()).collect();
        assert!(paths.contains(&"note.md".to_string()));
        assert!(!paths.iter().any(|p| p == "alias.md"));
        assert!(!paths.iter().any(|p| p == "sock"));
    }

    #[cfg(unix)]
    #[test]
    fn remove_rejects_symlink_with_special_file_error() {
        use std::os::unix::fs::symlink;
        let (_cfg, root, drive) = fixture();
        std::fs::write(root.path().join("real.md"), "hi").unwrap();
        symlink("real.md", root.path().join("alias.md")).unwrap();
        // Trash refuses to swallow non-regular non-directory entries:
        // restoring a symlink across a cross-fs trash is fragile, and
        // chan-drive never creates them on its own. Users delete them
        // out-of-band if they really want them gone.
        let err = drive.remove("alias.md").unwrap_err();
        assert!(matches!(err, ChanError::SpecialFile { .. }));
        // Both the symlink and its target are intact.
        assert!(root.path().join("alias.md").symlink_metadata().is_ok());
        assert!(root.path().join("real.md").exists());
    }

    #[test]
    fn remove_then_restore_round_trips() {
        let (_cfg, _root, drive) = fixture();
        drive.write_text("notes/a.md", "hello").unwrap();
        drive.remove("notes/a.md").unwrap();
        assert!(!drive.exists("notes/a.md"));
        let entries = drive.trash_list().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].original_path, "notes/a.md");
        drive.trash_restore(&entries[0].id).unwrap();
        assert_eq!(drive.read_text("notes/a.md").unwrap(), "hello");
        assert!(drive.trash_list().unwrap().is_empty());
    }

    #[test]
    fn remove_recursive_directory() {
        let (_cfg, _root, drive) = fixture();
        drive.write_text("notes/a.md", "a").unwrap();
        drive.write_text("notes/sub/b.md", "bb").unwrap();
        drive.remove("notes").unwrap();
        assert!(!drive.exists("notes/a.md"));
        let entries = drive.trash_list().unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_dir);
        drive.trash_restore(&entries[0].id).unwrap();
        assert_eq!(drive.read_text("notes/a.md").unwrap(), "a");
        assert_eq!(drive.read_text("notes/sub/b.md").unwrap(), "bb");
    }

    #[test]
    fn trash_restore_refuses_when_dest_exists() {
        let (_cfg, _root, drive) = fixture();
        drive.write_text("a.md", "v1").unwrap();
        drive.remove("a.md").unwrap();
        drive.write_text("a.md", "v2").unwrap();
        let id = drive.trash_list().unwrap()[0].id.clone();
        let err = drive.trash_restore(&id).unwrap_err();
        assert!(matches!(err, ChanError::TrashOccupied(_)));
        assert_eq!(drive.read_text("a.md").unwrap(), "v2");
        assert_eq!(drive.trash_list().unwrap().len(), 1);
    }

    #[test]
    fn trash_purge_and_empty() {
        let (_cfg, _root, drive) = fixture();
        drive.write_text("a.md", "x").unwrap();
        drive.write_text("b.md", "y").unwrap();
        drive.remove("a.md").unwrap();
        drive.remove("b.md").unwrap();
        let entries = drive.trash_list().unwrap();
        assert_eq!(entries.len(), 2);
        drive.trash_purge(&entries[0].id).unwrap();
        assert_eq!(drive.trash_list().unwrap().len(), 1);
        drive.trash_empty().unwrap();
        assert!(drive.trash_list().unwrap().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn remove_rejects_unix_socket() {
        use std::os::unix::net::UnixListener;
        let (_cfg, root, drive) = fixture();
        let _l = UnixListener::bind(root.path().join("s")).unwrap();
        let err = drive.remove("s").unwrap_err();
        assert!(matches!(err, ChanError::SpecialFile { .. }));
        // Socket survives the rejected remove.
        assert!(root.path().join("s").symlink_metadata().is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn stat_uses_lstat_for_symlinks() {
        use std::os::unix::fs::symlink;
        let (_cfg, root, drive) = fixture();
        std::fs::create_dir(root.path().join("d")).unwrap();
        symlink("d", root.path().join("link_to_dir")).unwrap();
        // lstat reports the symlink itself, which is a symlink (not
        // a directory). is_dir is false because symlink_metadata
        // does not follow.
        let st = drive.stat("link_to_dir").unwrap();
        assert!(!st.is_dir);
    }

    #[test]
    fn link_targets_finds_file_after_index() {
        let (_cfg, _root, drive) = fixture();
        drive
            .write_text("recipes/carbonara.md", "# Carbonara\n\n## Ingredients\n")
            .unwrap();
        drive.index_file("recipes/carbonara.md").unwrap();
        let hits = drive.link_targets("carb", 10).unwrap();
        assert!(hits
            .iter()
            .any(|h| h.path == "recipes/carbonara.md"
                && h.kind == crate::graph::LinkTargetKind::File));
        // Heading is also searchable by the same surface.
        let hits = drive.link_targets("ingred", 10).unwrap();
        assert!(hits
            .iter()
            .any(|h| h.kind == crate::graph::LinkTargetKind::Heading
                && h.heading.as_deref() == Some("Ingredients")));
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

    #[test]
    fn session_blob_round_trip() {
        let (_cfg, _root, drive) = fixture();
        drive.put_session("win-1", b"layout-v1").unwrap();
        assert_eq!(drive.get_session("win-1").unwrap().unwrap(), b"layout-v1");
        drive.put_session("win-1", b"layout-v2").unwrap();
        drive.put_session("win-2", b"other").unwrap();
        let mut keys = drive.list_sessions().unwrap();
        keys.sort();
        assert_eq!(keys, vec!["win-1", "win-2"]);
        drive.delete_session("win-1").unwrap();
        assert!(drive.get_session("win-1").unwrap().is_none());
        // Idempotent.
        drive.delete_session("win-1").unwrap();
    }

    #[test]
    fn assistant_blob_round_trip_and_clear() {
        let (_cfg, _root, drive) = fixture();
        drive.put_assistant("conv-a", b"chat-1").unwrap();
        drive.put_assistant("conv-b", b"chat-2").unwrap();
        assert_eq!(drive.list_assistant().unwrap().len(), 2);
        drive.clear_assistant().unwrap();
        assert!(drive.list_assistant().unwrap().is_empty());
    }

    #[test]
    fn session_and_assistant_buckets_are_separate() {
        let (_cfg, _root, drive) = fixture();
        drive.put_session("k", b"in-sessions").unwrap();
        drive.put_assistant("k", b"in-assistant").unwrap();
        assert_eq!(drive.get_session("k").unwrap().unwrap(), b"in-sessions");
        assert_eq!(drive.get_assistant("k").unwrap().unwrap(), b"in-assistant");
    }

    #[test]
    fn blob_key_validation_blocks_traversal() {
        let (_cfg, _root, drive) = fixture();
        let err = drive.put_session("../escape", b"x").unwrap_err();
        assert!(matches!(err, ChanError::InvalidKey(_)));
    }

    // ---- resolve_link ----

    fn link_fixture() -> (TempDir, TempDir, Arc<Drive>) {
        let (cfg, root, drive) = fixture();
        std::fs::create_dir_all(root.path().join("recipes")).unwrap();
        std::fs::write(root.path().join("recipes").join("pasta.md"), "# Pasta\n").unwrap();
        std::fs::write(root.path().join("intro.md"), "# Intro\n").unwrap();
        std::fs::write(root.path().join("note.txt"), "plain\n").unwrap();
        std::fs::write(root.path().join("README"), "no ext\n").unwrap();
        (cfg, root, drive)
    }

    #[test]
    fn resolve_link_md_extension() {
        let (_cfg, _root, drive) = link_fixture();
        let r = drive.resolve_link("recipes/pasta").unwrap();
        assert_eq!(r.path, "recipes/pasta.md");
        assert_eq!(r.anchor, None);
    }

    #[test]
    fn resolve_link_with_anchor() {
        let (_cfg, _root, drive) = link_fixture();
        let r = drive.resolve_link("recipes/pasta#ingredients").unwrap();
        assert_eq!(r.path, "recipes/pasta.md");
        assert_eq!(r.anchor.as_deref(), Some("ingredients"));
    }

    #[test]
    fn resolve_link_txt_fallback() {
        let (_cfg, _root, drive) = link_fixture();
        let r = drive.resolve_link("note").unwrap();
        assert_eq!(r.path, "note.txt");
    }

    #[test]
    fn resolve_link_exact_match_no_extension() {
        let (_cfg, _root, drive) = link_fixture();
        let r = drive.resolve_link("README").unwrap();
        assert_eq!(r.path, "README");
    }

    #[test]
    fn resolve_link_prefers_md_over_txt() {
        let (_cfg, root, drive) = link_fixture();
        // both intro.md AND intro.txt exist -> .md wins
        std::fs::write(root.path().join("intro.txt"), "plain\n").unwrap();
        let r = drive.resolve_link("intro").unwrap();
        assert_eq!(r.path, "intro.md");
    }

    #[test]
    fn resolve_link_nonexistent_returns_none() {
        let (_cfg, _root, drive) = link_fixture();
        assert!(drive.resolve_link("does/not/exist").is_none());
    }

    #[test]
    fn resolve_link_empty_target_returns_none() {
        let (_cfg, _root, drive) = link_fixture();
        assert!(drive.resolve_link("").is_none());
    }

    #[test]
    fn resolve_link_trailing_hash_drops_anchor() {
        let (_cfg, _root, drive) = link_fixture();
        // `target#` (empty anchor) resolves the path with anchor None.
        let r = drive.resolve_link("recipes/pasta#").unwrap();
        assert_eq!(r.path, "recipes/pasta.md");
        assert_eq!(r.anchor, None);
    }

    #[test]
    fn resolve_link_path_escape_rejected() {
        let (_cfg, _root, drive) = link_fixture();
        // resolve_link goes through Drive::exists which rejects
        // path traversal. Should return None, not panic.
        assert!(drive.resolve_link("../etc/passwd").is_none());
    }

    #[test]
    fn resolve_link_path_with_md_extension_unchanged() {
        let (_cfg, _root, drive) = link_fixture();
        // If the user already wrote `[[recipes/pasta.md]]`, our
        // first probe is `recipes/pasta.md.md` which doesn't
        // exist; second is `.txt`; third is the exact path which
        // does. Resolves to the original verbatim.
        let r = drive.resolve_link("recipes/pasta.md").unwrap();
        assert_eq!(r.path, "recipes/pasta.md");
    }
}
