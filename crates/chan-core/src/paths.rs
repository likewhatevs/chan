// OS-standard locations chan uses on this machine.
//
// Layout:
//
//                config_dir       state_dir            cache_dir
//                ---------------  -------------------  -------------------
//   all          ~/.chan          $XDG_DATA_HOME/chan  $XDG_CACHE_HOME/chan
//
// `~/.chan/config.toml` holds the registry of known drives and the
// default-drive setting (chan-core's responsibility). Editor / UI
// preferences (fonts, theme, API keys) live elsewhere and are an
// app-level concern; chan-core does not read or write them.
//
// State and cache stay XDG-shaped because they hold per-drive blobs
// where OS conventions help (Time Machine semantics on macOS,
// $XDG_RUNTIME_DIR cleanup on Linux). Per-drive subpaths are keyed
// by `drive_key()` (sha256 of the canonical absolute path, hex-
// truncated to 16). Renames invalidate the keys; rebuilds are cheap.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Default drive root for first-run / no-arg launches. The directory
/// is NOT created here; callers decide whether to auto-create.
///
/// Falls back to the platform-specific data dir when the canonical
/// "Documents" lookup fails (CI / headless boxes without a profile).
pub fn default_drive_root() -> PathBuf {
    if let Some(docs) = dirs::document_dir() {
        return docs.join("Chan");
    }
    if let Some(data) = dirs::data_dir() {
        return data.join("chan").join("default");
    }
    PathBuf::from("chan")
}

/// Per-user config dir. Holds the global `config.toml` (drive
/// registry + default-drive). `~/.chan/` on desktop targets;
/// co-located under the data dir on iOS / Android where the home
/// dir isn't user-writable.
pub fn config_dir() -> PathBuf {
    #[cfg(any(target_os = "ios", target_os = "android"))]
    {
        return state_dir();
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        dirs::home_dir()
            .map(|p| p.join(".chan"))
            .unwrap_or_else(|| PathBuf::from(".chan"))
    }
}

/// Per-user state dir. Per-drive sessions and assistant history,
/// optional process tokens. Persistent.
pub fn state_dir() -> PathBuf {
    dirs::data_dir()
        .map(|p| p.join("chan"))
        .unwrap_or_else(|| PathBuf::from(".chan-state"))
}

/// Per-user cache dir. Search index segments, embedding model
/// weights. Wipeable; everything inside rebuilds on demand.
pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .map(|p| p.join("chan"))
        .unwrap_or_else(|| PathBuf::from(".chan-cache"))
}

/// Global config file. Drive registry and per-machine defaults.
pub fn global_config_path() -> PathBuf {
    config_dir().join("config.toml")
}

/// Stable per-drive key. sha256(canonical_path)[..16] as hex.
/// `canonicalize` falls back to the input on error (typical for
/// not-yet-existing paths) so the key is still computable.
pub fn drive_key(drive_root: &Path) -> String {
    let canonical = drive_root
        .canonicalize()
        .unwrap_or_else(|_| drive_root.to_path_buf());
    let mut h = Sha256::new();
    h.update(canonical.as_os_str().to_string_lossy().as_bytes());
    let hex = format!("{:x}", h.finalize());
    hex[..16].to_owned()
}

/// Per-drive global paths. Computed once per Drive open.
#[derive(Debug, Clone)]
pub struct DrivePaths {
    /// Per-drive sessions directory. Opaque JSON; chan-core does
    /// not interpret. Apps put window/pane layout files here.
    pub sessions: PathBuf,
    /// Per-drive assistant conversation directory. Each file keyed
    /// by sha256 of the source markdown path.
    pub assistant: PathBuf,
    /// Per-drive search-index directory (tantivy segments + config).
    /// Lives in cache_dir so a wipe rebuilds without data loss.
    pub index: PathBuf,
    /// Per-drive graph database (sqlite). Lives in state_dir
    /// because it's authoritative for graph relationships derived
    /// from the source-of-truth markdown; it's regenerable but a
    /// rebuild is more expensive than a search reindex.
    pub graph_db: PathBuf,
    /// Per-drive lock dir. Holds the index-writer lockfile that
    /// prevents two processes from writing the same drive's index.
    pub lock: PathBuf,
    /// Per-drive tokens dir. App-level surface (chan-server stores
    /// its bearer token here, mode 0600). chan-core only allocates
    /// the directory; it does not read or write inside.
    pub tokens: PathBuf,
    /// Per-drive trash dir. Holds soft-deleted files / dirs as
    /// `<id>/{meta.json, payload[/]}`. Lazily GC'd on Drive::open
    /// and on every trash_* call.
    pub trash: PathBuf,
}

/// Resolve the per-drive global paths for `drive_root`.
pub fn drive_paths(drive_root: &Path) -> DrivePaths {
    let key = drive_key(drive_root);
    let state = state_dir();
    let cache = cache_dir();
    DrivePaths {
        sessions: state.join("sessions").join(&key),
        assistant: state.join("assistant").join(&key),
        index: cache.join("index").join(&key),
        graph_db: state.join("graph").join(&key).join("graph.sqlite"),
        lock: state.join("locks").join(&key),
        tokens: state.join("tokens").join(&key),
        trash: state.join("trash").join(&key),
    }
}

/// One cloud-storage provider's root the first-launch picker can
/// suggest as a chan drive location. The `suggested_root` is the
/// concrete directory chan would land its drive in (provider root
/// joined with "Chan" by convention so iOS / Android Files-app
/// users see a recognizable folder name across devices).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedCloud {
    /// User-facing label for the picker (e.g. "iCloud Drive",
    /// "Google Drive (alex@example.com)", "Dropbox").
    pub provider: String,
    /// Absolute path to the provider's mount point on this OS.
    pub provider_root: PathBuf,
    /// Recommended drive location: provider_root joined with
    /// "Chan". Not created here; the picker decides whether to
    /// auto-init or prompt.
    pub suggested_root: PathBuf,
}

/// Probe the OS for known cloud-storage mount points and return
/// the ones that exist. Used by the first-launch drive picker so
/// users on iCloud / Google Drive / Dropbox can land their drive
/// somewhere syncing across devices instead of in a local-only
/// `~/Documents/Chan`.
///
/// Per-OS coverage:
///
///   - macOS: iCloud Drive
///     (`~/Library/Mobile Documents/com~apple~CloudDocs`),
///     Google Drive
///     (`~/Library/CloudStorage/GoogleDrive-*/My Drive`, one
///     entry per signed-in account), Dropbox (`~/Dropbox`).
///   - Windows: iCloud Drive (`%USERPROFILE%\iCloudDrive`),
///     Google Drive (`G:\My Drive`, the default mapped drive),
///     Dropbox (`%USERPROFILE%\Dropbox`).
///   - Linux: Dropbox (`~/Dropbox`); iCloud isn't available and
///     Google Drive on Linux ships through third-party tools
///     (Insync, rclone) with user-chosen paths chan can't predict.
///   - iOS / Android: empty list. The platform's own document
///     picker handles cloud-storage discovery.
///
/// Empty list = no cloud drives detected; the picker should fall
/// back to "Local only" with `default_drive_root()`.
pub fn detected_cloud_drives() -> Vec<DetectedCloud> {
    let mut out = Vec::new();
    let Some(home) = dirs::home_dir() else {
        return out;
    };

    #[cfg(target_os = "macos")]
    {
        let icloud = home
            .join("Library")
            .join("Mobile Documents")
            .join("com~apple~CloudDocs");
        if icloud.is_dir() {
            out.push(DetectedCloud {
                provider: "iCloud Drive".into(),
                suggested_root: icloud.join("Chan"),
                provider_root: icloud,
            });
        }
        // Google Drive for Desktop mounts each signed-in account
        // under ~/Library/CloudStorage/GoogleDrive-<email>/My Drive.
        // Multiple accounts -> multiple picker entries.
        let cloud_storage = home.join("Library").join("CloudStorage");
        if let Ok(rd) = std::fs::read_dir(&cloud_storage) {
            for entry in rd.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if let Some(rest) = name.strip_prefix("GoogleDrive-") {
                    let my_drive = entry.path().join("My Drive");
                    if my_drive.is_dir() {
                        out.push(DetectedCloud {
                            provider: format!("Google Drive ({rest})"),
                            suggested_root: my_drive.join("Chan"),
                            provider_root: my_drive,
                        });
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let icloud = home.join("iCloudDrive");
        if icloud.is_dir() {
            out.push(DetectedCloud {
                provider: "iCloud Drive".into(),
                suggested_root: icloud.join("Chan"),
                provider_root: icloud,
            });
        }
        // Default G:\ mapping for Google Drive for Desktop.
        let g_my_drive = PathBuf::from("G:\\My Drive");
        if g_my_drive.is_dir() {
            out.push(DetectedCloud {
                provider: "Google Drive".into(),
                suggested_root: g_my_drive.join("Chan"),
                provider_root: g_my_drive,
            });
        }
    }

    let dropbox = home.join("Dropbox");
    if dropbox.is_dir() {
        out.push(DetectedCloud {
            provider: "Dropbox".into(),
            suggested_root: dropbox.join("Chan"),
            provider_root: dropbox,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_drive_root_is_non_empty() {
        let p = default_drive_root();
        assert!(!p.as_os_str().is_empty());
    }

    #[test]
    fn global_config_path_ends_in_config_toml() {
        let p = global_config_path();
        assert_eq!(p.file_name().and_then(|s| s.to_str()), Some("config.toml"));
    }

    #[test]
    fn drive_key_is_stable_and_hex16() {
        let tmp = tempfile::TempDir::new().unwrap();
        let k1 = drive_key(tmp.path());
        let k2 = drive_key(tmp.path());
        assert_eq!(k1, k2);
        assert_eq!(k1.len(), 16);
        assert!(k1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn drive_paths_share_the_same_key() {
        let tmp = tempfile::TempDir::new().unwrap();
        let key = drive_key(tmp.path());
        let p = drive_paths(tmp.path());
        for path in [
            &p.sessions,
            &p.assistant,
            &p.index,
            &p.lock,
            &p.tokens,
            &p.trash,
        ] {
            assert!(path.to_string_lossy().contains(&key));
        }
        assert!(p.graph_db.to_string_lossy().contains(&key));
    }

    #[test]
    fn detected_cloud_drives_returns_a_list() {
        // Smoke test: just exercises the probe paths. Result depends
        // on the test machine's actual cloud-drive setup so we only
        // assert structural invariants (each entry has a non-empty
        // provider and a suggested_root that ends in "Chan" sitting
        // directly under provider_root).
        let drives = detected_cloud_drives();
        for d in &drives {
            assert!(!d.provider.is_empty());
            assert_eq!(
                d.suggested_root.file_name().and_then(|s| s.to_str()),
                Some("Chan"),
                "suggested_root should end in Chan: {:?}",
                d.suggested_root,
            );
            assert_eq!(d.suggested_root.parent(), Some(d.provider_root.as_path()));
        }
    }
}
