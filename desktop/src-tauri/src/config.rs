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
//! the bearer token on every `chan open`, so a saved URL would
//! decay to garbage between launches. The URL lives in `AppState`
//! in memory while a serve is running, and the desktop webview
//! reloads it fresh on every On toggle.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use chan_server::{DevserverEntry, DevserverInput, DevserverRegistry};
use serde::{Deserialize, Serialize};

use crate::devserver::DevserverConns;

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

/// A devserver the desktop dials out to: a headless `chan devserver`
/// running on some box (often reached over an `ssh -L` local forward).
/// Unlike `OutboundWorkspace` (one remote URL = one workspace), a
/// devserver is a multi-workspace aggregator: the desktop groups its
/// workspaces under one `[DEVSERVER {host}]` launcher section and drives
/// them through the devserver's management API.
///
/// This struct is the *local* connection recipe only: the full devserver URL
/// (scheme included) the desktop dials plus the user's connect `script` (the
/// control terminal runs it, e.g. an `ssh -L` invocation). The devserver owns
/// the per-workspace URLs/tokens and their lifecycle; the desktop persists
/// just enough to re-offer the connection and re-open its windows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Devserver {
    /// Stable desktop-local identifier used for row actions, window
    /// restore, and the in-memory connection-state map. Not sent to the
    /// devserver.
    pub id: String,
    /// The full devserver URL the desktop dials, scheme included
    /// (`https://box.example.com:8787`, or `http://127.0.0.1:8787` for an
    /// `ssh -L` loopback forward). The scheme is load-bearing: the dial path
    /// branches raw-tunnel vs proxied-HTTPS on it (the proxied + OAuth branch
    /// is a deferred follow-up). The parsed host is also the default
    /// `[DEVSERVER {host}]` section label until the devserver reports its own
    /// `host_label`. A missing port defaults from the scheme at dial time.
    pub url: String,
    /// The connect script the CONTROL TERMINAL runs in its PTY (typically
    /// an `ssh user@box -L {port}:localhost:{port} chan devserver ...`).
    /// It blocks for the life of the session; its return means the
    /// session ended (the CONTROL TERMINAL then offers re-run / disconnect).
    #[serde(default)]
    pub script: String,
    /// Optional user label for the launcher section header and window
    /// titles. Empty falls back to the URL host.
    #[serde(default)]
    pub label: String,
    /// Bearer token for the devserver. Write-only over the launcher wire: the
    /// registry reports its presence via `has_token` and never echoes the
    /// value back. Stored so an edit can keep it; empty means none. (The
    /// connect flow still scrapes/reads a fresh token at dial — this is the
    /// launcher-supplied credential for the deferred proxied/OAuth dial.)
    #[serde(default)]
    pub token: String,
    /// Wall-clock millis when the devserver was added.
    #[serde(default)]
    pub added_at: u64,
    /// Whether the control terminal auto-hides on a successful connect (the
    /// devserver form's "auto-hide control terminal on success" checkbox). The
    /// connect flow reads it to hide the control terminal programmatically (no
    /// bury notice) once connected.
    #[serde(default)]
    pub auto_hide_control: bool,
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
    ///   * remote (devserver) workspaces: `"remote:<id>"`, namespaced by the
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
    /// Configured devservers (multi-workspace aggregators the desktop
    /// dials out to). Each renders its own `[DEVSERVER {host}]` launcher
    /// section. The per-workspace URLs/tokens are NOT persisted (the
    /// devserver rotates them); only the connection recipe is.
    #[serde(default)]
    pub devservers: Vec<Devserver>,
    /// LRU stack of closed window configs. Newest at index 0. A
    /// fresh workspace webview pops the most-recent matching entry on
    /// open so the user re-enters the same panes / tabs / overlays
    /// they left behind. Capped at `MAX_WINDOW_CONFIGS`; oldest
    /// evicted past that.
    #[serde(default)]
    pub window_configs: Vec<WindowConfig>,
    /// The LOCAL library's pane-highlight colour (hex `#rrggbb`), or `None` for
    /// the default accent. Backs the [`LocalColorStore`](chan_server::LocalColorStore)
    /// the host reads when minting local windows (terminals + workspaces). The
    /// launcher's local-colour route writes it; the per-devserver colour lives on
    /// each [`Devserver`].
    #[serde(default)]
    pub local_color: Option<String>,
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

/// Side effect run after a devserver row is removed via the registry's
/// [`remove`](DevserverRegistry::remove) (the HTTP `DELETE` path), so that path
/// reaps a live connection/windows (the desktop's `teardown_devserver_connection`).
/// Set once, after the Tauri `AppHandle` exists (the registry installs before it),
/// via the shared [`OnceLock`] cell — the chan-server-side registry can't see
/// the `AppHandle` directly, so the desktop injects the teardown as a closure.
/// A no-op when nothing is live (removing a not-connected devserver).
pub type DevserverRemoveHook = Arc<dyn Fn(&str) + Send + Sync>;

/// chan-desktop's [`DevserverRegistry`] implementation — the bridge the
/// launcher's `/api/library/devservers` routes reach through
/// [`WorkspaceHost::devserver_registry`](chan_server::WorkspaceHost::devserver_registry).
/// It wraps the SHARED [`ConfigStore`] handle (the same `Arc<Mutex<ConfigStore>>`
/// the desktop's own commands and the window-config LRU use), so every config
/// write — devserver CRUD, window stack, outbound attachments — serializes
/// through one lock and can't lose an update to a concurrent full-file rewrite.
///
/// The token is write-only: `add`/`update` accept it, `list` and the returned
/// entry only report [`has_token`](DevserverEntry::has_token). A blank/absent
/// token on update keeps the stored one.
pub struct DevserverConfigRegistry {
    store: Arc<Mutex<ConfigStore>>,
    /// Filled (once the `AppHandle` exists) with the live-connection teardown;
    /// `remove` fires it after dropping a row so the HTTP `DELETE` reaps the
    /// same connection/windows the Tauri command does. Empty until then (and on
    /// headless surfaces) — `remove` then only drops the config row.
    on_remove: Arc<OnceLock<DevserverRemoveHook>>,
    /// The live connection map (shared with `AppState.devservers`), so `list`
    /// reports each row's [`DevserverEntry::connected`] — the launcher shows
    /// Connect vs Disconnect + gates Edit read-only off it.
    conns: Arc<DevserverConns>,
    /// The connected-devserver feed (shared with `AppState.devserver_feed`), so
    /// `list` resolves each row's `library_id` from the live window snapshot.
    /// `WorkspaceHost::pane_color` matches a devserver window's `library_id`
    /// against these entries to find its colour, so the projection MUST carry it.
    feed: Arc<crate::DevserverFeed>,
}

impl DevserverConfigRegistry {
    pub fn new(
        store: Arc<Mutex<ConfigStore>>,
        on_remove: Arc<OnceLock<DevserverRemoveHook>>,
        conns: Arc<DevserverConns>,
        feed: Arc<crate::DevserverFeed>,
    ) -> Self {
        Self {
            store,
            on_remove,
            conns,
            feed,
        }
    }
}

/// chan-desktop's [`LocalColorStore`](chan_server::LocalColorStore): the local
/// library's pane-highlight colour persisted in the desktop config
/// (`~/.chan/desktop`, the same shared store the devserver registry + window LRU
/// use, so every write serializes through one lock). The host reads it when
/// minting local windows; the launcher's local-colour route writes it.
pub struct LocalColorConfig {
    store: Arc<Mutex<ConfigStore>>,
}

impl LocalColorConfig {
    pub fn new(store: Arc<Mutex<ConfigStore>>) -> Self {
        Self { store }
    }
}

impl chan_server::LocalColorStore for LocalColorConfig {
    fn get(&self) -> Option<String> {
        self.store
            .lock()
            .unwrap()
            .get()
            .ok()
            .and_then(|c| c.local_color)
    }

    fn set(&self, color: Option<String>) -> Result<(), String> {
        let mut store = self.store.lock().unwrap();
        let mut cfg = store.get().map_err(|e| e.to_string())?;
        cfg.local_color = color;
        store.save(&cfg).map_err(|e| e.to_string())
    }
}

/// Project a stored [`Devserver`] to the launcher's wire [`DevserverEntry`],
/// eliding the token (only its presence, `has_token`, crosses the wire) and
/// joining the live connection state (`connected`) from `conns`.
/// Form + validate the desktop's stored dial URL from the launcher's host+port
/// (the wire model since the devserver form switched back to Host+Port, smoke
/// #3). The desktop persists the URL (the dial path, dedup, and window-restore
/// key are URL-based); `entry_from_devserver` re-exposes host+port on the wire.
fn devserver_url(host: &str, port: u16) -> Result<String, String> {
    let host = host.trim();
    if host.is_empty() {
        return Err("devserver host is required".to_string());
    }
    let url = format!("http://{host}:{port}");
    crate::devserver::parse_devserver_url(&url)?;
    Ok(url)
}

fn entry_from_devserver(
    d: &Devserver,
    conns: &DevserverConns,
    feed: &crate::DevserverFeed,
) -> DevserverEntry {
    // The desktop stores the dial URL (formed from the user's host+port); the wire
    // entry exposes host+port. A stored URL is always valid (add/update validate
    // it), so the parse should not fail; fall back defensively to (raw, 0).
    let (host, port) =
        crate::devserver::parse_devserver_url(&d.url).unwrap_or_else(|_| (d.url.clone(), 0));
    DevserverEntry {
        id: d.id.clone(),
        host,
        port,
        label: d.label.clone(),
        script: d.script.clone(),
        has_token: !d.token.is_empty(),
        // A live connection in the desktop's in-memory map means connected; a
        // headless surface installs no registry, so this never runs there.
        connected: conns.is_connected(&d.id),
        // The connected library id, learned from the live window feed (`None`
        // until this devserver is connected with ≥1 window). `pane_color` matches
        // a devserver window's `library_id` against this (in the feed) to resolve
        // its colour; no colour lives on the entry anymore (each library's colour
        // is on its own host).
        library_id: feed.library_id_of(&d.id),
        // Whether the control terminal auto-hides on connect success.
        auto_hide_control: d.auto_hide_control,
    }
}

impl DevserverRegistry for DevserverConfigRegistry {
    fn list(&self) -> Vec<DevserverEntry> {
        // Infallible by contract (mirrors the window feed): a read error
        // surfaces as an empty list, not a 500.
        let store = self.store.lock().unwrap();
        store
            .get()
            .map(|cfg| {
                cfg.devservers
                    .iter()
                    .map(|d| entry_from_devserver(d, &self.conns, &self.feed))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn add(&self, input: DevserverInput) -> Result<DevserverEntry, String> {
        let url = devserver_url(&input.host, input.port)?;
        let token = input.token.unwrap_or_default().trim().to_string();
        let mut store = self.store.lock().unwrap();
        let mut cfg = store.get().map_err(|e| e.to_string())?;
        let entry = Devserver {
            id: uuid::Uuid::new_v4().to_string(),
            url,
            script: input.script.unwrap_or_default(),
            label: input.label.unwrap_or_default(),
            token,
            added_at: now_millis(),
            auto_hide_control: input.auto_hide_control,
        };
        cfg.devservers.push(entry.clone());
        store.save(&cfg).map_err(|e| e.to_string())?;
        Ok(entry_from_devserver(&entry, &self.conns, &self.feed))
    }

    fn update(&self, id: &str, input: DevserverInput) -> Result<Option<DevserverEntry>, String> {
        let url = devserver_url(&input.host, input.port)?;
        let mut store = self.store.lock().unwrap();
        let mut cfg = store.get().map_err(|e| e.to_string())?;
        let Some(ds) = cfg.devservers.iter_mut().find(|d| d.id == id) else {
            return Ok(None);
        };
        ds.url = url;
        // label/script are full-replace (None/empty clears, which the display
        // path reads as "derive the label from the URL host" / "no script").
        ds.label = input.label.unwrap_or_default();
        ds.script = input.script.unwrap_or_default();
        // The auto-hide flag is full-replace like label/script (the edit form resubmits it).
        ds.auto_hide_control = input.auto_hide_control;
        // Token is the lone keep-on-blank field: a write-only credential the
        // launcher never reads back, so its edit form can't resubmit it.
        if let Some(tok) = input.token {
            let tok = tok.trim();
            if !tok.is_empty() {
                ds.token = tok.to_string();
            }
        }
        let entry = entry_from_devserver(ds, &self.conns, &self.feed);
        store.save(&cfg).map_err(|e| e.to_string())?;
        Ok(Some(entry))
    }

    fn remove(&self, id: &str) -> Result<bool, String> {
        {
            let mut store = self.store.lock().unwrap();
            let mut cfg = store.get().map_err(|e| e.to_string())?;
            let before = cfg.devservers.len();
            cfg.devservers.retain(|d| d.id != id);
            if cfg.devservers.len() == before {
                return Ok(false);
            }
            store.save(&cfg).map_err(|e| e.to_string())?;
        }
        // Row dropped: reap any live connection/windows so the HTTP DELETE
        // matches the Tauri command's teardown. The store lock is released
        // above first — the teardown locks the other AppState maps, never the
        // store. A no-op when the devserver wasn't connected.
        if let Some(hook) = self.on_remove.get() {
            hook(id);
        }
        Ok(true)
    }
}

/// Identity key for a local-workspace WindowConfig. Matches the
/// `AppState.serves` key so a window-config lookup uses the same
/// canonical-path normalisation as the workspace registry.
pub fn local_window_key(workspace_key: &str) -> String {
    workspace_key.to_string()
}

/// Identity key for a remote (devserver) URL attachment.
pub fn remote_window_key(id: &str) -> String {
    format!("remote:{id}")
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

/// chan-desktop keeps its config under `~/.chan/desktop/` — the same
/// `~/.chan` home as the CLI registry (`config.toml`), not a separate
/// OS app-data directory. On Windows that resolves to
/// `%USERPROFILE%\.chan\desktop\config.json`.
fn config_path() -> io::Result<PathBuf> {
    Ok(chan_workspace::paths::config_dir()
        .join("desktop")
        .join("config.json"))
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
    fn remote_window_key_namespaced_apart_from_local() {
        let remote = remote_window_key("remote-1");
        assert_ne!(local_window_key("remote-1"), remote);
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

    #[test]
    fn config_defaults_devservers_empty() {
        let cfg = Config::default();
        assert!(cfg.devservers.is_empty());
    }

    #[test]
    fn config_loads_without_devservers_field() {
        // A config.json that predates devservers must still load: serde
        // reads the missing key as the empty set so the load never fails
        // and drops the rest of the config.
        let raw = r#"{ "outbound": [], "window_configs": [] }"#;
        let cfg: Config = serde_json::from_str(raw).expect("load without devservers");
        assert!(cfg.devservers.is_empty());
    }

    #[test]
    fn config_devservers_round_trip() {
        let cfg = Config {
            devservers: vec![Devserver {
                id: "ds-1".into(),
                url: "http://127.0.0.1:8787".into(),
                script: "ssh box -L 8787:localhost:8787 chan devserver".into(),
                label: "lab box".into(),
                token: "tok_secret".into(),
                added_at: 42,
                auto_hide_control: true,
            }],
            ..Default::default()
        };
        let json = serde_json::to_string(&cfg).expect("serialize");
        let back: Config = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.devservers, cfg.devservers);
    }

    #[test]
    fn devserver_optional_fields_default() {
        // Only the connection essentials are required; script/label/
        // added_at default so a hand-written config stays loadable.
        let raw = r#"{
            "id": "ds-1",
            "url": "http://127.0.0.1:8787"
        }"#;
        let ds: Devserver = serde_json::from_str(raw).expect("minimal load");
        assert_eq!(ds.id, "ds-1");
        assert_eq!(ds.url, "http://127.0.0.1:8787");
        assert_eq!(ds.script, "");
        assert_eq!(ds.label, "");
        assert_eq!(ds.token, "");
        assert_eq!(ds.added_at, 0);
    }

    /// The registry projects a stored `Devserver` to a wire `DevserverEntry`
    /// with the token elided and `has_token` reporting its presence; it never
    /// echoes the token value.
    #[test]
    fn registry_list_elides_token_reports_has_token() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Mutex::new(ConfigStore {
            path: dir.path().join("config.json"),
        }));
        let reg = DevserverConfigRegistry::new(
            Arc::clone(&store),
            Arc::new(OnceLock::new()),
            Arc::new(crate::devserver::DevserverConns::default()),
            Arc::new(crate::DevserverFeed::default()),
        );
        let added = reg
            .add(DevserverInput {
                host: "box.example.com".into(),
                port: 8787,
                label: Some("lab".into()),
                script: Some("ssh -L …".into()),
                token: Some("tok_secret".into()),
                auto_hide_control: false,
            })
            .expect("add");
        assert!(added.has_token);
        assert_eq!(added.host, "box.example.com");
        assert_eq!(added.port, 8787);
        assert_eq!(added.library_id, None);
        let listed = reg.list();
        assert_eq!(listed.len(), 1);
        assert!(listed[0].has_token);
        // The token value never appears on the wire entry (no field for it);
        // the on-disk config still holds it for the connect path.
        let cfg = store.lock().unwrap().get().unwrap();
        assert_eq!(cfg.devservers[0].token, "tok_secret");
    }

    /// The wire `connected` flag the launcher reads
    /// (`GET /api/library/devservers` -> `list` -> `entry_from_devserver`)
    /// derives from the shared `DevserverConns` membership, so dropping the conn
    /// — what the desktop's `flip_devserver_control_dead` does when a control
    /// terminal dies — flips it false while the config row stays. This is the
    /// mechanism that guarantees the launcher never shows a dead devserver as
    /// connected, regardless of how the SPA survey is answered.
    #[test]
    fn registry_list_connected_tracks_conns_membership() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Mutex::new(ConfigStore {
            path: dir.path().join("config.json"),
        }));
        let conns = Arc::new(crate::devserver::DevserverConns::default());
        let reg = DevserverConfigRegistry::new(
            Arc::clone(&store),
            Arc::new(OnceLock::new()),
            Arc::clone(&conns),
            Arc::new(crate::DevserverFeed::default()),
        );
        let id = reg
            .add(DevserverInput {
                host: "127.0.0.1".into(),
                port: 8787,
                ..Default::default()
            })
            .expect("add")
            .id;
        // No live conn yet: disconnected.
        assert!(!reg.list()[0].connected);
        // A live conn (what connect_devserver_impl sets on a successful dial).
        conns.set(
            id.clone(),
            crate::devserver::DevserverConn {
                host: "127.0.0.1".into(),
                port: 8787,
                token: "tok".into(),
                name: "box".into(),
            },
        );
        assert!(reg.list()[0].connected);
        // The control terminal dies -> the flip drops the conn -> the wire entry
        // flips connected:false, but the persisted config row is untouched (so a
        // re-run/edit can reconnect it).
        conns.remove(&id);
        assert!(!reg.list()[0].connected);
        assert_eq!(store.lock().unwrap().get().unwrap().devservers.len(), 1);
    }

    /// `update` with a blank/absent token keeps the stored one; a non-blank
    /// token replaces it. URL/label/script are full-replace.
    #[test]
    fn registry_update_keeps_token_on_blank_replaces_on_set() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Mutex::new(ConfigStore {
            path: dir.path().join("config.json"),
        }));
        let reg = DevserverConfigRegistry::new(
            Arc::clone(&store),
            Arc::new(OnceLock::new()),
            Arc::new(crate::devserver::DevserverConns::default()),
            Arc::new(crate::DevserverFeed::default()),
        );
        let id = reg
            .add(DevserverInput {
                host: "127.0.0.1".into(),
                port: 8787,
                token: Some("tok_one".into()),
                ..Default::default()
            })
            .expect("add")
            .id;
        // Blank token + new url/label: token survives, the rest replace.
        let updated = reg
            .update(
                &id,
                DevserverInput {
                    host: "127.0.0.1".into(),
                    port: 9000,
                    label: Some("renamed".into()),
                    token: None,
                    ..Default::default()
                },
            )
            .expect("update")
            .expect("found");
        assert_eq!(updated.host, "127.0.0.1");
        assert_eq!(updated.port, 9000);
        assert_eq!(updated.label, "renamed");
        assert!(updated.has_token);
        assert_eq!(
            store.lock().unwrap().get().unwrap().devservers[0].token,
            "tok_one"
        );
        // A non-blank token replaces it.
        reg.update(
            &id,
            DevserverInput {
                host: "127.0.0.1".into(),
                port: 9000,
                token: Some("tok_two".into()),
                ..Default::default()
            },
        )
        .expect("update")
        .expect("found");
        assert_eq!(
            store.lock().unwrap().get().unwrap().devservers[0].token,
            "tok_two"
        );
    }

    #[test]
    fn registry_update_and_remove_missing_id_signal_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Mutex::new(ConfigStore {
            path: dir.path().join("config.json"),
        }));
        let reg = DevserverConfigRegistry::new(
            store,
            Arc::new(OnceLock::new()),
            Arc::new(crate::devserver::DevserverConns::default()),
            Arc::new(crate::DevserverFeed::default()),
        );
        let missing = reg
            .update(
                "nope",
                DevserverInput {
                    host: "127.0.0.1".into(),
                    port: 8787,
                    ..Default::default()
                },
            )
            .expect("update ok");
        assert!(missing.is_none());
        assert!(!reg.remove("nope").expect("remove ok"));
    }

    /// `remove` fires the teardown hook ONLY when a row was actually dropped,
    /// with that id — so the HTTP DELETE reaps the live connection, and a
    /// missing-id remove (or a not-found) doesn't fire a spurious teardown.
    #[test]
    fn registry_remove_fires_hook_only_when_a_row_was_removed() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Mutex::new(ConfigStore {
            path: dir.path().join("config.json"),
        }));
        let fired: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let hook_cell: Arc<OnceLock<DevserverRemoveHook>> = Arc::new(OnceLock::new());
        let fired_for_hook = Arc::clone(&fired);
        hook_cell
            .set(Arc::new(move |id: &str| {
                fired_for_hook.lock().unwrap().push(id.to_string())
            }))
            .ok();
        let reg = DevserverConfigRegistry::new(
            Arc::clone(&store),
            Arc::clone(&hook_cell),
            Arc::new(crate::devserver::DevserverConns::default()),
            Arc::new(crate::DevserverFeed::default()),
        );
        let id = reg
            .add(DevserverInput {
                host: "127.0.0.1".into(),
                port: 8787,
                ..Default::default()
            })
            .expect("add")
            .id;
        // Missing id: nothing dropped → hook must NOT fire.
        assert!(!reg.remove("nope").expect("remove ok"));
        assert!(fired.lock().unwrap().is_empty());
        // Real id: row dropped → hook fires once with that id.
        assert!(reg.remove(&id).expect("remove ok"));
        assert_eq!(fired.lock().unwrap().as_slice(), &[id]);
    }

    #[test]
    fn registry_add_rejects_an_empty_host() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Mutex::new(ConfigStore {
            path: dir.path().join("config.json"),
        }));
        let reg = DevserverConfigRegistry::new(
            store,
            Arc::new(OnceLock::new()),
            Arc::new(crate::devserver::DevserverConns::default()),
            Arc::new(crate::DevserverFeed::default()),
        );
        assert!(reg
            .add(DevserverInput {
                host: "".into(),
                port: 8787,
                ..Default::default()
            })
            .is_err());
    }
}
