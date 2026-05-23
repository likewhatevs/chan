// Locations chan uses on this machine.
//
// Layout:
//
//                config_dir
//                ----------------
//   all          ~/.chan
//
// `~/.chan/config.toml` holds the registry of known drives and the
// default-drive setting (chan-drive's responsibility). Editor / UI
// preferences (fonts, theme, API keys) live elsewhere and are an
// app-level concern; chan-drive does not read or write them.
//
// Per-drive metadata lives under `~/.chan/drives/<metadata_key>/`.
// The key is derived from the canonical drive root at registration
// time and preserved across `Library::move_drive`, so moving the
// drive directory updates only the registry row.

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

/// Per-user state dir. Kept as `~/.chan` for callers that still ask
/// chan-drive for a global state root.
pub fn state_dir() -> PathBuf {
    config_dir()
}

/// Per-user cache dir. Kept as `~/.chan` for callers that still ask
/// chan-drive for a global cache root.
pub fn cache_dir() -> PathBuf {
    config_dir()
}

/// Global config file. Drive registry and per-machine defaults.
pub fn global_config_path() -> PathBuf {
    config_dir().join("config.toml")
}

/// Per-drive metadata parent.
pub fn drives_dir() -> PathBuf {
    config_dir().join("drives")
}

/// Stable metadata key for a drive root.
///
/// The readable prefix is the canonical absolute path with path
/// separators and filename-awkward characters replaced by `-`. The
/// 8-hex suffix is a deterministic hash of the same canonical path
/// string, preventing collisions between similar slugs.
pub fn metadata_key_for_root(drive_root: &Path) -> String {
    let canonical = drive_root
        .canonicalize()
        .unwrap_or_else(|_| drive_root.to_path_buf());
    let canonical_s = canonical.as_os_str().to_string_lossy();
    let slug = metadata_slug(&canonical_s);
    let mut h = Sha256::new();
    h.update(canonical_s.as_bytes());
    let hex = format!("{:x}", h.finalize());
    format!("{slug}-{}", &hex[..8])
}

fn metadata_slug(path: &str) -> String {
    path.chars()
        .map(|c| match c {
            '/' | '\\' => '-',
            c if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') => c,
            _ => '-',
        })
        .collect()
}

/// Per-drive global paths. Computed once per Drive open.
#[derive(Debug, Clone)]
pub struct DrivePaths {
    /// Metadata root for this drive, `~/.chan/drives/<metadata_key>/`.
    pub root: PathBuf,
    /// Per-drive sessions directory. Opaque JSON; chan-drive does
    /// not interpret. Apps put window/pane layout files here.
    pub sessions: PathBuf,
    /// Per-drive search-index directory (tantivy segments + config).
    pub index: PathBuf,
    /// Per-drive graph database (sqlite). Regenerable from the
    /// source-of-truth markdown, but a rebuild is more expensive
    /// than a search reindex.
    pub graph_db: PathBuf,
    /// Per-drive directory carrying graph-related sidecar state:
    /// the `rebuild.inprogress` marker (written before a graph
    /// rebuild starts, removed after the search index commits;
    /// presence on `Drive::open` flags the drive as needing a full
    /// reindex) and the persisted `rename_log.json`. Sibling of
    /// `graph_db` (same parent), so wiping this directory reclaims
    /// both the DB and the sidecars in one step.
    pub graph_dir: PathBuf,
    /// Per-drive lock dir. Holds the index-writer lockfile that
    /// prevents two processes from writing the same drive's index.
    pub lock: PathBuf,
    /// Per-drive tokens dir. App-level surface (chan-server stores
    /// its bearer token here, mode 0600). chan-drive only allocates
    /// the directory; it does not read or write inside.
    pub tokens: PathBuf,
    /// Per-drive trash dir. Holds soft-deleted files / dirs as
    /// `<id>/{meta.json, payload[/]}`. Lazily GC'd on Drive::open
    /// and on every trash_* call.
    pub trash: PathBuf,
    /// Per-drive code/SLOC report. JSONL serialized by
    /// `chan-report`, persisted atomically by chan-drive's
    /// ReportState writer thread. The report is regenerable from a
    /// full rescan if missing or corrupt.
    pub report: PathBuf,
    /// systacean-24: per-drive Drafts dir. Holds in-progress
    /// drafts as `<name>/draft.md + companions` (e.g.
    /// `untitled-1/draft.md` plus pasted images). The Drafts
    /// subtree sits in `~/.chan/drives/<metadata_key>/drafts/` so
    /// the user's drive root stays clean of uncommitted scratch
    /// work (SCM-friendly per the addendum-a spec). Rich Prompt
    /// history (`rich-prompt-N/`) lives here too. The watcher +
    /// indexer walk this subtree alongside the drive root so drafts
    /// participate in search + graph.
    pub drafts: PathBuf,
}

/// Resolve the per-drive global paths for a metadata key. The key is
/// the drive's `KnownDrive.metadata_key`, assigned at registration
/// time and preserved across `Library::move_drive`. Callers that
/// hold a `&Path` should look the key up through
/// `Library::drive_paths_for` rather than recomputing it from the
/// path, so the registry stays the source of truth after moves.
pub fn drive_paths_for_metadata_key(metadata_key: &str) -> DrivePaths {
    let root = drives_dir().join(metadata_key);
    let graph_dir = root.join("graph");
    DrivePaths {
        root: root.clone(),
        sessions: root.join("sessions"),
        index: root.join("index"),
        graph_db: graph_dir.join("graph.sqlite"),
        graph_dir,
        lock: root.join("locks"),
        tokens: root.join("tokens"),
        trash: root.join("trash"),
        report: root.join("report").join("report.jsonl"),
        drafts: root.join("drafts"),
    }
}

/// Create the standard per-drive metadata directory skeleton.
pub fn ensure_drive_metadata_dirs(metadata_key: &str) -> std::io::Result<DrivePaths> {
    let paths = drive_paths_for_metadata_key(metadata_key);
    std::fs::create_dir_all(&paths.sessions)?;
    std::fs::create_dir_all(&paths.trash)?;
    std::fs::create_dir_all(paths.report.parent().expect("report has parent"))?;
    std::fs::create_dir_all(&paths.lock)?;
    std::fs::create_dir_all(&paths.graph_dir)?;
    std::fs::create_dir_all(&paths.index)?;
    std::fs::create_dir_all(&paths.drafts)?;
    std::fs::create_dir_all(&paths.tokens)?;
    Ok(paths)
}

/// Per-drive metadata parent directories. Used by the orphan-sweep
/// path to walk metadata roots and reconcile against the registry's
/// metadata-key set. Returns absolute paths; it may not exist on a
/// fresh install, callers must handle that.
pub fn drive_subsystem_dirs() -> Vec<PathBuf> {
    vec![drives_dir()]
}

/// One cloud-storage provider's root the first-launch picker can
/// suggest as a chan drive location. The `suggested_root` is the
/// concrete directory chan would land its drive in (provider root
/// joined with "Chan" by convention so iOS / Android Files-app
/// users see a recognizable directory name across devices).
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
    fn metadata_key_is_stable_and_path_slugged() {
        let tmp = tempfile::TempDir::new().unwrap();
        let k1 = metadata_key_for_root(tmp.path());
        let k2 = metadata_key_for_root(tmp.path());
        assert_eq!(k1, k2);
        assert!(k1.contains('-'));
        let suffix = k1.rsplit_once('-').unwrap().1;
        assert_eq!(suffix.len(), 8);
        assert!(suffix.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn metadata_key_keeps_example_readable_prefix() {
        let p = PathBuf::from("/Users/fiorix/dev/github.com/fiorix/chan");
        let key = metadata_key_for_root(&p);
        assert!(key.starts_with("-Users-fiorix-dev-github.com-fiorix-chan-"));
        assert_eq!(key.rsplit_once('-').unwrap().1.len(), 8);
    }

    #[test]
    fn drive_paths_share_the_same_metadata_root() {
        let key = "-tmp-drive-deadbeef";
        let p = drive_paths_for_metadata_key(key);
        for path in [
            &p.sessions,
            &p.index,
            &p.lock,
            &p.tokens,
            &p.trash,
            &p.graph_dir,
            &p.drafts,
        ] {
            assert!(path.starts_with(&p.root));
        }
        assert_eq!(p.root.file_name().and_then(|s| s.to_str()), Some(key));
    }

    #[test]
    fn drive_subsystem_dirs_covers_each_sidecar_root() {
        let key = "-tmp-drive-deadbeef";
        let p = drive_paths_for_metadata_key(key);
        let dirs = drive_subsystem_dirs();
        assert_eq!(dirs, vec![drives_dir()]);
        assert_eq!(p.root.parent(), Some(drives_dir().as_path()));
    }

    #[test]
    fn ensure_drive_metadata_dirs_creates_expected_subdirs() {
        let key = format!("test-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap());
        let paths = ensure_drive_metadata_dirs(&key).unwrap();
        for dir in [
            &paths.sessions,
            &paths.trash,
            paths.report.parent().unwrap(),
            &paths.lock,
            &paths.graph_dir,
            &paths.index,
            &paths.drafts,
            &paths.tokens,
        ] {
            assert!(dir.is_dir(), "metadata subdir missing: {dir:?}");
        }
        std::fs::remove_dir_all(paths.root).unwrap();
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
