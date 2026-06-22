//! The persisted workspace on/off overlay shared by every chan-library
//! deployment.
//!
//! A chan-library's existence source is its registry (`chan workspace ls` /
//! `Library::list_workspaces`) — the set of workspaces it owns. What the
//! registry does NOT record is which of those a given deployment had MOUNTED
//! (`on`) versus registered-but-unmounted (`off`) at its last save. That on/off
//! state is this overlay: a [`PersistedWorkspace`] row keyed by path, owned by
//! the library and persisted in a [`WorkspaceOverlay`] store co-located with the
//! window registry (local `~/.chan/workspaces.json`, devserver
//! `~/.chan/devserver/workspaces.json`) so a restart comes back serving exactly
//! what was on. Both the desktop-local boot and the headless `run_devserver`
//! restore route through the same store — one implementation in the library.
//!
//! The route `prefix` a workspace mounts at is deliberately NOT persisted: it is
//! a pure function of the root path, derived per library by that library's own
//! scheme (the devserver's gateway-legible slug via
//! [`allocate_workspace_prefix`](crate::allocate_workspace_prefix); a hashed
//! window label for the local desktop). Persisting it would pin one library's
//! scheme into a shape the other reads — so each library re-derives its own
//! prefix at restore.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// One workspace's persisted on/off state: the `path` that identifies it (the
/// registry key) and whether it was mounted (`on`) at the last save. The
/// registry is the existence source; this is the on/off overlay over it. A row
/// absent from the overlay defaults to off — the registry still surfaces it.
/// The mount prefix is re-derived per library at restore, not stored here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistedWorkspace {
    /// Filesystem path identifying the workspace (the registry key).
    pub path: String,
    /// Whether the workspace was mounted (`on`) at the last save.
    pub on: bool,
}

/// The library's workspace on/off overlay store: the durable set of
/// [`PersistedWorkspace`] rows, persisted to `store_path`. Library-level (one
/// per library), cheap to share behind an `Arc`; installed on the
/// [`WorkspaceHost`](crate::WorkspaceHost) like the window registry, and both
/// the desktop boot and the devserver restore read/write it. Rows are sorted by
/// path on save for a stable file.
pub struct WorkspaceOverlay {
    store_path: PathBuf,
    rows: Mutex<Vec<PersistedWorkspace>>,
}

impl WorkspaceOverlay {
    /// Open the overlay at `store_path`, loading any persisted rows. An absent
    /// or unreadable store degrades to an empty set rather than refusing to
    /// start (the workspaces reappear off, surfaced by the registry, until the
    /// user turns them back on).
    pub fn open(store_path: PathBuf) -> Self {
        let rows = match std::fs::read(&store_path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        Self {
            store_path,
            rows: Mutex::new(rows),
        }
    }

    /// Upsert a workspace's on/off by path and persist. The toggle hook: turning
    /// a workspace off writes an `on:false` row (remembered-off), turning it on
    /// writes `on:true`.
    pub fn set(&self, path: &str, on: bool) {
        {
            let mut rows = self.rows.lock().unwrap_or_else(|e| e.into_inner());
            match rows.iter_mut().find(|r| r.path == path) {
                Some(row) => row.on = on,
                None => rows.push(PersistedWorkspace {
                    path: path.to_string(),
                    on,
                }),
            }
        }
        self.persist();
    }

    /// Forget a workspace entirely (it left the library) and persist.
    pub fn forget(&self, path: &str) {
        {
            let mut rows = self.rows.lock().unwrap_or_else(|e| e.into_inner());
            rows.retain(|r| r.path != path);
        }
        self.persist();
    }

    /// Replace the whole overlay with `new_rows` and persist. The bulk-snapshot
    /// hook (the desktop snapshots its live serve set; the devserver snapshots
    /// its registered-workspace map).
    pub fn replace(&self, new_rows: Vec<PersistedWorkspace>) {
        {
            let mut rows = self.rows.lock().unwrap_or_else(|e| e.into_inner());
            *rows = new_rows;
        }
        self.persist();
    }

    /// The paths currently on, for the boot/restore re-serve.
    pub fn on_paths(&self) -> Vec<String> {
        self.rows
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|r| r.on)
            .map(|r| r.path.clone())
            .collect()
    }

    /// Every row (on and off), for the devserver restore that tracks off rows.
    pub fn entries(&self) -> Vec<PersistedWorkspace> {
        self.rows.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Persist the current rows (sorted by path for a stable file) atomically.
    fn persist(&self) {
        let snapshot = {
            let mut rows = self.rows.lock().unwrap_or_else(|e| e.into_inner()).clone();
            rows.sort_by(|a, b| a.path.cmp(&b.path));
            rows
        };
        if let Err(e) = save_atomic(&self.store_path, &snapshot) {
            tracing::warn!("persisting workspace overlay: {e}");
        }
    }
}

/// Atomically persist `value` as pretty JSON: write a 0600 tmp, fsync it, rename
/// over the target, then best-effort fsync the parent dir. Mirrors the window
/// registry's discipline; renaming un-synced bytes is the partial-write risk on
/// a crash.
fn save_atomic<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    use std::io::Write as _;

    let dir = match path.parent() {
        Some(dir) => {
            std::fs::create_dir_all(dir)?;
            dir
        }
        None => Path::new("."),
    };
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let tmp = path.with_extension("json.tmp");
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(&bytes)?;
        f.sync_all()?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, path)?;
    if let Ok(dir_file) = std::fs::File::open(dir) {
        let _ = dir_file.sync_all();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overlay() -> (WorkspaceOverlay, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let ov = WorkspaceOverlay::open(dir.path().join("workspaces.json"));
        (ov, dir)
    }

    #[test]
    fn set_upserts_and_on_paths_filters() {
        let (ov, _dir) = overlay();
        ov.set("/a", true);
        ov.set("/b", false);
        ov.set("/a", true); // idempotent upsert, no dup
        assert_eq!(ov.on_paths(), vec!["/a".to_string()]);
        assert_eq!(ov.entries().len(), 2);
    }

    #[test]
    fn set_off_keeps_a_remembered_off_row() {
        let (ov, _dir) = overlay();
        ov.set("/a", true);
        ov.set("/a", false); // toggle off → on:false row, not removed
        assert!(ov.on_paths().is_empty());
        let entries = ov.entries();
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].on);
    }

    #[test]
    fn forget_drops_the_row() {
        let (ov, _dir) = overlay();
        ov.set("/a", true);
        ov.forget("/a");
        assert!(ov.entries().is_empty());
    }

    #[test]
    fn reopen_restores_persisted_rows() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("workspaces.json");
        {
            let ov = WorkspaceOverlay::open(path.clone());
            ov.set("/b", true);
            ov.set("/a", false);
        }
        let reopened = WorkspaceOverlay::open(path);
        let entries = reopened.entries();
        // Sorted by path on save.
        assert_eq!(entries[0].path, "/a");
        assert!(!entries[0].on);
        assert_eq!(entries[1].path, "/b");
        assert!(entries[1].on);
    }

    #[test]
    fn replace_overwrites_the_whole_set() {
        let (ov, _dir) = overlay();
        ov.set("/old", true);
        ov.replace(vec![PersistedWorkspace {
            path: "/new".into(),
            on: true,
        }]);
        assert_eq!(ov.on_paths(), vec!["/new".to_string()]);
        assert_eq!(ov.entries().len(), 1);
    }

    #[test]
    fn persisted_workspace_pins_field_names() {
        // The on-disk record field names are part of the persisted contract;
        // pin them so a rename is a visible, deliberate change.
        let ws = PersistedWorkspace {
            path: "/home/u/notes".into(),
            on: true,
        };
        let v = serde_json::to_value(&ws).unwrap();
        assert_eq!(
            v,
            serde_json::json!({ "path": "/home/u/notes", "on": true })
        );
        assert_eq!(ws, serde_json::from_value(v).unwrap());
    }
}
