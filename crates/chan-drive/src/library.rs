// Library: top-level handle. Owns the registry persisted at
// ~/.chan/config.toml and resolves OS state/cache paths.
//
// In practice apps create one Library at startup and keep it
// alive. Drives are opened against it. Cheap to clone (Arc inside).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};

use serde::{Deserialize, Serialize};

use crate::drive::Drive;
use crate::error::{ChanError, Result};
use crate::fs_ops::WalkFilter;
use crate::lock::DriveLock;
use crate::paths;
use crate::registry::{KnownDrive, Registry};

/// Selects how aggressive `Library::reset_drive` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResetMode {
    /// Wipe per-drive chan-managed state (search index, graph DB,
    /// session blobs, assistant blobs, app tokens). Keep the
    /// registry entry, the user's notes tree, and the trash.
    State,
    /// `State` plus drop the registry entry. The next `open_drive`
    /// against this path treats it as a fresh, never-seen drive.
    Everything,
}

/// What `Library::reset_drive` removed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetReport {
    /// Total file + subdirectory entries removed across the wiped
    /// state directories. Useful as a "removed N items" toast.
    pub removed_entries: usize,
}

/// Per-machine handle to the chan-drive registry + paths.
#[derive(Clone)]
pub struct Library {
    inner: Arc<LibraryInner>,
}

struct LibraryInner {
    config_path: PathBuf,
    /// In-memory registry. Persisted to `config_path` on every
    /// mutation. The Mutex serializes registry writes so
    /// `register_drive` calls from concurrent threads don't race.
    registry: Mutex<Registry>,
    /// Caller-supplied directory-name blocklist for indexing walks.
    /// Default is empty in chan-core; the chan binary populates it
    /// from its config so noise dirs (`node_modules`, `target`, ...)
    /// are pruned before the indexer descends into them. The Mutex
    /// lets the consumer swap the filter at runtime (e.g. after the
    /// user edits chan's config). Drives capture a snapshot at
    /// `open_drive` time; a swap here affects subsequent opens and
    /// the next reindex on already-open drives that re-read it.
    walk_filter: Mutex<Arc<WalkFilter>>,
    /// In-process map of currently-open Drives, keyed by canonical
    /// path. Each entry is a `Weak<Drive>` so the map doesn't
    /// keep drives alive past the caller's last `Arc`. The
    /// per-drive flock already prevents two processes (or two
    /// concurrent opens in this process) from racing on disk; the
    /// map adds two things on top:
    ///
    ///   1. A clearer in-process error: `DriveAlreadyOpen` instead
    ///      of `DriveLocked`. The latter implies cross-process
    ///      contention, which would mislead a developer who is
    ///      really fighting their own forgotten `Arc`.
    ///   2. Defense-in-depth on filesystems where flock is
    ///      unreliable (NFS-mounted state_dir, certain SMB
    ///      configurations). Even if the kernel-side lock is a
    ///      no-op, the in-process map still serializes within a
    ///      single Library handle.
    ///
    /// Dead entries (Weak that no longer upgrades) are GC'd lazily
    /// on every map access; no background thread.
    live_drives: Mutex<HashMap<PathBuf, Weak<Drive>>>,
}

impl Library {
    /// Open the default Library at `~/.chan/config.toml`. Creates
    /// the parent directory lazily on first save.
    pub fn open() -> Result<Self> {
        Self::open_at(paths::global_config_path())
    }

    /// Open a Library against an explicit config path. Used in
    /// tests and by callers that want a non-default location.
    pub fn open_at(config_path: PathBuf) -> Result<Self> {
        let registry = Registry::load_from(&config_path)?;
        Ok(Self {
            inner: Arc::new(LibraryInner {
                config_path,
                registry: Mutex::new(registry),
                live_drives: Mutex::new(HashMap::new()),
                walk_filter: Mutex::new(Arc::new(WalkFilter::default())),
            }),
        })
    }

    /// Replace the directory-name blocklist applied to reindex
    /// walks for drives opened against this Library. Empty filter
    /// is the chan-core default (only `.git` / `.chan` are skipped,
    /// hardcoded in `walk_drive`). The chan binary calls this once
    /// at startup with its noise list (`node_modules`, `target`,
    /// `__pycache__`, ...) so the indexer never wastes cycles on
    /// dependency directories. Live drives that re-read the filter
    /// on their next reindex pick up the change; in-flight reindexes
    /// keep their snapshot (no mid-walk reconfiguration).
    pub fn set_walk_filter(&self, filter: WalkFilter) {
        *self.inner.walk_filter.lock().unwrap() = Arc::new(filter);
    }

    /// Snapshot of the current filter. Cheap clone (Arc).
    pub fn walk_filter(&self) -> Arc<WalkFilter> {
        Arc::clone(&self.inner.walk_filter.lock().unwrap())
    }

    /// Snapshot of all registered drives, most-recent first.
    pub fn list_drives(&self) -> Vec<KnownDrive> {
        self.inner.registry.lock().unwrap().drives.clone()
    }

    /// Configured default drive root, if any.
    pub fn default_drive_root(&self) -> Option<PathBuf> {
        self.inner
            .registry
            .lock()
            .unwrap()
            .default_drive_root
            .clone()
    }

    /// Set or clear the configured default drive root. Persists.
    pub fn set_default_drive_root(&self, root: Option<PathBuf>) -> Result<()> {
        let mut reg = self.inner.registry.lock().unwrap();
        reg.default_drive_root = root;
        reg.save_to(&self.inner.config_path)
    }

    /// Effective default drive root: explicit override wins,
    /// otherwise the platform convention.
    pub fn effective_default_drive_root(&self) -> PathBuf {
        self.default_drive_root()
            .unwrap_or_else(paths::default_drive_root)
    }

    /// Add a drive to the registry. Idempotent: re-registering an
    /// existing drive only updates `last_opened`, never the name.
    /// Use `rename_drive` for explicit name changes. The directory
    /// itself is NOT created here; pass a path that already exists.
    pub fn register_drive(&self, root: &Path, name: Option<String>) -> Result<KnownDrive> {
        if !root.exists() {
            return Err(ChanError::DriveRootMissing(root.to_path_buf()));
        }
        let mut reg = self.inner.registry.lock().unwrap();
        let idx = reg.touch(root, name);
        let entry = reg.drives[idx].clone();
        reg.save_to(&self.inner.config_path)?;
        Ok(entry)
    }

    /// Drop a drive from the registry. Does not delete the
    /// directory or per-drive state on disk.
    pub fn unregister_drive(&self, root: &Path) -> Result<bool> {
        let mut reg = self.inner.registry.lock().unwrap();
        let removed = reg.remove(root);
        if removed {
            reg.save_to(&self.inner.config_path)?;
        }
        Ok(removed)
    }

    /// Set the display name on a registered drive.
    pub fn rename_drive(&self, root: &Path, name: Option<String>) -> Result<bool> {
        let mut reg = self.inner.registry.lock().unwrap();
        let ok = reg.set_name(root, name);
        if ok {
            reg.save_to(&self.inner.config_path)?;
        }
        Ok(ok)
    }

    /// Open a drive handle. The drive must already be registered;
    /// callers do `register_drive` first if needed (CLI does both
    /// in one shot for the "point at a folder and go" path).
    pub fn open_drive(&self, root: &Path) -> Result<Arc<Drive>> {
        let reg = self.inner.registry.lock().unwrap();
        let entry = reg
            .find(root)
            .ok_or_else(|| ChanError::DriveNotRegistered(root.to_path_buf()))?
            .clone();
        drop(reg);
        let key = canonical_key(&entry.path);
        // In-process pre-check: if we still hold an open handle to
        // this drive, return DriveAlreadyOpen rather than letting
        // the cross-process flock surface as DriveLocked. The lock
        // on `live_drives` is held only across the upgrade probe;
        // we drop it before calling Drive::open so a slow open
        // (canonicalize on a cloud root, lazy index init) never
        // blocks unrelated drives from registering / listing.
        {
            let mut map = self.inner.live_drives.lock().unwrap();
            gc_dead_entries(&mut map);
            if let Some(weak) = map.get(&key) {
                if weak.upgrade().is_some() {
                    return Err(ChanError::DriveAlreadyOpen);
                }
            }
        }
        let filter = Arc::clone(&self.inner.walk_filter.lock().unwrap());
        let drive = Drive::open(entry, filter)?;
        self.inner
            .live_drives
            .lock()
            .unwrap()
            .insert(key, Arc::downgrade(&drive));
        Ok(drive)
    }

    /// Wipe per-drive chan-managed state for `root`. The user's
    /// notes tree is never touched (chan-drive never writes inside
    /// it). The trash is preserved (it holds user-deleted files,
    /// recoverable user data). The lock dir is preserved (it holds
    /// no data, only cross-process coordination).
    ///
    /// Wipe set:
    ///   - search index (`<cache>/chan/index/<key>/`)
    ///   - graph DB and sqlite sidecars (`<state>/chan/graph/<key>/`)
    ///   - session blobs (`<state>/chan/sessions/<key>/`)
    ///   - assistant blobs (`<state>/chan/assistant/<key>/`)
    ///   - app tokens (`<state>/chan/tokens/<key>/`)
    ///
    /// `ResetMode::Everything` additionally drops the registry
    /// entry so the next `open_drive` treats this path as fresh.
    ///
    /// Preconditions:
    ///   - The caller MUST drop any open `Arc<Drive>` for `root`
    ///     before calling. We acquire the writer lock briefly to
    ///     verify exclusive access; if any process (including this
    ///     one) holds it, we fail with `ChanError::DriveLocked`.
    ///   - On Unix this is mostly defense-in-depth (open files
    ///     survive unlink). On Windows the lock check is load-
    ///     bearing because removing files-in-use fails.
    ///
    /// Idempotent: calling on a never-opened drive (no state dirs
    /// on disk) returns `removed_entries = 0` without erroring.
    /// Re-creation of the skeleton happens lazily on the next
    /// `open_drive` + first `index()` / `graph()` access.
    pub fn reset_drive(&self, root: &Path, mode: ResetMode) -> Result<ResetReport> {
        self.reset_drive_with(root, mode, &crate::progress::NoProgress)
    }

    /// `reset_drive` plus a `ProgressCallback`. Fires one
    /// `ProgressStage::Reset` event per subsystem (index, graph,
    /// sessions, assistant, tokens) as it is wiped, so a UI can
    /// surface "wiping <subsystem>..." without instrumenting each
    /// caller. The label carries the subsystem name; `current` /
    /// `total` count through the fixed five-subsystem list.
    pub fn reset_drive_with(
        &self,
        root: &Path,
        mode: ResetMode,
        progress: &dyn crate::progress::ProgressCallback,
    ) -> Result<ResetReport> {
        use crate::progress::{ProgressEvent, ProgressStage};
        // In-process pre-check: a buggy caller might hold a Drive
        // and call reset_drive from another thread, expecting the
        // flock to serialize. It does (DriveLock::acquire below
        // would fail with DriveLocked), but the clearer error tells
        // the developer they're racing themselves rather than a
        // mystery second process. Cross-process safety still rides
        // on the flock.
        let key = canonical_key(root);
        {
            let mut map = self.inner.live_drives.lock().unwrap();
            gc_dead_entries(&mut map);
            if let Some(weak) = map.get(&key) {
                if weak.upgrade().is_some() {
                    return Err(ChanError::DriveAlreadyOpen);
                }
            }
        }
        let drive_paths = paths::drive_paths(root);
        let _lock = DriveLock::acquire(&drive_paths.lock)?;
        let mut removed = 0;
        let subsystems: [(&str, &Path); 5] = [
            ("index", &drive_paths.index),
            ("graph", &drive_paths.graph_dir),
            ("sessions", &drive_paths.sessions),
            ("assistant", &drive_paths.assistant),
            ("tokens", &drive_paths.tokens),
        ];
        let total = subsystems.len() as u64;
        for (idx, (name, dir)) in subsystems.iter().enumerate() {
            progress.on_progress(ProgressEvent {
                stage: ProgressStage::Reset,
                current: idx as u64,
                total,
                label: Some((*name).to_string()),
                eta_secs: None,
            });
            removed += wipe_dir(dir)?;
        }
        // Hold the writer lock across the registry update so a
        // concurrent open_drive cannot lazily recreate the state we
        // just wiped, lazily commit a half-formed index/graph dir,
        // and then notice its registry entry has been dropped. The
        // registry mutex composes cleanly here: it's a lock we own,
        // the flock is process-wide, and no path acquires them in
        // the opposite order. _lock is dropped at the end of the
        // function after the registry write completes.
        if matches!(mode, ResetMode::Everything) {
            let mut reg = self.inner.registry.lock().unwrap();
            if reg.remove(root) {
                reg.save_to(&self.inner.config_path)?;
            }
        }
        Ok(ResetReport {
            removed_entries: removed,
        })
    }
}

/// Canonical-form key for the live-drives map. Falls back to the
/// input path when the filesystem can't canonicalize (drive root
/// missing or asleep), so the map still tracks "this exact request
/// path" through the rest of the operation.
fn canonical_key(root: &Path) -> PathBuf {
    root.canonicalize().unwrap_or_else(|_| root.to_path_buf())
}

/// Drop dead entries from the live-drives map. A `Weak<Drive>`
/// whose Arc has been dropped will fail to upgrade; we remove it
/// so the map's footprint stays bounded by the actually-open
/// drives, not by every drive ever opened in the process.
fn gc_dead_entries(map: &mut HashMap<PathBuf, Weak<Drive>>) {
    map.retain(|_, w| w.strong_count() > 0);
}

/// Recursively delete `dir` and return the number of entries
/// (files + subdirectories, not counting `dir` itself) that were
/// inside it. Missing dir contributes 0.
fn wipe_dir(dir: &Path) -> Result<usize> {
    if !dir.exists() {
        return Ok(0);
    }
    let count = walkdir::WalkDir::new(dir)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .count();
    std::fs::remove_dir_all(dir)?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn lib() -> (Library, TempDir, TempDir) {
        let cfg = TempDir::new().unwrap();
        let drive = TempDir::new().unwrap();
        let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
        (lib, cfg, drive)
    }

    #[test]
    fn register_then_list() {
        let (lib, _cfg, drive) = lib();
        lib.register_drive(drive.path(), Some("Notes".into()))
            .unwrap();
        let drives = lib.list_drives();
        assert_eq!(drives.len(), 1);
        assert_eq!(drives[0].name.as_deref(), Some("Notes"));
    }

    #[test]
    fn register_missing_path_errors() {
        let (lib, _cfg, _drive) = lib();
        let bogus = std::path::PathBuf::from("/nonexistent/path/to/nowhere/12345");
        let err = lib.register_drive(&bogus, None).unwrap_err();
        assert!(matches!(err, ChanError::DriveRootMissing(_)));
    }

    #[test]
    fn unregister_returns_false_when_absent() {
        let (lib, _cfg, drive) = lib();
        assert!(!lib.unregister_drive(drive.path()).unwrap());
    }

    #[test]
    fn rename_persists() {
        let (lib, _cfg, drive) = lib();
        lib.register_drive(drive.path(), None).unwrap();
        assert!(lib
            .rename_drive(drive.path(), Some("Renamed".into()))
            .unwrap());
        assert_eq!(lib.list_drives()[0].name.as_deref(), Some("Renamed"));
    }

    #[test]
    fn default_drive_root_round_trip() {
        let (lib, _cfg, drive) = lib();
        lib.set_default_drive_root(Some(drive.path().to_path_buf()))
            .unwrap();
        assert_eq!(lib.default_drive_root(), Some(drive.path().to_path_buf()));
        lib.set_default_drive_root(None).unwrap();
        assert!(lib.default_drive_root().is_none());
    }

    #[test]
    fn walk_filter_excludes_dir_from_reindex() {
        // Library-set filter must reach the indexer: a `node_modules`
        // directory under the drive should not show up in the search
        // index even when it contains markdown. The editor's file
        // tree still sees it (list_tree is unfiltered) so the user
        // can open files there on demand.
        use crate::SearchMode;
        let (lib, _cfg, drive) = lib();
        lib.set_walk_filter(WalkFilter::new(["node_modules"]));
        lib.register_drive(drive.path(), None).unwrap();
        std::fs::create_dir_all(drive.path().join("notes")).unwrap();
        std::fs::write(
            drive.path().join("notes/a.md"),
            "# alpha\nfoo unique-keep-token bar\n",
        )
        .unwrap();
        std::fs::create_dir_all(drive.path().join("node_modules/pkg")).unwrap();
        std::fs::write(
            drive.path().join("node_modules/pkg/README.md"),
            "# junk\nbaz unique-skip-token qux\n",
        )
        .unwrap();
        let d = lib.open_drive(drive.path()).unwrap();
        d.reindex(None).unwrap();
        let opts = crate::drive::SearchOpts {
            mode: SearchMode::Bm25,
            limit: 10,
            scope: None,
        };
        let kept = d.search("unique-keep-token", &opts).unwrap();
        assert_eq!(kept.hits.len(), 1, "kept file should be indexed");
        let skipped = d.search("unique-skip-token", &opts).unwrap();
        assert!(
            skipped.hits.is_empty(),
            "skipped file should not be indexed; got {:?}",
            skipped.hits
        );
        // list_tree must still surface the noise dir so the editor's
        // tree view doesn't lie about what's on disk.
        let entries = d.list_tree().unwrap();
        assert!(entries.iter().any(|e| e.path.starts_with("node_modules")));
    }

    #[test]
    fn open_unregistered_errors() {
        let (lib, _cfg, drive) = lib();
        let err = lib.open_drive(drive.path()).unwrap_err();
        assert!(matches!(err, ChanError::DriveNotRegistered(_)));
    }

    /// Populate per-drive state so we have something to wipe:
    /// reindex (creates index segments + graph DB), put a session
    /// blob, put an assistant blob, drop a fake token. Also writes
    /// a markdown file inside the drive so the test can verify
    /// reset doesn't touch the user's notes.
    fn populate_state(lib: &Library, root: &Path) {
        let drive = lib.open_drive(root).unwrap();
        drive
            .write_text("notes/keep.md", "kept across reset")
            .unwrap();
        drive.reindex(None).unwrap();
        drive.put_session("win-1", b"layout").unwrap();
        drive.put_assistant("conv-a", b"chat").unwrap();
        let p = drive.paths();
        std::fs::create_dir_all(&p.tokens).unwrap();
        std::fs::write(p.tokens.join("server.token"), b"deadbeef").unwrap();
    }

    fn paths_of(root: &Path) -> paths::DrivePaths {
        paths::drive_paths(root)
    }

    #[test]
    fn reset_state_wipes_chan_state_and_keeps_user_notes_and_registry() {
        let (lib, _cfg, drive) = lib();
        lib.register_drive(drive.path(), Some("Notes".into()))
            .unwrap();
        populate_state(&lib, drive.path());

        let p = paths_of(drive.path());
        // Sanity: state dirs populated.
        assert!(p.index.exists());
        assert!(p.graph_db.exists());
        assert!(p.sessions.exists());
        assert!(p.assistant.exists());
        assert!(p.tokens.exists());

        let report = lib.reset_drive(drive.path(), ResetMode::State).unwrap();
        assert!(report.removed_entries > 0);

        // State dirs gone.
        assert!(!p.index.exists());
        assert!(!p.graph_db.parent().unwrap().exists());
        assert!(!p.sessions.exists());
        assert!(!p.assistant.exists());
        assert!(!p.tokens.exists());

        // User's notes and the registry survive.
        assert!(drive.path().join("notes/keep.md").exists());
        let drives = lib.list_drives();
        assert_eq!(drives.len(), 1);
        assert_eq!(drives[0].name.as_deref(), Some("Notes"));
    }

    #[test]
    fn reset_everything_also_drops_registry_entry() {
        let (lib, _cfg, drive) = lib();
        lib.register_drive(drive.path(), None).unwrap();
        populate_state(&lib, drive.path());

        lib.reset_drive(drive.path(), ResetMode::Everything)
            .unwrap();

        assert!(lib.list_drives().is_empty());
        // User's notes still survive (chan-drive never owns them).
        assert!(drive.path().join("notes/keep.md").exists());
    }

    #[test]
    fn reset_drive_rejects_when_drive_is_open_in_process() {
        let (lib, _cfg, drive) = lib();
        lib.register_drive(drive.path(), None).unwrap();
        let _open = lib.open_drive(drive.path()).unwrap();
        // In-process pre-check fires first: clearer error than the
        // cross-process flock would surface, since we know we're
        // racing ourselves rather than another process.
        let err = lib.reset_drive(drive.path(), ResetMode::State).unwrap_err();
        assert!(matches!(err, ChanError::DriveAlreadyOpen));
    }

    #[test]
    fn reset_drive_returns_locked_when_other_process_holds_lock() {
        // Hand-crafted second Library handle on the same config to
        // simulate another process: each Library has its own
        // live_drives map, so the in-process check on `lib2`
        // doesn't fire, and we hit the flock instead.
        let (lib, cfg, drive) = lib();
        lib.register_drive(drive.path(), None).unwrap();
        let _open = lib.open_drive(drive.path()).unwrap();
        let lib2 = Library::open_at(cfg.path().join("config.toml")).unwrap();
        let err = lib2
            .reset_drive(drive.path(), ResetMode::State)
            .unwrap_err();
        assert!(matches!(err, ChanError::DriveLocked));
    }

    #[test]
    fn second_open_in_same_process_returns_already_open() {
        let (lib, _cfg, drive) = lib();
        lib.register_drive(drive.path(), None).unwrap();
        let first = lib.open_drive(drive.path()).unwrap();
        let err = lib.open_drive(drive.path()).unwrap_err();
        assert!(matches!(err, ChanError::DriveAlreadyOpen));
        // Once the first handle is dropped, the second open succeeds.
        drop(first);
        let _second = lib.open_drive(drive.path()).unwrap();
    }

    #[test]
    fn reset_is_idempotent_on_never_opened_drive() {
        let (lib, _cfg, drive) = lib();
        lib.register_drive(drive.path(), None).unwrap();
        let report = lib.reset_drive(drive.path(), ResetMode::State).unwrap();
        assert_eq!(report.removed_entries, 0);
        // Registry still has it.
        assert_eq!(lib.list_drives().len(), 1);
    }

    #[test]
    fn reset_does_not_touch_other_drives_state() {
        let (lib, _cfg, drive_a) = lib();
        let drive_b = TempDir::new().unwrap();
        lib.register_drive(drive_a.path(), None).unwrap();
        lib.register_drive(drive_b.path(), None).unwrap();
        populate_state(&lib, drive_a.path());
        populate_state(&lib, drive_b.path());

        let pa = paths_of(drive_a.path());
        let pb = paths_of(drive_b.path());

        lib.reset_drive(drive_a.path(), ResetMode::State).unwrap();

        // A wiped.
        assert!(!pa.index.exists());
        assert!(!pa.sessions.exists());
        // B intact.
        assert!(pb.index.exists());
        assert!(pb.sessions.exists());

        // Cleanup B so we don't leak state for the next run.
        let _ = lib.reset_drive(drive_b.path(), ResetMode::State);
    }

    #[test]
    fn reset_state_preserves_trash() {
        let (lib, _cfg, drive) = lib();
        lib.register_drive(drive.path(), None).unwrap();
        {
            let d = lib.open_drive(drive.path()).unwrap();
            d.write_text("doomed.md", "bye").unwrap();
            d.remove("doomed.md").unwrap();
            assert_eq!(d.trash_list().unwrap().len(), 1);
        }
        let p = paths_of(drive.path());
        assert!(p.trash.exists());

        lib.reset_drive(drive.path(), ResetMode::State).unwrap();

        // Trash survives a State-mode reset.
        assert!(p.trash.exists());
        let d = lib.open_drive(drive.path()).unwrap();
        assert_eq!(d.trash_list().unwrap().len(), 1);
    }
}
