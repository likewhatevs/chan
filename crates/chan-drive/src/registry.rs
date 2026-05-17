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
///
/// `uuid` is the stable per-drive identity, 16 hex chars, minted at
/// first register and preserved across `Library::move_drive`. All
/// per-drive sidecar paths (graph DB, search index, sessions,
/// tokens, trash, report) live under this uuid, so the
/// drive's filesystem path can move freely without invalidating its
/// state. Two registrations of the same path at different times
/// produce different uuids: that is the structural fix for
/// "delete-and-recreate at the same path surfaces stale state".
///
/// Empty `uuid` on load means the row was written by a pre-uuid
/// version; `Registry::migrate_uuids` fills it with the legacy
/// `paths::drive_key(path)` so the existing on-disk sidecars stay
/// valid. `Library::open_at` runs that migration once on load and
/// persists the result.
///
/// `canonical_path` is the canonicalized form of `path`, computed
/// once at insert / load time and reused for comparisons. The field
/// is intentionally `#[serde(skip)]`: the canonical form is a
/// per-machine artifact, recomputable on load, and including it in
/// the on-disk TOML would invite the registry to disagree with the
/// filesystem after a `mv`. Falls back to a clone of `path` when
/// the path doesn't currently canonicalize (drive root deleted /
/// network mount asleep), keeping the entry comparable lexically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownDrive {
    pub path: PathBuf,
    /// Stable per-drive identity. See struct-level doc.
    #[serde(default)]
    pub uuid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub last_opened: DateTime<Utc>,
    #[serde(skip)]
    pub(crate) canonical_path: Option<PathBuf>,
}

impl KnownDrive {
    /// Cached canonical path; falls back to a stat if the cache
    /// hasn't been primed (e.g. an entry constructed by tests
    /// outside the Registry).
    fn canonical(&self) -> PathBuf {
        if let Some(p) = &self.canonical_path {
            return p.clone();
        }
        self.path
            .canonicalize()
            .unwrap_or_else(|_| self.path.clone())
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
        // (find / touch / remove / set_name) are then pure and don't
        // re-canonicalize per call. Failure here is non-fatal: an
        // entry whose drive root is missing or asleep stays
        // comparable lexically.
        for d in &mut reg.drives {
            d.canonical_path = Some(d.path.canonicalize().unwrap_or_else(|_| d.path.clone()));
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

    /// Find a known drive by absolute path (canonicalized when
    /// possible). Matches by canonical path so symlink wiggles
    /// don't create duplicate registry entries.
    pub fn find(&self, root: &Path) -> Option<&KnownDrive> {
        let target = canonicalize_or_keep(root);
        match self.drives.iter().position(|d| d.canonical() == target) {
            Some(i) => Some(&self.drives[i]),
            None => self.drives.iter().find(|d| fresh_canonical(d) == target),
        }
    }

    /// Touch-or-append the drive entry, then sort most-recent first.
    /// Returns the entry's index after the operation.
    ///
    /// `name` is only set on first insert: re-touching an existing
    /// drive never clobbers a user-set name. Pass `set_name` from
    /// the explicit rename path instead.
    ///
    /// New rows are minted a fresh `uuid` via `paths::mint_uuid`.
    /// Re-touching an existing row preserves the uuid (and so
    /// preserves its sidecar dirs). Two distinct registrations of
    /// the same path at different times therefore get distinct
    /// uuids only if the previous row was removed first, which is
    /// exactly what `Library::unregister_drive` enforces.
    pub fn touch(&mut self, root: &Path, name: Option<String>) -> usize {
        let canonical = canonicalize_or_keep(root);
        let now = Utc::now();
        let idx = position_match(&self.drives, &canonical);
        if let Some(i) = idx {
            self.drives[i].last_opened = now;
            // Refresh the cache: a relinked drive (registered dir
            // replaced by a symlink to elsewhere) would otherwise
            // keep the stale canonical, then the next touch wouldn't
            // find it on the fast path. Belt and braces.
            self.drives[i].canonical_path = Some(canonical.clone());
        } else {
            self.drives.push(KnownDrive {
                path: canonical.clone(),
                uuid: paths::mint_uuid(&canonical),
                name,
                last_opened: now,
                canonical_path: Some(canonical.clone()),
            });
        }
        self.drives
            .sort_by_key(|d| std::cmp::Reverse(d.last_opened));
        position_match(&self.drives, &canonical).unwrap_or(0)
    }

    /// Fill any empty `uuid` fields with the legacy
    /// `paths::drive_key(path)` value. Returns `true` when at least
    /// one row was migrated so the caller knows to persist the
    /// registry back to disk. Idempotent: re-running on an already-
    /// migrated registry is a no-op and returns `false`.
    ///
    /// Why `drive_key(path)` as the migrated value: pre-uuid
    /// installs keyed every sidecar directory by sha256(path)[..16].
    /// Adopting that exact value as the new uuid keeps every
    /// existing graph DB / index segment / trash dir reachable
    /// under its new home, so the migration is zero file motion.
    pub fn migrate_uuids(&mut self) -> bool {
        let mut changed = false;
        for d in &mut self.drives {
            if d.uuid.is_empty() {
                d.uuid = paths::drive_key(&d.path);
                changed = true;
            }
        }
        changed
    }

    /// Update the `path` of an existing registry row, preserving
    /// the uuid (and therefore every sidecar). Used by
    /// `Library::move_drive` to record an `mv` of the drive
    /// directory without rebuilding any state. No-op + `false`
    /// return when `old` is not registered.
    pub fn set_path(&mut self, old: &Path, new: &Path) -> bool {
        let old_canon = canonicalize_or_keep(old);
        let Some(i) = position_match(&self.drives, &old_canon) else {
            return false;
        };
        let new_canon = canonicalize_or_keep(new);
        self.drives[i].path = new_canon.clone();
        self.drives[i].canonical_path = Some(new_canon);
        true
    }

    /// Remove a registry entry. Does not delete the directory or
    /// the per-drive state on disk; the caller decides whether to
    /// purge that separately.
    pub fn remove(&mut self, root: &Path) -> bool {
        let canonical = canonicalize_or_keep(root);
        let before = self.drives.len();
        self.drives
            .retain(|d| d.canonical() != canonical && fresh_canonical(d) != canonical);
        self.drives.len() != before
    }

    /// Set the display name on an existing drive. No-op if the
    /// drive isn't registered.
    pub fn set_name(&mut self, root: &Path, name: Option<String>) -> bool {
        let canonical = canonicalize_or_keep(root);
        let Some(i) = position_match(&self.drives, &canonical) else {
            return false;
        };
        self.drives[i].name = name;
        true
    }
}

/// Canonicalize-or-fall-back-to-input. Used for the per-call target
/// path; entries cache their own canonical form on insert / load.
fn canonicalize_or_keep(root: &Path) -> PathBuf {
    root.canonicalize().unwrap_or_else(|_| root.to_path_buf())
}

/// Re-canonicalize an entry's `path` ignoring its cache. Used as the
/// slow-path fallback when the cached canonical doesn't match the
/// target (drive moved or relinked since last load).
fn fresh_canonical(d: &KnownDrive) -> PathBuf {
    d.path.canonicalize().unwrap_or_else(|_| d.path.clone())
}

/// Index of the drive whose canonical (cached, then fresh) matches
/// `canonical`. Centralises the fast-path / slow-path lookup so
/// touch / find / set_name all behave consistently.
fn position_match(drives: &[KnownDrive], canonical: &Path) -> Option<usize> {
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

    #[test]
    fn touch_mints_uuid_on_insert_and_preserves_on_retouch() {
        let tmp = TempDir::new().unwrap();
        let mut reg = Registry::default();
        reg.touch(tmp.path(), None);
        let uuid1 = reg.drives[0].uuid.clone();
        assert_eq!(uuid1.len(), 16, "uuid should be 16 hex chars");
        assert!(uuid1.chars().all(|c| c.is_ascii_hexdigit()));
        // Re-touch must keep the uuid stable; otherwise sidecar
        // directories would orphan every time a drive is re-opened.
        reg.touch(tmp.path(), Some("Renamed".into()));
        assert_eq!(reg.drives[0].uuid, uuid1);
    }

    #[test]
    fn migrate_uuids_fills_legacy_rows_only() {
        // Build a registry the way a pre-uuid version would have:
        // empty uuid strings in each row.
        let tmp = TempDir::new().unwrap();
        let mut reg = Registry::default();
        reg.touch(tmp.path(), Some("Notes".into()));
        // Force the uuid empty as if loaded from a pre-uuid TOML.
        reg.drives[0].uuid.clear();

        assert!(reg.migrate_uuids());
        let migrated = reg.drives[0].uuid.clone();
        assert_eq!(migrated, paths::drive_key(tmp.path()));

        // Idempotent: second call returns false and changes nothing.
        assert!(!reg.migrate_uuids());
        assert_eq!(reg.drives[0].uuid, migrated);
    }

    #[test]
    fn set_path_preserves_uuid() {
        let old = TempDir::new().unwrap();
        let new = TempDir::new().unwrap();
        let mut reg = Registry::default();
        reg.touch(old.path(), Some("Notes".into()));
        let uuid_before = reg.drives[0].uuid.clone();

        assert!(reg.set_path(old.path(), new.path()));
        assert_eq!(
            reg.drives[0].uuid, uuid_before,
            "uuid must survive a path move so sidecars stay reachable",
        );
        // find() now resolves via the new path.
        assert!(reg.find(new.path()).is_some());
        assert!(reg.find(old.path()).is_none());
    }

    #[test]
    fn delete_then_recreate_mints_distinct_uuid() {
        // The reproducer that PR2 is structurally fixing: register,
        // unregister, register again at the same path. The new
        // uuid must differ from the old, otherwise sidecars from
        // the deleted drive would collide with the new one.
        let tmp = TempDir::new().unwrap();
        let mut reg = Registry::default();
        reg.touch(tmp.path(), None);
        let first = reg.drives[0].uuid.clone();
        assert!(reg.remove(tmp.path()));
        // Sleep so the nanosecond nonce in mint_uuid definitely
        // advances. The runtime check inside mint_uuid keeps this
        // robust on coarse clocks, but a tiny sleep makes the test
        // intent explicit.
        std::thread::sleep(std::time::Duration::from_millis(1));
        reg.touch(tmp.path(), None);
        let second = reg.drives[0].uuid.clone();
        assert_ne!(first, second);
    }
}
