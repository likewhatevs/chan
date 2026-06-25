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

use chan_server::{DevserverEntry, DevserverInput, DevserverRegistry, DevserverStatus};
use serde::{Deserialize, Serialize};

use crate::devserver::DevserverConns;

/// Cap on how many window configs we retain in the LRU stack.
/// Newest first; older entries past the cap are evicted on save.
/// Twenty is roomy enough for several concurrently-open workspaces
/// without risking unbounded growth from an open-close-reopen loop.
pub const MAX_WINDOW_CONFIGS: usize = 20;

/// Cap on how many distinct monitor signatures we remember per window in the
/// geometry LRU. Five covers a laptop that docks / undocks across a couple of
/// external-monitor layouts and flips back without losing any layout's stored
/// size + position. Newest signature first; older ones evicted past the cap.
pub const MAX_WINDOW_GEOMETRIES: usize = 5;

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

/// Plain monitor descriptor, decoupled from Tauri so the geometry math stays
/// unit-testable without a window system (this module links no Tauri). `serve`
/// maps each `tauri::Monitor` to one of these: the full bounds + `scale` form
/// the monitor SIGNATURE; the `work_*` usable area drives the on-screen clamp.
/// Physical pixels throughout (OS desktop coordinates).
#[derive(Debug, Clone, PartialEq)]
pub struct MonitorDesc {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub work_x: i32,
    pub work_y: i32,
    pub work_w: u32,
    pub work_h: u32,
    pub scale: f64,
}

/// One captured OS window geometry, tagged with the monitor signature it was
/// captured under. Physical pixels (OS desktop coordinates) so the clamp /
/// restore math is unambiguous across mixed-DPI monitors; the signature pins the
/// scale factor, so a matching-signature restore re-applies these verbatim with
/// no rescale. `x,y` is the OUTER (top-left) position; `w,h` the INNER (content)
/// size — matching `WebviewWindow::{outer_position, inner_size}` at capture and
/// `set_position` / `set_size` at apply.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowGeometry {
    /// Monitor signature at capture time (see [`monitor_signature`]). The
    /// geometry is only re-applied as a full restore when the live signature
    /// matches, so a stored `x,y` can't open a window off a desktop it no
    /// longer fits.
    pub monitor_sig: String,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    /// Wall-clock millis at capture; newest first within a record's
    /// per-signature LRU. Diagnostics + eviction order only.
    pub saved_at: u64,
}

/// Desktop-owned OS window geometry for one window, with a small
/// per-monitor-signature LRU so a machine that flips monitor layout and back
/// restores each layout's own size + position. Keyed by the (stable across a
/// bury / reopen) native window label — sibling to [`WindowConfig`], which holds
/// SPA restore state for outbound windows only. Geometry lives here for ALL
/// window classes (local / devserver / outbound) because only chan-desktop can
/// read / set OS window pixels — even when the SPA session itself is server-owned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowGeometryRecord {
    /// Native Tauri window label — the join key. Stable across a bury / reopen:
    /// outbound windows reuse their label; watcher windows reopen at the same
    /// `{library_id}::{window_id}`.
    pub window_label: String,
    /// Per-signature geometry LRU, newest first, capped at
    /// [`MAX_WINDOW_GEOMETRIES`].
    #[serde(default)]
    pub geometries: Vec<WindowGeometry>,
    /// Wall-clock millis of the most recent capture for this window; newest
    /// record first in the stack. Diagnostics + LRU eviction.
    pub saved_at: u64,
}

/// Result of resolving a window's stored geometry against the CURRENT monitor
/// signature ([`lookup_window_geometry`]).
#[derive(Debug, Clone, PartialEq)]
pub enum GeometryMatch {
    /// Signature matched: restore full position + size (the apply path clamps it
    /// on-screen as a safety net).
    Exact(WindowGeometry),
    /// No signature match: offer the SIZE only (the apply path clamps it to the
    /// current primary monitor); the OS picks the position.
    SizeOnly(WindowGeometry),
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
    /// Desktop-owned OS window geometry, one [`WindowGeometryRecord`] per window
    /// label, each with its own per-monitor-signature LRU. Sibling to
    /// `window_configs` (which is outbound-only SPA restore state): geometry is
    /// keyed by the stable native label and covers every window class, since only
    /// the desktop can read / set OS window pixels. Newest record first; capped
    /// at `MAX_WINDOW_CONFIGS` windows.
    #[serde(default)]
    pub window_geometry: Vec<WindowGeometryRecord>,
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
    /// reports each row's [`DevserverEntry::status`] — the launcher shows
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
        // The desktop's in-memory conn map is binary: present = connected,
        // absent = disconnected. It tracks no in-flight connect, so `connecting`
        // is never produced here; the launcher's optimistic bridge covers the
        // connect click until the next feed refetch settles the status. A
        // headless surface installs no registry, so this never runs there.
        status: if conns.is_connected(&d.id) {
            DevserverStatus::Connected
        } else {
            DevserverStatus::Disconnected
        },
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

/// Order-independent monitor signature: the monitor count plus each monitor's
/// full bounds and scale factor, sorted so the OS reporting monitors in a
/// different order doesn't change the string. The scale is stringified
/// (`{:.2}`) so float equality never bites. Geometry restore is gated on an
/// exact match of this, so it changes whenever the physical monitor layout or a
/// DPI scale changes (the cases where a stored position would land off-screen).
pub fn monitor_signature(mons: &[MonitorDesc]) -> String {
    let mut parts: Vec<String> = mons
        .iter()
        .map(|m| format!("{},{},{},{}@{:.2}", m.x, m.y, m.w, m.h, m.scale))
        .collect();
    parts.sort();
    format!("{}|{}", mons.len(), parts.join("|"))
}

/// Upsert a freshly-captured geometry into the window's per-signature LRU and
/// move the window's record to the front. The new signature replaces any prior
/// entry for the same signature (dedup) and goes to the front, capped at
/// [`MAX_WINDOW_GEOMETRIES`] so flipping monitor layouts and back keeps each
/// layout's own geometry. The records stack is capped at [`MAX_WINDOW_CONFIGS`]
/// windows. Best-effort callers save afterwards; this only mutates `cfg`.
pub fn push_window_geometry(cfg: &mut Config, label: &str, mut geom: WindowGeometry) {
    if geom.saved_at == 0 {
        geom.saved_at = now_millis();
    }
    let saved_at = geom.saved_at;
    if let Some(pos) = cfg
        .window_geometry
        .iter()
        .position(|r| r.window_label == label)
    {
        let mut rec = cfg.window_geometry.remove(pos);
        rec.geometries.retain(|g| g.monitor_sig != geom.monitor_sig);
        rec.geometries.insert(0, geom);
        rec.geometries.truncate(MAX_WINDOW_GEOMETRIES);
        rec.saved_at = saved_at;
        cfg.window_geometry.insert(0, rec);
    } else {
        cfg.window_geometry.insert(
            0,
            WindowGeometryRecord {
                window_label: label.to_string(),
                geometries: vec![geom],
                saved_at,
            },
        );
    }
    cfg.window_geometry.truncate(MAX_WINDOW_CONFIGS);
}

/// Resolve the geometry to apply for `label` under `current_sig`: an exact
/// signature match restores position + size ([`GeometryMatch::Exact`]);
/// otherwise the most-recent stored geometry is offered SIZE-only
/// ([`GeometryMatch::SizeOnly`]). `None` when nothing is stored for the label.
pub fn lookup_window_geometry(
    cfg: &Config,
    label: &str,
    current_sig: &str,
) -> Option<GeometryMatch> {
    let rec = cfg
        .window_geometry
        .iter()
        .find(|r| r.window_label == label)?;
    if let Some(g) = rec.geometries.iter().find(|g| g.monitor_sig == current_sig) {
        return Some(GeometryMatch::Exact(g.clone()));
    }
    rec.geometries.first().cloned().map(GeometryMatch::SizeOnly)
}

/// Shrink a size so it never exceeds the given (primary) work area, with a
/// 1px floor. Position is the OS's choice in the size-only fallback, so only
/// `w/h` are bounded.
pub fn clamp_size(w: u32, h: u32, work_w: u32, work_h: u32) -> (u32, u32) {
    (w.min(work_w).max(1), h.min(work_h).max(1))
}

/// Bounding box `(min_x, min_y, max_x, max_y)` of every monitor's WORK area, or
/// `None` when there are no monitors. The clamp keeps a restored window inside
/// this box so it can't open off the visible desktop.
pub fn union_work_bbox(mons: &[MonitorDesc]) -> Option<(i32, i32, i32, i32)> {
    if mons.is_empty() {
        return None;
    }
    let min_x = mons.iter().map(|m| m.work_x).min().unwrap();
    let min_y = mons.iter().map(|m| m.work_y).min().unwrap();
    let max_x = mons
        .iter()
        .map(|m| m.work_x + m.work_w as i32)
        .max()
        .unwrap();
    let max_y = mons
        .iter()
        .map(|m| m.work_y + m.work_h as i32)
        .max()
        .unwrap();
    Some((min_x, min_y, max_x, max_y))
}

/// Clamp a window rect so the WHOLE window stays inside the work-area bounding
/// box: shrink the size to fit if larger, then pull the top-left back so the
/// window can't open off the visible desktop. For a signature MATCH this is a
/// near-no-op (the stored rect was valid then); it earns its keep when the work
/// area shrank under the same physical layout (a dock / taskbar appeared).
pub fn clamp_rect_to_bbox(
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    bbox: (i32, i32, i32, i32),
) -> (i32, i32, u32, u32) {
    let (min_x, min_y, max_x, max_y) = bbox;
    let bw = (max_x - min_x).max(0) as u32;
    let bh = (max_y - min_y).max(0) as u32;
    let w = w.min(bw).max(1);
    let h = h.min(bh).max(1);
    // Highest top-left that still fits the window inside the bbox.
    let max_x_pos = (max_x - w as i32).max(min_x);
    let max_y_pos = (max_y - h as i32).max(min_y);
    let cx = x.clamp(min_x, max_x_pos);
    let cy = y.clamp(min_y, max_y_pos);
    (cx, cy, w, h)
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
        assert_eq!(reg.list()[0].status, DevserverStatus::Disconnected);
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
        assert_eq!(reg.list()[0].status, DevserverStatus::Connected);
        // The control terminal dies -> the flip drops the conn -> the wire entry
        // flips to status:disconnected, but the persisted config row is untouched
        // (so a re-run/edit can reconnect it).
        conns.remove(&id);
        assert_eq!(reg.list()[0].status, DevserverStatus::Disconnected);
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

    // --- window geometry ---

    fn mon(x: i32, y: i32, w: u32, h: u32, scale: f64) -> MonitorDesc {
        // Work area = full bounds minus a 40px top bar, to exercise the clamp.
        MonitorDesc {
            x,
            y,
            w,
            h,
            work_x: x,
            work_y: y + 40,
            work_w: w,
            work_h: h - 40,
            scale,
        }
    }

    fn geom(sig: &str, x: i32, y: i32, w: u32, h: u32, saved_at: u64) -> WindowGeometry {
        WindowGeometry {
            monitor_sig: sig.to_string(),
            x,
            y,
            w,
            h,
            saved_at,
        }
    }

    #[test]
    fn signature_is_order_independent() {
        let a = mon(0, 0, 2560, 1440, 2.0);
        let b = mon(2560, 0, 1920, 1080, 1.0);
        assert_eq!(
            monitor_signature(&[a.clone(), b.clone()]),
            monitor_signature(&[b, a]),
        );
    }

    #[test]
    fn signature_encodes_count_and_scale() {
        let sig = monitor_signature(&[mon(0, 0, 1920, 1080, 1.5)]);
        assert!(sig.starts_with("1|"), "sig={sig}");
        assert!(sig.contains("0,0,1920,1080@1.50"), "sig={sig}");
        // A different scale under the same bounds is a DIFFERENT signature.
        assert_ne!(sig, monitor_signature(&[mon(0, 0, 1920, 1080, 2.0)]));
    }

    #[test]
    fn signature_changes_with_monitor_count() {
        let one = monitor_signature(&[mon(0, 0, 1920, 1080, 1.0)]);
        let two = monitor_signature(&[mon(0, 0, 1920, 1080, 1.0), mon(1920, 0, 1920, 1080, 1.0)]);
        assert_ne!(one, two);
    }

    #[test]
    fn push_geometry_inserts_and_dedupes_by_signature() {
        let mut cfg = Config::default();
        push_window_geometry(&mut cfg, "w1", geom("sigA", 10, 20, 800, 600, 100));
        // Same window + same signature: replaces, not appends; stays length 1.
        push_window_geometry(&mut cfg, "w1", geom("sigA", 30, 40, 900, 700, 200));
        assert_eq!(cfg.window_geometry.len(), 1);
        assert_eq!(cfg.window_geometry[0].geometries.len(), 1);
        assert_eq!(cfg.window_geometry[0].geometries[0].x, 30);
        assert_eq!(cfg.window_geometry[0].geometries[0].w, 900);
        assert_eq!(cfg.window_geometry[0].saved_at, 200);
    }

    #[test]
    fn push_geometry_keeps_per_signature_lru_capped_newest_first() {
        let mut cfg = Config::default();
        for i in 0..(MAX_WINDOW_GEOMETRIES as i32 + 3) {
            let sig = format!("sig{i}");
            push_window_geometry(&mut cfg, "w1", geom(&sig, i, i, 800, 600, 100 + i as u64));
        }
        assert_eq!(cfg.window_geometry.len(), 1);
        assert_eq!(
            cfg.window_geometry[0].geometries.len(),
            MAX_WINDOW_GEOMETRIES
        );
        let newest = format!("sig{}", MAX_WINDOW_GEOMETRIES as i32 + 2);
        assert_eq!(cfg.window_geometry[0].geometries[0].monitor_sig, newest);
    }

    #[test]
    fn push_geometry_caps_records_and_moves_touched_to_front() {
        let mut cfg = Config::default();
        for i in 0..(MAX_WINDOW_CONFIGS + 3) {
            let label = format!("w{i}");
            push_window_geometry(&mut cfg, &label, geom("s", 0, 0, 1, 1, 100 + i as u64));
        }
        // Capped at MAX_WINDOW_CONFIGS windows; newest label at the front, the
        // three oldest evicted.
        assert_eq!(cfg.window_geometry.len(), MAX_WINDOW_CONFIGS);
        let newest = format!("w{}", MAX_WINDOW_CONFIGS + 2);
        assert_eq!(cfg.window_geometry[0].window_label, newest);
        assert!(!cfg.window_geometry.iter().any(|r| r.window_label == "w0"));
        // Re-touching a surviving window moves its record to the front.
        let survivor = format!("w{}", MAX_WINDOW_CONFIGS);
        push_window_geometry(&mut cfg, &survivor, geom("s", 5, 5, 1, 1, 999));
        assert_eq!(cfg.window_geometry[0].window_label, survivor);
        assert_eq!(cfg.window_geometry.len(), MAX_WINDOW_CONFIGS);
    }

    #[test]
    fn lookup_geometry_exact_on_signature_match() {
        let mut cfg = Config::default();
        push_window_geometry(&mut cfg, "w1", geom("sigA", 10, 20, 800, 600, 100));
        match lookup_window_geometry(&cfg, "w1", "sigA") {
            Some(GeometryMatch::Exact(g)) => assert_eq!((g.x, g.y, g.w, g.h), (10, 20, 800, 600)),
            other => panic!("expected Exact, got {other:?}"),
        }
    }

    #[test]
    fn lookup_geometry_size_only_on_signature_mismatch() {
        let mut cfg = Config::default();
        push_window_geometry(&mut cfg, "w1", geom("sigA", 10, 20, 800, 600, 100));
        match lookup_window_geometry(&cfg, "w1", "sigOTHER") {
            Some(GeometryMatch::SizeOnly(g)) => assert_eq!((g.w, g.h), (800, 600)),
            other => panic!("expected SizeOnly, got {other:?}"),
        }
    }

    #[test]
    fn lookup_geometry_none_when_label_absent() {
        let cfg = Config::default();
        assert!(lookup_window_geometry(&cfg, "missing", "sigA").is_none());
    }

    #[test]
    fn geometry_flip_layout_and_back_restores_each_signature() {
        // The dual-monitor flip guardrail: capture under A, then under B, then
        // flip back to A -> A's own geometry is still remembered (Exact), not
        // overwritten by B.
        let mut cfg = Config::default();
        push_window_geometry(
            &mut cfg,
            "lib-ab::w-1",
            geom("sigA", 100, 100, 1200, 800, 100),
        );
        push_window_geometry(&mut cfg, "lib-ab::w-1", geom("sigB", 50, 50, 900, 700, 200));
        match lookup_window_geometry(&cfg, "lib-ab::w-1", "sigA") {
            Some(GeometryMatch::Exact(g)) => {
                assert_eq!((g.x, g.y, g.w, g.h), (100, 100, 1200, 800))
            }
            other => panic!("flip-back to A: expected Exact A, got {other:?}"),
        }
        match lookup_window_geometry(&cfg, "lib-ab::w-1", "sigB") {
            Some(GeometryMatch::Exact(g)) => assert_eq!((g.x, g.y, g.w, g.h), (50, 50, 900, 700)),
            other => panic!("on B: expected Exact B, got {other:?}"),
        }
    }

    #[test]
    fn clamp_size_shrinks_oversize_keeps_fitting() {
        assert_eq!(clamp_size(3000, 2000, 1920, 1080), (1920, 1080));
        assert_eq!(clamp_size(800, 600, 1920, 1080), (800, 600));
    }

    #[test]
    fn clamp_rect_leaves_onscreen_rect_unchanged() {
        let bbox = union_work_bbox(&[mon(0, 0, 2560, 1440, 2.0)]).unwrap();
        assert_eq!(
            clamp_rect_to_bbox(100, 100, 1200, 800, bbox),
            (100, 100, 1200, 800)
        );
    }

    #[test]
    fn clamp_rect_pulls_offscreen_rect_back() {
        // work area: x[0,2560], y[40,1440]
        let bbox = union_work_bbox(&[mon(0, 0, 2560, 1440, 2.0)]).unwrap();
        // Bottom-right overflow: top-left pulled so the 1200x800 window fits.
        assert_eq!(
            clamp_rect_to_bbox(9000, 9000, 1200, 800, bbox),
            (1360, 640, 1200, 800)
        );
        // Negative origin pulled to the work-area min (x=0, y=40).
        let (x, y, _, _) = clamp_rect_to_bbox(-500, -500, 1200, 800, bbox);
        assert_eq!((x, y), (0, 40));
    }

    #[test]
    fn clamp_rect_shrinks_window_larger_than_desktop() {
        let bbox = union_work_bbox(&[mon(0, 0, 1280, 800, 1.0)]).unwrap(); // work 1280x760
        assert_eq!(
            clamp_rect_to_bbox(0, 40, 4000, 4000, bbox),
            (0, 40, 1280, 760)
        );
    }

    #[test]
    fn clamp_rect_keeps_window_on_second_monitor() {
        // Two side-by-side monitors; a window on the right one stays put.
        let bbox =
            union_work_bbox(&[mon(0, 0, 1920, 1080, 1.0), mon(1920, 0, 1920, 1080, 1.0)]).unwrap(); // x[0,3840], y[40,1080]
        assert_eq!(
            clamp_rect_to_bbox(2000, 100, 1000, 700, bbox),
            (2000, 100, 1000, 700)
        );
    }

    #[test]
    fn union_work_bbox_none_for_empty() {
        assert!(union_work_bbox(&[]).is_none());
    }

    #[test]
    fn config_loads_without_window_geometry_field() {
        // A config.json predating window geometry must still load: serde reads
        // the missing key as the empty set, so the load never fails and drops
        // the rest of the config.
        let raw = r#"{ "outbound": [], "window_configs": [] }"#;
        let cfg: Config = serde_json::from_str(raw).expect("load without window_geometry");
        assert!(cfg.window_geometry.is_empty());
    }

    #[test]
    fn window_geometry_round_trips() {
        let mut cfg = Config::default();
        push_window_geometry(
            &mut cfg,
            "lib-ab::w-1",
            geom("sigA", 100, 100, 1200, 800, 100),
        );
        push_window_geometry(&mut cfg, "lib-ab::w-1", geom("sigB", 50, 50, 900, 700, 200));
        let json = serde_json::to_string(&cfg).expect("serialize");
        let back: Config = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.window_geometry.len(), 1);
        let rec = &back.window_geometry[0];
        assert_eq!(rec.window_label, "lib-ab::w-1");
        assert_eq!(rec.geometries.len(), 2);
        // Newest (sigB) first.
        assert_eq!(rec.geometries[0].monitor_sig, "sigB");
        assert_eq!(rec.geometries[1].monitor_sig, "sigA");
    }
}
