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
    /// session blobs, app tokens). Keep the registry entry, the
    /// user's notes tree, and the trash.
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

/// What `Library::sweep_orphans` reclaimed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepReport {
    /// Distinct uuids whose sidecars were reclaimed. Sorted and
    /// deduplicated across subsystem parents.
    pub removed_uuids: Vec<String>,
    /// Total file + subdirectory entries removed across all wiped
    /// sidecar dirs.
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
    ///
    /// Runs the pre-uuid registry migration once on load: any row
    /// missing a `uuid` adopts the legacy `drive_key(path)` value
    /// so its existing sidecar dirs (graph DB, search index, ...)
    /// remain reachable under the new uuid-keyed layout. If any
    /// row was migrated, the registry is persisted back so the
    /// next process load reads the upgraded shape directly.
    pub fn open_at(config_path: PathBuf) -> Result<Self> {
        let mut registry = Registry::load_from(&config_path)?;
        if registry.migrate_uuids() {
            registry.save_to(&config_path)?;
        }
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

    /// Drop a drive from the registry AND wipe its per-drive
    /// chan-managed state (search index, graph DB, session blobs,
    /// app tokens). Equivalent to
    /// `reset_drive(root, ResetMode::Everything)` plus a `false`
    /// return when the drive wasn't registered.
    ///
    /// The user's notes tree is never touched; chan-drive never
    /// writes inside it. The trash is preserved (it holds
    /// recoverable user data, semantically owned by the user even
    /// after the drive is forgotten).
    ///
    /// Why state is wiped here: per-drive sidecars are keyed by
    /// `sha256(canonical_path)[..16]`. Without this wipe, deleting
    /// the drive directory and re-creating it at the same path
    /// reuses the old sidecar and the new "fresh" drive surfaces
    /// graph + index entries that belong to the deleted drive.
    /// Wiping on unregister closes that window.
    ///
    /// Preconditions: same as `reset_drive`. The caller must drop
    /// any open `Arc<Drive>` for `root` first; otherwise this
    /// returns `ChanError::DriveAlreadyOpen`.
    ///
    /// Returns `Ok(false)` when no registry row matched `root` and
    /// no wipe was attempted.
    pub fn unregister_drive(&self, root: &Path) -> Result<bool> {
        // Peek before delegating so we can preserve the previous
        // bool semantic. reset_drive itself is idempotent on a
        // never-opened drive (returns removed_entries = 0), but we
        // don't want to wipe state for a path the user never
        // registered with this Library, just in case it collides
        // with an unrelated cached entry from an earlier install.
        let registered = self.inner.registry.lock().unwrap().find(root).is_some();
        if !registered {
            return Ok(false);
        }
        self.reset_drive(root, ResetMode::Everything)?;
        Ok(true)
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
    /// in one shot for the "point at a directory and go" path).
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
    /// sessions, tokens) as it is wiped, so a UI can
    /// surface "wiping <subsystem>..." without instrumenting each
    /// caller. The label carries the subsystem name; `current` /
    /// `total` count through the fixed four-subsystem list.
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
        // Sidecar identity comes from the registry's uuid, not the
        // path. An unregistered root has no uuid in the registry,
        // so there is nothing for this Library to wipe under the
        // current scheme; surface that as an idempotent no-op
        // rather than guessing at a path-derived key (which could
        // belong to a different drive that does happen to be
        // registered). Stale path-derived sidecars from a previous
        // chan version land in the orphan-sweep path instead.
        let Some(uuid) = self
            .inner
            .registry
            .lock()
            .unwrap()
            .find(root)
            .map(|e| e.uuid.clone())
        else {
            return Ok(ResetReport { removed_entries: 0 });
        };
        let drive_paths = paths::drive_paths_for_uuid(&uuid);
        let _lock = DriveLock::acquire(&drive_paths.lock)?;
        let mut removed = 0;
        let subsystems: [(&str, &Path); 4] = [
            ("index", &drive_paths.index),
            ("graph", &drive_paths.graph_dir),
            ("sessions", &drive_paths.sessions),
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

    /// Record an `mv` of a registered drive's directory. Preserves
    /// the drive's `uuid` (and therefore all its sidecar state,
    /// graph DB, search index, sessions, tokens, trash,
    /// report), only rewriting the `path` field on the registry
    /// row.
    ///
    /// Refuses if:
    ///   - `old` is not registered (`Ok(false)`),
    ///   - `new` does not exist on disk (`DriveRootMissing`),
    ///   - `new` is already registered to a different uuid
    ///     (`DriveAlreadyRegistered`), since collapsing two
    ///     registry rows onto one path would orphan one drive's
    ///     sidecars under a uuid the registry no longer references.
    ///   - any `Arc<Drive>` for `old` is still alive
    ///     (`DriveAlreadyOpen`), since the live drive is caching
    ///     `entry.path` and would silently disagree with the
    ///     registry after the move.
    ///
    /// The caller is responsible for actually moving the directory
    /// on disk (`std::fs::rename(old, new)` or an `mv` from the
    /// shell). This call only updates the registry.
    pub fn move_drive(&self, old: &Path, new: &Path) -> Result<bool> {
        if !new.exists() {
            return Err(ChanError::DriveRootMissing(new.to_path_buf()));
        }
        let key = canonical_key(old);
        {
            let mut map = self.inner.live_drives.lock().unwrap();
            gc_dead_entries(&mut map);
            if let Some(weak) = map.get(&key) {
                if weak.upgrade().is_some() {
                    return Err(ChanError::DriveAlreadyOpen);
                }
            }
        }
        let mut reg = self.inner.registry.lock().unwrap();
        let Some(old_entry) = reg.find(old) else {
            return Ok(false);
        };
        let old_uuid = old_entry.uuid.clone();
        if let Some(existing) = reg.find(new) {
            if existing.uuid != old_uuid {
                return Err(ChanError::DriveAlreadyRegistered(new.to_path_buf()));
            }
            // Same uuid means `new` is already a name for this
            // drive (idempotent retry after a partial move). Drop
            // through to set_path which will canonicalize and
            // overwrite, then save.
        }
        let ok = reg.set_path(old, new);
        if ok {
            reg.save_to(&self.inner.config_path)?;
        }
        Ok(ok)
    }

    /// Per-drive paths for a registered root. `None` when the
    /// drive isn't registered (no uuid, so no sidecar identity to
    /// resolve). Use this rather than `paths::drive_paths_for_uuid`
    /// directly so the registry stays the only source of truth for
    /// "which uuid is this path."
    pub fn drive_paths_for(&self, root: &Path) -> Option<paths::DrivePaths> {
        let reg = self.inner.registry.lock().unwrap();
        let entry = reg.find(root)?;
        Some(paths::drive_paths_for_uuid(&entry.uuid))
    }

    /// Reclaim sidecar directories whose uuid no longer appears in
    /// the registry. Walks every per-subsystem root from
    /// `paths::drive_subsystem_dirs` and deletes any immediate
    /// subdirectory whose name isn't a current uuid.
    ///
    /// Use cases:
    ///   - A previous chan version `unregister`'d a drive without
    ///     wiping the sidecar (the bug PR1 closes).
    ///   - A `Library::move_drive` happened, the drive was later
    ///     deleted at the new location, leaving an orphaned uuid
    ///     dir under a previous registry row that has since been
    ///     removed.
    ///   - A registry was hand-edited and the matching uuid dirs
    ///     stayed behind.
    ///
    /// Cross-process safety: this routine snapshots the registry
    /// under the in-process mutex and walks each subsystem dir
    /// independently. A concurrent `register_drive` on another
    /// process can race: it mints a new uuid, creates its sidecar
    /// dir, and saves the registry; our sweep, working from the
    /// snapshot, then deletes the just-created dir. The window is
    /// tight (uuid creation -> dir creation -> registry save) and
    /// the worst case is "the next index access on the new drive
    /// rebuilds from scratch", which is the same outcome as a
    /// fresh open. We accept the race rather than introduce a
    /// cross-process registry lock for what is fundamentally a
    /// garbage-collection pass.
    pub fn sweep_orphans(&self) -> Result<SweepReport> {
        let known: std::collections::HashSet<String> = self
            .inner
            .registry
            .lock()
            .unwrap()
            .drives
            .iter()
            .map(|d| d.uuid.clone())
            .collect();
        sweep_orphans_in(&paths::drive_subsystem_dirs(), &known)
    }
}

/// Inner workhorse for `Library::sweep_orphans`: walk each parent
/// in `parents` and remove any immediate subdirectory whose name
/// is not in `known` AND looks like a uuid (16 lowercase hex). Pure
/// in its arguments so tests can drive it against a TempDir tree
/// without mutating the host's real XDG state/cache.
///
/// Tolerates concurrent removal: a sidecar deleted between
/// `read_dir` and `wipe_dir` simply contributes zero entries to
/// the report.
fn sweep_orphans_in(
    parents: &[PathBuf],
    known: &std::collections::HashSet<String>,
) -> Result<SweepReport> {
    let mut removed_uuids: Vec<String> = Vec::new();
    let mut removed_entries: usize = 0;
    for parent in parents {
        let read = match std::fs::read_dir(parent) {
            Ok(r) => r,
            // Not yet created on a fresh install; nothing to sweep.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(ChanError::Io(format!("read {parent:?}: {e}"))),
        };
        for entry in read.flatten() {
            let name = entry.file_name();
            let Some(name_str) = name.to_str() else {
                continue;
            };
            if known.contains(name_str) {
                continue;
            }
            // Defensive: if a future schema adds a non-uuid
            // file/dir under a subsystem parent, don't delete it.
            // Only sweep directories that look like uuids: 16
            // lowercase hex chars. The legacy drive_key and
            // mint_uuid both produce that shape.
            if !looks_like_uuid(name_str) {
                continue;
            }
            let path = entry.path();
            let entry_count = wipe_dir(&path)?;
            removed_entries += entry_count;
            removed_uuids.push(name_str.to_string());
        }
    }
    removed_uuids.sort();
    removed_uuids.dedup();
    Ok(SweepReport {
        removed_uuids,
        removed_entries,
    })
}

/// Shape of `drive_key` / `mint_uuid` outputs: 16 lowercase hex.
fn looks_like_uuid(s: &str) -> bool {
    s.len() == 16
        && s.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
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
/// inside it. Missing dir contributes 0. Tolerates a race where
/// the directory disappears between the walk and the remove (a
/// second sweep, a concurrent drive teardown, an external tool)
/// by treating NotFound on remove as zero-impact rather than an
/// error.
fn wipe_dir(dir: &Path) -> Result<usize> {
    if !dir.exists() {
        return Ok(0);
    }
    let count = walkdir::WalkDir::new(dir)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .count();
    match std::fs::remove_dir_all(dir) {
        Ok(()) => Ok(count),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(e.into()),
    }
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
    /// blob, drop a fake token. Also writes a markdown file inside
    /// the drive so the test can verify reset doesn't touch the
    /// user's notes.
    fn populate_state(lib: &Library, root: &Path) {
        let drive = lib.open_drive(root).unwrap();
        drive
            .write_text("notes/keep.md", "kept across reset")
            .unwrap();
        drive.reindex(None).unwrap();
        drive.put_session("win-1", b"layout").unwrap();
        let p = drive.paths();
        std::fs::create_dir_all(&p.tokens).unwrap();
        std::fs::write(p.tokens.join("server.token"), b"deadbeef").unwrap();
    }

    fn paths_of(lib: &Library, root: &Path) -> paths::DrivePaths {
        lib.drive_paths_for(root)
            .expect("test helper expects a registered drive")
    }

    #[test]
    fn open_at_migrates_legacy_registry_uuid_and_persists() {
        // Write a config.toml the way a pre-uuid version would have:
        // no uuid field on the entry. Then Library::open_at must
        // fill it in with drive_key(path) and save the registry
        // back. A subsequent open_at sees the uuid pre-filled and
        // does not rewrite (idempotent).
        let cfg = TempDir::new().unwrap();
        let drive = TempDir::new().unwrap();
        let cfg_path = cfg.path().join("config.toml");
        let legacy = format!(
            "[[drives]]\npath = {:?}\nname = \"Legacy\"\nlast_opened = \"2024-01-01T00:00:00Z\"\n",
            drive.path(),
        );
        std::fs::write(&cfg_path, legacy).unwrap();

        let lib = Library::open_at(cfg_path.clone()).unwrap();
        let drives = lib.list_drives();
        assert_eq!(drives.len(), 1);
        let migrated_uuid = drives[0].uuid.clone();
        assert!(
            !migrated_uuid.is_empty(),
            "uuid must be filled by migration"
        );
        assert_eq!(
            migrated_uuid,
            paths::drive_key(drive.path()),
            "migration must adopt the legacy path-derived key so existing sidecars stay reachable",
        );

        // File on disk now has the uuid field.
        let on_disk = std::fs::read_to_string(&cfg_path).unwrap();
        assert!(
            on_disk.contains("uuid"),
            "migration should persist uuid back to disk; got: {on_disk}",
        );

        // Re-open: no further migration, uuid stays the same.
        let lib2 = Library::open_at(cfg_path).unwrap();
        assert_eq!(lib2.list_drives()[0].uuid, migrated_uuid);
    }

    #[test]
    fn drive_paths_for_returns_none_for_unregistered_root() {
        let (lib, _cfg, drive) = lib();
        assert!(lib.drive_paths_for(drive.path()).is_none());
        lib.register_drive(drive.path(), None).unwrap();
        assert!(lib.drive_paths_for(drive.path()).is_some());
    }

    #[test]
    fn move_drive_preserves_uuid_and_sidecars() {
        let (lib, _cfg, drive_a) = lib();
        let drive_b = TempDir::new().unwrap();
        lib.register_drive(drive_a.path(), Some("Notes".into()))
            .unwrap();
        populate_state(&lib, drive_a.path());

        let uuid_before = lib.list_drives()[0].uuid.clone();
        let pa = paths_of(&lib, drive_a.path());
        assert!(pa.graph_db.exists());

        // Move the drive's registry entry. The user is responsible
        // for the actual directory move; we simulate that by writing
        // notes into drive_b after the registry update.
        assert!(lib.move_drive(drive_a.path(), drive_b.path()).unwrap());

        // Registry now points at drive_b with the same uuid; the
        // sidecar dir on disk is untouched (still at the uuid path).
        let after = lib.list_drives();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].uuid, uuid_before, "uuid must survive a move");
        assert_eq!(after[0].path, drive_b.path().canonicalize().unwrap());

        let pb = paths_of(&lib, drive_b.path());
        assert_eq!(pb.graph_db, pa.graph_db, "sidecar paths follow the uuid");
        assert!(pb.graph_db.exists(), "graph DB still present after move");
    }

    #[test]
    fn move_drive_refuses_when_target_missing() {
        let (lib, _cfg, drive_a) = lib();
        lib.register_drive(drive_a.path(), None).unwrap();
        let missing = std::path::PathBuf::from("/nonexistent/destination/12345");
        let err = lib.move_drive(drive_a.path(), &missing).unwrap_err();
        assert!(matches!(err, ChanError::DriveRootMissing(_)));
    }

    #[test]
    fn move_drive_refuses_when_target_is_another_registered_drive() {
        let (lib, _cfg, drive_a) = lib();
        let drive_b = TempDir::new().unwrap();
        lib.register_drive(drive_a.path(), None).unwrap();
        lib.register_drive(drive_b.path(), None).unwrap();
        let err = lib.move_drive(drive_a.path(), drive_b.path()).unwrap_err();
        assert!(matches!(err, ChanError::DriveAlreadyRegistered(_)));
        // Both registry rows survive untouched.
        assert_eq!(lib.list_drives().len(), 2);
    }

    #[test]
    fn move_drive_refuses_when_source_is_open() {
        let (lib, _cfg, drive_a) = lib();
        let drive_b = TempDir::new().unwrap();
        lib.register_drive(drive_a.path(), None).unwrap();
        let _open = lib.open_drive(drive_a.path()).unwrap();
        let err = lib.move_drive(drive_a.path(), drive_b.path()).unwrap_err();
        assert!(matches!(err, ChanError::DriveAlreadyOpen));
    }

    #[test]
    fn move_drive_returns_false_when_source_unregistered() {
        let (lib, _cfg, _drive_a) = lib();
        let drive_b = TempDir::new().unwrap();
        let missing = TempDir::new().unwrap();
        // Source is never registered; destination exists but is irrelevant.
        assert!(!lib.move_drive(missing.path(), drive_b.path()).unwrap());
    }

    /// Drives `sweep_orphans_in` against an isolated TempDir tree
    /// so the test never touches the host's real XDG_STATE_HOME /
    /// XDG_CACHE_HOME. The public `Library::sweep_orphans` is a
    /// thin wrapper that supplies `paths::drive_subsystem_dirs()`
    /// and the registry's uuid set; the structural behavior we
    /// care about (reclaim unknown uuids, preserve known uuids,
    /// preserve non-uuid-shaped names) lives in the inner fn.
    #[test]
    fn sweep_orphans_in_reclaims_unknown_uuids() {
        use std::collections::HashSet;
        let root = TempDir::new().unwrap();
        let parents = vec![
            root.path().join("graph"),
            root.path().join("sessions"),
            root.path().join("index"),
        ];
        let known_uuid = "feedfacecafebab0";
        let orphan_uuid = "0123456789abcdef";
        let mut known = HashSet::new();
        known.insert(known_uuid.to_string());

        for parent in &parents {
            std::fs::create_dir_all(parent.join(known_uuid)).unwrap();
            std::fs::write(parent.join(known_uuid).join("keep"), b"keep").unwrap();
            std::fs::create_dir_all(parent.join(orphan_uuid)).unwrap();
            std::fs::write(parent.join(orphan_uuid).join("junk"), b"junk").unwrap();
        }
        // Plant a non-uuid-shaped dir to confirm the shape filter.
        let preserve = parents[0].join("not-a-uuid");
        std::fs::create_dir_all(&preserve).unwrap();
        std::fs::write(preserve.join("keep"), b"keep").unwrap();

        let report = sweep_orphans_in(&parents, &known).unwrap();
        assert_eq!(report.removed_uuids, vec![orphan_uuid.to_string()]);
        assert!(report.removed_entries >= 3);

        for parent in &parents {
            assert!(parent.join(known_uuid).exists(), "known uuid must survive");
            assert!(
                !parent.join(orphan_uuid).exists(),
                "orphan uuid must be gone"
            );
        }
        assert!(preserve.exists(), "non-uuid-shaped dir must survive");
    }

    #[test]
    fn sweep_orphans_in_handles_missing_parent_dirs() {
        // Parents that don't exist (fresh install, no drives ever
        // opened) must not error: the sweep simply skips them.
        use std::collections::HashSet;
        let root = TempDir::new().unwrap();
        let parents = vec![
            root.path().join("never-created"),
            root.path().join("also-not-here"),
        ];
        let known = HashSet::new();
        let report = sweep_orphans_in(&parents, &known).unwrap();
        assert!(report.removed_uuids.is_empty());
        assert_eq!(report.removed_entries, 0);
    }

    #[test]
    fn looks_like_uuid_guards_sweep_shape_filter() {
        assert!(looks_like_uuid("0123456789abcdef"));
        assert!(looks_like_uuid("feedfacecafebab0"));
        // Wrong length, wrong case, wrong charset, or has slashes.
        assert!(!looks_like_uuid("0123456789abcde"));
        assert!(!looks_like_uuid("0123456789abcdef0"));
        assert!(!looks_like_uuid("0123456789ABCDEF"));
        assert!(!looks_like_uuid("0123456789abcdez"));
        assert!(!looks_like_uuid("not-a-uuid-here1"));
    }

    #[test]
    fn reset_state_wipes_chan_state_and_keeps_user_notes_and_registry() {
        let (lib, _cfg, drive) = lib();
        lib.register_drive(drive.path(), Some("Notes".into()))
            .unwrap();
        populate_state(&lib, drive.path());

        let p = paths_of(&lib, drive.path());
        // Sanity: state dirs populated.
        assert!(p.index.exists());
        assert!(p.graph_db.exists());
        assert!(p.sessions.exists());
        assert!(p.tokens.exists());

        let report = lib.reset_drive(drive.path(), ResetMode::State).unwrap();
        assert!(report.removed_entries > 0);

        // State dirs gone.
        assert!(!p.index.exists());
        assert!(!p.graph_db.parent().unwrap().exists());
        assert!(!p.sessions.exists());
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

    // systacean-20: gated on Unix because Windows lock primitive
    // doesn't surface DriveLocked the same way flock does. Real
    // cross-platform fix tracked in phase-8-bugs.md "Windows lock
    // contract parity"; revert this gate when the LockFileEx-backed
    // bridge in lock.rs lands.
    #[cfg(unix)]
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

        let pa = paths_of(&lib, drive_a.path());
        let pb = paths_of(&lib, drive_b.path());

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

    /// Regression for the "delete-and-recreate at the same path
    /// surfaces stale graph data" bug. Before PR1, `unregister_drive`
    /// only dropped the registry row; the per-drive sidecars
    /// (graph DB, tantivy segments) lived on, keyed by
    /// `sha256(canonical_path)[..16]`. Re-registering the same path
    /// reused the key, so the next graph view returned nodes for
    /// files that no longer existed on disk.
    #[test]
    fn unregister_wipes_state_so_recreate_at_same_path_starts_fresh() {
        let (lib, _cfg, drive) = lib();
        lib.register_drive(drive.path(), Some("First".into()))
            .unwrap();
        populate_state(&lib, drive.path());

        let p = paths_of(&lib, drive.path());
        assert!(p.graph_db.exists(), "graph DB should exist after populate");
        // Sanity: the graph actually has the file we wrote.
        {
            let d = lib.open_drive(drive.path()).unwrap();
            let entries = d.list_tree().unwrap();
            assert!(entries.iter().any(|e| e.path == "notes/keep.md"));
        }

        assert!(lib.unregister_drive(drive.path()).unwrap());

        // Per-drive state is gone.
        assert!(!p.index.exists());
        assert!(!p.graph_db.parent().unwrap().exists());
        assert!(!p.sessions.exists());
        assert!(!p.tokens.exists());
        assert!(lib.list_drives().is_empty());

        // Re-register at the same path. Sidecar dirs must be absent
        // until the new drive lazily creates them, and the new
        // drive's graph must not surface anything until the user
        // reindexes (here: nothing on disk, so nothing to surface).
        std::fs::remove_dir_all(drive.path().join("notes")).ok();
        lib.register_drive(drive.path(), Some("Second".into()))
            .unwrap();
        let d = lib.open_drive(drive.path()).unwrap();
        d.reindex(None).unwrap();
        let opts = crate::drive::SearchOpts {
            mode: crate::SearchMode::Bm25,
            limit: 10,
            scope: None,
        };
        // The token used in populate_state's reindexed file was
        // "kept across reset"; searching for it must return zero
        // results, because the underlying file was removed before
        // this reindex.
        let hits = d.search("kept", &opts).unwrap();
        assert!(
            hits.hits.is_empty(),
            "stale index entries leaked across unregister/re-register; got {:?}",
            hits.hits
        );
    }

    #[test]
    fn unregister_returns_drive_already_open_when_handle_is_live() {
        // unregister_drive now wipes state, which requires exclusive
        // access. Holding an open handle must produce a clear error
        // rather than silently leaving the registry row gone and
        // sidecars half-wiped.
        let (lib, _cfg, drive) = lib();
        lib.register_drive(drive.path(), None).unwrap();
        let _open = lib.open_drive(drive.path()).unwrap();
        let err = lib.unregister_drive(drive.path()).unwrap_err();
        assert!(matches!(err, ChanError::DriveAlreadyOpen));
        // Registry row survives, because we bailed before touching it.
        assert_eq!(lib.list_drives().len(), 1);
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
        let p = paths_of(&lib, drive.path());
        assert!(p.trash.exists());

        lib.reset_drive(drive.path(), ResetMode::State).unwrap();

        // Trash survives a State-mode reset.
        assert!(p.trash.exists());
        let d = lib.open_drive(drive.path()).unwrap();
        assert_eq!(d.trash_list().unwrap().len(), 1);
    }
}
