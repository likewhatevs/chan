//! Local-workspace runtime and workspace-window helpers.
//!
//! chan-desktop opens local workspaces through the embedded chan-server
//! `WorkspaceHost`. Each running workspace is tracked in `AppState.serves`
//! with its route prefix and token-bearing URL. chan-desktop links
//! `chan-workspace` and `chan-server` directly; there is no `chan`
//! binary at runtime. Registry mutations and feature toggles run
//! in-process against the embedded host's shared `Library`, and
//! local serving never spawns `chan serve`.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Per-process monotonic counter appended to every workspace-window
/// label so the user can open more than one window for the same
/// workspace (local or tunneled). Tauri requires unique window labels
/// per process; the prefix encodes the workspace identity and the seq
/// disambiguates instances.
static WINDOW_SEQ: AtomicU64 = AtomicU64::new(0);

fn next_window_seq() -> u64 {
    WINDOW_SEQ.fetch_add(1, Ordering::Relaxed)
}

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::config::{self, WindowConfig};
use crate::AppState;

/// Tauri event emitted when any local runtime starts or stops. The
/// frontend reacts by re-fetching the workspace list.
pub const SERVES_CHANGED: &str = "serves-changed";

const MAX_WINDOWS_PER_WORKSPACE: usize = 10;

/// Window-title kind glyphs. A workspace window's title leads with one of
/// these so the OS title bar + window switcher encode the kind at a glance,
/// then the locator (path / URL / listen address). Emoji render as color
/// glyphs in the macOS title bar; named constants so swapping to a monochrome
/// set (e.g. arrows for outbound/inbound) is a one-line change each.
const ICON_LOCAL_HOME: &str = "\u{1F3E0}"; // house: local disk, under $HOME
const ICON_LOCAL_OTHER: &str = "\u{1F5A5}\u{FE0F}"; // desktop computer: local, elsewhere
const ICON_OUTBOUND: &str = "\u{1F4E4}"; // outbox tray: we dial OUT to a URL
const ICON_INBOUND: &str = "\u{1F4E5}"; // inbox tray: a remote dials IN to us

/// Live state for one running serve. Held in `AppState.serves`
/// keyed by canonical workspace path.
pub struct ServeHandle {
    prefix: String,
    pub url: Option<String>,
}

impl ServeHandle {
    fn embedded(prefix: String, url: String) -> Self {
        Self {
            prefix,
            url: Some(url),
        }
    }
}

/// Open a local workspace through the embedded chan-server host.
pub async fn start(app: AppHandle, state: Arc<AppState>, key: String) -> Result<(), String> {
    if state.serves.lock().unwrap().contains_key(&key) {
        return Ok(());
    }
    let Some(embedded) = state.embedded.get() else {
        return Err("embedded local server is unavailable".to_string());
    };
    let url = embedded.open_workspace(&key).await?;
    let prefix = url_prefix_from_local_url(&url)?;
    {
        let mut serves = state.serves.lock().unwrap();
        if serves.contains_key(&key) {
            drop(serves);
            if let Err(e) = embedded.close_prefix(&prefix) {
                tracing::warn!(key = %key, error = %e, "closing duplicate embedded workspace failed");
            }
            return Ok(());
        }
        serves.insert(key.clone(), ServeHandle::embedded(prefix, url.clone()));
    }
    let _ = app.emit(SERVES_CHANGED, ());
    if let Err(e) = spawn_local_workspace_window(&app, &key, &url) {
        if let Some(handle) = state.serves.lock().unwrap().remove(&key) {
            stop_handle(None, &state, &key, handle);
        }
        let _ = app.emit(SERVES_CHANGED, ());
        return Err(e);
    }
    Ok(())
}

fn url_prefix_from_local_url(url: &str) -> Result<String, String> {
    let parsed = url
        .parse::<url::Url>()
        .map_err(|e| format!("parsing embedded workspace URL: {e}"))?;
    let path = parsed.path().trim_end_matches('/');
    let path = path.strip_suffix("/index.html").unwrap_or(path);
    if path.is_empty() {
        Ok(String::new())
    } else {
        Ok(path.to_string())
    }
}

/// Stop a running serve. No-op if the workspace isn't running. Removes
/// the live entry before waiting so an immediate stop -> start can
/// mount a fresh runtime instead of observing stale map state.
pub fn stop(app: Option<&AppHandle>, state: &AppState, key: &str) {
    let handle = state.serves.lock().unwrap().remove(key);
    if let Some(h) = handle {
        stop_handle(app, state, key, h);
    }
}

/// Stop every running serve. Called from the Tauri Exit hook so
/// embedded workspace state shuts down before the desktop exits.
pub fn stop_all(state: &AppState) {
    let handles: Vec<(String, ServeHandle)> = state.serves.lock().unwrap().drain().collect();
    for (key, h) in handles {
        stop_handle(None, state, &key, h);
    }
}

fn stop_handle(app: Option<&AppHandle>, state: &AppState, key: &str, handle: ServeHandle) {
    if let Some(embedded) = state.embedded.get() {
        if let Err(e) = embedded.close_prefix(&handle.prefix) {
            tracing::warn!(key = %key, error = %e, "closing embedded workspace failed");
        }
    }
    if let Some(app) = app {
        close_local_workspace_windows(app, key);
        let _ = app.emit(SERVES_CHANGED, ());
    }
}

/// Stable Tauri window-label prefix for a local workspace. Used to
/// recognise every window that belongs to the workspace when the user
/// has opened more than one (close-all on serve exit, capability
/// matching). Tauri labels must match `[a-zA-Z0-9_-]+`, and workspace
/// keys are filesystem paths, so we hash the key.
pub fn workspace_window_prefix(key: &str) -> String {
    let mut h = DefaultHasher::new();
    key.hash(&mut h);
    format!("workspace-{:016x}", h.finish())
}

/// Fresh, unique window label for a new local-workspace webview.
/// Every call yields a distinct label so multi-window works; the
/// prefix is still identifiable for cleanup. Format:
/// `workspace-<hash>-<seq>` where `seq` is a per-process atomic.
pub fn new_workspace_window_label(key: &str) -> String {
    format!("{}-{}", workspace_window_prefix(key), next_window_seq())
}

/// Window title for a local-workspace webview: a kind glyph (home vs this
/// machine) then the workspace path. The path is the locator (the
/// disambiguating signal in the OS window switcher); the glyph prefix
/// makes the kind read at a glance.
fn workspace_title(key: &str) -> String {
    local_title(key, dirs::home_dir().as_deref())
}

/// Pure home-vs-elsewhere title formatting, split from `workspace_title` so it
/// is testable without depending on the process's real home dir. A path under
/// `home` gets the house glyph; anything else (or no resolvable home) gets the
/// computer glyph.
fn local_title(key: &str, home: Option<&Path>) -> String {
    let icon = match home {
        Some(home) if Path::new(key).starts_with(home) => ICON_LOCAL_HOME,
        _ => ICON_LOCAL_OTHER,
    };
    format!("{icon} {key}")
}

/// The local listen address a tunneled-workspace window connects to: the
/// host:port (authority) of the per-tenant loopback `url`. Used as the inbound
/// title locator. Falls back to the raw `url` if it has no authority (a
/// loopback workspace URL always does, so the fallback is defensive).
fn listen_addr_from_url(url: &str) -> String {
    url.parse::<url::Url>()
        .ok()
        .and_then(|u| {
            let host = u.host_str()?.to_string();
            Some(match u.port() {
                Some(port) => format!("{host}:{port}"),
                None => host,
            })
        })
        .unwrap_or_else(|| url.to_string())
}

/// Stable window-label prefix for a tunneled workspace, namespaced
/// separately from `workspace-*` so a local workspace path and a tunneled
/// workspace slug don't collide.
pub fn tunnel_window_prefix(tenant_label: &str, workspace: &str) -> String {
    let mut h = DefaultHasher::new();
    tenant_label.hash(&mut h);
    workspace.hash(&mut h);
    format!("tunnel-{:016x}", h.finish())
}

/// Fresh, unique window label for a tunneled workspace webview. Same
/// shape as `new_workspace_window_label`.
pub fn new_tunnel_window_label(tenant_label: &str, workspace: &str) -> String {
    format!(
        "{}-{}",
        tunnel_window_prefix(tenant_label, workspace),
        next_window_seq()
    )
}

/// Stable window-label prefix for an outbound URL attachment.
pub fn outbound_window_prefix(id: &str) -> String {
    let mut h = DefaultHasher::new();
    id.hash(&mut h);
    format!("outbound-{:016x}", h.finish())
}

/// Fresh, unique window label for an outbound URL webview.
pub fn new_outbound_window_label(id: &str) -> String {
    format!("{}-{}", outbound_window_prefix(id), next_window_seq())
}

/// True when a Tauri label belongs to an embedded-served SPA webview
/// (workspace / tunnel / outbound / standalone terminal). All four host
/// the chan SPA and accept the `chan:command` dispatch bridge, so menu
/// items that defer to the focused window (Settings, New Terminal's
/// toggle branch) target any of them.
pub fn is_workspace_webview_label(label: &str) -> bool {
    label.starts_with("workspace-")
        || label.starts_with("tunnel-")
        || label.starts_with("outbound-")
        || label.starts_with("terminal-")
}

/// Spawn a new local-workspace webview window pointing at `url`. Each
/// call opens an independent window; multiple windows per workspace are
/// supported. Pops the most-recent WindowConfig for this workspace (if
/// any) so the new window reuses the previous `?w=<label>` and URL
/// hash, restoring panes / tabs (via `session.json`) and overlay
/// state across the close/reopen cycle. A user-initiated close
/// pushes the closing window's state back to the stack so the next
/// open repeats the restore. The Tauri close handler does NOT stop
/// the underlying local runtime; the On toggle (plus
/// `close_local_workspace_windows` on runtime teardown) remains the single
/// authority on workspace lifecycle.
pub fn spawn_local_workspace_window(app: &AppHandle, key: &str, url: &str) -> Result<(), String> {
    let prefix = workspace_window_prefix(key);
    let config_key = config::local_window_key(key);
    let Some(restore) = unbury_or_restore(app, &prefix, &config_key, || {
        new_workspace_window_label(key)
    })?
    else {
        return Ok(());
    };
    let title = workspace_title(key);
    build_workspace_window(
        app,
        WindowSpec {
            label: &restore.label,
            title: &title,
            url,
            url_hash_seed: &restore.url_hash,
            config_key,
            zoom_seed: restore.zoom,
            connecting: None,
            kind: None,
        },
    )
}

/// Spawn a new tunneled-workspace webview window. Same multi-window
/// semantics and config-stack restore as the local variant.
pub fn spawn_tunneled_workspace_window(
    app: &AppHandle,
    tenant_label: &str,
    workspace: &str,
    url: &str,
) -> Result<(), String> {
    let prefix = tunnel_window_prefix(tenant_label, workspace);
    let config_key = config::tunnel_window_key(tenant_label, workspace);
    let Some(restore) = unbury_or_restore(app, &prefix, &config_key, || {
        new_tunnel_window_label(tenant_label, workspace)
    })?
    else {
        return Ok(());
    };
    // Inbound (a remote dialed in over the tunnel) is reached through a
    // local per-tenant loopback listener; the window's `url` points at it.
    // Title with the inbound glyph + that listener's host:port, the locator
    // analogous to the local path / outbound URL.
    let title = tunnel_window_title(url);
    let built = build_workspace_window(
        app,
        WindowSpec {
            label: &restore.label,
            title: &title,
            url,
            url_hash_seed: &restore.url_hash,
            config_key,
            zoom_seed: restore.zoom,
            connecting: None,
            kind: None,
        },
    );
    // A tunnel window just appeared: re-poll the remote's window list
    // so the Window menu's remote section reflects it.
    crate::refresh_remote_windows_menu(app);
    built
}

/// Spawn a new outbound URL webview window. The desktop does not own
/// the remote process; this only creates another webview pointed at
/// the persisted URL.
pub fn spawn_outbound_workspace_window(app: &AppHandle, id: &str, url: &str) -> Result<(), String> {
    let prefix = outbound_window_prefix(id);
    let config_key = config::outbound_window_key(id);
    let Some(restore) =
        unbury_or_restore(app, &prefix, &config_key, || new_outbound_window_label(id))?
    else {
        return Ok(());
    };
    // Outbound title is the outbound glyph + the URL (the locator),
    // not the user's label (which still names the launcher row).
    let title = outbound_window_title(url);
    // Outbound = an outgoing connection to a remote we do not own. Route
    // through the connecting screen so a down remote shows a retrying
    // surface instead of a blank white webview. `url` is the display +
    // probe URL; `build_workspace_window` assembles the navigate target.
    let built = build_workspace_window(
        app,
        WindowSpec {
            label: &restore.label,
            title: &title,
            url,
            url_hash_seed: &restore.url_hash,
            config_key,
            zoom_seed: restore.zoom,
            connecting: Some(url),
            kind: None,
        },
    );
    // An outbound window just appeared: re-poll the remote's window
    // list so the Window menu's remote section reflects it.
    crate::refresh_remote_windows_menu(app);
    built
}

/// Spawn a standalone terminal-only window. Unlike a workspace window there
/// is no registry entry and no On-toggle lifecycle: every terminal window
/// loads the ONE shared `/terminal` tenant (mounted on first use), in
/// `kind=terminal` mode.
///
/// Each window gets a unique `terminal-win-<seq>` label so its layout
/// persists separately (keyed by `?w=`) and the OS window switcher
/// disambiguates - the label is not the route prefix. The shared
/// tenant is never torn down per window (it lives for the process lifetime;
/// orphaned PTYs idle-prune), which is what lets a terminal moved into
/// another window keep its live PTY.
pub async fn spawn_local_terminal_window(
    app: AppHandle,
    state: Arc<AppState>,
) -> Result<(), String> {
    let Some(embedded) = state.embedded.get() else {
        return Err("embedded local server is unavailable".to_string());
    };
    let url = embedded.open_terminal().await?;
    let label = format!("terminal-win-{}", next_window_seq());
    // `config_key` is unused for terminal windows (no LRU restore), but
    // `build_workspace_window` takes one; an empty key never matches a real
    // workspace/tunnel/outbound key and the terminal close branch skips the
    // capture entirely. No per-window tenant teardown on build failure: the
    // tenant is shared and persistent.
    build_workspace_window(
        &app,
        WindowSpec {
            label: &label,
            title: "Terminal",
            url: &url,
            url_hash_seed: "",
            config_key: String::new(),
            zoom_seed: 1.0,
            connecting: None,
            kind: Some("terminal"),
        },
    )
}

/// Base window title for an outbound (we-dial-out) workspace window.
/// pub: the remote Window-menu refresh derives the same title without
/// opening a window.
pub fn outbound_window_title(url: &str) -> String {
    format!("{ICON_OUTBOUND} {url}")
}

/// Base window title for a tunneled (remote-dials-in) workspace window.
pub fn tunnel_window_title(url: &str) -> String {
    format!("{ICON_INBOUND} {}", listen_addr_from_url(url))
}

/// Reopen a REMOTE-known window (a `saved && !connected` row from the
/// remote serve's `GET /api/windows`) by building a webview with that
/// exact label: the `?w=<label>` the build appends makes the remote
/// hydrate that window's session blob, so the panes/tabs the user left
/// there come back. No LRU pop — the restore state lives remote-side.
pub fn reopen_remote_window(
    app: &AppHandle,
    label: &str,
    entry: &crate::RemoteReopen,
) -> Result<(), String> {
    build_workspace_window(
        app,
        WindowSpec {
            label,
            title: &entry.base_title,
            url: &entry.url,
            url_hash_seed: "",
            config_key: entry.config_key.clone(),
            zoom_seed: 1.0,
            // Outbound remotes route through the connecting screen like
            // any other outbound window (a down remote must not paint a
            // blank webview); tunnel loopbacks load directly.
            connecting: entry.connecting.then_some(entry.url.as_str()),
            kind: None,
        },
    )
}

/// True when the webview is still showing the bundled connecting/retry
/// screen (`connecting.html`, the outbound pre-navigation page). Such a
/// window has no per-window session, no shells, and nothing to restore,
/// so close affordances treat it as cancel-and-really-close instead of
/// burying. The URL read is guarded like `capture_window_config`: a
/// dead webview's `url()` can panic on a nil URL, and any failure reads
/// as "not the connecting screen" (bury — the safe pre-existing path).
pub fn window_on_connecting_screen(app: &AppHandle, label: &str) -> bool {
    let Some(window) = app.get_webview_window(label) else {
        return false;
    };
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| window.url())) {
        Ok(Ok(url)) => url.path().ends_with("connecting.html"),
        _ => false,
    }
}

/// Reopen this workspace family's most recently buried window instead
/// of spawning a new one, when one exists. Every "open a window for
/// this workspace" entry point (launcher Open, Cmd/Ctrl+Shift+N's
/// spawn fallback, deep links) funnels through the spawn fns, so the
/// check lives here: a window the user put away via the close button
/// IS the window they get back — a reopens-the-last-closed-window
/// feel, with live state. `prefix` is the family
/// prefix WITHOUT the trailing dash (the spawn fns' label prefix).
fn unbury_instead_of_spawn(app: &AppHandle, prefix: &str) -> bool {
    let family = format!("{prefix}-");
    let Some(buried) = app.state::<Arc<AppState>>().most_recent_buried(&family) else {
        return false;
    };
    crate::unbury_window(app, &buried)
}

/// Label + restore state for a window about to be (re)built, popped
/// from the window-config stack or freshly minted.
struct RestoredWindow {
    label: String,
    url_hash: String,
    zoom: f64,
}

/// Shared open preamble for the local / tunnel / outbound spawn fns:
/// prefer unburying the family's most recent hidden window, enforce the
/// per-family window cap, then pop a compatible WindowConfig for the
/// label + restore state (fresh label, empty hash, default zoom when
/// nothing restorable exists). `Ok(None)` means an unburied window
/// already satisfied the open and no new window should be built.
fn unbury_or_restore(
    app: &AppHandle,
    prefix: &str,
    config_key: &str,
    fresh_label: impl FnOnce() -> String,
) -> Result<Option<RestoredWindow>, String> {
    if unbury_instead_of_spawn(app, prefix) {
        return Ok(None);
    }
    ensure_window_capacity(app, prefix)?;
    let restore = pop_compatible_config(app, config_key, prefix);
    Ok(Some(RestoredWindow {
        label: restore
            .as_ref()
            .map(|c| c.window_label.clone())
            .unwrap_or_else(fresh_label),
        url_hash: restore
            .as_ref()
            .map(|c| c.url_hash.clone())
            .unwrap_or_default(),
        zoom: restore.as_ref().map(|c| c.zoom_level).unwrap_or(1.0),
    }))
}

/// Pop the top-of-stack window config for `config_key` only if the
/// stored label is safe to reuse. Live-label entries are SKIPPED in
/// place (not popped): a buried window's entry must survive for the
/// quit-while-buried restore, and Tauri labels are unique per process
/// so reusing one would collide. The popped label must additionally
/// match the workspace's current hash prefix (defends against the
/// workspace key changing canonicalisation under us); a stale-prefix
/// entry gets dropped on the floor — we don't keep cycling through
/// stale stack entries, since the next bury pushes a fresh one anyway.
fn pop_compatible_config(
    app: &AppHandle,
    config_key: &str,
    expected_prefix: &str,
) -> Option<WindowConfig> {
    let state = app.state::<Arc<AppState>>();
    let entry =
        state.pop_window_config(config_key, |label| app.get_webview_window(label).is_some())?;
    if !entry.window_label.starts_with(expected_prefix) {
        tracing::debug!(
            label = %entry.window_label,
            prefix = %expected_prefix,
            "discarding window config with stale prefix",
        );
        return None;
    }
    Some(entry)
}

/// Inputs for one SPA webview window build: identity (label/title),
/// where to point it, what to restore, and how to load.
struct WindowSpec<'a> {
    /// Unique Tauri window label (also the `?w=` per-window session key).
    label: &'a str,
    /// Base title; the builder suffixes a reused " Window N" display number.
    title: &'a str,
    /// The workspace/terminal URL the webview ultimately shows.
    url: &'a str,
    /// URL fragment from the window-config stack: applied verbatim so
    /// overlay state (file browser path, search query, graph scope)
    /// restores alongside the panes/tabs that come back from
    /// `session.json`. Empty when there's nothing to restore.
    url_hash_seed: &'a str,
    /// WindowConfig identity key (`local_window_key` or
    /// `tunnel_window_key`). Stamped onto the close handler so a
    /// user-initiated close pushes the window's final URL hash back
    /// into the LRU stack. Empty for terminal windows (no LRU restore).
    config_key: String,
    /// Zoom level to restore; 1.0 (the default) skips the IPC round-trip.
    zoom_seed: f64,
    /// Load strategy. `None` (local + tunnel) loads `url` directly via
    /// `WebviewUrl::External`: those backends are up before the window
    /// opens. `Some(display_url)` (outbound) instead loads the bundled
    /// `connecting.html` and hands it the display URL plus the assembled
    /// navigate target through an injected `window.__CHAN_CONNECTING__`;
    /// the page probes the remote via `probe_url` and navigates on
    /// success. A direct External load of a down outbound remote paints a
    /// blank white webview (WKWebView never finishes navigating, see
    /// `capture_window_config`), which is the bug the connecting screen
    /// fixes.
    connecting: Option<&'a str>,
    /// `Some("terminal")` makes the SPA boot in terminal-only mode (no
    /// workspace fetch); `None` is full workspace mode.
    kind: Option<&'a str>,
}

/// Build and show a chan-style workspace webview window on the main
/// thread. Internal: call `spawn_local_workspace_window` /
/// `spawn_tunneled_workspace_window` / `spawn_outbound_workspace_window`
/// from outside. Centralising the
/// key-bridge JS, the size defaults, the zoom-hotkey polyfill, and
/// the drag-drop handler off in one place means workspace UX changes
/// don't fork between the local and tunneled paths.
fn build_workspace_window(app: &AppHandle, spec: WindowSpec<'_>) -> Result<(), String> {
    let WindowSpec {
        label: window_label,
        title,
        url,
        url_hash_seed,
        config_key,
        zoom_seed,
        connecting,
        kind,
    } = spec;
    let Ok(mut parsed) = url.parse::<tauri::Url>() else {
        return Err(format!("bad chan URL for {window_label}: {url}"));
    };
    parsed.query_pairs_mut().append_pair("w", window_label);
    // `kind=terminal` is the SPA's only signal to enter terminal-only mode
    // (no workspace fetch, terminal panes only). Workspace/tunnel/outbound
    // windows pass `None` and the SPA stays in full workspace mode.
    if let Some(kind) = kind {
        parsed.query_pairs_mut().append_pair("kind", kind);
    }
    if !url_hash_seed.is_empty() {
        parsed.set_fragment(Some(url_hash_seed));
    }
    // The connecting page receives its inputs before any page script runs
    // (same mechanism as KEY_BRIDGE_JS). `target` is the fully-assembled
    // navigate URL (remote + ?w=<label> + restored #fragment) so the SPA's
    // per-window state + restore survive the success navigation.
    let (webview_url, init_script) = match connecting {
        Some(display_url) => {
            let payload = serde_json::json!({
                "url": display_url,
                "target": parsed.as_str(),
            });
            let script = format!("window.__CHAN_CONNECTING__ = {payload};\n{KEY_BRIDGE_JS}");
            (WebviewUrl::App("connecting.html".into()), script)
        }
        None => (WebviewUrl::External(parsed), KEY_BRIDGE_JS.to_string()),
    };
    let app_owned = app.clone();
    let label_owned = window_label.to_string();
    let title_owned = title.to_string();
    let res = app.run_on_main_thread(move || {
        // Defensive: window labels are unique-per-instance now, so
        // a collision shouldn't happen. If it ever does (e.g. some
        // future code reusing a stable label), destroy the stale
        // window so `build` doesn't panic.
        if let Some(old) = app_owned.get_webview_window(&label_owned) {
            let _ = old.destroy();
        }
        // Suffix a reused, lowest-free display number so the OS Window
        // menu disambiguates windows that share a base title (two
        // windows on one workspace, several standalone terminals). The
        // number is freed on close (Ok branch handler / Err branch
        // below) so the next same-base window reuses it.
        let window_number = app_owned
            .state::<Arc<AppState>>()
            .assign_window_number(&label_owned, &title_owned);
        let display_title = format!("{title_owned} Window {window_number}");
        match WebviewWindowBuilder::new(&app_owned, &label_owned, webview_url)
            .title(display_title)
            .inner_size(1200.0, 800.0)
            .min_inner_size(640.0, 400.0)
            .resizable(true)
            .initialization_script(init_script.as_str())
            // The explicit `zoom_in` / `zoom_out` / `zoom_reset`
            // IPC commands fired from KEY_BRIDGE_JS
            // are the primary path; this Tauri-level polyfill stays
            // on as a mousewheel + pinch fallback (the chord
            // overlap is harmless because KEY_BRIDGE_JS's capture-
            // phase listener calls preventDefault before the
            // polyfill's bubble-phase listener sees the keydown).
            // Requires `core:webview:allow-set-webview-zoom` on
            // workspace-* / tunnel-* / outbound-* windows per
            // capabilities/workspace.json.
            .zoom_hotkeys_enabled(true)
            // Hand HTML5 drag-and-drop to the page — this must stay
            // disabled. With wry's native handler enabled, WebKit
            // never sees ANY drag on macOS (wry forwards to the OS
            // default only when the handler returns false, and
            // tauri-runtime-wry's handler returns true
            // unconditionally), which kills the editor/file-browser
            // drop zones AND in-page pane-to-pane tab moves. The
            // SPA's window-level drop guard owns the no-takeover
            // guarantee for stray OS file drops, and the terminal
            // path-print reads the drag pasteboard via
            // `read_dropped_paths` (dropped_paths.rs) instead of
            // native drag events.
            .disable_drag_drop_handler()
            .build()
        {
            Ok(window) => {
                // Restore the persisted zoom level from
                // the popped WindowConfig (if any). 1.0 is the chan-
                // desktop default; skip the IPC round-trip when there's
                // nothing to apply. Best-effort: a Tauri set_zoom error
                // here just leaves the new window at default zoom; the
                // user can re-press Cmd++/Cmd+- to recover.
                if (zoom_seed - 1.0).abs() > f64::EPSILON {
                    if let Err(e) = window.set_zoom(zoom_seed) {
                        tracing::warn!(
                            label = %label_owned,
                            error = %e,
                            "restoring window zoom level failed",
                        );
                    } else {
                        let state = app_owned.state::<Arc<AppState>>();
                        state
                            .live_window_zooms
                            .lock()
                            .unwrap()
                            .insert(label_owned.clone(), zoom_seed);
                    }
                }
                let app_for_close = app_owned.clone();
                let label_for_close = label_owned.clone();
                let key_for_close = config_key.clone();
                window.on_window_event(move |event| match event {
                    // The OS close button BURIES an SPA window instead of
                    // destroying it: the webview hides, live terminals and
                    // layout state stay warm, and the Window menu (or
                    // Cmd/Ctrl+Shift+N) reopens it. Two exceptions really
                    // close: a standalone terminal window with NO live
                    // shells left, and a window still showing the
                    // connecting/retry screen (no session, no shells —
                    // burying it would leave an unkillable hidden retry
                    // loop).
                    // Programmatic closes (the SPA's empty-window cascade,
                    // workspace-off teardown, tunnel drop) call `destroy()`
                    // and never reach this branch.
                    WindowEvent::CloseRequested { api, .. } => {
                        let state = app_for_close.state::<Arc<AppState>>();
                        let bury = if label_for_close.starts_with("terminal-") {
                            state
                                .embedded
                                .get()
                                .map(|e| e.terminal_window_has_live_shells(&label_for_close))
                                .unwrap_or(false)
                        } else {
                            !window_on_connecting_screen(&app_for_close, &label_for_close)
                        };
                        if !bury {
                            // Real close; the Destroyed branch cleans up.
                            return;
                        }
                        api.prevent_close();
                        // Capture the restore snapshot NOW (webview alive,
                        // URL hash + zoom readable): burying replaces
                        // closing as the moment "the user put this window
                        // away", and the entry also covers an app quit
                        // while buried. The zoom stays in
                        // `live_window_zooms` (peek, not drain) — the
                        // window is still alive and may be unburied.
                        if !label_for_close.starts_with("terminal-") {
                            capture_window_config(
                                &app_for_close,
                                &label_for_close,
                                &key_for_close,
                                false,
                            );
                        }
                        let Some(window) = app_for_close.get_webview_window(&label_for_close)
                        else {
                            return;
                        };
                        let title = window.title().unwrap_or_else(|_| label_for_close.clone());
                        let _ = window.hide();
                        state.bury_window(&label_for_close, &title);
                        crate::rebuild_window_menu(&app_for_close);
                        show_bury_notice(&app_for_close, &title);
                    }
                    // Single cleanup point for EVERY destroy path: the
                    // no-live-shells close above, the SPA cascade destroy,
                    // workspace-off / tunnel-drop / outbound-forget
                    // teardown, and app exit. Frees the display number,
                    // drops the zoom entry, and clears a stale buried
                    // registry entry if the window died while hidden.
                    WindowEvent::Destroyed => {
                        let state = app_for_close.state::<Arc<AppState>>();
                        state.release_window_number(&label_for_close);
                        state
                            .live_window_zooms
                            .lock()
                            .unwrap()
                            .remove(&label_for_close);
                        if state.remove_buried(&label_for_close) {
                            crate::rebuild_window_menu(&app_for_close);
                        }
                        // A destroyed remote-backed window may now be a
                        // reopenable `saved && !connected` row on the
                        // remote — re-poll so the menu offers it.
                        if label_for_close.starts_with("tunnel-")
                            || label_for_close.starts_with("outbound-")
                        {
                            crate::refresh_remote_windows_menu(&app_for_close);
                        }
                    }
                    _ => {}
                });
            }
            Err(e) => {
                // Build failed: hand the just-assigned number back so it
                // isn't leaked out of the live set.
                app_owned
                    .state::<Arc<AppState>>()
                    .release_window_number(&label_owned);
                tracing::warn!(label = %label_owned, error = %e, "opening workspace window failed")
            }
        }
    });
    res.map_err(|e| format!("scheduling workspace window for {window_label}: {e}"))
}

/// Informational notice shown EVERY time the OS close button buries a
/// window: the dialog is the teaching surface for the hide-not-close
/// behaviour (smoke tests assert it appears). Async
/// `.show` only — a blocking dialog on the event-loop thread deadlocks.
fn show_bury_notice(app: &AppHandle, title: &str) {
    use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
    let chord = if cfg!(target_os = "macos") {
        "Cmd+Shift+N"
    } else {
        "Ctrl+Shift+N"
    };
    app.dialog()
        .message(format!(
            "\"{title}\" is now hidden, not closed.\n\nIts terminals and layout keep running. Reopen it from the Window menu, or with {chord}."
        ))
        .title("Window Hidden")
        .kind(MessageDialogKind::Info)
        .show(|_| {});
}

/// Snapshot the window's URL hash and push the resulting WindowConfig
/// onto the LRU stack. Called at BURY time (the hide-not-close moment;
/// webview alive, URL readable) and from any explicit capture path.
/// Best-effort: a webview that's already torn down reports no URL and
/// we skip the push. The hash is read from `WebviewWindow::url()`
/// because the webview SPA writes the latest state to `location.hash`
/// via `persistStateToHash`, and Tauri's URL reflection picks that up
/// on platforms with the WKWebView / WebView2 backends.
///
/// Also captures the live zoom level for this window
/// into `WindowConfig.zoom_level` so the next open of the same
/// workspace restores the zoom. `drain_zoom` controls whether the
/// `live_window_zooms` entry is removed (a window on its way out) or
/// peeked (a buried window stays alive and keeps zooming rights; its
/// entry is dropped by the `Destroyed` cleanup instead).
fn capture_window_config(app: &AppHandle, window_label: &str, config_key: &str, drain_zoom: bool) {
    let Some(window) = app.get_webview_window(window_label) else {
        return;
    };
    // Reading the URL hash is best-effort and must never crash the app on a
    // window close. Two nil-URL failure modes trip a panic deep in the runtime
    // (a nil/empty webview URL fails tauri-runtime-wry's `.parse().expect()` /
    // wry's `.URL().unwrap()`); that panic runs on the event-loop thread and
    // takes the WHOLE app down. The chan-side `match` below cannot catch it
    // because the panic is upstream of the returned `Result`.
    //   - Outbound windows point at a remote we do not own; when that remote is
    //     down the WKWebView never finishes navigating and reports a nil URL.
    //     The hash is chan-SPA restore state, meaningless for an outbound
    //     remote, so skip the read entirely (no url() call, no panic).
    //   - A local/tunnel window whose backend died before close can hit the
    //     same nil-URL panic, so guard that read with catch_unwind (the release
    //     profile unwinds, so this is catchable) and degrade to an empty hash.
    let url_hash = if window_label.starts_with("outbound-") {
        String::new()
    } else {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| window.url())) {
            Ok(Ok(u)) => u.fragment().unwrap_or("").to_string(),
            Ok(Err(e)) => {
                tracing::debug!(
                    label = %window_label,
                    error = %e,
                    "could not read url for closing window; pushing empty hash",
                );
                String::new()
            }
            Err(_) => {
                tracing::warn!(
                    label = %window_label,
                    "reading url for a closing window panicked (dead webview); pushing empty hash",
                );
                String::new()
            }
        }
    };
    let state = app.state::<Arc<AppState>>();
    let zoom_level = {
        let mut zooms = state.live_window_zooms.lock().unwrap();
        if drain_zoom {
            zooms.remove(window_label)
        } else {
            zooms.get(window_label).copied()
        }
        .unwrap_or(1.0)
    };
    state.push_window_config(WindowConfig {
        key: config_key.to_string(),
        window_label: window_label.to_string(),
        url_hash,
        zoom_level,
        saved_at: 0,
    });
}

fn ensure_window_capacity(app: &AppHandle, prefix: &str) -> Result<(), String> {
    let count = app
        .webview_windows()
        .keys()
        .filter(|label| label.starts_with(prefix))
        .count();
    if count >= MAX_WINDOWS_PER_WORKSPACE {
        return Err(format!(
            "Workspace already has {MAX_WINDOWS_PER_WORKSPACE} open windows; close one before opening another."
        ));
    }
    Ok(())
}

/// Destroy every webview window opened for this local workspace when
/// the local runtime is closed. Walks `webview_windows()` and
/// matches by prefix because the user may have opened several
/// windows for the same workspace.
pub fn close_local_workspace_windows(app: &AppHandle, key: &str) {
    close_windows_with_prefix(app, &workspace_window_prefix(key))
}

/// Destroy every webview window opened for this tunneled workspace.
/// Used by the tunnel supervisor when a (label, workspace) pair drops
/// out of the registry; the remote has gone away, so the per-tenant
/// listener no longer routes for it and any open window now points
/// at nothing useful.
pub fn close_tunneled_workspace_windows(app: &AppHandle, tenant_label: &str, workspace: &str) {
    close_windows_with_prefix(app, &tunnel_window_prefix(tenant_label, workspace))
}

/// Destroy every webview window opened for this outbound URL
/// attachment. Used when the user forgets the attachment row.
pub fn close_outbound_workspace_windows(app: &AppHandle, id: &str) {
    close_windows_with_prefix(app, &outbound_window_prefix(id))
}

/// Destroy every tunneled-workspace webview window in the process,
/// regardless of which (label, workspace) it belongs to. Used by the
/// tunnel module on `stop_listening`: the tunnel listener and
/// every per-tenant listener are about to be cancelled, so the
/// open windows would all error on their next request anyway.
pub fn close_all_tunneled_workspace_windows(app: &AppHandle) {
    close_windows_with_prefix(app, "tunnel-")
}

fn close_windows_with_prefix(app: &AppHandle, prefix: &str) {
    let app_owned = app.clone();
    let prefix_owned = prefix.to_string();
    let _ = app.run_on_main_thread(move || {
        // Snapshot first; destroying inside the iterator would
        // mutate the underlying map mid-walk.
        let labels: Vec<String> = app_owned
            .webview_windows()
            .keys()
            .filter(|l| l.starts_with(&prefix_owned))
            .cloned()
            .collect();
        for l in labels {
            if let Some(w) = app_owned.get_webview_window(&l) {
                let _ = w.destroy();
            }
        }
    });
}

/// Native keyboard shortcuts for workspace webviews. Translates chords
/// into the host-agnostic `chan:command` window event that chan's
/// App.svelte listens for. Runs before any page script, in capture
/// phase with stopImmediatePropagation, so this script is the sole
/// authority on every chord it claims, so chan's onWindowKey doesn't
/// fire for these even if its keymap drifts.
///
/// Layout mirrors VS Code; chords that browsers reserve at OS level
/// (Cmd+W, Cmd+N, Cmd+Shift+[/], Cmd+1..9) are bound here because
/// the native webview doesn't have those reservations. chan's web
/// fallbacks (Alt+Shift, Ctrl+Alt) keep working independently.
const KEY_BRIDGE_JS: &str = r#"
(() => {
  function fire(e, name, detail) {
    e.preventDefault();
    e.stopImmediatePropagation();
    window.dispatchEvent(new CustomEvent('chan:command',
      { detail: Object.assign({ name: name }, detail || {}) }));
  }
  // Cmd+R reloads the webview, Cmd+Opt+I opens
  // DevTools. Both bypass the SPA event bus and invoke their
  // Tauri IPC commands directly so a frozen Svelte runtime or a
  // broken chord registry can't lock the dev affordances away.
  function invokeIpc(e, cmd) {
    e.preventDefault();
    e.stopImmediatePropagation();
    const tauri = window.__TAURI__;
    if (tauri && tauri.core && typeof tauri.core.invoke === 'function') {
      tauri.core.invoke(cmd).catch((err) => {
        console.error('[chan] IPC ' + cmd + ' failed:', err);
      });
    }
  }
  // Chord policy: actions reachable through Pane Mode (Cmd+K) stay
  // unbound here (Cmd+`, Cmd+Shift+F) so the native layer claims as
  // little as possible. Direct chords exist where Pane Mode is no
  // substitute: Cmd+W (close tab; pairs with the SPA's context-aware
  // Ctrl+D), Cmd+F/G (find on page), Cmd+1..9 (jump to tab),
  // Cmd+[/Cmd+] (pane nav), Cmd+S (search), Cmd+/ and Cmd+Shift+/
  // (split right / down), Cmd+Shift+T (reopen closed), Cmd+Shift+[/]
  // (tab nav), Cmd+Shift+G (find prev), plus the context-aware spawn
  // family Cmd+T (terminal) / Cmd+O (File Browser) / Cmd+P (Team
  // Work) / Cmd+Shift+M (Graph), whose `app.files.toggle` /
  // `app.terminal.teamWork` / `app.graph.toggle` commands route
  // through the context-aware helpers in App.svelte. Universal
  // Hybrid NAV `t/o/p/v` covers the web/Win/Linux fallback path.
  function onKey(e) {
    const meta = e.metaKey || e.ctrlKey;
    if (!meta) return;
    const shift = e.shiftKey;
    const alt = e.altKey;
    const code = e.code;
    if (alt) {
      // Cmd+Opt+I (macOS) / Ctrl+Alt+I (Linux/Windows) → DevTools.
      // No other meta+alt chord today; bail out for everything else
      // so we don't shadow the webview's defaults.
      if (!shift && code === 'KeyI') {
        invokeIpc(e, 'open_devtools');
      }
      return;
    }
    // Zoom chords route regardless of shift so
    // Cmd+= (US) and Cmd+Shift+= (= Cmd++) both fire zoom_in.
    // NumpadAdd / NumpadSubtract similarly. Cmd+0 / Cmd+Numpad0
    // reset to 100 %.
    switch (code) {
      case 'Equal':
      case 'NumpadAdd':
        invokeIpc(e, 'zoom_in');
        return;
      case 'Minus':
      case 'NumpadSubtract':
        invokeIpc(e, 'zoom_out');
        return;
      case 'Digit0':
      case 'Numpad0':
        invokeIpc(e, 'zoom_reset');
        return;
    }
    if (!shift) {
      switch (code) {
        // Reload. macOS binds Cmd+R (metaKey); Linux/Windows moves to
        // Ctrl+Shift+R (shift branch below) so plain Ctrl+R reaches a
        // focused terminal's shell reverse-search. Gating on metaKey
        // here leaves Linux/macOS plain Ctrl+R untouched (no
        // preventDefault -> falls through to xterm), mirroring the
        // Cmd+W idiom below.
        case 'KeyR': if (e.metaKey) invokeIpc(e, 'reload_window'); return;
        case 'KeyT': fire(e, 'app.terminal.toggle'); return;
        case 'KeyO': fire(e, 'app.files.toggle');    return;
        case 'KeyP': fire(e, 'app.terminal.teamWork'); return;
        // Cmd+W closes the tab
        // on macOS. On Linux the platform mod is Ctrl and Ctrl+W is
        // readline delete-word inside a focused terminal, so DON'T
        // claim it - let it reach xterm. Linux closes tabs with Ctrl+D
        // (context-aware via the SPA's onCtrlDCapture, which leaves a
        // focused terminal to its EOF). Gating on metaKey (Cmd) leaves
        // Linux Ctrl+W untouched (no preventDefault -> reaches xterm).
        case 'KeyW':
          if (e.metaKey) {
            // On the connecting/retry page there are no tabs and the
            // app.tab.close dispatch is dead: Cmd+W means cancel, so
            // close the window for real (request_close_window
            // destroys, bypassing bury-on-close). The bridge claims
            // KeyW with stopImmediatePropagation BEFORE the page's own
            // listener AND before the File menu accelerator gets a
            // look-in, so the routing must happen here. Gate on the
            // CURRENT document (this init script re-runs after the
            // success navigation, where Cmd+W must stay tab-close).
            if (location.pathname.endsWith('/connecting.html')) {
              invokeIpc(e, 'request_close_window');
              return;
            }
            fire(e, 'app.tab.close');
          }
          return;
        case 'KeyS': fire(e, 'app.search.toggle');    return;
        case 'KeyF': fire(e, 'app.find.open');        return;
        case 'KeyG': fire(e, 'app.find.next');        return;
        // Cmd+I does NOT open Dashboard; it is reserved for the
        // editor's italic chord (bound in
        // Wysiwyg.svelte's CM6 keymap). Dashboard is reachable via
        // Hybrid Nav `Cmd+. i` + the Dashboard hamburger. With no
        // `KeyI` case here, Cmd+I falls through to the focused webview
        // (the editor toggles italic; otherwise inert). Cmd+Opt+I
        // (DevTools, alt branch above) and Cmd+Shift+I (broadcast,
        // shift branch below) are unaffected.
        case 'BracketLeft':  fire(e, 'app.pane.prev'); return;
        case 'BracketRight': fire(e, 'app.pane.next'); return;
        // Cmd+/ split right. Split
        // bottom is Cmd+Shift+/ (shift branch below). Cmd+\ is
        // deliberately NOT used: 1Password's system-wide Cmd+\
        // hotkey is dispatched by macOS before the key reaches this
        // webview, so chan never receives it. Web reaches splits via
        // Hybrid Nav `/` and `?`.
        case 'Slash':        fire(e, 'app.pane.splitRight'); return;
      }
      const m = code.match(/^Digit([1-9])$/);
      if (m) {
        fire(e, 'app.tab.jump', { index: Number(m[1]) - 1 });
        return;
      }
    } else {
      switch (code) {
        // Reload on Linux/Windows: Ctrl+Shift+R. Gate on !metaKey so
        // macOS Cmd+Shift+R does NOT reload (macOS reloads on Cmd+R in
        // the !shift branch above); the !metaKey form fires only for the
        // Ctrl+Shift+R that Linux/Windows users press.
        case 'KeyR': if (!e.metaKey) invokeIpc(e, 'reload_window'); return;
        // Close on Linux/Windows: Ctrl+Shift+W (plain Ctrl+W stays
        // readline delete-word inside a focused terminal, which is why
        // the !shift branch never claims it off macOS). Gate on
        // !metaKey so macOS keeps plain Cmd+W and Cmd+Shift+W stays
        // unclaimed there. Same routing as Cmd+W: cancel-close on the
        // connecting screen, tab-close everywhere else.
        case 'KeyW':
          if (!e.metaKey) {
            if (location.pathname.endsWith('/connecting.html')) {
              invokeIpc(e, 'request_close_window');
              return;
            }
            fire(e, 'app.tab.close');
          }
          return;
        case 'KeyG':         fire(e, 'app.find.prev');     return;
        case 'KeyT':         fire(e, 'app.tab.reopenClosed'); return;
        case 'KeyM':         fire(e, 'app.graph.toggle');  return;
        // Cmd+Shift+I (mac) / Ctrl+Shift+I (Linux,
        // Windows) toggles broadcast-input select-all/deselect-all for the
        // active terminal (mirrors iTerm). Ungated within the shift branch so
        // both platform mods fire; DevTools lives on the ALT chord
        // (Cmd+Opt+I / Ctrl+Alt+I, the `alt` branch above), and `fire()`
        // preventDefaults so the webview's built-in Ctrl+Shift+I DevTools
        // chord is suppressed. Web has no binding (cmd+shift+i is the browser
        // DevTools there).
        case 'KeyI': fire(e, 'app.terminal.broadcastToggle'); return;
        case 'BracketLeft':  fire(e, 'app.tab.prev');      return;
        case 'BracketRight': fire(e, 'app.tab.next');      return;
        // Cmd+Shift+/ (= Cmd+?) splits the active pane
        // bottom, pairing with Cmd+/ split-right above. Cmd+\ is
        // avoided - 1Password's global hotkey shadows it.
        case 'Slash':        fire(e, 'app.pane.splitDown');  return;
      }
    }
  }
  window.addEventListener('keydown', onKey, true);
})();
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoke_handler_registers_reload_window_and_open_devtools() {
        // The IPC commands `reload_window` and
        // `open_devtools` MUST be in the `tauri::generate_handler!`
        // list so the SPA's tab context-menu and the
        // accelerator path can reach them. The generate_handler!
        // macro does not catch a missing handler at compile time,
        // so we pin it here against the source file. Tests live in
        // serve.rs because main.rs has no test module today; using
        // `include_str!` keeps the pin source-of-truth-correct.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(MAIN_RS.contains("reload_window,"));
        assert!(MAIN_RS.contains("open_devtools,"));
        assert!(MAIN_RS.contains("fn reload_window(window: tauri::WebviewWindow)"));
        assert!(MAIN_RS.contains("fn open_devtools(window: tauri::WebviewWindow)"));
    }

    #[test]
    fn key_bridge_wires_zoom_chords_to_ipc() {
        // Cmd+= / Cmd+- / Cmd+0 (and their
        // Numpad variants) route directly to the chan-desktop
        // zoom IPC commands. Routed BEFORE the shift branch so
        // Cmd+Shift+= (= Cmd++) also zooms in. Capture-phase
        // listener stops the keydown so Tauri's `zoom_hotkeys_enabled`
        // polyfill (still on as a mousewheel + pinch fallback)
        // doesn't double-fire.
        assert!(KEY_BRIDGE_JS.contains("invokeIpc(e, 'zoom_in')"));
        assert!(KEY_BRIDGE_JS.contains("invokeIpc(e, 'zoom_out')"));
        assert!(KEY_BRIDGE_JS.contains("invokeIpc(e, 'zoom_reset')"));
        assert!(KEY_BRIDGE_JS.contains("case 'Equal':"));
        assert!(KEY_BRIDGE_JS.contains("case 'Minus':"));
        assert!(KEY_BRIDGE_JS.contains("case 'Digit0':"));
        assert!(KEY_BRIDGE_JS.contains("case 'NumpadAdd':"));
        assert!(KEY_BRIDGE_JS.contains("case 'NumpadSubtract':"));
        assert!(KEY_BRIDGE_JS.contains("case 'Numpad0':"));
    }

    #[test]
    fn key_bridge_wires_shift_i_to_broadcast_toggle_on_both_mods() {
        // Cmd+Shift+I (mac) / Ctrl+Shift+I (Linux, Windows) toggles
        // broadcast-input select-all/deselect-all for the active terminal.
        // Ungated within the shift branch (no `metaKey` gate) so both
        // platform mods fire; DevTools stays on the ALT chord.
        assert!(KEY_BRIDGE_JS.contains("case 'KeyI': fire(e, 'app.terminal.broadcastToggle');"));
        assert!(!KEY_BRIDGE_JS.contains("if (e.metaKey) fire(e, 'app.terminal.broadcastToggle')"));
    }

    #[test]
    fn invoke_handler_registers_zoom_commands() {
        // zoom_in / zoom_out / zoom_reset must be
        // in `tauri::generate_handler!` so KEY_BRIDGE_JS's IPC
        // invocations reach a registered command. generate_handler!
        // doesn't catch missing entries at compile time; pin here.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(MAIN_RS.contains("zoom_in,"));
        assert!(MAIN_RS.contains("zoom_out,"));
        assert!(MAIN_RS.contains("zoom_reset,"));
    }

    #[test]
    fn invoke_handler_registers_outbound_attach_ipcs() {
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(MAIN_RS.contains("add_outbound_workspace,"));
        assert!(MAIN_RS.contains("open_outbound_workspace,"));
        assert!(MAIN_RS.contains("remove_outbound_workspace,"));
        assert!(MAIN_RS.contains("fn add_outbound_workspace("));
        assert!(MAIN_RS.contains("fn open_outbound_workspace("));
        assert!(MAIN_RS.contains("fn remove_outbound_workspace("));
    }

    #[test]
    fn outbound_windows_load_the_connecting_page_not_the_remote() {
        // Blank-white outbound bug: a direct WebviewUrl::External(remote)
        // paints white when the remote is down. Outbound windows load the
        // bundled connecting page instead, which probes via `probe_url` and
        // navigates on success. Needles are built at runtime so this test's
        // own source text doesn't satisfy the `contains` checks (the
        // bin_status test uses the same trick).
        let serve_rs = include_str!("serve.rs");
        let app_load = format!("WebviewUrl::App({q}connecting.html", q = '"');
        let handoff = format!("__CHAN{u}CONNECTING__", u = '_');
        assert!(
            serve_rs.contains(&app_load),
            "outbound windows must load connecting.html, not the remote directly",
        );
        assert!(
            serve_rs.contains(&handoff),
            "the connecting page must receive its inputs via window.__CHAN_CONNECTING__",
        );
    }

    #[test]
    fn invoke_handler_registers_probe_url() {
        // The connecting screen's retry loop calls `probe_url` each attempt;
        // it must be in `tauri::generate_handler!` or the IPC denies and the
        // screen never detects a reachable remote. generate_handler! doesn't
        // catch a missing entry at compile time, so pin it here.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(MAIN_RS.contains("probe_url,"));
        assert!(MAIN_RS.contains("fn probe_url("));
    }

    #[test]
    fn invoke_handler_registers_default_workspace_ipcs() {
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(MAIN_RS.contains("default_workspace_status,"));
        assert!(MAIN_RS.contains("choose_default_workspace,"));
        assert!(MAIN_RS.contains("create_default_workspace,"));
        assert!(MAIN_RS.contains("factory_reset_default_workspace,"));
        assert!(MAIN_RS.contains("fn default_workspace_status("));
        assert!(MAIN_RS.contains("fn choose_default_workspace("));
        assert!(MAIN_RS.contains("fn create_default_workspace("));
        assert!(MAIN_RS.contains("fn factory_reset_default_workspace("));
    }

    #[test]
    fn new_workspace_local_choice_has_no_desktop_preflight() {
        // The desktop must not run its own first-boot pre-flight:
        // chan's SPA owns workspace readiness (PreflightOverlay.svelte)
        // plus the optional Semantic / Reports layer toggles, and a
        // desktop-side scan would double-dialog with, and race, the
        // SPA boot surface. The [New] modal's Local choice just
        // registers the folder and opens it. Pin the shape so a
        // refactor can't reintroduce a desktop-side pre-flight.
        const MAIN_JS: &str = include_str!("../../src/main.js");
        const MAIN_RS: &str = include_str!("main.rs");
        // The [New] modal registers via add_workspace
        // WITHOUT threading a desktop-chosen feature pair (the SPA's
        // onboarding card enables the optional layers post-boot).
        assert!(
            MAIN_JS.contains("showNewWorkspaceDialog("),
            "main.js must open the [New] workspace modal (showNewWorkspaceDialog)",
        );
        assert!(
            MAIN_JS.contains("invoke('add_workspace', { path: localPath }"),
            "the Local choice must register the chosen folder via add_workspace",
        );
        // No desktop pre-flight wiring may exist: no scan
        // IPC, no report renderer, no feature toggles, and none of the
        // explanatory copy the SPA owns.
        for gone in [
            "compute_workspace_preflight",
            "renderPreflightReport",
            "data-feat=\"bge\"",
            "data-feat=\"reports\"",
            "BM25 keyword search is",
            "dense-vector embeddings",
        ] {
            assert!(
                !MAIN_JS.contains(gone),
                "main.js must not carry the removed desktop pre-flight ({gone})",
            );
        }
        // The Rust IPC backend is gone too: no pre-flight scan in the app.
        assert!(
            !MAIN_RS.contains("fn compute_workspace_preflight("),
            "the desktop compute_workspace_preflight IPC must be removed",
        );
    }

    #[test]
    fn registry_commands_run_in_process_not_via_chan_cli() {
        // chan-desktop runs without a `chan` binary: `add_workspace`
        // and `remove_workspace` route through the embedded host's
        // shared `Library` rather than spawning chan. (Optional-layer
        // enablement is not a desktop concern at all: the SPA's
        // onboarding card drives it post-boot through chan-server.)
        // Pin the in-process call shape so a future change can't
        // silently reintroduce a subprocess dependency, and assert
        // the deleted subprocess argument shapes are gone.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(
            MAIN_RS.contains("embedded.library()"),
            "registry commands must route through the embedded shared Library",
        );
        assert!(
            MAIN_RS.contains("register_workspace") && MAIN_RS.contains("unregister_workspace"),
            "add_workspace/remove_workspace must use Library register/unregister in-process",
        );
        assert!(
            !MAIN_RS.contains("read_features_via_chan_index_status"),
            "the `chan index status --json` read path must be gone",
        );
        assert!(
            !MAIN_RS.contains("\"--semantic-search\"") && !MAIN_RS.contains("\"enable-semantic\""),
            "no chan CLI feature-flag arguments may remain",
        );
    }

    #[test]
    fn bin_status_machinery_is_gone() {
        // chan-desktop has no bundled-binary preflight or gating
        // (no subprocess paths). Pin the absence so a future change can't
        // quietly re-add a `chan` binary dependency or its gating.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(
            !MAIN_RS.contains("chan_bin_status"),
            "chan_bin_status command + registration must be gone",
        );
        assert!(
            !MAIN_RS.contains("fn require_bin"),
            "require_bin gating helper must be gone",
        );
        assert!(
            !MAIN_RS.contains("struct BinStatus"),
            "BinStatus struct must be gone",
        );
        // serve.rs must no longer carry the binary resolver. Build
        // the needle at runtime so this assertion's own source text
        // doesn't satisfy the `contains` check it performs.
        let serve_rs = include_str!("serve.rs");
        let resolver_sig = format!("fn resolve{}binary", "_chan_");
        assert!(
            !serve_rs.contains(&resolver_sig),
            "binary resolution helpers must be gone from serve.rs",
        );
    }

    #[test]
    fn launcher_prompts_for_existing_user_default_workspace() {
        const MAIN_JS: &str = include_str!("../../src/main.js");
        assert!(
            MAIN_JS.contains("invoke('default_workspace_status'"),
            "launcher must query default-workspace migration status",
        );
        assert!(
            MAIN_JS.contains("showDefaultWorkspaceDialog"),
            "launcher must prompt when a default workspace choice is needed",
        );
        assert!(
            MAIN_JS.contains("invoke('choose_default_workspace'"),
            "launcher must let users choose an existing default workspace",
        );
        assert!(
            MAIN_JS.contains("invoke('create_default_workspace'"),
            "launcher must let users create Documents/Chan as default",
        );
        assert!(
            MAIN_JS.contains("showMissingDefaultWorkspaceDialog"),
            "launcher must confirm before factory-resetting missing default workspace metadata",
        );
        assert!(
            MAIN_JS.contains("invoke('factory_reset_default_workspace'"),
            "launcher must route confirmed missing-default reset to Rust",
        );
    }

    #[test]
    fn new_window_accelerator_uses_cmd_shift_n() {
        // The "New Window" menu item binds
        // `CmdOrCtrl+Shift+N`; plain Cmd+N belongs to
        // the SPA's New Draft handler. Pin the
        // chord so a future menu edit can't silently land on
        // plain Cmd+N and clash with the SPA chord.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(
            MAIN_RS.contains(".accelerator(\"CmdOrCtrl+Shift+N\")"),
            "main.rs must bind the New Window menu item to CmdOrCtrl+Shift+N"
        );
        assert!(
            !MAIN_RS.contains(".accelerator(\"CmdOrCtrl+N\")"),
            "main.rs must NOT bind any menu item to plain CmdOrCtrl+N (reserved for SPA New Draft)"
        );
    }

    #[test]
    fn key_bridge_wires_reload_and_devtools_ipc() {
        // Cmd+R fires the `reload_window` IPC and
        // Cmd+Opt+I fires `open_devtools`, bypassing the SPA event
        // bus so a frozen Svelte runtime can't lock the dev
        // affordances away. The accelerator path goes through
        // `invokeIpc(...)` (not the `chan:command` `fire(...)`
        // bridge), so the contract pin checks both the IPC command
        // names and the case-label they're bound from.
        assert!(KEY_BRIDGE_JS.contains("invokeIpc(e, 'reload_window')"));
        assert!(KEY_BRIDGE_JS.contains("invokeIpc(e, 'open_devtools')"));
        // Reload is per-OS: Cmd+R (metaKey, no-shift branch) on macOS and
        // Ctrl+Shift+R (!metaKey, shift branch) on Linux/Windows, so plain
        // Ctrl+R is never claimed and reaches the terminal's reverse-search.
        assert!(KEY_BRIDGE_JS.contains("case 'KeyR': if (e.metaKey) invokeIpc(e, 'reload_window')"));
        assert!(
            KEY_BRIDGE_JS.contains("case 'KeyR': if (!e.metaKey) invokeIpc(e, 'reload_window')")
        );
        assert!(KEY_BRIDGE_JS.contains("code === 'KeyI'"));
    }

    #[test]
    fn embedded_url_prefix_parser_strips_query_and_trailing_slash() {
        let prefix = url_prefix_from_local_url("http://127.0.0.1:1234/workspace-abcd/?t=token")
            .expect("prefix");
        assert_eq!(prefix, "/workspace-abcd");
    }

    #[test]
    fn embedded_url_prefix_parser_strips_index_html() {
        let prefix =
            url_prefix_from_local_url("http://127.0.0.1:1234/workspace-abcd/index.html?t=token")
                .expect("prefix");
        assert_eq!(prefix, "/workspace-abcd");
    }

    #[test]
    fn key_bridge_invokes_tauri_ipc_via_core_invoke() {
        // The `invokeIpc` helper grabs `window.__TAURI__.core.invoke`
        // (Tauri 2's invoke surface; was `window.__TAURI__.invoke`
        // in Tauri 1). Pin so a future bridge rewrite doesn't
        // silently regress to the v1 shape. The new shape returns
        // undefined from a webview without the v2 IPC surface
        // attached, which silently swallows the Cmd+R / Cmd+Opt+I
        // accelerators.
        assert!(KEY_BRIDGE_JS.contains("window.__TAURI__"));
        assert!(KEY_BRIDGE_JS.contains("tauri.core.invoke"));
    }

    #[test]
    fn key_bridge_drops_chords_covered_by_pane_mode() {
        // Chords with a Pane Mode equivalent stay out of the native
        // bridge. The direct-chord exceptions (Cmd+T terminal, Cmd+O
        // files, Cmd+Shift+M graph, Cmd+P Team Work, Cmd+S search)
        // are asserted in `key_bridge_keeps_independent_chords`; the
        // absences here catch accidental reverts of chords that
        // should go through Pane Mode only.
        assert!(!KEY_BRIDGE_JS.contains("app.file.new"));
        assert!(!KEY_BRIDGE_JS.contains("Backquote"));
    }

    #[test]
    fn key_bridge_keeps_independent_chords() {
        // Tab close + reopen + Find on page + tab nav + tab jump
        // are NOT duplicated by Pane Mode and must stay reachable
        // through the native bridge. Cmd+T / Cmd+O / Cmd+P /
        // Cmd+Shift+M are the context-aware
        // spawn chord family.
        assert!(KEY_BRIDGE_JS.contains("app.terminal.toggle"));
        assert!(KEY_BRIDGE_JS.contains("app.files.toggle"));
        assert!(KEY_BRIDGE_JS.contains("app.terminal.teamWork"));
        assert!(KEY_BRIDGE_JS.contains("app.pane.prev"));
        assert!(KEY_BRIDGE_JS.contains("app.pane.next"));
        assert!(KEY_BRIDGE_JS.contains("app.graph.toggle"));
        assert!(KEY_BRIDGE_JS.contains("app.tab.close"));
        assert!(KEY_BRIDGE_JS.contains("app.tab.reopenClosed"));
        assert!(KEY_BRIDGE_JS.contains("app.find.open"));
        assert!(KEY_BRIDGE_JS.contains("app.tab.jump"));
        assert!(KEY_BRIDGE_JS.contains("app.tab.next"));
        assert!(KEY_BRIDGE_JS.contains("app.tab.prev"));
        // Cmd+S search + Cmd+/ (right)
        // / Cmd+Shift+/ (bottom) splits route through the native bridge
        // too.
        assert!(KEY_BRIDGE_JS.contains("app.search.toggle"));
        assert!(KEY_BRIDGE_JS.contains("app.pane.splitRight"));
        assert!(KEY_BRIDGE_JS.contains("app.pane.splitDown"));
        // Cmd+I is reserved for the editor's
        // italic chord, so the native bridge must not map it to
        // Dashboard. Pin the absence so a regression that re-adds the
        // case is caught (Dashboard is Hybrid-Nav-only).
        assert!(!KEY_BRIDGE_JS.contains("app.dashboard.open"));
    }

    #[test]
    fn local_title_prefixes_home_vs_computer_glyph_then_path() {
        // The local title leads with the kind glyph (home when under the
        // user's home dir, computer otherwise) then the path verbatim
        // (the path is the disambiguating window-switcher
        // signal). `local_title` takes home explicitly so the test does not
        // depend on the process's real home dir.
        let home = Path::new("/Users/alex");
        assert_eq!(
            local_title("/Users/alex/dev/github.com/fiorix/chan", Some(home)),
            format!("{ICON_LOCAL_HOME} /Users/alex/dev/github.com/fiorix/chan"),
        );
        // Outside home -> computer glyph. Trailing slash passed through.
        assert_eq!(
            local_title("/tmp/scratch/", Some(home)),
            format!("{ICON_LOCAL_OTHER} /tmp/scratch/"),
        );
        // No resolvable home dir -> computer glyph (never mislabels as home).
        assert_eq!(
            local_title("/Users/alex/notes", None),
            format!("{ICON_LOCAL_OTHER} /Users/alex/notes"),
        );
    }

    #[test]
    fn listen_addr_from_url_extracts_host_port_authority() {
        // The inbound (tunnel) title locator is the per-tenant loopback
        // host:port the window connects to.
        assert_eq!(
            listen_addr_from_url("http://127.0.0.1:54321/notes/?t=tok"),
            "127.0.0.1:54321",
        );
        assert_eq!(
            listen_addr_from_url("http://localhost:8787/index.html"),
            "localhost:8787",
        );
        // No authority to extract -> defensive fall back to the raw string.
        assert_eq!(listen_addr_from_url("not a url"), "not a url");
    }

    // Workspace and tunnel webviews host the SPA, which
    // routes external http(s) link clicks through tauri-plugin-opener
    // via the `plugin:opener|open_url` IPC. Without these permissions
    // the IPC denies, the SPA falls back to the clipboard-copy notify
    // branch, and "click external link" looks like a no-op to the
    // user. Pin the capability
    // shape here so a future capability-file edit can't silently drop
    // the permissions without the test catching it.
    const WORKSPACE_CAPABILITY_JSON: &str = include_str!("../capabilities/workspace.json");
    const DEFAULT_CAPABILITY_JSON: &str = include_str!("../capabilities/default.json");
    const LOCAL_DROP_CAPABILITY_JSON: &str = include_str!("../capabilities/local-drop.json");
    const APP_PERMISSIONS_TOML: &str = include_str!("../permissions/app.toml");

    fn capability_permissions(raw: &str) -> Vec<String> {
        let v: serde_json::Value = serde_json::from_str(raw).expect("capability JSON parses");
        v["permissions"]
            .as_array()
            .expect("permissions is an array")
            .iter()
            .map(|p| p.as_str().expect("permission is a string").to_string())
            .collect()
    }

    fn capability_windows(raw: &str) -> Vec<String> {
        let v: serde_json::Value = serde_json::from_str(raw).expect("capability JSON parses");
        v["windows"]
            .as_array()
            .expect("windows is an array")
            .iter()
            .map(|w| w.as_str().expect("window glob is a string").to_string())
            .collect()
    }

    fn capability_remote_urls(raw: &str) -> Vec<String> {
        let v: serde_json::Value = serde_json::from_str(raw).expect("capability JSON parses");
        v["remote"]["urls"]
            .as_array()
            .expect("remote urls is an array")
            .iter()
            .map(|u| {
                u.as_str()
                    .expect("remote URL pattern is a string")
                    .to_string()
            })
            .collect()
    }

    fn app_permission_set(id: &str) -> Vec<String> {
        let v: toml::Value = toml::from_str(APP_PERMISSIONS_TOML).expect("app permissions parse");
        v["set"]
            .as_array()
            .expect("permission sets is an array")
            .iter()
            .find(|set| set["identifier"].as_str() == Some(id))
            .unwrap_or_else(|| panic!("missing app permission set {id}"))["permissions"]
            .as_array()
            .expect("permission set entries are an array")
            .iter()
            .map(|p| p.as_str().expect("permission id is a string").to_string())
            .collect()
    }

    #[test]
    fn workspace_capability_grants_opener_to_workspace_tunnel_and_outbound_windows() {
        let windows = capability_windows(WORKSPACE_CAPABILITY_JSON);
        assert!(
            windows.iter().any(|w| w == "workspace-*"),
            "workspace capability must target workspace-* windows: {windows:?}",
        );
        assert!(
            windows.iter().any(|w| w == "tunnel-*"),
            "workspace capability must target tunnel-* windows: {windows:?}",
        );
        assert!(
            windows.iter().any(|w| w == "outbound-*"),
            "workspace capability must target outbound-* windows: {windows:?}",
        );
        let perms = capability_permissions(WORKSPACE_CAPABILITY_JSON);
        assert!(
            perms.iter().any(|p| p == "workspace-window"),
            "workspace capability must include workspace-window app commands: {perms:?}",
        );
        assert!(
            perms.iter().any(|p| p == "opener:allow-open-url"),
            "workspace capability must include opener:allow-open-url: {perms:?}",
        );
    }

    #[test]
    fn workspace_capability_covers_loopback_server_urls() {
        // Workspace windows load chan-server through loopback HTTP
        // origins. Without a remote URL match, Tauri omits the IPC
        // bridge and workspace-window app commands such as reload_window
        // or the zoom chords never reach Rust.
        let remote_urls = capability_remote_urls(WORKSPACE_CAPABILITY_JSON);
        assert!(
            remote_urls.iter().any(|u| u == "http://127.0.0.1:*"),
            "workspace capability must include 127.0.0.1 loopback: {remote_urls:?}",
        );
        assert!(
            remote_urls.iter().any(|u| u == "http://localhost:*"),
            "workspace capability must include localhost loopback: {remote_urls:?}",
        );
    }

    #[test]
    fn app_acl_allows_workspace_window_commands() {
        let workspace_set = app_permission_set("workspace-window");
        for expected in [
            "allow-reload-window",
            "allow-open-devtools",
            "allow-save-file-to-downloads",
            "allow-zoom-in",
            "allow-zoom-out",
            "allow-zoom-reset",
            // The connecting screen (outbound-* windows) probes the remote
            // through this command; without the ACL grant the IPC denies and
            // the screen never detects a reachable remote.
            "allow-probe-url",
        ] {
            assert!(
                workspace_set.iter().any(|p| p == expected),
                "workspace-window app permission set must include {expected}: {workspace_set:?}",
            );
        }
    }

    #[test]
    fn invoke_handler_registers_read_dropped_paths() {
        // The SPA's terminal drop handler invokes `read_dropped_paths`
        // at DOM drop time; it must be in `tauri::generate_handler!`
        // or the IPC denies and the terminal path-print silently
        // no-ops. generate_handler! doesn't catch missing entries at
        // compile time, so pin it here.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(MAIN_RS.contains("dropped_paths::read_dropped_paths,"));
        const DROPPED_PATHS_RS: &str = include_str!("dropped_paths.rs");
        assert!(DROPPED_PATHS_RS.contains("pub async fn read_dropped_paths("));
        // NSPasteboard is AppKit state: the read must run on the main
        // thread, not the IPC worker thread.
        assert!(DROPPED_PATHS_RS.contains("run_on_main_thread"));
    }

    #[test]
    fn drag_pasteboard_read_is_scoped_to_locally_served_windows() {
        // The macOS drag pasteboard is system-wide and persists after
        // the drag ends: a remote-served SPA (tunnel-* / outbound-*
        // windows) must NOT be able to poll `read_dropped_paths` and
        // harvest paths the user drags around in other applications.
        // The grant therefore lives in its own capability targeting
        // only the locally-served window kinds...
        let windows = capability_windows(LOCAL_DROP_CAPABILITY_JSON);
        assert!(
            windows.iter().any(|w| w == "workspace-*"),
            "local-drop capability must cover workspace-* windows: {windows:?}",
        );
        assert!(
            windows.iter().any(|w| w == "terminal-*"),
            "local-drop capability must cover terminal-* windows: {windows:?}",
        );
        assert!(
            windows
                .iter()
                .all(|w| w != "tunnel-*" && w != "outbound-*" && w != "main"),
            "local-drop capability must stay off remote-served and launcher windows: {windows:?}",
        );
        let perms = capability_permissions(LOCAL_DROP_CAPABILITY_JSON);
        assert!(
            perms.iter().any(|p| p == "allow-read-dropped-paths"),
            "local-drop capability must grant allow-read-dropped-paths: {perms:?}",
        );
        // ...and must not leak in through the broad surfaces that
        // tunnel-* / outbound-* windows DO receive.
        let workspace_perms = capability_permissions(WORKSPACE_CAPABILITY_JSON);
        assert!(
            workspace_perms
                .iter()
                .all(|p| p != "allow-read-dropped-paths"),
            "workspace capability must not carry the drag-pasteboard grant: {workspace_perms:?}",
        );
        let workspace_set = app_permission_set("workspace-window");
        assert!(
            workspace_set.iter().all(|p| p != "allow-read-dropped-paths"),
            "workspace-window permission set must not carry the drag-pasteboard grant: {workspace_set:?}",
        );
        // Belt symmetry: the launcher (default capability) is
        // locally-served and outside the harvest threat model, but it
        // has no drop surface either — pin it off so the grant can't
        // drift in through the third broad capability.
        let default_perms = capability_permissions(DEFAULT_CAPABILITY_JSON);
        assert!(
            default_perms.iter().all(|p| p != "allow-read-dropped-paths"),
            "launcher default capability must not carry the drag-pasteboard grant: {default_perms:?}",
        );
        let main_set = app_permission_set("main-window");
        assert!(
            main_set.iter().all(|p| p != "allow-read-dropped-paths"),
            "main-window permission set must not carry the drag-pasteboard grant: {main_set:?}",
        );
    }

    #[test]
    fn connecting_screen_windows_close_for_real() {
        // A window still on connecting.html must be CLOSABLE: the red
        // dot really closes (no bury - a buried connecting window is an
        // unkillable hidden retry loop), and the page itself offers
        // Cmd/Ctrl+W + Ctrl+D chords that invoke request_close_window
        // (destroy, bypassing the bury handler). concat! so the source
        // pin doesn't match this test.
        const SERVE_RS: &str = include_str!("serve.rs");
        assert!(SERVE_RS.contains(concat!(
            "!window_on_connecting",
            "_screen(&app_for_close, &label_for_close)"
        )));
        // KEY_BRIDGE_JS claims the close chord (window capture +
        // stopImmediatePropagation) before BOTH the page's listener and
        // the File-menu accelerator, so the bridge itself must route
        // KeyW to request_close_window while on connecting.html — a
        // page-level chord alone never sees the key (dead Cmd+W).
        // TWO routings: macOS plain Cmd+W (!shift branch) and the
        // Linux/Windows Ctrl+Shift+W (shift branch).
        let close_invoke = concat!("invokeIpc(e, 'request_close", "_window')");
        assert_eq!(SERVE_RS.matches(close_invoke).count(), 2);
        assert!(SERVE_RS.contains("location.pathname.endsWith('/connecting.html')"));
        const CONNECTING_JS: &str = include_str!("../../src/connecting.js");
        assert!(CONNECTING_JS.contains("request_close_window"));
        assert!(CONNECTING_JS.contains("key === 'd'"));
        assert!(CONNECTING_JS.contains("key === 'w'"));
    }

    #[test]
    fn default_capability_covers_extra_launcher_windows() {
        // The default capability covers `main-*` alongside the
        // singleton `main`: any launcher-class window must inherit
        // the same capability as `main`, or external link handling
        // and other plugin IPCs break the moment one exists.
        let windows = capability_windows(DEFAULT_CAPABILITY_JSON);
        assert!(
            windows.iter().any(|w| w == "main"),
            "default capability must still target main: {windows:?}",
        );
        assert!(
            windows.iter().any(|w| w == "main-*"),
            "default capability must target additional main-N launchers: {windows:?}",
        );
        let perms = capability_permissions(DEFAULT_CAPABILITY_JSON);
        assert!(
            perms.iter().any(|p| p == "main-window"),
            "default capability must include main-window app commands: {perms:?}",
        );
        assert!(
            perms.iter().any(|p| p == "opener:allow-open-url"),
            "default capability must include opener:allow-open-url: {perms:?}",
        );
    }
}
