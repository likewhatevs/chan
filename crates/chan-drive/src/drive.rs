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

/// Hard cap on `write_text` content size. Markdown / txt notes are
/// human-authored; 2 MiB is roughly 2M characters of dense English,
/// far past any realistic note. Anything larger is almost certainly
/// either a bug, a binary file mislabelled with `.md`, or an LLM tool
/// running away. We stop it at the boundary so a misbehaving caller
/// cannot fill the user's drive without an explicit code change.
pub const TEXT_WRITE_LIMIT: u64 = 2 * 1024 * 1024;

/// Hard cap on `write_bytes` (binary attachments / media). 50 MiB
/// covers typical PDF / image / short audio attachments with margin.
/// Same rationale as `TEXT_WRITE_LIMIT`: defense against runaway
/// callers, not a UX feature; raise via a code change if a real use
/// case appears.
pub const BYTES_WRITE_LIMIT: u64 = 50 * 1024 * 1024;

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
    /// Last modification time as Unix seconds. Coarse, useful for
    /// display and for the graph DB which stores i64 seconds.
    pub mtime: Option<i64>,
    /// Last modification time as Unix nanoseconds. Fits an i64
    /// until year 2262. Used by `write_text_if_unchanged` so two
    /// edits within the same wall-clock second are still detected
    /// as a conflict on filesystems with sub-second mtime (ext4 /
    /// xfs / APFS / btrfs). May be None on filesystems that only
    /// expose seconds (FAT, some network mounts), in which case
    /// the optimistic-concurrency check degrades to seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mtime_ns: Option<i64>,
    pub is_dir: bool,
}

/// A wiki-link resolved to an actual drive file. `path` is the
/// POSIX rel path of the file that exists today; `anchor` is the
/// `#section` fragment from the original target, passed through
/// unchanged. `kind` is the graph-recorded node kind (file vs
/// contact); callers (the editor) use it to render a kind-aware
/// pill without re-parsing the target's frontmatter. See
/// `Drive::resolve_link` for the resolution rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedLink {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
    /// Defaults to `File` when the graph hasn't indexed the target
    /// yet (fresh file, indexer behind), or when a future variant
    /// is introduced and an older client deserializes the response.
    /// `#[serde(default)]` keeps wire compatibility with payloads
    /// that omit the field entirely.
    #[serde(default)]
    pub kind: crate::graph::NodeKind,
}

/// One open drive. Holds the writer lock for as long as it lives,
/// so two processes can't both write the same drive's index/graph.
/// Cheap reads are unlocked; writes go through the locked handle.
pub struct Drive {
    entry: KnownDrive,
    /// Canonical form of `entry.path`, computed once at open. Used
    /// where we need an absolute path (display, paths::drive_paths
    /// keying) and as the slow-path baseline for trash::restore.
    root_canon: std::path::PathBuf,
    /// Capability-based handle to the drive root. All filesystem
    /// ops on user-controllable paths go through this so a mid-path
    /// symlink swap between path-resolution and the actual op
    /// cannot escape the sandbox: cap-std opens each path component
    /// with O_NOFOLLOW and refuses paths that walk outside the
    /// dir handle. The previous resolve_safe_strict + std::fs::op
    /// pair had a small TOCTOU window between the lexical sandbox
    /// check and the kernel-side path walk; cap-std closes it.
    dir: cap_std::fs::Dir,
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
        // Defensive check: the registered path must still resolve to
        // a directory. A user (or another tool) could have replaced
        // the drive directory with a symlink, file, or socket since
        // the registry entry was written, in which case our path
        // sandbox and per-op gates would still apply but the drive
        // shape itself is no longer what the user signed up for.
        // `exists()` follows symlinks, so we use lstat here to catch
        // a "directory turned into a symlink" replacement.
        let meta = match std::fs::symlink_metadata(&entry.path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(ChanError::DriveRootMissing(entry.path.clone()));
            }
            Err(e) => return Err(ChanError::Io(e.to_string())),
        };
        let ft = meta.file_type();
        if !ft.is_dir() || ft.is_symlink() {
            return Err(ChanError::SpecialFile {
                kind: fs_ops::describe_file_kind(&ft).to_string(),
                path: entry.path.clone(),
            });
        }
        let root_canon = entry
            .path
            .canonicalize()
            .map_err(|e| ChanError::Io(format!("canonicalize drive root: {e}")))?;
        let dir = cap_std::fs::Dir::open_ambient_dir(&entry.path, cap_std::ambient_authority())
            .map_err(|e| ChanError::Io(format!("open drive root: {e}")))?;
        let paths = drive_paths(&entry.path);
        let lock = DriveLock::acquire(&paths.lock)?;
        // Lazy GC: reclaim expired trash entries on every open. No
        // background thread, matches the codebase's sync-only rule.
        // Errors are swallowed: a corrupt trash dir must never block
        // a legitimate drive open.
        let _ = trash::sweep_expired(&paths.trash, TRASH_RETENTION_SECS);
        Ok(Arc::new(Self {
            entry,
            root_canon,
            dir,
            paths,
            _lock: lock,
            index: std::sync::OnceLock::new(),
            graph: std::sync::OnceLock::new(),
        }))
    }

    /// Validate `rel` for use with the cap-std `Dir`. Returns a
    /// pure-Component::Normal PathBuf or a `PathEmpty` / `PathEscape`
    /// error. cap-std would refuse a bad path anyway; this gate
    /// gives crisp error variants.
    fn rel(&self, rel: &str) -> Result<std::path::PathBuf> {
        fs_ops::validate_rel(rel)
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
    // Every entry point here routes through the cap-std `Dir`
    // opened at `Drive::open`. cap-std uses openat-per-component
    // with O_NOFOLLOW (or RESOLVE_BENEATH on Linux openat2), so a
    // mid-path symlink swap between path validation and the actual
    // op cannot escape the drive root. Reads additionally call
    // `ensure_regular_file_in` (lstat) so we never block on a FIFO,
    // drain a device, or follow a symlink off the drive. Writes
    // that target an existing path do the same check via
    // `ensure_writable_in`; writes to a fresh path skip it because
    // there's nothing to inspect yet (cap-std guarded the parent
    // walk on the way in).

    /// Read raw bytes from a file relative to the drive root. No
    /// editable-text gate: callers like image previews need binary
    /// reads. The path must resolve to a regular file under the
    /// drive root; symlinks, FIFOs, sockets, and devices are
    /// rejected.
    pub fn read(&self, rel: &str) -> Result<Vec<u8>> {
        let rel_path = self.rel(rel)?;
        ensure_regular_file_in(&self.dir, &rel_path)?;
        let mut f = self
            .dir
            .open(&rel_path)
            .map_err(|e| ChanError::Io(e.to_string()))?;
        use std::io::Read;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        Ok(buf)
    }

    /// Read UTF-8 text. Errors if the file isn't on the editable-
    /// text whitelist or isn't a regular file.
    pub fn read_text(&self, rel: &str) -> Result<String> {
        if !fs_ops::is_editable_text(rel) {
            return Err(ChanError::NotEditableText(rel.to_string()));
        }
        let rel_path = self.rel(rel)?;
        ensure_regular_file_in(&self.dir, &rel_path)?;
        let mut f = self
            .dir
            .open(&rel_path)
            .map_err(|e| ChanError::Io(e.to_string()))?;
        use std::io::Read;
        let mut buf = String::new();
        f.read_to_string(&mut buf)?;
        Ok(buf)
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
        let rel_path = self.rel(rel)?;
        ensure_regular_file_in(&self.dir, &rel_path)?;
        let mut f = self
            .dir
            .open(&rel_path)
            .map_err(|e| ChanError::Io(e.to_string()))?;
        let meta = f.metadata()?;
        let mut content = String::new();
        f.read_to_string(&mut content)?;
        let stat = FileStat {
            size: meta.len(),
            mtime: mtime_secs_cap(&meta),
            mtime_ns: mtime_ns_cap(&meta),
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
        let rel_path = self.rel(rel)?;
        let prev = ensure_writable_in(&self.dir, &rel_path)?;
        check_size(
            "text",
            content.len(),
            TEXT_WRITE_LIMIT,
            prev.as_ref().map(|m| m.len()),
        )?;
        fs_ops::atomic_write_in(&self.dir, &rel_path, content.as_bytes())
    }

    /// Optimistic-concurrency write: succeeds only when the file's
    /// current mtime (nanoseconds) matches `expected_mtime_ns`. The
    /// editor pairs this with `read_text_with_stat`: it reads
    /// (content, stat), the user edits, then it writes back with
    /// `expected_mtime_ns = stat.mtime_ns`. If the file changed under
    /// the editor (another process, another pane), the write fails
    /// with `ChanError::WriteConflict` and the editor can prompt to
    /// reload, merge, or overwrite.
    ///
    /// Why nanoseconds: most filesystems chan runs on (ext4, xfs,
    /// APFS, btrfs) expose nanosecond mtime. Two saves landing within
    /// the same wall-clock second produce identical second-resolution
    /// mtimes; an editor saving on top of an autosave from a tool a
    /// few hundred ms earlier would silently win. Ns resolution
    /// catches that. On filesystems that only carry seconds (FAT,
    /// some SMB mounts), `mtime_ns` is the seconds value times 1e9
    /// and the check degrades gracefully.
    ///
    /// Conventions for `expected_mtime_ns`:
    ///   - `None` + missing file: create.
    ///   - `None` + existing file: `WriteConflict`. The caller did
    ///     not know a file was there; treating that as a silent
    ///     overwrite would be the bug we're trying to prevent.
    ///   - `Some(m)` + current mtime_ns == m: write.
    ///   - any other case: `WriteConflict { current_mtime_ns }`.
    ///
    /// Residual race: between the mtime check and the atomic rename,
    /// another writer can land. The window is small (no syscalls
    /// between the two) and the next watcher event will surface the
    /// foreign change so the editor can re-prompt. Callers that need
    /// stronger semantics must serialize at a higher level.
    pub fn write_text_if_unchanged(
        &self,
        rel: &str,
        expected_mtime_ns: Option<i64>,
        content: &str,
    ) -> Result<()> {
        if !fs_ops::is_editable_text(rel) {
            return Err(ChanError::NotEditableText(rel.to_string()));
        }
        let rel_path = self.rel(rel)?;
        let prev = ensure_writable_in(&self.dir, &rel_path)?;
        let (current, exists, prev_size) = match prev.as_ref() {
            Some(meta) => (mtime_ns_cap(meta), true, Some(meta.len())),
            None => (None, false, None),
        };
        let conflict = match (expected_mtime_ns, exists) {
            (None, false) => false,
            (Some(m), true) => current != Some(m),
            _ => true,
        };
        if conflict {
            return Err(ChanError::WriteConflict {
                current_mtime_ns: current,
            });
        }
        check_size("text", content.len(), TEXT_WRITE_LIMIT, prev_size)?;
        fs_ops::atomic_write_in(&self.dir, &rel_path, content.as_bytes())
    }

    /// Atomically write raw bytes. NOT gated by editable-text;
    /// used by attachments and the future media browser. Callers
    /// that surface this to the editor must apply their own gate.
    /// Same special-file refusal as `write_text`.
    pub fn write_bytes(&self, rel: &str, content: &[u8]) -> Result<()> {
        let rel_path = self.rel(rel)?;
        let prev = ensure_writable_in(&self.dir, &rel_path)?;
        check_size(
            "bytes",
            content.len(),
            BYTES_WRITE_LIMIT,
            prev.as_ref().map(|m| m.len()),
        )?;
        fs_ops::atomic_write_in(&self.dir, &rel_path, content)
    }

    /// True iff the path resolves under the drive and refers to a
    /// regular file. Matches the gate `read` / `read_text` apply,
    /// so a `true` return is a strong signal that a read will
    /// succeed.
    pub fn exists(&self, rel: &str) -> bool {
        let Ok(rel_path) = self.rel(rel) else {
            return false;
        };
        match self.dir.symlink_metadata(&rel_path) {
            Ok(m) => m.is_file() && !m.file_type().is_symlink(),
            Err(_) => false,
        }
    }

    /// Stat the path using `lstat` semantics (so a symlink reports
    /// as such, not as its target). Refuses paths that escape the
    /// drive root through a mid-path symlink.
    pub fn stat(&self, rel: &str) -> Result<FileStat> {
        let rel_path = self.rel(rel)?;
        let meta = self
            .dir
            .symlink_metadata(&rel_path)
            .map_err(|e| ChanError::Io(e.to_string()))?;
        Ok(FileStat {
            size: if meta.is_dir() { 0 } else { meta.len() },
            mtime: mtime_secs_cap(&meta),
            mtime_ns: mtime_ns_cap(&meta),
            is_dir: meta.is_dir(),
        })
    }

    /// One-level directory listing. Use `list_tree` for the
    /// recursive variant. Skips drive-internal noise (`.chan/`,
    /// `.git/`) at the top level and drops non-regular non-dir
    /// entries (symlinks, FIFOs, sockets, devices) at every level.
    /// Errors with `ListingTooLarge` past `LIST_DIR_LIMIT`.
    ///
    /// Per-entry errors (a vanished entry mid-iteration, a
    /// permission denied on stat) are logged at warn and the entry
    /// is skipped rather than aborting the listing. Without the log
    /// the editor's tree view would silently miss entries and the
    /// user has no signal anything went wrong; with it, the issue
    /// shows up in `tracing` output and the indexer / status surface
    /// can act on the count later if needed.
    pub fn list(&self, rel: &str) -> Result<Vec<DirEntry>> {
        let at_root = rel.is_empty() || rel == "." || rel == "/";
        let read = if at_root {
            self.dir
                .read_dir(".")
                .map_err(|e| ChanError::Io(e.to_string()))?
        } else {
            let rel_path = self.rel(rel)?;
            self.dir
                .read_dir(&rel_path)
                .map_err(|e| ChanError::Io(e.to_string()))?
        };
        let mut out = Vec::new();
        let mut skipped = 0usize;
        for entry in read {
            if out.len() >= fs_ops::LIST_DIR_LIMIT {
                return Err(ChanError::ListingTooLarge {
                    observed: out.len(),
                    limit: fs_ops::LIST_DIR_LIMIT,
                });
            }
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!(?rel, ?e, "list: read_dir entry error; skipping");
                    skipped += 1;
                    continue;
                }
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            if at_root && (name == ".chan" || name == ".git") {
                continue;
            }
            let ft = match entry.file_type() {
                Ok(ft) => ft,
                Err(e) => {
                    tracing::warn!(?rel, ?name, ?e, "list: file_type failed; skipping");
                    skipped += 1;
                    continue;
                }
            };
            if !(ft.is_dir() || (ft.is_file() && !ft.is_symlink())) {
                continue;
            }
            out.push(DirEntry {
                name,
                is_dir: ft.is_dir(),
            });
        }
        if skipped > 0 {
            tracing::warn!(
                ?rel,
                skipped,
                returned = out.len(),
                "list: directory listing partial",
            );
        }
        Ok(out)
    }

    pub fn list_tree(&self) -> Result<Vec<TreeEntry>> {
        fs_ops::list_tree(self.root())
    }

    pub fn create_dir(&self, rel: &str) -> Result<()> {
        let rel_path = self.rel(rel)?;
        self.dir
            .create_dir_all(&rel_path)
            .map_err(|e| ChanError::Io(e.to_string()))?;
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
        let rel_path = self.rel(rel)?;
        // cap-std lstat: TOCTOU-free type check. The subsequent
        // trash::move_into still operates path-based (it has to
        // bridge into the trash dir which lives outside the cap-std
        // Drive sandbox), so the rename itself has a small residual
        // TOCTOU window. The damage if exploited is "wrong file
        // goes to trash" - recoverable via `trash_restore`.
        let meta = self
            .dir
            .symlink_metadata(&rel_path)
            .map_err(|e| ChanError::Io(e.to_string()))?;
        let ft = meta.file_type();
        let is_dir = ft.is_dir();
        let is_regular_file = ft.is_file() && !ft.is_symlink();
        let abs = self.entry.path.join(&rel_path);
        if !(is_dir || is_regular_file) {
            return Err(ChanError::SpecialFile {
                kind: describe_cap_file_kind(&ft).to_string(),
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
        trash::restore(&self.paths.trash, self.root(), &self.root_canon, id)
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

    /// Write one markdown note per `Contact` into `dir` (drive-
    /// relative; created if missing). Each note carries a
    /// `chan.kind: contact` frontmatter so downstream consumers
    /// (graph builder, editor `@` picker) can classify it without a
    /// separate index. Path collisions are handled per `opts`:
    /// either skipped or overwritten.
    ///
    /// All writes go through `write_text`, so the path sandbox,
    /// editable-text gate, and atomic-rename rules apply per file.
    /// One bad contact does not abort the batch; per-file errors
    /// land in the returned `ImportSummary` as `Failed`.
    pub fn import_contacts(
        &self,
        dir: &str,
        contacts: Vec<crate::contacts::Contact>,
        opts: crate::contacts::ImportOpts,
    ) -> Result<crate::contacts::ImportSummary> {
        crate::contacts::import::run(self, dir, contacts, opts)
    }

    pub fn rename(&self, from: &str, to: &str) -> Result<()> {
        let from_rel = self.rel(from)?;
        let to_rel = self.rel(to)?;
        // Source must exist as a regular file or directory; refuse
        // to move a symlink or special file. (renaming a symlink
        // is well-defined at the syscall level but not something
        // the editor should ever do silently.)
        let src_meta = self
            .dir
            .symlink_metadata(&from_rel)
            .map_err(|e| ChanError::Io(e.to_string()))?;
        let src_ft = src_meta.file_type();
        if !(src_ft.is_dir() || (src_ft.is_file() && !src_ft.is_symlink())) {
            return Err(ChanError::SpecialFile {
                kind: describe_cap_file_kind(&src_ft).to_string(),
                path: self.entry.path.join(&from_rel),
            });
        }
        ensure_writable_in(&self.dir, &to_rel)?;
        if let Some(parent) = to_rel.parent() {
            if !parent.as_os_str().is_empty() {
                self.dir
                    .create_dir_all(parent)
                    .map_err(|e| ChanError::Io(e.to_string()))?;
            }
        }
        // cap-std rename within the same Dir is TOCTOU-free: source
        // and destination resolve through the dir handle, no
        // path-walk through swappable ancestors.
        self.dir
            .rename(&from_rel, &self.dir, &to_rel)
            .map_err(|e| ChanError::Io(e.to_string()))?;
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
        // Two-phase: collect everything in memory, then commit in a
        // single sqlite transaction via `GraphView::replace_all`. The
        // alternative (clear + per-file replace_file) left the graph
        // half-populated on mid-rebuild error, lying to the server's
        // auto-rebuild trigger about freshness. Memory cost is bounded
        // by drive size and is small (a 10k-file drive holds tens of
        // MB of headings + edges, well within the editor's footprint).
        let entries = self.list_tree()?;
        struct Owned {
            rel: String,
            title: Option<String>,
            node_kind: crate::graph::NodeKind,
            mtime: Option<i64>,
            edges: Vec<crate::graph::Edge>,
            headings: Vec<markdown::Heading>,
            emails: Option<String>,
        }
        let mut owned: Vec<Owned> = Vec::new();
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
            let (title, node_kind, headings, edges, emails) = parse_for_graph(&e.path, &content);
            owned.push(Owned {
                rel: e.path.clone(),
                title,
                node_kind,
                mtime: e.mtime,
                edges,
                headings,
                emails,
            });
        }
        if let Some(c) = cancel {
            if c.load(Ordering::Relaxed) {
                return Err(ChanError::Cancelled);
            }
        }
        let borrowed: Vec<crate::graph::FileGraph<'_>> = owned
            .iter()
            .map(|o| crate::graph::FileGraph {
                rel: &o.rel,
                title: o.title.as_deref(),
                mtime: o.mtime,
                node_kind: o.node_kind,
                edges: &o.edges,
                headings: &o.headings,
                emails: o.emails.as_deref(),
            })
            .collect();
        self.graph()?.replace_all(&borrowed)?;
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
        let (title, node_kind, headings, edges, emails) = parse_for_graph(rel, &content);
        // Graph first, then search index. The graph is what the
        // editor consults for backlinks and link-autocomplete on
        // every keystroke; a stale graph is the more user-visible
        // failure mode. The search index is queried explicitly and
        // a stale BM25 row gets corrected on the next save or
        // rebuild. If graph succeeds and index_one fails, the
        // resulting drift is "search slightly stale", which the
        // user is less likely to notice and can recover via the
        // server's auto-rebuild trigger. The opposite ordering
        // (search-then-graph) made backlinks the silent victim.
        self.graph()?.replace_file(
            rel,
            title.as_deref(),
            mtime,
            node_kind,
            &edges,
            &headings,
            emails.as_deref(),
        )?;
        // Hand the already-read content to the index so the read
        // goes through the Drive sandbox exactly once.
        self.index()?.index_one(rel, &content)?;
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
                // Best-effort kind lookup against the graph. If the
                // graph isn't open yet, the row is missing (file
                // indexed after this resolve, or never indexed), or
                // the query fails for any reason, fall back to
                // `File` so the caller still gets a resolution.
                // Kind-aware rendering is a hint, not a contract.
                let kind = self
                    .graph()
                    .ok()
                    .and_then(|g| g.node_kind(&candidate).ok().flatten())
                    .unwrap_or_default();
                return Some(ResolvedLink {
                    path: candidate,
                    anchor,
                    kind,
                });
            }
        }
        None
    }

    /// Drop a single file from the search index and graph. Used
    /// when the watcher reports a deletion.
    ///
    /// Graph first, search second, mirroring `index_file`. If the
    /// graph delete succeeds and the search delete fails, queries
    /// surface a "ghost" search hit pointing at a missing file; the
    /// caller then asks `Drive::read` and gets `NotFound`, which is
    /// recoverable. The reverse ordering (search-then-graph) would
    /// leave backlinks pointing at a missing file, a silently broken
    /// state the editor cannot self-heal.
    pub fn forget_file(&self, rel: &str) -> Result<()> {
        self.graph()?.forget_file(rel)?;
        self.index()?.forget(rel)?;
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

    /// All contact-kind notes in the drive, sorted by display name.
    /// Pass-through to `GraphView::contacts`. Convenience for callers
    /// (CLI, tests) that want the full list; the editor `@` picker
    /// and `GET /api/contacts` should call `contacts_filtered`
    /// instead so the case-insensitive contains filter and result cap
    /// run inside SQLite.
    pub fn contacts(&self) -> Result<Vec<crate::graph::ContactNode>> {
        self.graph()?.contacts()
    }

    /// Filtered contact-kind notes. `query` is matched case-
    /// insensitively against title, basename, and the joined email
    /// column inside SQLite, and `limit` caps the row count so
    /// picker keystrokes stay O(limit) regardless of how many
    /// contacts the drive holds. `query` of `None` or empty returns
    /// up to `limit` contacts in display-name order.
    pub fn contacts_filtered(
        &self,
        query: Option<&str>,
        limit: usize,
    ) -> Result<Vec<crate::graph::ContactNode>> {
        self.graph()?.contacts_filtered(query, limit)
    }

    /// True when at least one contact-kind row in the graph DB has
    /// `emails IS NULL`. Set after a v3 schema migration finds
    /// pre-v3 contact rows that haven't been re-indexed yet (the
    /// migration cannot walk the filesystem itself, so the column
    /// stays NULL until something re-parses the file). The chan-
    /// server indexer reads this on boot and fires a one-shot full
    /// rebuild so email-aware @ matching works without the user
    /// having to think about it.
    pub fn contacts_need_email_backfill(&self) -> Result<bool> {
        self.graph()?.contacts_need_email_backfill()
    }

    // ---- watch ----

    /// Start a recursive filesystem watcher on the drive. Drop
    /// the returned `WatchHandle` to stop. Events for `.chan/`
    /// and `.git/` are filtered out.
    pub fn watch(self: &Arc<Self>, cb: Arc<dyn WatchCallback>) -> Result<WatchHandle> {
        WatchHandle::start(self.root(), cb)
    }
}

/// Hard size guard for write_* paths. `kind` is the static label
/// surfaced in the error so the caller can distinguish text vs
/// bytes vs (future) blob caps.
///
/// The configured `limit` is a fresh-file cap. When the target
/// already exists and is itself larger than the cap (legacy file,
/// pre-cap content, a binary attached as `.txt`), the caller is
/// already past the policy boundary and we let edits up to the
/// existing size through. The intent is "stop runaway growth", not
/// "make all your files read-only the moment we ship a cap".
///
/// Effective limit = max(prev_size, limit). Refusal carries the
/// effective limit so the editor can show the user the exact
/// number it has to stay under.
fn check_size(kind: &'static str, size: usize, limit: u64, prev_size: Option<u64>) -> Result<()> {
    let size = size as u64;
    let effective = std::cmp::max(prev_size.unwrap_or(0), limit);
    if size > effective {
        return Err(ChanError::WriteTooLarge {
            kind,
            size,
            limit: effective,
        });
    }
    Ok(())
}

/// Map a `std::io::Error` returned by a cap-std op into our error
/// enum. cap-std rejects sandbox escapes (mid-path symlink pointing
/// outside the dir handle, absolute path passed as rel, `..` that
/// would walk above the root) with a generic io::Error; the message
/// it produces ("a path led outside of the filesystem") is the only
/// portable signal we have to distinguish "you tried to escape"
/// from "regular I/O error". Fragile if cap-std changes the string;
/// a regression test in this module pins it.
fn map_cap_err(err: std::io::Error, rel: &std::path::Path) -> ChanError {
    let msg = err.to_string();
    if msg.contains("outside of the filesystem") || msg.contains("path escape") {
        return ChanError::SymlinkEscape(rel.to_path_buf());
    }
    ChanError::Io(msg)
}

/// cap-std variant of `mtime_secs` for `cap_std::fs::Metadata`.
fn mtime_secs_cap(meta: &cap_std::fs::Metadata) -> Option<i64> {
    meta.modified()
        .ok()
        .map(|t| t.into_std())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
}

/// cap-std variant of `mtime_ns` for `cap_std::fs::Metadata`.
fn mtime_ns_cap(meta: &cap_std::fs::Metadata) -> Option<i64> {
    meta.modified()
        .ok()
        .map(|t| t.into_std())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|d| i64::try_from(d.as_nanos()).ok())
}

/// Human-readable name for a cap-std `FileType`. Mirrors
/// `fs_ops::describe_file_kind`. cap-std exposes the same is_*
/// predicates plus the unix-only fifo/socket/char/block via
/// `FileTypeExt`.
fn describe_cap_file_kind(ft: &cap_std::fs::FileType) -> &'static str {
    if ft.is_dir() {
        return "directory";
    }
    if ft.is_symlink() {
        return "symlink";
    }
    if ft.is_file() {
        return "regular";
    }
    #[cfg(unix)]
    {
        use cap_std::fs::FileTypeExt;
        if ft.is_fifo() {
            return "fifo";
        }
        if ft.is_socket() {
            return "socket";
        }
        if ft.is_char_device() {
            return "char_device";
        }
        if ft.is_block_device() {
            return "block_device";
        }
    }
    "unknown"
}

/// cap-std equivalent of `fs_ops::ensure_regular_file`. Lstat
/// through the sandboxed `Dir`; refuse anything that isn't a real
/// regular file (symlink / FIFO / socket / device / directory).
fn ensure_regular_file_in(dir: &cap_std::fs::Dir, rel: &std::path::Path) -> Result<()> {
    let meta = dir.symlink_metadata(rel).map_err(|e| map_cap_err(e, rel))?;
    let ft = meta.file_type();
    if ft.is_file() && !ft.is_symlink() {
        return Ok(());
    }
    Err(ChanError::SpecialFile {
        kind: describe_cap_file_kind(&ft).to_string(),
        path: rel.to_path_buf(),
    })
}

/// cap-std equivalent of `ensure_writable`. Returns the existing
/// file's metadata when the leaf is a regular file, `None` when
/// missing, error when it's something we refuse to overwrite.
fn ensure_writable_in(
    dir: &cap_std::fs::Dir,
    rel: &std::path::Path,
) -> Result<Option<cap_std::fs::Metadata>> {
    match dir.symlink_metadata(rel) {
        Ok(meta) => {
            let ft = meta.file_type();
            if ft.is_file() && !ft.is_symlink() {
                Ok(Some(meta))
            } else {
                Err(ChanError::SpecialFile {
                    kind: describe_cap_file_kind(&ft).to_string(),
                    path: rel.to_path_buf(),
                })
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(map_cap_err(e, rel)),
    }
}

/// Parse a file's content into the graph-side structures: the
/// title (for the graph node), the heading list (for graph
/// headings), the outgoing edges (links + tokens), and, for
/// contact-kind files, the joined email list (space-separated,
/// lowercased). The search-side chunking is done separately by the
/// index facade.
fn parse_for_graph(
    rel: &str,
    raw: &str,
) -> (
    Option<String>,
    crate::graph::NodeKind,
    Vec<markdown::Heading>,
    Vec<crate::graph::Edge>,
    Option<String>,
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
    // Contact-kind tag lives under the chan namespace so user
    // frontmatter (which may already carry a `kind:` of its own
    // for app-specific reasons) can't accidentally tip a regular
    // note into the contacts surface.
    let node_kind = fm
        .data
        .get("chan")
        .and_then(|v| v.get("kind"))
        .and_then(|v| v.as_str())
        .filter(|s| s.eq_ignore_ascii_case("contact"))
        .map(|_| crate::graph::NodeKind::Contact)
        .unwrap_or(crate::graph::NodeKind::File);
    let links = markdown::extract_links(body_src);
    let tokens = markdown::extract_tokens(body_src);
    let edges = build_edges(rel, &links, &tokens);
    // Email extraction runs only for contact-kind files: a regular
    // note that mentions an email in passing should not get its
    // address mirrored into a `nodes.emails` column the picker can
    // surface, and skipping the scan for File-kind keeps the
    // hot-path cost zero on normal indexes.
    let emails = if matches!(node_kind, crate::graph::NodeKind::Contact) {
        let list = crate::contacts::extract_emails(body_src);
        if list.is_empty() {
            None
        } else {
            Some(list.join(" "))
        }
    } else {
        None
    };
    (title, node_kind, headings, edges, emails)
}

/// Whether `path` lies under the `prefix` directory. POSIX
/// separators on both sides; ASCII case-insensitive comparison.
///
/// Why case-insensitive: scope filters live in the search/UI layer
/// and reflect the user's mental model of folders, not the
/// filesystem's strict casing. APFS (default) and NTFS are
/// case-insensitive, so a scope of `"Notes"` matching a stored path
/// of `"notes/foo.md"` is what the user wants. ext4 is technically
/// case-sensitive, so the over-match here is theoretical: a user
/// would have to maintain `Notes/` and `notes/` as distinct folders
/// AND ask the search to scope into one without bleeding the other.
/// We accept that minor risk in exchange for predictable UX across
/// platforms. ASCII-only fold (no Unicode) matches the rest of the
/// crate's case-folding policy.
fn path_under(path: &str, prefix: &str) -> bool {
    let prefix = prefix.trim_end_matches('/');
    if prefix.is_empty() {
        return true;
    }
    if path.eq_ignore_ascii_case(prefix) {
        return true;
    }
    // Byte-level check sidesteps any UTF-8 boundary risk if a path
    // contains multibyte chars. We're comparing ASCII, so this is
    // safe and equivalent to the str form.
    let pb = prefix.as_bytes();
    let path_b = path.as_bytes();
    if path_b.len() < pb.len() + 1 {
        return false;
    }
    path_b[..pb.len()].eq_ignore_ascii_case(pb) && path_b[pb.len()] == b'/'
}

#[cfg(test)]
mod cap_err_tests {
    use super::map_cap_err;
    use crate::error::ChanError;
    use std::io;
    use std::path::Path;

    /// Pin the cap-std error-string match. If cap-std ever rewords
    /// the message we use to detect a sandbox escape, this test
    /// fails and we fix the matcher in `map_cap_err`. Without this
    /// pin a silent regression would let "you tried to escape" land
    /// as a generic `Io` and break `ChanError::SymlinkEscape`
    /// callers.
    #[test]
    fn maps_cap_std_escape_message_to_symlink_escape() {
        let e = io::Error::other("a path led outside of the filesystem");
        let mapped = map_cap_err(e, Path::new("notes/x.md"));
        assert!(matches!(mapped, ChanError::SymlinkEscape(_)));
    }

    #[test]
    fn maps_other_io_errors_passthrough_to_io() {
        let e = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
        let mapped = map_cap_err(e, Path::new("notes/x.md"));
        assert!(matches!(mapped, ChanError::Io(_)));
    }
}

#[cfg(test)]
mod path_under_tests {
    use super::path_under;

    #[test]
    fn matches_exact_and_descendant() {
        assert!(path_under("notes", "notes"));
        assert!(path_under("notes/a.md", "notes"));
        assert!(path_under("notes/sub/a.md", "notes"));
    }

    #[test]
    fn empty_prefix_matches_anything() {
        assert!(path_under("any/thing.md", ""));
        assert!(path_under("any/thing.md", "/"));
    }

    #[test]
    fn rejects_sibling_prefix_share() {
        // "notes-archive" must not match scope "notes".
        assert!(!path_under("notes-archive/a.md", "notes"));
    }

    #[test]
    fn case_insensitive_ascii() {
        assert!(path_under("Notes/a.md", "notes"));
        assert!(path_under("notes/a.md", "NOTES"));
        assert!(path_under("Notes/Sub/a.md", "notes/sub"));
    }

    #[test]
    fn trailing_slash_on_prefix_is_normalized() {
        assert!(path_under("notes/a.md", "notes/"));
        assert!(path_under("notes", "notes/"));
    }

    #[test]
    fn does_not_panic_on_multibyte_paths() {
        // Real-world: a folder named with non-ASCII. The byte-level
        // boundary check shouldn't trip.
        assert!(path_under("Café/a.md", "Café"));
        assert!(!path_under("Other/a.md", "Café"));
    }
}

/// Convert links + tokens into graph edges. Wiki links and
/// internal markdown links produce `Link` edges; tokens produce
/// `Tag` / `Mention` edges. External links (http://, mailto:) are
/// dropped because they don't connect to anything else in the
/// drive's graph.
///
/// Markdown link hrefs (`[label](href)`) and image embeds
/// (`![alt](src)`) are run through `markdown::normalize_href` so
/// `/abs` and `../rel` write the same drive-relative dst as the
/// equivalent bare path. Wiki-link targets (`[[name]]`) keep the
/// existing drive-rooted-by-default convention; an explicit `./`
/// or `..` prefix opts into source-relative resolution.
fn build_edges(
    src: &str,
    links: &[markdown::Link],
    tokens: &[markdown::Token],
) -> Vec<crate::graph::Edge> {
    use crate::graph::{Edge, EdgeKind};
    let source_dir = match src.rfind('/') {
        Some(i) => &src[..i],
        None => "",
    };
    let mut out = Vec::new();
    for l in links {
        // Wiki-link convention: bare `[[name]]` and `[[a/b]]` are
        // drive-rooted (the picker has always inserted them this
        // way). An explicit `./` / `..` prefix flips to source-
        // relative. Markdown links use standard relative semantics.
        let normalize_target = if l.wiki && !is_relative_marker(&l.target) {
            format!("/{}", l.target.trim_start_matches('/'))
        } else {
            l.target.clone()
        };
        let Some(dst) = markdown::normalize_href(&normalize_target, source_dir) else {
            continue;
        };
        // Anchors only mean something for markdown targets (heading
        // refs like `note.md#section`). For images/PDFs/etc. the `#`
        // suffix is an Obsidian-style param (`img.png#width=300`),
        // not a heading anchor, so drop it.
        let (_, anchor) = split_anchor(&l.target);
        let anchor = if dst.ends_with(".md") { anchor } else { None };
        out.push(Edge {
            src: src.to_string(),
            dst,
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

/// True when a wiki-link target opts into source-relative resolution
/// via a leading `./` or `..` segment. Plain `[[name]]` and `[[a/b]]`
/// stay drive-rooted (the picker's existing convention).
fn is_relative_marker(target: &str) -> bool {
    target == "." || target == ".." || target.starts_with("./") || target.starts_with("../")
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
        assert!(stat.mtime_ns.is_some());
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
                current_mtime_ns: Some(_)
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
                current_mtime_ns: None
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
            .write_text_if_unchanged("a.md", stat.mtime_ns, "v2")
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
            ChanError::WriteConflict { current_mtime_ns } => {
                assert!(current_mtime_ns.is_some());
                assert_ne!(current_mtime_ns, stale);
            }
            other => panic!("expected WriteConflict, got {other:?}"),
        }
        assert_eq!(drive.read_text("a.md").unwrap(), "v1");
    }

    /// Two saves landing within the same wall-clock second on a
    /// nanosecond-resolution filesystem must still produce a conflict
    /// when the second writer presents the first writer's stat token.
    /// Filesystems without ns mtime degrade to seconds; on those the
    /// assertion may not exercise the new precision, so we only run
    /// it when we can observe distinct ns values from back-to-back
    /// writes.
    #[test]
    fn write_text_if_unchanged_detects_subsecond_conflict() {
        let (_cfg, _root, drive) = fixture();
        drive.write_text("a.md", "v1").unwrap();
        let stale_ns = drive.stat("a.md").unwrap().mtime_ns;
        // Tight loop until mtime_ns advances. On filesystems with
        // only seconds resolution this would spin until the next
        // second boundary; cap at 200ms.
        let start = std::time::Instant::now();
        loop {
            drive.write_text("a.md", "v2").unwrap();
            let now_ns = drive.stat("a.md").unwrap().mtime_ns;
            if now_ns != stale_ns {
                break;
            }
            if start.elapsed() > std::time::Duration::from_millis(200) {
                // FS likely lacks sub-second resolution; skip rather
                // than spin into the next wall-clock second.
                return;
            }
        }
        // Now an attempt to write back with the original (pre-v2)
        // stat must conflict. Without ns precision, two same-second
        // writes would collide and let this through.
        let err = drive
            .write_text_if_unchanged("a.md", stale_ns, "v3")
            .unwrap_err();
        assert!(matches!(err, ChanError::WriteConflict { .. }));
        assert_eq!(drive.read_text("a.md").unwrap(), "v2");
    }

    #[test]
    fn write_bytes_allows_binary() {
        let (_cfg, _root, drive) = fixture();
        drive.write_bytes("img.png", &[0xff, 0xd8, 0xff]).unwrap();
        assert_eq!(drive.read("img.png").unwrap(), vec![0xff, 0xd8, 0xff]);
    }

    #[test]
    fn write_text_rejects_oversize_content_for_new_file() {
        let (_cfg, _root, drive) = fixture();
        // One byte over the cap. Allocating 2 MiB+1 is fine; the
        // guard rejects before any I/O.
        let big = "x".repeat(TEXT_WRITE_LIMIT as usize + 1);
        let err = drive.write_text("a.md", &big).unwrap_err();
        match err {
            ChanError::WriteTooLarge { kind, size, limit } => {
                assert_eq!(kind, "text");
                assert_eq!(limit, TEXT_WRITE_LIMIT);
                assert_eq!(size, TEXT_WRITE_LIMIT + 1);
            }
            other => panic!("unexpected error: {other:?}"),
        }
        assert!(!drive.exists("a.md"));
    }

    #[test]
    fn write_bytes_rejects_oversize_content_for_new_file() {
        let (_cfg, _root, drive) = fixture();
        // 50 MiB+1 byte. Heap-alloc once; cheap.
        let big = vec![0u8; BYTES_WRITE_LIMIT as usize + 1];
        let err = drive.write_bytes("blob.bin", &big).unwrap_err();
        assert!(matches!(
            err,
            ChanError::WriteTooLarge { kind: "bytes", .. }
        ));
        assert!(!drive.exists("blob.bin"));
    }

    /// A pre-cap file (or a binary mistakenly named `.md`) larger
    /// than the configured limit must remain editable: writes up to
    /// its current size go through, only growth beyond it is
    /// rejected. Without this rule, shipping the cap would silently
    /// turn every legacy big file read-only on next save.
    #[test]
    fn write_text_allows_edits_to_legacy_oversize_file() {
        let (_cfg, root, drive) = fixture();
        // Plant a 3 MiB file directly via std (bypasses the cap).
        let path = root.path().join("legacy.md");
        let big = "y".repeat(TEXT_WRITE_LIMIT as usize + 1024 * 1024);
        std::fs::write(&path, &big).unwrap();
        // Editing the file at the same size succeeds.
        let same_size = "z".repeat(big.len());
        drive.write_text("legacy.md", &same_size).unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().len() as usize, big.len());
        // Shrinking succeeds (well within max(prev, limit)).
        drive.write_text("legacy.md", "shrunk").unwrap();
        assert_eq!(drive.read_text("legacy.md").unwrap(), "shrunk");
    }

    /// Growing a legacy oversize file past its current size IS
    /// rejected: the effective limit is max(prev_size, configured
    /// limit), so a 3 MiB file caps at 3 MiB on the next write.
    #[test]
    fn write_text_rejects_growth_past_legacy_size() {
        let (_cfg, root, drive) = fixture();
        let path = root.path().join("legacy.md");
        let prev = "y".repeat(TEXT_WRITE_LIMIT as usize + 1024);
        std::fs::write(&path, &prev).unwrap();
        // One byte over the existing size, well above the configured cap.
        let grown = "z".repeat(prev.len() + 1);
        let err = drive.write_text("legacy.md", &grown).unwrap_err();
        match err {
            ChanError::WriteTooLarge { limit, size, .. } => {
                assert_eq!(limit, prev.len() as u64, "effective limit = prev size");
                assert_eq!(size, grown.len() as u64);
            }
            other => panic!("unexpected error: {other:?}"),
        }
        // File on disk unchanged.
        assert_eq!(std::fs::metadata(&path).unwrap().len() as usize, prev.len());
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

    /// Defensive: if the registered drive path has been replaced by
    /// a symlink (or a regular file) between registration and the
    /// next open, refuse rather than carry on as if it were still a
    /// real directory.
    #[cfg(unix)]
    #[test]
    fn open_refuses_when_root_is_symlink() {
        use std::os::unix::fs::symlink;
        let cfg = TempDir::new().unwrap();
        let real = TempDir::new().unwrap();
        let staging = TempDir::new().unwrap();
        // Register a real directory ...
        let registered_path = staging.path().join("drive");
        std::fs::create_dir(&registered_path).unwrap();
        let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_drive(&registered_path, None).unwrap();
        // ... then swap it for a symlink to a different directory.
        std::fs::remove_dir(&registered_path).unwrap();
        symlink(real.path(), &registered_path).unwrap();
        let err = lib.open_drive(&registered_path).unwrap_err();
        assert!(matches!(err, ChanError::SpecialFile { .. }));
    }

    #[test]
    fn open_refuses_when_root_is_regular_file() {
        let cfg = TempDir::new().unwrap();
        let staging = TempDir::new().unwrap();
        let registered_path = staging.path().join("drive");
        std::fs::create_dir(&registered_path).unwrap();
        let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_drive(&registered_path, None).unwrap();
        // Replace the directory with a regular file.
        std::fs::remove_dir(&registered_path).unwrap();
        std::fs::write(&registered_path, b"not a drive").unwrap();
        let err = lib.open_drive(&registered_path).unwrap_err();
        assert!(matches!(err, ChanError::SpecialFile { .. }));
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

    /// The graph row drives the kind. An unindexed file still
    /// resolves (we found it on disk) but the kind defaults to
    /// `File` so the editor can render a generic doc pill while the
    /// indexer catches up.
    #[test]
    fn resolve_link_kind_defaults_to_file_when_unindexed() {
        let (_cfg, _root, drive) = link_fixture();
        let r = drive.resolve_link("recipes/pasta").unwrap();
        assert_eq!(r.kind, crate::graph::NodeKind::File);
    }

    /// After indexing a contact-frontmatter file, resolve_link's kind
    /// matches what the picker put in the graph. This is the path
    /// that drives the editor's kind-aware pill rendering.
    #[test]
    fn resolve_link_returns_contact_kind_for_contact_node() {
        let (_cfg, root, drive) = link_fixture();
        std::fs::create_dir_all(root.path().join("Contacts")).unwrap();
        let contact = "---\nchan:\n  kind: contact\n---\n\n# Alice Anderson\n\n- **Email**: alice@example.com\n";
        std::fs::write(root.path().join("Contacts").join("Alice.md"), contact).unwrap();
        drive.index_file("Contacts/Alice.md").unwrap();
        let r = drive.resolve_link("Contacts/Alice").unwrap();
        assert_eq!(r.path, "Contacts/Alice.md");
        assert_eq!(r.kind, crate::graph::NodeKind::Contact);
    }

    /// Plain markdown files index as `NodeKind::File`; round-trip
    /// resolve_link to confirm the kind reflects the indexed value
    /// (not a constant default).
    #[test]
    fn resolve_link_returns_file_kind_for_plain_note() {
        let (_cfg, _root, drive) = link_fixture();
        drive.index_file("recipes/pasta.md").unwrap();
        let r = drive.resolve_link("recipes/pasta").unwrap();
        assert_eq!(r.kind, crate::graph::NodeKind::File);
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

    // ---- build_edges (link normalization) ----

    fn md_link(target: &str) -> markdown::Link {
        markdown::Link {
            target: target.to_string(),
            label: None,
            wiki: false,
        }
    }

    fn wiki_link(target: &str) -> markdown::Link {
        markdown::Link {
            target: target.to_string(),
            label: None,
            wiki: true,
        }
    }

    fn dsts(edges: &[crate::graph::Edge]) -> Vec<&str> {
        edges.iter().map(|e| e.dst.as_str()).collect()
    }

    #[test]
    fn build_edges_normalizes_drive_rooted_markdown_link() {
        // `[link](/images/foo.png)` from notes/post.md should land
        // at dst=images/foo.png, not /images/foo.png.
        let edges = build_edges("notes/post.md", &[md_link("/images/foo.png")], &[]);
        assert_eq!(dsts(&edges), vec!["images/foo.png"]);
    }

    #[test]
    fn build_edges_normalizes_parent_relative_markdown_link() {
        // `[link](../images/foo.png)` from notes/post.md collapses
        // lexically to images/foo.png.
        let edges = build_edges("notes/post.md", &[md_link("../images/foo.png")], &[]);
        assert_eq!(dsts(&edges), vec!["images/foo.png"]);
    }

    #[test]
    fn build_edges_skips_external_markdown_link() {
        let edges = build_edges("notes/post.md", &[md_link("https://example.com/x")], &[]);
        assert!(edges.is_empty());
    }

    #[test]
    fn build_edges_skips_fragment_only_link() {
        let edges = build_edges("notes/post.md", &[md_link("#section")], &[]);
        assert!(edges.is_empty());
    }

    #[test]
    fn build_edges_skips_drive_escape() {
        // `../../x.md` from a depth-1 file pops past the drive root.
        let edges = build_edges("notes/post.md", &[md_link("../../x.md")], &[]);
        assert!(edges.is_empty());
    }

    #[test]
    fn build_edges_preserves_anchor_column() {
        let edges = build_edges("notes/post.md", &[md_link("/a.md#sec")], &[]);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].dst, "a.md");
        assert_eq!(edges[0].anchor.as_deref(), Some("sec"));
    }

    #[test]
    fn build_edges_drops_anchor_for_non_markdown_target() {
        // `![alt](img.png#width=300)` is the Obsidian image-sizing
        // syntax; the `#...` suffix is a render hint, not a heading
        // anchor. The graph edge must point at `img.png` with an
        // empty anchor so the inspector's backlinks query and the
        // anchor column itself stay semantically clean.
        let edges = build_edges("notes/post.md", &[md_link("img.png#width=300")], &[]);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].dst, "notes/img.png");
        assert!(edges[0].anchor.is_none(), "got: {:?}", edges[0].anchor);
    }

    #[test]
    fn build_edges_wiki_default_drive_rooted() {
        // Plain `[[Contacts/Jane Doe]]` from any source dir resolves
        // to the drive root. Matches the picker's existing insertion
        // form and keeps the smoke-test invariant.
        let edges = build_edges("notes/post.md", &[wiki_link("Contacts/Jane Doe")], &[]);
        assert_eq!(dsts(&edges), vec!["Contacts/Jane Doe"]);
    }

    #[test]
    fn build_edges_wiki_explicit_absolute() {
        let edges = build_edges("notes/post.md", &[wiki_link("/Contacts/Jane Doe")], &[]);
        assert_eq!(dsts(&edges), vec!["Contacts/Jane Doe"]);
    }

    #[test]
    fn build_edges_wiki_relative_walks_up() {
        // `[[../foo]]` from notes/post.md walks up to drive root.
        let edges = build_edges("notes/post.md", &[wiki_link("../foo")], &[]);
        assert_eq!(dsts(&edges), vec!["foo"]);
    }

    #[test]
    fn build_edges_wiki_dot_relative_resolves_to_source_dir() {
        let edges = build_edges("notes/post.md", &[wiki_link("./sibling")], &[]);
        assert_eq!(dsts(&edges), vec!["notes/sibling"]);
    }
}
