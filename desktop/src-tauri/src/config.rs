//! Desktop-only sidecar config.
//!
//! The chan registry (`~/.chan/config.toml`) is the source of truth
//! for which drives exist. This file holds only desktop-specific
//! state that has no place in chan proper:
//!
//! - `sidecar`: per-drive UI state (currently just the last bound
//!   port), keyed by canonical drive path so a `mv` on disk doesn't
//!   silently revive stale state for a different drive.
//! - `window_configs`: LRU stack of closed-window labels + URL hashes
//!   so a freshly-opened drive window picks up the panes / tabs /
//!   selections / overlay state of the previous window for that
//!   drive instead of starting blank.
//!
//! Per-drive serve URLs are intentionally NOT persisted: chan rotates
//! the bearer token on every `chan serve`, so a saved URL would
//! decay to garbage between launches. The URL lives in `AppState`
//! in memory while a serve is running, and the desktop webview
//! reloads it fresh on every On toggle.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Cap on how many window configs we retain in the LRU stack.
/// Newest first; older entries past the cap are evicted on save.
/// Twenty matches the bug report's "keep up to 20" ask and is
/// roomy enough for several concurrently-open drives without
/// risking unbounded growth from an open-close-reopen loop.
pub const MAX_WINDOW_CONFIGS: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DriveSidecar {
    /// Port the drive's `chan serve` last bound to, persisted so a
    /// stop-then-start cycle reuses the same port and any browser
    /// tabs the user has open keep their URL valid.
    #[serde(default)]
    pub last_port: Option<u16>,
    /// `fullstack-b-28a`: per-drive feature toggle stub. Persisted
    /// in chan-desktop's sidecar config until `systacean-27` lands
    /// the chan-drive-side config API; `-b-28b` will swap this stub
    /// for the real API call without changing the SPA-facing IPC
    /// shape.
    ///
    /// Both default OFF (lean drive; BM25-only). Toggled via the
    /// launcher row's expandable feature panel; `-a-76` will mirror
    /// the same toggles into Settings.
    #[serde(default)]
    pub features: DriveFeatures,
}

/// `fullstack-b-28a`: per-drive feature toggles. Surfaced via the
/// launcher row's expand panel. The pair is wide enough to absorb
/// future toggles (chan-report variants, alternate embedding
/// models, etc.) without re-shaping the IPC contract.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct DriveFeatures {
    /// Semantic search via BGE-small embeddings. Default OFF.
    /// Round-2-plan §"Pre-flight feature toggles": enabling
    /// triggers a download of the shared model file (~63 MB) +
    /// a per-drive dense-vector index pass alongside the BM25
    /// walk.
    #[serde(default)]
    pub bge: bool,
    /// File classification + stats reports (tokei + per-language
    /// SLOC + Basic COCOMO). Default OFF. Round-2-plan §"Pre-
    /// flight feature toggles": enabling triggers a chan-report
    /// pass maintained incrementally from filesystem events.
    #[serde(default)]
    pub reports: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TunnelConfig {
    /// Port the tunnel listener should try to bind on the next
    /// "Listen On" toggle. `0` means "let the OS pick". Persisted
    /// across desktop restarts so a user who has a specific port in
    /// mind (matched by an `ssh -R` config) does not have to retype
    /// it on every launch.
    ///
    /// The "listening" state itself is NOT persisted: every desktop
    /// start comes up with the tunnel off, matching the explicit
    /// click-to-listen UX.
    #[serde(default)]
    pub preferred_port: u16,
    /// Last bearer/label the user typed into the listen panel.
    /// Empty means "no preference; suggest a default". Persisted so
    /// a user who picked a memorable label keeps it across restarts.
    /// Sanitised before save: enforced to pass
    /// `chan_tunnel_proto::is_valid_username` on the way in.
    #[serde(default)]
    pub preferred_label: String,
    /// Last drive name the user typed. Empty means "no preference".
    /// Persisted with the same sanitisation contract as
    /// `preferred_label` (`is_valid_drive_name`).
    #[serde(default)]
    pub preferred_drive: String,
}

/// Per-window layout snapshot pushed when a drive webview closes,
/// popped when the same drive opens its next webview. The Tauri
/// window label is the join key: reusing it forwards the SPA's
/// `?w=<label>` lookup so the per-window `session.json` in the
/// drive hydrates the panes / tabs that were open before. The URL
/// hash carries the overlay state (file browser selection, search
/// query, graph scope, etc.) that chan deliberately keeps out of
/// `session.json` so shareable URLs stay shareable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Drive identity:
    ///   * local drives: canonical filesystem path (matches the
    ///     `AppState.serves` key).
    ///   * tunneled drives: `"tunnel:<label>/<drive>"`, namespaced
    ///     to keep local and remote drives with colliding names
    ///     distinct.
    pub key: String,
    /// Tauri window label this config was last bound to. The label
    /// is hash-prefixed (`drive-<16hex>-<seq>` /
    /// `tunnel-<16hex>-<seq>`) so it implicitly encodes the drive
    /// identity too — reusing it produces the same prefix and the
    /// per-drive close-on-exit cleanup walker still matches.
    pub window_label: String,
    /// URL hash (everything after `#`, without the leading hash
    /// character). Empty when the SPA never wrote a hash. Applied
    /// verbatim on the next open so file-browser selection, search
    /// query, graph scope, and other overlay-encoded knobs round
    /// trip across the close/open cycle.
    #[serde(default)]
    pub url_hash: String,
    /// Browser-style zoom level, 1.0 = 100 %. Persists across the
    /// close/open cycle so Cmd++ / Cmd+- / Cmd+0 chord state from
    /// `fullstack-b-19` survives a session restart. `#[serde(default
    /// = "default_zoom")]` keeps backward compat with pre-`-b-19`
    /// `config.json` entries (missing field reads as 1.0).
    #[serde(default = "default_zoom")]
    pub zoom_level: f64,
    /// Wall-clock millis when this config was pushed. Newest first
    /// in the stack; only used for diagnostics + LRU eviction.
    pub saved_at: u64,
}

fn default_zoom() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Per-drive UI state, keyed by canonical drive path.
    #[serde(default)]
    pub sidecar: HashMap<String, DriveSidecar>,
    /// Tunnel listener preferences. Defaults to `preferred_port = 0`
    /// (OS-assigned) until the user types a specific number.
    #[serde(default)]
    pub tunnel: TunnelConfig,
    /// LRU stack of closed window configs. Newest at index 0. A
    /// fresh drive webview pops the most-recent matching entry on
    /// open so the user re-enters the same panes / tabs / overlays
    /// they left behind. Capped at `MAX_WINDOW_CONFIGS`; oldest
    /// evicted past that.
    #[serde(default)]
    pub window_configs: Vec<WindowConfig>,
}

pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            path: config_path()?,
        })
    }

    pub fn get(&self) -> io::Result<Config> {
        match fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(e),
        }
    }

    pub fn save(&mut self, cfg: &Config) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let bytes = serde_json::to_vec_pretty(cfg)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, bytes)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

/// Identity key for a local-drive WindowConfig. Matches the
/// `AppState.serves` key so a window-config lookup uses the same
/// canonical-path normalisation as the drive registry.
pub fn local_window_key(drive_key: &str) -> String {
    drive_key.to_string()
}

/// Identity key for a tunneled-drive WindowConfig. Namespaced so a
/// local drive at `/home/alex/notes` and a tunneled drive with
/// `(label, drive) = (_, "notes")` don't share the same stack
/// entry (they have different session.json files in different
/// drives).
pub fn tunnel_window_key(tenant_label: &str, drive: &str) -> String {
    format!("tunnel:{tenant_label}/{drive}")
}

/// Push a window config to the top of the LRU stack and persist.
/// Older entries with the same `window_label` are dropped so the
/// stack stays compact (one entry per label across all keys).
/// Trims to `MAX_WINDOW_CONFIGS`.
pub fn push_window_config(cfg: &mut Config, mut entry: WindowConfig) {
    if entry.saved_at == 0 {
        entry.saved_at = now_millis();
    }
    cfg.window_configs
        .retain(|w| w.window_label != entry.window_label);
    cfg.window_configs.insert(0, entry);
    cfg.window_configs.truncate(MAX_WINDOW_CONFIGS);
}

/// Pop the most-recent WindowConfig matching `key`, removing it
/// from the stack. Returns `None` when no entry exists. Callers
/// save the config afterwards; this function only mutates the
/// in-memory `Config`.
pub fn pop_window_config(cfg: &mut Config, key: &str) -> Option<WindowConfig> {
    let pos = cfg.window_configs.iter().position(|w| w.key == key)?;
    Some(cfg.window_configs.remove(pos))
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn config_path() -> io::Result<PathBuf> {
    let base = if cfg!(target_os = "linux") {
        dirs::config_dir()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no config dir"))?
            .join("chan-desktop")
    } else {
        dirs::config_dir()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no config dir"))?
            .join("Chan Desktop")
    };
    Ok(base.join("config.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str, label: &str, hash: &str, saved_at: u64) -> WindowConfig {
        WindowConfig {
            key: key.to_string(),
            window_label: label.to_string(),
            url_hash: hash.to_string(),
            zoom_level: 1.0,
            saved_at,
        }
    }

    #[test]
    fn push_inserts_at_front() {
        let mut cfg = Config::default();
        push_window_config(&mut cfg, entry("/drive/a", "drive-a-0", "", 100));
        push_window_config(&mut cfg, entry("/drive/b", "drive-b-0", "files=1", 200));
        assert_eq!(cfg.window_configs[0].window_label, "drive-b-0");
        assert_eq!(cfg.window_configs[1].window_label, "drive-a-0");
    }

    #[test]
    fn push_dedupes_by_window_label() {
        // Pushing twice for the same label collapses to one entry
        // at the top, not two. Prevents stack growth from
        // re-opening + re-closing the same window in a loop.
        let mut cfg = Config::default();
        push_window_config(&mut cfg, entry("/drive/a", "drive-a-0", "old", 100));
        push_window_config(&mut cfg, entry("/drive/a", "drive-a-0", "new", 200));
        assert_eq!(cfg.window_configs.len(), 1);
        assert_eq!(cfg.window_configs[0].url_hash, "new");
    }

    #[test]
    fn push_caps_at_max() {
        let mut cfg = Config::default();
        for i in 0..MAX_WINDOW_CONFIGS + 5 {
            let label = format!("drive-a-{i}");
            push_window_config(&mut cfg, entry("/drive/a", &label, "", 100 + i as u64));
        }
        assert_eq!(cfg.window_configs.len(), MAX_WINDOW_CONFIGS);
        // The five oldest got evicted; the newest stays at the top.
        let newest = format!("drive-a-{}", MAX_WINDOW_CONFIGS + 4);
        assert_eq!(cfg.window_configs[0].window_label, newest);
    }

    #[test]
    fn pop_returns_most_recent_for_key() {
        let mut cfg = Config::default();
        push_window_config(&mut cfg, entry("/drive/a", "drive-a-0", "older", 100));
        push_window_config(&mut cfg, entry("/drive/b", "drive-b-0", "", 200));
        push_window_config(&mut cfg, entry("/drive/a", "drive-a-1", "newer", 300));
        let popped = pop_window_config(&mut cfg, "/drive/a").unwrap();
        assert_eq!(popped.window_label, "drive-a-1");
        assert_eq!(popped.url_hash, "newer");
        // The older /drive/a entry is still on the stack.
        let popped2 = pop_window_config(&mut cfg, "/drive/a").unwrap();
        assert_eq!(popped2.window_label, "drive-a-0");
        // /drive/b is untouched.
        assert_eq!(cfg.window_configs.len(), 1);
        assert_eq!(cfg.window_configs[0].window_label, "drive-b-0");
    }

    #[test]
    fn pop_returns_none_when_no_match() {
        let mut cfg = Config::default();
        push_window_config(&mut cfg, entry("/drive/a", "drive-a-0", "", 100));
        assert!(pop_window_config(&mut cfg, "/drive/missing").is_none());
        assert_eq!(cfg.window_configs.len(), 1);
    }

    #[test]
    fn window_config_zoom_level_defaults_to_one_on_missing_field() {
        // `fullstack-b-19`: existing `config.json` files predate
        // the `zoom_level` field. The serde-default keeps them
        // loadable as 1.0 instead of failing the load and dropping
        // the entire window-config stack on the floor.
        let pre_b19 = r#"{
            "key": "/drive/legacy",
            "window_label": "drive-legacy-0",
            "url_hash": "files=1",
            "saved_at": 12345
        }"#;
        let cfg: WindowConfig = serde_json::from_str(pre_b19).expect("legacy load");
        assert_eq!(cfg.zoom_level, 1.0);
        assert_eq!(cfg.url_hash, "files=1");
    }

    #[test]
    fn window_config_zoom_level_round_trips() {
        let entry = WindowConfig {
            key: "/drive/a".to_string(),
            window_label: "drive-a-0".to_string(),
            url_hash: String::new(),
            zoom_level: 1.4,
            saved_at: 0,
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        let back: WindowConfig = serde_json::from_str(&json).expect("deserialize");
        assert!((back.zoom_level - 1.4).abs() < f64::EPSILON);
    }

    #[test]
    fn tunnel_window_key_namespaced_apart_from_local() {
        // A local drive at /home/alex/notes and a tunneled drive
        // exposing `(_, "notes")` must not collide in the stack.
        assert_ne!(
            local_window_key("/home/alex/notes"),
            tunnel_window_key("alice", "notes"),
        );
    }

    #[test]
    fn drive_features_default_off() {
        // `fullstack-b-28a`: per-drive feature toggles default OFF
        // so a lean drive opens BM25-only. The user opts into BGE +
        // reports explicitly via the launcher's expand panel.
        let f = DriveFeatures::default();
        assert!(!f.bge);
        assert!(!f.reports);
    }

    #[test]
    fn drive_sidecar_features_missing_field_defaults_off() {
        // `fullstack-b-28a`: existing `config.json` predates the
        // `features` field. The serde-default keeps legacy entries
        // loadable as `{bge: false, reports: false}` instead of
        // failing the load and dropping the entire sidecar map.
        let pre_b28 = r#"{
            "last_port": 49991
        }"#;
        let cfg: DriveSidecar = serde_json::from_str(pre_b28).expect("legacy load");
        assert_eq!(cfg.last_port, Some(49991));
        assert!(!cfg.features.bge);
        assert!(!cfg.features.reports);
    }

    #[test]
    fn drive_sidecar_features_round_trip() {
        // The toggle pair survives a save+load cycle so a flip in
        // the launcher panel sticks across desktop restarts.
        let sidecar = DriveSidecar {
            last_port: Some(50000),
            features: DriveFeatures {
                bge: true,
                reports: false,
            },
        };
        let json = serde_json::to_string(&sidecar).expect("serialize");
        let back: DriveSidecar = serde_json::from_str(&json).expect("deserialize");
        assert!(back.features.bge);
        assert!(!back.features.reports);
        assert_eq!(back.last_port, Some(50000));
    }

    #[test]
    fn drive_sidecar_features_missing_partial_field_defaults() {
        // Partial migration: a future config might carry `bge: true`
        // but not `reports`. The serde-default on each field keeps
        // the missing one as false rather than failing the load.
        let partial = r#"{
            "features": { "bge": true }
        }"#;
        let cfg: DriveSidecar = serde_json::from_str(partial).expect("partial load");
        assert!(cfg.features.bge);
        assert!(!cfg.features.reports);
    }
}
