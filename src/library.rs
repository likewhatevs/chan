// Library: top-level handle. Owns the registry persisted at
// ~/.chan/config.toml and resolves OS state/cache paths.
//
// In practice apps create one Library at startup and keep it
// alive. Drives are opened against it. Cheap to clone (Arc inside).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::drive::Drive;
use crate::error::{ChanError, Result};
use crate::paths;
use crate::registry::{KnownDrive, Registry};

/// Per-machine handle to the chan-core registry + paths.
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
            }),
        })
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
        Drive::open(entry)
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
    fn open_unregistered_errors() {
        let (lib, _cfg, drive) = lib();
        let err = lib.open_drive(drive.path()).unwrap_err();
        assert!(matches!(err, ChanError::DriveNotRegistered(_)));
    }
}
