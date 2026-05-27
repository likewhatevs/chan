// Workspace registry: the per-machine list of directories the user has
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

/// Default directory basenames excluded from indexing and graph rebuild walks.
///
/// Stored in `~/.chan/config.toml` as `index_excluded_dirs` so users can
/// add or remove names without rebuilding chan. `.git` and `.chan` are still
/// hard-skipped by the drive walker as internal invariants.
pub const DEFAULT_INDEX_EXCLUDED_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    "target",
    "__pycache__",
    ".venv",
    "venv",
    ".tox",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".cache",
    "dist",
    "build",
];

/// On-disk shape of the chan-drive config TOML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registry {
    /// Default drive root for the no-arg launch. When None, the
    /// resolver falls back to `paths::default_workspace_root()`.
    // chunk-1 wire preservation: Rust field renamed, but the on-disk TOML key
    // stays `default_drive_root` until chunk 2 flips the format (clean break).
    #[serde(
        rename = "default_drive_root",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub default_workspace_root: Option<PathBuf>,
    /// Directory basenames skipped by index and graph rebuild walks.
    /// Matched at any depth by exact basename, case-insensitive.
    #[serde(default = "default_index_excluded_dirs")]
    pub index_excluded_dirs: Vec<String>,
    /// Known drives the user has opened on this machine. Sorted
    /// most-recent first by `last_seen_at`.
    #[serde(default)]
    pub drives: Vec<KnownWorkspace>,
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            default_workspace_root: None,
            index_excluded_dirs: default_index_excluded_dirs(),
            drives: Vec::new(),
        }
    }
}

fn default_index_excluded_dirs() -> Vec<String> {
    DEFAULT_INDEX_EXCLUDED_DIRS
        .iter()
        .map(|name| (*name).to_owned())
        .collect()
}

/// One entry in the registry.
///
/// `root_path` is the current canonical local drive path. It is the
/// user-content boundary. `metadata_key` is the stable storage key
/// under `~/.chan/drives/`, allocated from the canonical path when
/// the drive is first registered and preserved across
/// `Library::move_workspace`.
///
/// The registry intentionally carries no user-editable display name.
/// UIs that need a label derive it from `root_path`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownWorkspace {
    pub root_path: PathBuf,
    /// Stable per-drive metadata storage key under `~/.chan/drives/`.
    pub metadata_key: String,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    #[serde(skip)]
    pub(crate) canonical_path: Option<PathBuf>,
}

impl KnownWorkspace {
    /// Cached canonical path; falls back to a stat if the cache
    /// hasn't been primed, e.g. an entry constructed by tests
    /// outside the Registry.
    fn canonical(&self) -> PathBuf {
        if let Some(p) = &self.canonical_path {
            return p.clone();
        }
        self.root_path
            .canonicalize()
            .unwrap_or_else(|_| self.root_path.clone())
    }
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
        let mut reg: Self = toml::from_str(&raw).map_err(|e| ChanError::ConfigDecode {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;
        // Prime the canonical-path cache once at load. Comparisons
        // are then pure and don't re-canonicalize per call. Failure
        // here is non-fatal: an entry whose drive root is missing or
        // asleep stays comparable lexically.
        for d in &mut reg.drives {
            d.canonical_path = Some(
                d.root_path
                    .canonicalize()
                    .unwrap_or_else(|_| d.root_path.clone()),
            );
        }
        Ok(reg)
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&paths::global_config_path())
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        let body = toml::to_string_pretty(self)?;
        fs_ops::atomic_write(path, body.as_bytes())
    }

    /// Find a known drive by absolute path, canonicalized when
    /// possible. Matches by canonical path so symlink wiggles don't
    /// create duplicate registry entries.
    pub fn find(&self, root: &Path) -> Option<&KnownWorkspace> {
        let target = canonicalize_or_keep(root);
        match self.drives.iter().position(|d| d.canonical() == target) {
            Some(i) => Some(&self.drives[i]),
            None => self.drives.iter().find(|d| fresh_canonical(d) == target),
        }
    }

    /// Touch-or-append the drive entry, then sort most-recent first.
    /// Returns the entry's index after the operation.
    ///
    /// Re-touching an existing row preserves `metadata_key`, so
    /// opening the same canonical path reuses the same metadata
    /// directory.
    pub fn touch(&mut self, root: &Path) -> usize {
        let canonical = canonicalize_or_keep(root);
        let now = Utc::now();
        let idx = position_match(&self.drives, &canonical);
        if let Some(i) = idx {
            self.drives[i].last_seen_at = now;
            // Refresh the cache: a relinked drive would otherwise
            // keep the stale canonical, then the next touch wouldn't
            // find it on the fast path.
            self.drives[i].canonical_path = Some(canonical.clone());
        } else {
            self.drives.push(KnownWorkspace {
                root_path: canonical.clone(),
                metadata_key: paths::metadata_key_for_root(&canonical),
                created_at: now,
                last_seen_at: now,
                canonical_path: Some(canonical.clone()),
            });
        }
        self.drives
            .sort_by_key(|d| std::cmp::Reverse(d.last_seen_at));
        position_match(&self.drives, &canonical).unwrap_or(0)
    }

    /// Update the `root_path` of an existing registry row,
    /// preserving the metadata key and therefore every metadata
    /// directory. Used by `Library::move_workspace` to record an `mv` of
    /// the drive directory without moving chan-managed state.
    pub fn set_path(&mut self, old: &Path, new: &Path) -> bool {
        let old_canon = canonicalize_or_keep(old);
        let Some(i) = position_match(&self.drives, &old_canon) else {
            return false;
        };
        let new_canon = canonicalize_or_keep(new);
        self.drives[i].root_path = new_canon.clone();
        self.drives[i].last_seen_at = Utc::now();
        self.drives[i].canonical_path = Some(new_canon);
        true
    }

    /// Remove a registry entry. Does not delete the directory or the
    /// per-drive metadata on disk; the caller decides whether to
    /// purge that separately.
    pub fn remove(&mut self, root: &Path) -> bool {
        let canonical = canonicalize_or_keep(root);
        let before = self.drives.len();
        self.drives
            .retain(|d| d.canonical() != canonical && fresh_canonical(d) != canonical);
        self.drives.len() != before
    }
}

/// Canonicalize-or-fall-back-to-input. Used for the per-call target
/// path; entries cache their own canonical form on insert / load.
fn canonicalize_or_keep(root: &Path) -> PathBuf {
    root.canonicalize().unwrap_or_else(|_| root.to_path_buf())
}

/// Re-canonicalize an entry's `root_path` ignoring its cache. Used
/// as the slow-path fallback when the cached canonical doesn't match
/// the target.
fn fresh_canonical(d: &KnownWorkspace) -> PathBuf {
    d.root_path
        .canonicalize()
        .unwrap_or_else(|_| d.root_path.clone())
}

/// Index of the drive whose canonical, cached then fresh, matches
/// `canonical`. Centralises lookup so touch / find / remove behave
/// consistently.
fn position_match(drives: &[KnownWorkspace], canonical: &Path) -> Option<usize> {
    if let Some(i) = drives.iter().position(|d| d.canonical() == *canonical) {
        return Some(i);
    }
    drives.iter().position(|d| fresh_canonical(d) == *canonical)
}

/// Effective default drive root: registry override wins, otherwise
/// the platform default. Best-effort: a malformed registry falls
/// back to the platform default so a user can still launch.
pub fn effective_default_drive_root() -> PathBuf {
    Registry::load()
        .ok()
        .and_then(|r| r.default_workspace_root)
        .unwrap_or_else(paths::default_workspace_root)
}

pub(crate) fn config_declares_index_excluded_dirs(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(value) = raw.parse::<toml::Value>() else {
        return false;
    };
    value.get("index_excluded_dirs").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn touch_inserts_then_updates() {
        let tmp = TempDir::new().unwrap();
        let mut reg = Registry::default();
        let idx1 = reg.touch(tmp.path());
        assert_eq!(idx1, 0);
        assert_eq!(reg.drives.len(), 1);
        let key = reg.drives[0].metadata_key.clone();
        let first_seen = reg.drives[0].last_seen_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        let idx2 = reg.touch(tmp.path());
        assert_eq!(idx2, 0);
        assert_eq!(reg.drives.len(), 1);
        assert_eq!(reg.drives[0].metadata_key, key);
        assert!(reg.drives[0].last_seen_at > first_seen);
    }

    #[test]
    fn remove_drops_entry() {
        let tmp = TempDir::new().unwrap();
        let mut reg = Registry::default();
        reg.touch(tmp.path());
        assert!(reg.remove(tmp.path()));
        assert!(reg.drives.is_empty());
        assert!(!reg.remove(tmp.path()));
    }

    #[test]
    fn save_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let cfg_path = tmp.path().join("config.toml");
        let mut reg = Registry::default();
        reg.touch(tmp.path());
        let key = reg.drives[0].metadata_key.clone();
        reg.save_to(&cfg_path).unwrap();
        let raw = std::fs::read_to_string(&cfg_path).unwrap();
        assert!(raw.contains("index_excluded_dirs"));
        assert!(raw.contains("root_path"));
        assert!(raw.contains("metadata_key"));
        assert!(!raw.lines().any(|line| line.starts_with("path =")));
        assert!(!raw.lines().any(|line| line.starts_with("uuid =")));
        assert!(!raw.lines().any(|line| line.starts_with("name =")));
        let loaded = Registry::load_from(&cfg_path).unwrap();
        assert!(loaded
            .index_excluded_dirs
            .iter()
            .any(|name| name == "node_modules"));
        assert_eq!(loaded.drives.len(), 1);
        assert_eq!(loaded.drives[0].metadata_key, key);
    }

    #[test]
    fn load_missing_index_excluded_dirs_uses_default() {
        let tmp = TempDir::new().unwrap();
        let cfg_path = tmp.path().join("config.toml");
        std::fs::write(&cfg_path, "drives = []\n").unwrap();
        let loaded = Registry::load_from(&cfg_path).unwrap();
        assert!(loaded
            .index_excluded_dirs
            .iter()
            .any(|name| name == "node_modules"));
    }

    #[test]
    fn load_empty_index_excluded_dirs_preserves_user_choice() {
        let tmp = TempDir::new().unwrap();
        let cfg_path = tmp.path().join("config.toml");
        std::fs::write(&cfg_path, "index_excluded_dirs = []\ndrives = []\n").unwrap();
        let loaded = Registry::load_from(&cfg_path).unwrap();
        assert!(loaded.index_excluded_dirs.is_empty());
    }

    #[test]
    fn most_recent_sorts_first() {
        let a = TempDir::new().unwrap();
        let b = TempDir::new().unwrap();
        let mut reg = Registry::default();
        reg.touch(a.path());
        std::thread::sleep(std::time::Duration::from_millis(10));
        reg.touch(b.path());
        assert_eq!(reg.drives[0].root_path, b.path().canonicalize().unwrap());
    }

    #[test]
    fn touch_allocates_deterministic_metadata_key() {
        let tmp = TempDir::new().unwrap();
        let mut reg = Registry::default();
        reg.touch(tmp.path());
        let key = reg.drives[0].metadata_key.clone();
        assert_eq!(key, paths::metadata_key_for_root(tmp.path()));

        assert!(reg.remove(tmp.path()));
        reg.touch(tmp.path());
        assert_eq!(reg.drives[0].metadata_key, key);
    }

    #[test]
    fn touch_matches_trailing_slash_to_same_canonical_path() {
        let tmp = TempDir::new().unwrap();
        let mut reg = Registry::default();
        reg.touch(tmp.path());
        let key = reg.drives[0].metadata_key.clone();
        let with_slash = tmp.path().join("");
        reg.touch(&with_slash);

        assert_eq!(reg.drives.len(), 1);
        assert_eq!(reg.drives[0].metadata_key, key);
    }

    #[cfg(unix)]
    #[test]
    fn touch_matches_symlink_to_same_canonical_path() {
        use std::os::unix::fs::symlink;

        let tmp = TempDir::new().unwrap();
        let link_parent = TempDir::new().unwrap();
        let link = link_parent.path().join("drive-link");
        symlink(tmp.path(), &link).unwrap();

        let mut reg = Registry::default();
        reg.touch(tmp.path());
        let key = reg.drives[0].metadata_key.clone();
        reg.touch(&link);

        assert_eq!(reg.drives.len(), 1);
        assert_eq!(reg.drives[0].metadata_key, key);
    }

    #[test]
    fn set_path_preserves_metadata_key() {
        let old = TempDir::new().unwrap();
        let new = TempDir::new().unwrap();
        let mut reg = Registry::default();
        reg.touch(old.path());
        let key_before = reg.drives[0].metadata_key.clone();

        assert!(reg.set_path(old.path(), new.path()));
        assert_eq!(
            reg.drives[0].metadata_key, key_before,
            "metadata key must survive a path move so metadata stays reachable",
        );
        assert!(reg.find(new.path()).is_some());
        assert!(reg.find(old.path()).is_none());
    }
}
