// Drive registry: the per-machine list of directories the user has
// registered as chan drives. Persisted to ~/.chan/config.toml.
//
// This file holds ONLY chan-drive's own state: the registry and
// default-drive setting. Editor preferences (fonts, theme, API
// keys) are an app-level concern and live in a separate file
// owned by the consuming app.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{ChanError, Result};
use crate::fs_ops;
use crate::paths;

/// On-disk shape of the chan-drive config TOML.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registry {
    /// Default drive root for the no-arg launch. When None, the
    /// resolver falls back to `paths::default_drive_root()`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_drive_root: Option<PathBuf>,
    /// Known drives the user has opened on this machine. Sorted
    /// most-recent first by `last_opened`.
    #[serde(default)]
    pub drives: Vec<KnownDrive>,
}

/// One entry in the registry. `name` is user-editable and shown in
/// recents lists / window titles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownDrive {
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub last_opened: DateTime<Utc>,
}

impl Registry {
    /// Load from the default location, falling back to defaults
    /// when the file is absent. A malformed file is an error; we
    /// never silently overwrite a user's edit.
    pub fn load() -> Result<Self> {
        Self::load_from(&paths::global_config_path())
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)?;
        toml::from_str(&raw).map_err(|e| ChanError::ConfigDecode {
            path: path.to_path_buf(),
            message: e.to_string(),
        })
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&paths::global_config_path())
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        let body = toml::to_string_pretty(self)?;
        fs_ops::atomic_write(path, body.as_bytes())
    }

    /// Find a known drive by absolute path (canonicalized when
    /// possible). Matches by canonical path so symlink wiggles
    /// don't create duplicate registry entries.
    pub fn find(&self, root: &Path) -> Option<&KnownDrive> {
        let target = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        self.drives.iter().find(|d| {
            let dp = d.path.canonicalize().unwrap_or_else(|_| d.path.clone());
            dp == target
        })
    }

    /// Touch-or-append the drive entry, then sort most-recent first.
    /// Returns the entry's index after the operation.
    ///
    /// `name` is only set on first insert: re-touching an existing
    /// drive never clobbers a user-set name. Pass `set_name` from
    /// the explicit rename path instead.
    pub fn touch(&mut self, root: &Path, name: Option<String>) -> usize {
        let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let now = Utc::now();
        if let Some(d) = self.drives.iter_mut().find(|d| {
            let dp = d.path.canonicalize().unwrap_or_else(|_| d.path.clone());
            dp == canonical
        }) {
            d.last_opened = now;
        } else {
            self.drives.push(KnownDrive {
                path: canonical.clone(),
                name,
                last_opened: now,
            });
        }
        self.drives
            .sort_by_key(|d| std::cmp::Reverse(d.last_opened));
        self.drives
            .iter()
            .position(|d| {
                let dp = d.path.canonicalize().unwrap_or_else(|_| d.path.clone());
                dp == canonical
            })
            .unwrap_or(0)
    }

    /// Remove a registry entry. Does not delete the directory or
    /// the per-drive state on disk; the caller decides whether to
    /// purge that separately.
    pub fn remove(&mut self, root: &Path) -> bool {
        let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let before = self.drives.len();
        self.drives.retain(|d| {
            let dp = d.path.canonicalize().unwrap_or_else(|_| d.path.clone());
            dp != canonical
        });
        self.drives.len() != before
    }

    /// Set the display name on an existing drive. No-op if the
    /// drive isn't registered.
    pub fn set_name(&mut self, root: &Path, name: Option<String>) -> bool {
        let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        if let Some(d) = self.drives.iter_mut().find(|d| {
            let dp = d.path.canonicalize().unwrap_or_else(|_| d.path.clone());
            dp == canonical
        }) {
            d.name = name;
            true
        } else {
            false
        }
    }
}

/// Effective default drive root: registry override wins, otherwise
/// the platform default. Best-effort: a malformed registry falls
/// back to the platform default so a user can still launch.
pub fn effective_default_drive_root() -> PathBuf {
    Registry::load()
        .ok()
        .and_then(|r| r.default_drive_root)
        .unwrap_or_else(paths::default_drive_root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn touch_inserts_then_updates() {
        let tmp = TempDir::new().unwrap();
        let mut reg = Registry::default();
        let idx1 = reg.touch(tmp.path(), Some("First".into()));
        assert_eq!(idx1, 0);
        assert_eq!(reg.drives.len(), 1);
        assert_eq!(reg.drives[0].name.as_deref(), Some("First"));
        let first_opened = reg.drives[0].last_opened;

        std::thread::sleep(std::time::Duration::from_millis(10));
        let idx2 = reg.touch(tmp.path(), Some("Renamed".into()));
        assert_eq!(idx2, 0);
        assert_eq!(reg.drives.len(), 1);
        // Touch must not clobber an existing name.
        assert_eq!(reg.drives[0].name.as_deref(), Some("First"));
        assert!(reg.drives[0].last_opened > first_opened);
    }

    #[test]
    fn set_name_updates_in_place() {
        let tmp = TempDir::new().unwrap();
        let mut reg = Registry::default();
        reg.touch(tmp.path(), None);
        assert!(reg.set_name(tmp.path(), Some("Notes".into())));
        assert_eq!(reg.drives[0].name.as_deref(), Some("Notes"));
    }

    #[test]
    fn remove_drops_entry() {
        let tmp = TempDir::new().unwrap();
        let mut reg = Registry::default();
        reg.touch(tmp.path(), None);
        assert!(reg.remove(tmp.path()));
        assert!(reg.drives.is_empty());
        assert!(!reg.remove(tmp.path()));
    }

    #[test]
    fn save_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let cfg_path = tmp.path().join("config.toml");
        let mut reg = Registry::default();
        reg.touch(tmp.path(), Some("Notes".into()));
        reg.save_to(&cfg_path).unwrap();
        let loaded = Registry::load_from(&cfg_path).unwrap();
        assert_eq!(loaded.drives.len(), 1);
        assert_eq!(loaded.drives[0].name.as_deref(), Some("Notes"));
    }

    #[test]
    fn most_recent_sorts_first() {
        let a = TempDir::new().unwrap();
        let b = TempDir::new().unwrap();
        let mut reg = Registry::default();
        reg.touch(a.path(), Some("A".into()));
        std::thread::sleep(std::time::Duration::from_millis(10));
        reg.touch(b.path(), Some("B".into()));
        assert_eq!(reg.drives[0].name.as_deref(), Some("B"));
    }
}
