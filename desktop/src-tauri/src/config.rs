//! Desktop-only config.
//!
//! The chan registry (`~/.chan/config.toml`) is the source of truth
//! for which workspaces exist. This file holds only desktop-specific
//! state that has no place in chan proper:
//!
//! - `outbound`: remote-workspace URLs the user explicitly attached.
//!   The desktop owns only the webview/window state for these
//!   entries, not the remote process or token lifecycle.
//! - `window_configs`: LRU stack of closed-window labels + URL hashes
//!   so a freshly-opened workspace window picks up the panes / tabs /
//!   selections / overlay state of the previous window for that
//!   workspace instead of starting blank.
//!
//! Per-workspace serve URLs are intentionally NOT persisted: chan rotates
//! the bearer token on every `chan serve`, so a saved URL would
//! decay to garbage between launches. The URL lives in `AppState`
//! in memory while a serve is running, and the desktop webview
//! reloads it fresh on every On toggle.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Cap on how many window configs we retain in the LRU stack.
/// Newest first; older entries past the cap are evicted on save.
/// Twenty is roomy enough for several concurrently-open workspaces
/// without risking unbounded growth from an open-close-reopen loop.
pub const MAX_WINDOW_CONFIGS: usize = 20;

/// An already-running chan server that chan-desktop opens by URL.
/// The URL may carry a bearer token. It is persisted verbatim after
/// validation because the remote server owns token rotation and
/// shutdown; desktop owns only the attachment row and webview
/// window state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutboundWorkspace {
    /// Stable desktop-local identifier used for row actions and
    /// window restore. Not sent to the remote server.
    pub id: String,
    /// User-pasted HTTP(S) URL, including any bearer token.
    pub url: String,
    /// Optional user label for the launcher row and window title.
    #[serde(default)]
    pub label: String,
    /// Wall-clock millis when the attachment was created.
    #[serde(default)]
    pub added_at: u64,
}

/// Per-window layout snapshot pushed when a workspace webview closes,
/// popped when the same workspace opens its next webview. The Tauri
/// window label is the join key: reusing it forwards the SPA's
/// `?w=<label>` lookup so the per-window `session.json` in the
/// workspace hydrates the panes / tabs that were open before. The URL
/// hash carries the overlay state (file browser selection, search
/// query, graph scope, etc.) that chan deliberately keeps out of
/// `session.json` so shareable URLs stay shareable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Workspace identity:
    ///   * local workspaces: canonical filesystem path (matches the
    ///     `AppState.serves` key).
    ///   * outbound workspaces: `"outbound:<id>"`, namespaced by the
    ///     desktop-local attachment id because the URL can change.
    pub key: String,
    /// Tauri window label this config was last bound to. The label
    /// is hash-prefixed (`workspace-<16hex>-<seq>`) so it implicitly
    /// encodes the workspace identity too — reusing it produces the
    /// same prefix and the per-workspace close-on-exit cleanup walker
    /// still matches.
    pub window_label: String,
    /// URL hash (everything after `#`, without the leading hash
    /// character). Empty when the SPA never wrote a hash. Applied
    /// verbatim on the next open so file-browser selection, search
    /// query, graph scope, and other overlay-encoded knobs round
    /// trip across the close/open cycle.
    #[serde(default)]
    pub url_hash: String,
    /// Browser-style zoom level, 1.0 = 100 %. Persists across the
    /// close/open cycle so Cmd++ / Cmd+- / Cmd+0 chord state
    /// survives a session restart. `#[serde(default
    /// = "default_zoom")]` keeps `config.json` entries without the
    /// field loadable (missing reads as 1.0).
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
    /// Explicit outbound URL attachments. These are non-owned
    /// remote workspaces that desktop opens by URL.
    #[serde(default)]
    pub outbound: Vec<OutboundWorkspace>,
    /// LRU stack of closed window configs. Newest at index 0. A
    /// fresh workspace webview pops the most-recent matching entry on
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

/// Identity key for a local-workspace WindowConfig. Matches the
/// `AppState.serves` key so a window-config lookup uses the same
/// canonical-path normalisation as the workspace registry.
pub fn local_window_key(workspace_key: &str) -> String {
    workspace_key.to_string()
}

/// Identity key for an outbound URL attachment.
pub fn outbound_window_key(id: &str) -> String {
    format!("outbound:{id}")
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

/// Pop the most-recent WindowConfig matching `key` whose label is NOT
/// currently live, removing it from the stack. Returns `None` when no
/// such entry exists. Callers save the config afterwards; this
/// function only mutates the in-memory `Config`.
///
/// `is_label_live` exists for bury-on-close: a buried (hidden) window
/// is still a live webview AND has a fresh stack entry captured at
/// bury time. A new same-workspace window must neither reuse that
/// label (Tauri labels are unique per process) nor pop-and-discard the
/// entry — the buried window still needs it if the app quits before an
/// unbury. Skipping live-label entries leaves them in place; across an
/// app restart nothing is live and the stack pops normally.
pub fn pop_window_config(
    cfg: &mut Config,
    key: &str,
    is_label_live: impl Fn(&str) -> bool,
) -> Option<WindowConfig> {
    let pos = cfg
        .window_configs
        .iter()
        .position(|w| w.key == key && !is_label_live(&w.window_label))?;
    Some(cfg.window_configs.remove(pos))
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn current_millis() -> u64 {
    now_millis()
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
        push_window_config(&mut cfg, entry("/workspace/a", "workspace-a-0", "", 100));
        push_window_config(
            &mut cfg,
            entry("/workspace/b", "workspace-b-0", "files=1", 200),
        );
        assert_eq!(cfg.window_configs[0].window_label, "workspace-b-0");
        assert_eq!(cfg.window_configs[1].window_label, "workspace-a-0");
    }

    #[test]
    fn push_dedupes_by_window_label() {
        // Pushing twice for the same label collapses to one entry
        // at the top, not two. Prevents stack growth from
        // re-opening + re-closing the same window in a loop.
        let mut cfg = Config::default();
        push_window_config(&mut cfg, entry("/workspace/a", "workspace-a-0", "old", 100));
        push_window_config(&mut cfg, entry("/workspace/a", "workspace-a-0", "new", 200));
        assert_eq!(cfg.window_configs.len(), 1);
        assert_eq!(cfg.window_configs[0].url_hash, "new");
    }

    #[test]
    fn push_caps_at_max() {
        let mut cfg = Config::default();
        for i in 0..MAX_WINDOW_CONFIGS + 5 {
            let label = format!("workspace-a-{i}");
            push_window_config(&mut cfg, entry("/workspace/a", &label, "", 100 + i as u64));
        }
        assert_eq!(cfg.window_configs.len(), MAX_WINDOW_CONFIGS);
        // The five oldest got evicted; the newest stays at the top.
        let newest = format!("workspace-a-{}", MAX_WINDOW_CONFIGS + 4);
        assert_eq!(cfg.window_configs[0].window_label, newest);
    }

    #[test]
    fn pop_returns_most_recent_for_key() {
        let mut cfg = Config::default();
        push_window_config(
            &mut cfg,
            entry("/workspace/a", "workspace-a-0", "older", 100),
        );
        push_window_config(&mut cfg, entry("/workspace/b", "workspace-b-0", "", 200));
        push_window_config(
            &mut cfg,
            entry("/workspace/a", "workspace-a-1", "newer", 300),
        );
        let popped = pop_window_config(&mut cfg, "/workspace/a", |_| false).unwrap();
        assert_eq!(popped.window_label, "workspace-a-1");
        assert_eq!(popped.url_hash, "newer");
        // The older /workspace/a entry is still on the stack.
        let popped2 = pop_window_config(&mut cfg, "/workspace/a", |_| false).unwrap();
        assert_eq!(popped2.window_label, "workspace-a-0");
        // /workspace/b is untouched.
        assert_eq!(cfg.window_configs.len(), 1);
        assert_eq!(cfg.window_configs[0].window_label, "workspace-b-0");
    }

    #[test]
    fn pop_returns_none_when_no_match() {
        let mut cfg = Config::default();
        push_window_config(&mut cfg, entry("/workspace/a", "workspace-a-0", "", 100));
        assert!(pop_window_config(&mut cfg, "/workspace/missing", |_| false).is_none());
        assert_eq!(cfg.window_configs.len(), 1);
    }

    #[test]
    fn pop_skips_live_labels_and_leaves_them_on_the_stack() {
        // Bury-on-close: `workspace-a-1` is a buried (hidden but live)
        // window with a bury-time entry on the stack. A new window of
        // the same workspace must pop PAST it to the older dead entry,
        // leaving the live one in place for the quit-while-buried
        // restore.
        let mut cfg = Config::default();
        push_window_config(
            &mut cfg,
            entry("/workspace/a", "workspace-a-0", "dead", 100),
        );
        push_window_config(
            &mut cfg,
            entry("/workspace/a", "workspace-a-1", "live", 200),
        );
        let popped =
            pop_window_config(&mut cfg, "/workspace/a", |label| label == "workspace-a-1").unwrap();
        assert_eq!(popped.window_label, "workspace-a-0");
        assert_eq!(cfg.window_configs.len(), 1);
        assert_eq!(cfg.window_configs[0].window_label, "workspace-a-1");
        // Every entry live -> nothing pops, nothing is dropped.
        assert!(pop_window_config(&mut cfg, "/workspace/a", |_| true).is_none());
        assert_eq!(cfg.window_configs.len(), 1);
    }

    #[test]
    fn window_config_zoom_level_defaults_to_one_on_missing_field() {
        // A `config.json` entry without a `zoom_level` field must
        // stay loadable as 1.0 instead of failing the load and
        // dropping the entire window-config stack on the floor.
        let missing_zoom = r#"{
            "key": "/workspace/legacy",
            "window_label": "workspace-legacy-0",
            "url_hash": "files=1",
            "saved_at": 12345
        }"#;
        let cfg: WindowConfig = serde_json::from_str(missing_zoom).expect("legacy load");
        assert_eq!(cfg.zoom_level, 1.0);
        assert_eq!(cfg.url_hash, "files=1");
    }

    #[test]
    fn window_config_zoom_level_round_trips() {
        let entry = WindowConfig {
            key: "/workspace/a".to_string(),
            window_label: "workspace-a-0".to_string(),
            url_hash: String::new(),
            zoom_level: 1.4,
            saved_at: 0,
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        let back: WindowConfig = serde_json::from_str(&json).expect("deserialize");
        assert!((back.zoom_level - 1.4).abs() < f64::EPSILON);
    }

    #[test]
    fn outbound_window_key_namespaced_apart_from_local() {
        let outbound = outbound_window_key("remote-1");
        assert_ne!(local_window_key("remote-1"), outbound);
    }

    #[test]
    fn config_defaults_outbound_empty() {
        let cfg = Config::default();
        assert!(cfg.outbound.is_empty());
    }

    #[test]
    fn outbound_workspace_label_defaults_empty() {
        let raw = r#"{
            "id": "remote-1",
            "url": "http://127.0.0.1:4000/?t=abc"
        }"#;
        let workspace: OutboundWorkspace = serde_json::from_str(raw).expect("legacy load");
        assert_eq!(workspace.id, "remote-1");
        assert_eq!(workspace.label, "");
        assert_eq!(workspace.added_at, 0);
    }
}
