#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod config;
mod cs_install;
mod default_workspace;
mod download;
mod embedded;
mod linux_gui_stack;
#[cfg(target_os = "macos")]
mod pdf;
mod registry;
mod serve;
mod tunnel;
mod watcher;

use std::collections::HashMap;
#[cfg(unix)]
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;
#[cfg(target_os = "macos")]
use tauri::menu::{Menu, PredefinedMenuItem, WINDOW_SUBMENU_ID};
use tauri::menu::{MenuItemBuilder, MenuItemKind, Submenu};
use tauri::{Emitter, Manager, RunEvent, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use config::{Config, ConfigStore, OutboundWorkspace, WindowConfig, WorkspaceFeatures};
use serve::ServeHandle;
use tunnel::TunnelState;

const CHAN_BUSY_CHANGED: &str = "chan-busy";
const SYSTEM_NOTICE: &str = "system-notice";

/// Process-wide state. Shared via `Arc` because Tauri commands and
/// background runtime owners need the same state handle.
pub struct AppState {
    store: Mutex<ConfigStore>,
    /// Live embedded local workspaces keyed by canonical workspace path.
    serves: Mutex<HashMap<String, ServeHandle>>,
    /// In-process chan-server host for normal local workspaces.
    /// Initialized during Tauri setup, after the async runtime is
    /// available for Tokio listener registration.
    embedded: OnceLock<embedded::EmbeddedServer>,
    /// Embedded chan-tunnel-server. Owns the tunnel listener on
    /// 127.0.0.1:7777, the shared registry, and the per-tenant
    /// loopback listeners that proxy into registered remote
    /// `chan serve` instances.
    tunnel: Arc<TunnelState>,
    /// `fullstack-b-19`: per-live-window zoom level. Tracks the
    /// current zoom for every open webview keyed by window label so
    /// `zoom_in` / `zoom_out` / `zoom_reset` can compute the next
    /// level without spawning a JS eval round-trip to read the
    /// current. Drained into `WindowConfig.zoom_level` by the close
    /// handler so the LRU restore from `-b-1` picks the level up on
    /// the next open. Missing entry reads as 1.0 (the chan-desktop
    /// default).
    pub live_window_zooms: Mutex<HashMap<String, f64>>,
    /// Per-live-window display number, keyed by window label, with the
    /// base title it was assigned under. Drives the `"{title} Window
    /// {N}"` suffix so the OS Window menu disambiguates windows that
    /// share a base title (two windows on the same workspace, several
    /// standalone terminals). `N` is the lowest free number among live
    /// windows with the SAME base title, so a number freed by a closed
    /// window gets reused — mirroring `Registry::next_terminal_name`'s
    /// lowest-free `Terminal-N` scheme. Freed on window destroy; a
    /// BURIED (hidden) window keeps its number so its Window-menu entry
    /// and title stay stable across the hide/reopen cycle.
    pub window_numbers: Mutex<HashMap<String, (String, u64)>>,
    /// Windows hidden ("buried") by the OS close button instead of
    /// destroyed, in bury order (most recent last). The webview stays
    /// alive — live terminals keep running, layout state stays warm —
    /// and the Window menu lists each entry for reopening (also
    /// Cmd/Ctrl+Shift+N, which unburies the most recent of the focused
    /// family). Entries leave the list on unbury or window destroy.
    pub buried_windows: Mutex<Vec<BuriedWindow>>,
    /// Reopenable REMOTE windows, keyed by remote window label: the
    /// `saved && !connected` rows from each remote connection's
    /// (outbound attachment / tunnel tenant) `GET /api/windows`,
    /// refreshed by `refresh_remote_windows_menu`. The Window menu
    /// lists them under `remote:` ids; clicking one opens a webview
    /// with that exact label so the remote restores its session blob.
    pub remote_reopen: Mutex<HashMap<String, RemoteReopen>>,
}

/// One reopenable remote window: see `AppState::remote_reopen`.
#[derive(Debug, Clone)]
pub struct RemoteReopen {
    /// The connection's webview URL (outbound URL with its token /
    /// tunnel per-tenant loopback URL).
    pub url: String,
    /// Base window title (`📤 <url>` / `📥 <host:port>`); the build
    /// suffixes " Window N".
    pub base_title: String,
    /// Menu entry text (base title + the remote window's tail).
    pub menu_title: String,
    /// WindowConfig identity key for the connection (close/bury of the
    /// reopened window captures restore state under it).
    pub config_key: String,
    /// Route through the connecting screen (outbound remotes; a down
    /// remote must not paint a blank webview).
    pub connecting: bool,
}

/// One buried (hidden, not closed) window: see `AppState::buried_windows`.
#[derive(Debug, Clone)]
pub struct BuriedWindow {
    /// Tauri window label (`workspace-<16hex>-<seq>` / `terminal-win-<seq>` /
    /// tunnel / outbound). Also the Window-menu item id suffix.
    pub label: String,
    /// OS display title at bury time ("🏠 /path Window 2",
    /// "Terminal Window 1") — shown verbatim in the Window menu.
    pub title: String,
    /// Wall-clock millis at bury time; diagnostics only (the Vec's
    /// push order is the recency authority).
    pub buried_at: u64,
}

/// Family prefix for unbury matching: the label with its trailing
/// `-<seq>` segment removed (everything through the LAST dash).
/// `terminal-win-3` -> `terminal-win-` (all standalone terminals are
/// one family); `workspace-<16hex>-2` -> `workspace-<16hex>-` (one
/// family per workspace; same shape for tunnel / outbound labels).
fn window_family_prefix(label: &str) -> &str {
    match label.rfind('-') {
        Some(idx) => &label[..=idx],
        None => label,
    }
}

/// Most recently buried label starting with `prefix`, scanning the
/// bury-ordered slice from the newest end. Free function so the
/// recency/family logic is unit-testable without an `AppState`.
fn most_recent_buried_with_prefix<'a>(buried: &'a [BuriedWindow], prefix: &str) -> Option<&'a str> {
    buried
        .iter()
        .rev()
        .find(|b| b.label.starts_with(prefix))
        .map(|b| b.label.as_str())
}

/// Defense-in-depth local runtime teardown: `RunEvent::Exit` is the
/// primary path, but a panic unwinding through `tauri::App` can
/// bypass it. Dropping the last `Arc<AppState>` signals every
/// running local workspace via `serve::stop_all`. Idempotent: stop_all
/// drains the serves map, so a normal-exit run followed by Drop is a
/// no-op on the second pass.
impl Drop for AppState {
    fn drop(&mut self) {
        serve::stop_all(self);
    }
}

/// Lowest free display number (`>= 1`) for `base` among the live
/// window-number entries, ignoring any slot already held by `label`
/// itself (so a re-assign of the same window keeps its number stable).
/// Split out as a free function so the reuse logic is unit-testable
/// without constructing a full `AppState`.
fn lowest_free_window_number(
    numbers: &HashMap<String, (String, u64)>,
    label: &str,
    base: &str,
) -> u64 {
    let taken: std::collections::HashSet<u64> = numbers
        .iter()
        .filter(|(l, (b, _))| l.as_str() != label && b == base)
        .map(|(_, (_, n))| *n)
        .collect();
    (1u64..)
        .find(|n| !taken.contains(n))
        .expect("the naturals always contain a free slot")
}

impl AppState {
    /// Push a closing window's layout onto the LRU stack. Best
    /// effort: any I/O error is logged and dropped so a flaky
    /// config disk doesn't leak through the WindowEvent handler.
    pub fn push_window_config(&self, entry: WindowConfig) {
        let mut store = self.store.lock().unwrap();
        let mut cfg = match store.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "loading config to push window state failed");
                return;
            }
        };
        config::push_window_config(&mut cfg, entry);
        if let Err(e) = store.save(&cfg) {
            tracing::warn!(error = %e, "persisting window config stack failed");
        }
    }

    /// Pop the most-recent WindowConfig matching `key` whose label
    /// isn't a live webview (see `config::pop_window_config`),
    /// removing it from the stack on disk. Returns `None` when no
    /// entry exists or the config file can't be read. Same best-effort
    /// posture as `push_window_config`.
    pub fn pop_window_config(
        &self,
        key: &str,
        is_label_live: impl Fn(&str) -> bool,
    ) -> Option<WindowConfig> {
        let mut store = self.store.lock().unwrap();
        let mut cfg = match store.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "loading config to pop window state failed");
                return None;
            }
        };
        let popped = config::pop_window_config(&mut cfg, key, is_label_live)?;
        if let Err(e) = store.save(&cfg) {
            tracing::warn!(error = %e, "persisting window config stack failed");
        }
        Some(popped)
    }

    /// Assign the lowest-free display number for `base` among live
    /// windows that share the same base title, record it under
    /// `label`, and return it. The first window of a given base is
    /// `1`; a number freed by `release_window_number` is handed back
    /// out on the next assign — mirroring the lowest-free reuse of
    /// `Registry::next_terminal_name`. Re-assigning the same `label`
    /// (a defensive double-build) refreshes its slot.
    pub fn assign_window_number(&self, label: &str, base: &str) -> u64 {
        let mut numbers = self.window_numbers.lock().unwrap();
        let n = lowest_free_window_number(&numbers, label, base);
        numbers.insert(label.to_string(), (base.to_string(), n));
        n
    }

    /// Release the display number held by `label` so it can be reused
    /// by the next window with the same base title. Called from the
    /// window-destroy handler. A no-op for an unknown label.
    pub fn release_window_number(&self, label: &str) {
        self.window_numbers.lock().unwrap().remove(label);
    }

    /// Record `label` as buried (most recent). Re-burying a label
    /// drops its older entry first so the list holds one entry per
    /// window and recency stays truthful.
    pub fn bury_window(&self, label: &str, title: &str) {
        let mut buried = self.buried_windows.lock().unwrap();
        buried.retain(|b| b.label != label);
        buried.push(BuriedWindow {
            label: label.to_string(),
            title: title.to_string(),
            buried_at: config::current_millis(),
        });
    }

    /// Drop `label` from the buried list (unburied or destroyed).
    /// Returns whether an entry was actually removed, so callers know
    /// if the Window menu needs a rebuild.
    pub fn remove_buried(&self, label: &str) -> bool {
        let mut buried = self.buried_windows.lock().unwrap();
        let before = buried.len();
        buried.retain(|b| b.label != label);
        buried.len() != before
    }

    /// Most recently buried window label whose label starts with
    /// `prefix` (a window-family prefix, see `window_family_prefix`).
    pub fn most_recent_buried(&self, prefix: &str) -> Option<String> {
        let buried = self.buried_windows.lock().unwrap();
        most_recent_buried_with_prefix(&buried, prefix).map(str::to_string)
    }

    /// (label, title) pairs of every buried window, most recent first
    /// (Window-menu display order).
    pub fn buried_snapshot(&self) -> Vec<(String, String)> {
        self.buried_windows
            .lock()
            .unwrap()
            .iter()
            .rev()
            .map(|b| (b.label.clone(), b.title.clone()))
            .collect()
    }
}

/// Merged workspace view returned to the frontend. Two flavours share
/// the wire shape so the existing renderer can iterate one list:
///
/// * `kind = "local"`: a chan-registry entry, backed by a
///   workspace mounted into the embedded server. Includes the canonical
///   filesystem path and live URL.
/// * `kind = "tunneled"`: a remote `chan serve` that dialed into
///   the embedded tunnel server. No path; `url` points at the
///   per-tenant loopback listener.
/// * `kind = "outbound"`: a remote `chan serve` explicitly attached
///   by URL. No desktop-owned lifecycle; `id` points at the stored
///   attachment row.
///
/// Fields specific to tunneled rows are optional so the JSON shape
/// is a strict superset of the local row; the renderer reads `kind`
/// once and chooses which optionals to surface.
#[derive(Debug, Clone, Serialize)]
struct Workspace {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    path: String,
    on: bool,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    public: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    peer_addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    connected_at: Option<String>,
}

#[tauri::command]
fn list_workspaces(state: State<Arc<AppState>>) -> Result<Vec<Workspace>, String> {
    let serves = state.serves.lock().unwrap();
    let entries = registry::read().map_err(err)?;

    // `on` is derived from a live serve handle, never persisted.
    // That way a desktop restart comes up with everything off
    // (matching reality: nothing is actually running yet) and
    // there is no chance of a stale on=true sticking around after
    // chan died unexpectedly.
    let mut merged: Vec<Workspace> = entries
        .into_iter()
        .map(|e| {
            let key = canonical_key(&e.root_path);
            let display_path = key.clone();
            let handle = serves.get(&key);
            let on = handle.is_some();
            let url = handle.and_then(|h| h.url.clone()).unwrap_or_default();
            Workspace {
                kind: "local",
                id: None,
                path: display_path,
                on,
                url,
                label: None,
                workspace: None,
                public: None,
                peer_addr: None,
                connected_at: None,
            }
        })
        .collect();

    // Tunneled rows: one per registered (label, workspace) in the
    // embedded chan-tunnel-server. URL is populated by the
    // supervisor as soon as the per-tenant listener binds; an
    // empty URL means "just registered, the listener will follow
    // on the next 500ms tick".
    for t in state.tunnel.snapshot() {
        merged.push(Workspace {
            kind: "tunneled",
            id: None,
            path: String::new(),
            on: true,
            url: t.url,
            label: Some(t.label),
            workspace: Some(t.workspace),
            public: Some(t.public),
            peer_addr: t.peer_addr,
            connected_at: Some(t.connected_at),
        });
    }

    let outbound_workspaces = state.store.lock().unwrap().get().map_err(err)?.outbound;
    for outbound in outbound_workspaces {
        let label = outbound_label(&outbound);
        let id = outbound.id;
        let url = outbound.url;
        merged.push(Workspace {
            kind: "outbound",
            id: Some(id),
            path: url.clone(),
            on: true,
            url,
            label,
            workspace: None,
            public: None,
            peer_addr: None,
            connected_at: None,
        });
    }

    Ok(merged)
}

/// `fullstack-b-28b` slice iii: the pre-flight modal collects the
/// user's feature choices BEFORE the workspace is registered + passes
/// them through to `chan add`. The chan CLI's `--semantic-search`
/// + `--reports` flags from `systacean-27` are the right
/// registration-time entry point so chan-workspace's BOOT process
/// picks up the chosen state on the FIRST open (no stub +
/// re-toggle cycle).
///
/// `features` is optional for SPA-side backward compatibility +
/// for the CLI-level `add_workspace` calls that don't surface the
/// pre-flight UX. Missing or default `features` opens the workspace
/// lean (BM25-only, no reports).
#[tauri::command]
async fn add_workspace(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    path: String,
    features: Option<WorkspaceFeatures>,
) -> Result<(), String> {
    let path = canonical_key(Path::new(&path));
    let features = features.unwrap_or_default();
    let Some(embedded) = state.embedded.get() else {
        return Err("embedded local server is unavailable".to_string());
    };
    // Route through the SINGLE embedded Library so the in-memory
    // registry the host opens workspaces against learns about the new
    // row immediately. A subprocess `chan add` would mutate only
    // the on-disk registry, leaving the host's boot-time snapshot
    // stale, which is the "workspace not registered" bug this replaces.
    let library = embedded.library().clone();
    let path_for_block = path.clone();

    emit_chan_busy(&app, true, "add", &path);
    // register_workspace + boot run off the async executor: boot can
    // walk a large workspace on first reports activation.
    let result =
        tokio::task::spawn_blocking(move || register_and_boot(&library, &path_for_block, features))
            .await;
    emit_chan_busy(&app, false, "add", &path);
    match result {
        Ok(inner) => inner?,
        Err(e) => return Err(format!("registering workspace panicked: {e}")),
    }

    // `fullstack-b-28b` slice iii: mirror the chosen features into
    // the desktop cache so `get_workspace_features` returns the
    // authoritative state immediately, before the user toggles
    // anything in the launcher row.
    if features != WorkspaceFeatures::default() {
        let mut store = state.store.lock().unwrap();
        let mut cfg = store.get().map_err(err)?;
        cfg.workspaces.entry(path.clone()).or_default().features = features;
        store.save(&cfg).map_err(err)?;
    }

    // Auto-start: opening a workspace from the desktop is the user's
    // way of saying "make this workspace usable now". Spinning up the
    // serve immediately is what they expect; otherwise the freshly
    // added row sits there with On=off and Launch disabled, which
    // looks broken.
    serve::start(app, Arc::clone(&state), path).await?;
    Ok(())
}

/// Register `path` with the shared embedded Library and, if any
/// optional feature was requested, open the workspace once to persist
/// the flags and kick the BOOT scan. Mirrors `chan/src/main.rs`'s
/// `cmd_add`. The transient `Arc<Workspace>` is dropped before this
/// returns so the immediately-following `serve::start` can mount
/// the workspace without tripping `WorkspaceAlreadyOpen` against the
/// lifetime flock. Blocking: `register_workspace` writes the registry
/// and `boot()` can run a slow initial scan, so callers invoke it
/// via `spawn_blocking`.
fn register_and_boot(
    library: &chan_workspace::Library,
    path: &str,
    features: WorkspaceFeatures,
) -> Result<(), String> {
    let root = Path::new(path);
    if !root.exists() {
        std::fs::create_dir_all(root)
            .map_err(|e| format!("creating workspace root {path}: {e}"))?;
    }
    let entry = library
        .register_workspace(root)
        .map_err(|e| format!("registering workspace {path}: {e}"))?;
    if features.bge || features.reports {
        let workspace = library
            .open_workspace(&entry.root_path)
            .map_err(|e| format!("opening workspace {}: {e}", entry.root_path.display()))?;
        if features.bge {
            workspace
                .set_semantic_enabled(true)
                .map_err(|e| format!("enabling semantic search: {e}"))?;
        }
        if features.reports {
            workspace
                .set_reports_enabled(true)
                .map_err(|e| format!("enabling reports: {e}"))?;
        }
        workspace
            .boot()
            .map_err(|e| format!("boot after enabling features: {e}"))?;
        // Drop the transient handle before serve::start re-opens it.
        drop(workspace);
    }
    Ok(())
}

#[tauri::command]
async fn remove_workspace(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    path: String,
) -> Result<(), String> {
    let key = canonical_key(Path::new(&path));
    // Stop the serve first: this removes the runtime synchronously
    // and drops the host's Arc<Workspace>, but background indexer /
    // request tasks may briefly keep their own clone, so the
    // unregister below tolerates a short contention window.
    serve::stop(Some(&app), &state, &key);

    let Some(embedded) = state.embedded.get() else {
        return Err("embedded local server is unavailable".to_string());
    };
    let library = embedded.library().clone();
    let key_for_block = key.clone();

    emit_chan_busy(&app, true, "remove", &key);
    let result =
        tokio::task::spawn_blocking(move || unregister_with_retry(&library, &key_for_block)).await;
    emit_chan_busy(&app, false, "remove", &key);
    match result {
        Ok(inner) => inner?,
        Err(e) => return Err(format!("unregistering workspace panicked: {e}")),
    }

    let mut store = state.store.lock().unwrap();
    let mut cfg = store.get().map_err(err)?;
    cfg.workspaces.remove(&key);
    store.save(&cfg).map_err(err)?;
    Ok(())
}

/// Drop a workspace from the shared registry after its serve has been
/// stopped. `serve::stop` removes the runtime synchronously, but a
/// background indexer rebuild or an in-flight HTTP/WS handler can
/// still hold an `Arc<Workspace>` for a moment. `unregister_workspace`
/// wipes per-workspace state and so needs exclusive access; until the
/// last handle drops it returns `WorkspaceAlreadyOpen` (this process)
/// or `WorkspaceLocked` (the flock). `reset_workspace` takes the flock
/// before any registry mutation, so a failed attempt leaves no
/// half-state and a retry is safe. Any other error surfaces
/// immediately. Blocking: sleeps between attempts, so callers
/// invoke it via `spawn_blocking`.
fn unregister_with_retry(library: &chan_workspace::Library, key: &str) -> Result<(), String> {
    use chan_workspace::ChanError;
    const MAX_ATTEMPTS: usize = 20;
    const BACKOFF: std::time::Duration = std::time::Duration::from_millis(150);
    let root = Path::new(key);
    for attempt in 1..=MAX_ATTEMPTS {
        match library.unregister_workspace(root) {
            // Ok(false) means it was already absent; both forms are
            // success for a Forget action.
            Ok(_) => return Ok(()),
            Err(e @ (ChanError::WorkspaceAlreadyOpen | ChanError::WorkspaceLocked)) => {
                if attempt == MAX_ATTEMPTS {
                    return Err(format!(
                        "workspace {key} is still shutting down ({e}); try Forget again in a moment"
                    ));
                }
                std::thread::sleep(BACKOFF);
            }
            Err(e) => return Err(format!("unregistering workspace {key}: {e}")),
        }
    }
    unreachable!("retry loop returns on the final attempt")
}

#[tauri::command]
async fn set_workspace_on(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    path: String,
    on: bool,
) -> Result<(), String> {
    let key = canonical_key(Path::new(&path));
    if on {
        serve::start(app, Arc::clone(&state), key).await?;
    } else {
        serve::stop(Some(&app), &state, &key);
    }
    Ok(())
}

#[tauri::command]
fn get_config(state: State<Arc<AppState>>) -> Result<Config, String> {
    state.store.lock().unwrap().get().map_err(err)
}

#[derive(Debug, Clone, Serialize)]
struct TunnelStatus {
    /// True while the tunnel listener is bound.
    listening: bool,
    /// Actual bound port (only populated while `listening`).
    port: Option<u16>,
    /// User's preferred port from desktop config. `0` means
    /// "let the OS assign one". UI uses this to populate the port
    /// input field.
    preferred_port: u16,
    /// Either the user's saved label or a freshly-suggested one if
    /// they've never typed anything. Suggestions avoid colliding
    /// with labels currently registered in the running tunnel:
    /// "tunnel" → "tunnel-1" → ... up to 999.
    preferred_label: String,
    /// User's saved workspace name or a default ("notes"). No
    /// collision check — workspace uniqueness is scoped per label, and
    /// the desktop doesn't track which labels are remotely
    /// preferred.
    preferred_workspace: String,
    /// Pre-formatted `ssh -R` reverse-forward snippet. `None` when
    /// the tunnel isn't listening (no port to reference yet).
    ssh_snippet: Option<String>,
    /// Pre-formatted `chan serve` command with the bound port,
    /// canonical TUNNEL_PATH, and the user's chosen label/workspace
    /// already substituted. Copy-paste ready.
    chan_serve_snippet: Option<String>,
}

/// Build the `ssh -R` and `chan serve` snippets that the listen
/// panel renders verbatim. Pre-formatting them here means JS does
/// zero templating — and the canonical URL path (with
/// `TUNNEL_PATH`) lives in exactly one place in the codebase.
fn build_snippets(port: u16, label: &str, workspace: &str) -> (String, String) {
    let ssh = format!("ssh -R {port}:localhost:{port} user@remote");
    // `--no-browser` keeps chan serve from launching the remote's
    // default browser at startup (it has nothing to point at — the
    // visitor URL belongs to chan-desktop, which is what auto-opens
    // the workspace webview on this side instead). `PATH` goes last so
    // the user only needs to edit one trailing argument.
    let chan = format!(
        "chan serve --tunnel-url=http://127.0.0.1:{port}{path} \
         --tunnel-token={label} --tunnel-workspace-name={workspace} --no-browser PATH",
        path = chan_tunnel_proto::TUNNEL_PATH,
    );
    (ssh, chan)
}

/// Pick a label suggestion: if the user has one saved, use it
/// verbatim. Otherwise try "tunnel"; if a remote is already
/// registered under that label, walk "tunnel-1", "tunnel-2", ...
/// until we find a free one. Falls back to `tunnel` at the end of
/// the range (uniqueness is best-effort; the registry's
/// last-writer-wins eviction is the real arbiter).
fn suggest_label(saved: &str, state: &AppState) -> String {
    if !saved.is_empty() {
        return saved.to_string();
    }
    let in_use: std::collections::HashSet<String> = state
        .tunnel
        .snapshot()
        .into_iter()
        .map(|d| d.label)
        .collect();
    let base = "tunnel";
    if !in_use.contains(base) {
        return base.to_string();
    }
    for i in 1..1000 {
        let candidate = format!("{base}-{i}");
        if !in_use.contains(&candidate) {
            return candidate;
        }
    }
    base.to_string()
}

fn suggest_workspace(saved: &str) -> String {
    if saved.is_empty() {
        "notes".to_string()
    } else {
        saved.to_string()
    }
}

#[tauri::command]
fn tunnel_status(state: State<Arc<AppState>>) -> Result<TunnelStatus, String> {
    let cfg = state.store.lock().unwrap().get().map_err(err)?.tunnel;
    let preferred_label = suggest_label(&cfg.preferred_label, &state);
    let preferred_workspace = suggest_workspace(&cfg.preferred_workspace);
    let port = state.tunnel.tunnel_port();
    let listening = state.tunnel.is_listening();
    let (ssh_snippet, chan_serve_snippet) = match (listening, port) {
        (true, Some(p)) => {
            let (s, c) = build_snippets(p, &preferred_label, &preferred_workspace);
            (Some(s), Some(c))
        }
        _ => (None, None),
    };
    Ok(TunnelStatus {
        listening,
        port,
        preferred_port: cfg.preferred_port,
        preferred_label,
        preferred_workspace,
        ssh_snippet,
        chan_serve_snippet,
    })
}

/// Start the tunnel listener with the user's chosen port, label,
/// and workspace. Validates `label` / `workspace` against the protocol's
/// charset rules so the rendered snippet matches what the wire
/// will actually accept. Persists all three for the next session.
#[tauri::command]
async fn tunnel_start(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    preferred_port: u16,
    label: String,
    workspace: String,
) -> Result<u16, String> {
    let label = label.trim().to_string();
    let workspace = workspace.trim().to_string();
    if !chan_tunnel_proto::is_valid_username(&label) {
        return Err(format!(
            "invalid label {label:?}: ASCII alphanumerics plus '-' / '_', \
             first char alphanumeric, ≤64 chars",
        ));
    }
    if !chan_tunnel_proto::is_valid_workspace_name(&workspace) {
        return Err(format!(
            "invalid workspace name {workspace:?}: lowercase ASCII alphanumerics plus '-', \
             first and last char alphanumeric, ≤32 chars",
        ));
    }
    {
        let mut store = state.store.lock().unwrap();
        let mut cfg = store.get().map_err(err)?;
        cfg.tunnel.preferred_port = preferred_port;
        cfg.tunnel.preferred_label = label;
        cfg.tunnel.preferred_workspace = workspace;
        store.save(&cfg).map_err(err)?;
    }
    let tunnel = Arc::clone(&state.tunnel);
    tunnel::start_listening(app, tunnel, preferred_port).await
}

#[tauri::command]
fn tunnel_stop(app: tauri::AppHandle, state: State<Arc<AppState>>) {
    tunnel::stop_listening(&app, &state.tunnel);
}

#[tauri::command]
fn default_workspace_status() -> Result<default_workspace::DefaultWorkspaceStatus, String> {
    default_workspace::status()
}

#[tauri::command]
fn choose_default_workspace(path: String) -> Result<(), String> {
    default_workspace::choose_existing(Path::new(&path)).map(|_| ())
}

#[tauri::command]
async fn create_default_workspace(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let created = default_workspace::create_default_workspace()?;
    reconcile_default_workspace(&state, &created.root)?;
    let key = canonical_key(&created.root);
    serve::start(app, Arc::clone(&state), key).await
}

#[tauri::command]
async fn factory_reset_default_workspace(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let created = default_workspace::factory_reset_default_workspace()?;
    reconcile_default_workspace(&state, &created.root)?;
    let key = canonical_key(&created.root);
    serve::start(app, Arc::clone(&state), key).await
}

/// `default_workspace` registers + seeds through its own throwaway
/// `Library` handle. Mirror that registration into the embedded
/// host's in-memory `Library` so the immediately-following
/// `serve::start` opens against an up-to-date registry rather than
/// the host's stale boot-time snapshot (the same staleness class as
/// the "workspace not registered" bug). `register_workspace` is idempotent
/// (touch + persist), so re-registering the row default_workspace just
/// wrote is safe, and `set_default_workspace_root` keeps the in-memory
/// default aligned with what default_workspace persisted.
fn reconcile_default_workspace(state: &AppState, root: &Path) -> Result<(), String> {
    let Some(embedded) = state.embedded.get() else {
        // No embedded host (e.g. it failed to start at boot);
        // default_workspace already persisted to disk, so a later serve
        // through a fresh handle still sees the row.
        return Ok(());
    };
    let library = embedded.library();
    library
        .register_workspace(root)
        .map_err(|e| format!("reconciling default workspace {}: {e}", root.display()))?;
    library
        .set_default_workspace_root(Some(root.to_path_buf()))
        .map_err(|e| format!("persisting default workspace root {}: {e}", root.display()))?;
    Ok(())
}

const OUTBOUND_LABEL_MAX_CHARS: usize = 120;

/// Persist an explicit outbound URL attachment and open it in a
/// workspace webview. The remote server owns its own lifecycle; desktop
/// only stores enough state to show and reopen the row.
#[tauri::command]
fn add_outbound_workspace(
    app: tauri::AppHandle,
    state: State<Arc<AppState>>,
    url: String,
    label: String,
) -> Result<String, String> {
    let url = normalize_outbound_url(&url)?;
    let label = normalize_outbound_label(&label)?;
    let (id, stored_url) = {
        let mut store = state.store.lock().unwrap();
        let mut cfg = store.get().map_err(err)?;
        let (id, stored_url) = match cfg.outbound.iter_mut().find(|d| d.url == url) {
            Some(existing) => {
                if !label.is_empty() {
                    existing.label = label.clone();
                }
                (existing.id.clone(), existing.url.clone())
            }
            None => {
                let entry = OutboundWorkspace {
                    id: uuid::Uuid::new_v4().to_string(),
                    url: url.clone(),
                    label,
                    added_at: config::current_millis(),
                };
                let id = entry.id.clone();
                cfg.outbound.push(entry);
                (id, url)
            }
        };
        store.save(&cfg).map_err(err)?;
        (id, stored_url)
    };
    serve::spawn_outbound_workspace_window(&app, &id, &stored_url)?;
    let _ = app.emit(serve::SERVES_CHANGED, ());
    Ok(id)
}

/// Open another webview for a stored outbound URL attachment.
#[tauri::command]
fn open_outbound_workspace(
    app: tauri::AppHandle,
    state: State<Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    let url = {
        let cfg = state.store.lock().unwrap().get().map_err(err)?;
        let outbound = cfg
            .outbound
            .iter()
            .find(|d| d.id == id)
            .ok_or_else(|| format!("no outbound workspace attachment {id}"))?;
        outbound.url.clone()
    };
    serve::spawn_outbound_workspace_window(&app, &id, &url)
}

/// Forget an outbound URL attachment. The remote server is not
/// stopped; only desktop config and open webviews for this
/// attachment are removed.
#[tauri::command]
fn remove_outbound_workspace(
    app: tauri::AppHandle,
    state: State<Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    {
        let mut store = state.store.lock().unwrap();
        let mut cfg = store.get().map_err(err)?;
        let before = cfg.outbound.len();
        cfg.outbound.retain(|d| d.id != id);
        if cfg.outbound.len() != before {
            store.save(&cfg).map_err(err)?;
        }
    }
    serve::close_outbound_workspace_windows(&app, &id);
    let _ = app.emit(serve::SERVES_CHANGED, ());
    Ok(())
}

fn normalize_outbound_url(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("remote URL is required".to_string());
    }
    let mut parsed =
        url::Url::parse(raw).map_err(|e| format!("invalid remote URL {raw:?}: {e}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("remote URL must use http:// or https://".to_string());
    }
    if parsed.host_str().is_none() {
        return Err("remote URL must include a host".to_string());
    }
    strip_query_param(&mut parsed, "w");
    Ok(parsed.to_string())
}

fn strip_query_param(parsed: &mut url::Url, name: &str) {
    if !parsed.query_pairs().any(|(key, _)| key == name) {
        return;
    }
    let pairs: Vec<(String, String)> = parsed
        .query_pairs()
        .filter(|(key, _)| key != name)
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    let mut query = parsed.query_pairs_mut();
    query.clear();
    for (key, value) in pairs {
        query.append_pair(&key, &value);
    }
}

fn normalize_outbound_label(raw: &str) -> Result<String, String> {
    let label = raw.trim().to_string();
    if label.chars().count() > OUTBOUND_LABEL_MAX_CHARS {
        return Err(format!(
            "remote label must be {OUTBOUND_LABEL_MAX_CHARS} characters or fewer",
        ));
    }
    Ok(label)
}

fn outbound_label(outbound: &OutboundWorkspace) -> Option<String> {
    let label = outbound.label.trim();
    if label.is_empty() {
        None
    } else {
        Some(label.to_string())
    }
}

/// Open an additional in-app Tauri webview for a running local
/// workspace. The first window is auto-opened by the serve supervisor
/// when chan prints its URL; subsequent clicks on Launch reach
/// here and add new windows alongside it. Errors if the workspace is
/// not currently running (no URL captured yet).
#[tauri::command]
fn open_local_workspace(
    app: tauri::AppHandle,
    state: State<Arc<AppState>>,
    path: String,
) -> Result<(), String> {
    let key = canonical_key(Path::new(&path));
    let url = state
        .serves
        .lock()
        .unwrap()
        .get(&key)
        .and_then(|h| h.url.clone())
        .ok_or_else(|| format!("workspace {key} is not running"))?;
    serve::spawn_local_workspace_window(&app, &key, &url)?;
    Ok(())
}

/// Open a workspace in a native window in response to a CLI handoff
/// request (`chan serve <workspace>` while this desktop is running).
///
/// Mirrors the `add_workspace` flow: register + boot the workspace through the
/// shared embedded Library, then `serve::start` (mount + spawn the
/// first window). If the workspace is ALREADY running, `serve::start`
/// returns early without spawning a window, so we raise an additional
/// window via `spawn_local_workspace_window` to match the user's intent
/// ("show me this workspace now").
///
/// The slow work (registry write, boot scan, mount) runs on a spawned
/// task so the callback returns promptly and the CLI doesn't block on
/// the handshake. The synchronous return therefore reports only that
/// the request was accepted, not that the window is fully up; on a
/// genuine mount failure the desktop emits a system notice (same as
/// the first-launch default-workspace path) rather than blocking the CLI.
#[cfg(unix)]
fn open_workspace_from_handoff(
    app: tauri::AppHandle,
    state: Arc<AppState>,
    path: PathBuf,
) -> Result<(), String> {
    let key = canonical_key(&path);

    // Already running: raise an additional window immediately. This is
    // synchronous and gives the user the window without a mount cycle.
    let running_url = state
        .serves
        .lock()
        .unwrap()
        .get(&key)
        .and_then(|h| h.url.clone());
    if let Some(url) = running_url {
        return serve::spawn_local_workspace_window(&app, &key, &url);
    }

    // Not running: register (creating the dir for a fresh path) + boot
    // through the shared Library, then mount + spawn the window. Off
    // the listener task so the CLI gets a prompt response.
    let Some(embedded) = state.embedded.get() else {
        return Err("embedded local server is unavailable".to_string());
    };
    let library = embedded.library().clone();
    let key_for_block = key.clone();
    tauri::async_runtime::spawn(async move {
        let library_for_register = library.clone();
        let key_for_register = key_for_block.clone();
        let registered = tokio::task::spawn_blocking(move || {
            register_and_boot(
                &library_for_register,
                &key_for_register,
                WorkspaceFeatures::default(),
            )
        })
        .await;
        match registered {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                emit_system_notice(
                    &app,
                    "warning",
                    format!("Could not open {key_for_block} from chan serve: {e}"),
                );
                return;
            }
            Err(e) => {
                emit_system_notice(
                    &app,
                    "warning",
                    format!("Opening {key_for_block} from chan serve panicked: {e}"),
                );
                return;
            }
        }
        if let Err(e) = serve::start(app.clone(), Arc::clone(&state), key_for_block.clone()).await {
            emit_system_notice(
                &app,
                "warning",
                format!("Could not open {key_for_block} from chan serve: {e}"),
            );
        }
    });
    Ok(())
}

/// Open an additional in-app Tauri webview for a tunneled workspace.
/// Each call yields a NEW window — the first one is opened by the
/// supervisor on registration, and the Launch button calls this
/// for subsequent windows. Errors if the per-tenant listener
/// hasn't bound yet (URL not formed).
#[tauri::command]
fn open_tunneled_workspace(
    app: tauri::AppHandle,
    state: State<Arc<AppState>>,
    label: String,
    workspace: String,
) -> Result<(), String> {
    let url = state
        .tunnel
        .snapshot()
        .into_iter()
        .find(|d| d.label == label && d.workspace == workspace)
        .map(|d| d.url)
        .ok_or_else(|| format!("no tunneled workspace {label}/{workspace}"))?;
    if url.is_empty() {
        return Err(format!(
            "tunneled workspace {label}/{workspace} has no URL yet; per-tenant listener still binding",
        ));
    }
    serve::spawn_tunneled_workspace_window(&app, &label, &workspace, &url)?;
    Ok(())
}

/// Result of a connecting-screen reachability probe. `reachable` is
/// true when the remote returned ANY HTTP response (even 401 / 404:
/// the server is up and serving). It is false only on a transport
/// failure (connection refused / DNS / TLS / timeout), which is exactly
/// the blank-white case the connecting screen retries past. `detail` is
/// a short ASCII reason shown in the per-attempt row; `status` is the
/// HTTP code when reachable.
#[derive(Debug, Clone, Serialize)]
struct ProbeResult {
    reachable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<u16>,
    detail: String,
}

/// Server-side cap so a black-hole host (packets dropped, no RST) can't
/// hang the probe and stack up overlapping in-flight requests behind the
/// page's retry loop.
const PROBE_TIMEOUT_SECS: u64 = 5;

/// Reachability probe for the chan-desktop connecting screen. Outbound
/// windows load `connecting.html` instead of pointing the webview
/// straight at the remote (a down remote paints a blank white webview);
/// that page calls this command on a retry loop until the remote answers,
/// then navigates. Runs from Rust because the page's CSP
/// (`default-src 'self'`) blocks a cross-origin `fetch`.
#[tauri::command]
async fn probe_url(url: String) -> ProbeResult {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(PROBE_TIMEOUT_SECS))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return ProbeResult {
                reachable: false,
                status: None,
                detail: format!("probe client error: {e}"),
            }
        }
    };
    match client.get(&url).send().await {
        Ok(resp) => ProbeResult {
            reachable: true,
            status: Some(resp.status().as_u16()),
            detail: resp.status().to_string(),
        },
        Err(e) => ProbeResult {
            reachable: false,
            status: None,
            detail: probe_error_detail(&e),
        },
    }
}

/// Collapse a reqwest error to the transport-failure class the
/// connecting screen's row cares about. reqwest's own Display is verbose
/// and embeds the full URL, so we surface a short ASCII label instead.
fn probe_error_detail(e: &reqwest::Error) -> String {
    if e.is_timeout() {
        "timed out".to_string()
    } else if e.is_connect() {
        "could not connect".to_string()
    } else if e.is_request() {
        "request failed".to_string()
    } else {
        "unreachable".to_string()
    }
}

/// Host OS the desktop shell is running on, as `std::env::consts::OS`
/// (`"macos"`, `"linux"`, `"windows"`, ...). The SPA branches features
/// that only exist on one platform; "Export to PDF" uses this to keep
/// the native WKWebView `createPDF` path on macOS and hide the button
/// elsewhere. Sourced from the compiled-in target triple rather than a
/// `navigator.userAgent` sniff so the answer is exact and cannot be
/// spoofed by a webview UA string.
#[tauri::command]
fn platform_os() -> String {
    std::env::consts::OS.to_string()
}

/// Clipboard text for the terminal's right-click "Paste". Read natively
/// via `arboard` rather than the webview's `navigator.clipboard.readText()`,
/// which pops WKWebView's DOM-paste "Paste" button (a WebKit privacy
/// affordance with no JS opt-out). Sync so it runs on the main thread,
/// which macOS's NSPasteboard expects. An empty / non-text clipboard maps
/// to "" so the SPA just treats it as nothing-to-paste; other failures
/// surface as an Err the SPA logs before falling back to the web API.
#[tauri::command]
fn read_clipboard_text() -> Result<String, String> {
    match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
        Ok(text) => Ok(text),
        Err(arboard::Error::ContentNotAvailable) => Ok(String::new()),
        Err(e) => Err(e.to_string()),
    }
}

/// User's home directory as a plain string, for the Workspaces window
/// to abbreviate paths to `~/...`. Returns an empty string when the
/// platform can't resolve it.
#[tauri::command]
fn home_dir() -> String {
    dirs::home_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default()
}

/// Open the given folder in the OS file manager. macOS: Finder,
/// Linux: default file manager, Windows: Explorer. Used by the
/// Workspaces window's path cell so users can jump to the workspace folder
/// from the row. Trusts the caller to pass a path the user just saw
/// in the list — paths come from `list_workspaces`, which sources from
/// the chan registry; no shell interpolation, args are passed as
/// argv to the OS open command.
#[tauri::command]
fn reveal_in_finder(path: String) -> Result<(), String> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };
    let status = std::process::Command::new(opener)
        .arg(&path)
        .status()
        .map_err(|e| format!("opening {path}: {e}"))?;
    if !status.success() {
        return Err(format!("opening {path}: {opener} exited with {status}"));
    }
    Ok(())
}

fn show_window(app: &tauri::AppHandle, label: &str) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(label) {
        w.show().map_err(err)?;
        w.set_focus().map_err(err)?;
    }
    Ok(())
}

/// Reload the calling webview window. Workspaces the SPA's tab
/// context-menu "Reload" entry (via `fullstack-a-36`) AND the
/// `Cmd+R` accelerator wired in `KEY_BRIDGE_JS`. The accelerator
/// path bypasses the SPA event bus and invokes this command
/// directly so a SPA-side fault (frozen Svelte runtime, JS error
/// in the chord handler) doesn't lock the dev affordance away.
#[tauri::command]
fn reload_window(window: tauri::WebviewWindow) -> Result<(), String> {
    // Tauri 2's `WebviewWindow::eval` runs JS inside the webview;
    // we use it instead of the missing-in-2 `reload()` method.
    window
        .eval("window.location.reload()")
        .map_err(|e| format!("reloading window: {e}"))
}

/// Open the DevTools inspector on the calling webview. Mirrors
/// the SPA's "Open Inspector" context-menu entry from `-a-36`
/// AND the `Cmd+Opt+I` accelerator in `KEY_BRIDGE_JS`. Requires
/// the `devtools` Cargo feature on the `tauri` crate (enabled in
/// `desktop/src-tauri/Cargo.toml`) so release builds carry the
/// inspector affordance, not just debug builds. Tauri 2 removed
/// the `app.devTools` JSON config key in favour of this
/// compile-time flag.
#[tauri::command]
fn open_devtools(window: tauri::WebviewWindow) {
    window.open_devtools();
}

/// `phase-12 lane-e` (addendum-2 Q6): close-cascade tail. The SPA
/// invokes this when the last tab and then the last empty pane of a
/// workspace window are closed: close the window, and — only if this
/// was the LAST chan SPA window — bring the launcher (the
/// native-desktop workspace list) back to the foreground so the user
/// isn't left with no window. The launcher's CloseRequested handler
/// hides rather than destroys it (see the setup hook), so re-showing
/// is instant.
///
/// When OTHER SPA windows remain we must NOT raise the launcher: a
/// cross-window terminal MOVE empties (and thus closes) the source
/// window, and unconditionally focusing the launcher there stole focus
/// from the drop-target window. Leaving the launcher alone lets the OS
/// keep focus on the frontmost remaining window — the window the user
/// just dropped the terminal into.
#[tauri::command]
fn request_close_window(app: tauri::AppHandle, window: tauri::WebviewWindow) -> Result<(), String> {
    let closing = window.label();
    let others_remain = app
        .webview_windows()
        .keys()
        .any(|label| label != closing && serve::is_workspace_webview_label(label));
    if !others_remain {
        let _ = show_window(&app, "main");
    }
    // `destroy()`, not `close()`: this is the SPA's DELIBERATE
    // close-cascade (last tab, then last pane, just closed — the window
    // is empty). `close()` would fire `CloseRequested`, where the
    // bury-on-close handler hides SPA windows instead of closing them;
    // an empty window is worthless buried. Destroy skips the request
    // phase and goes straight to the `Destroyed` cleanup.
    window.destroy().map_err(err)
}

/// `fullstack-b-19`: browser-style zoom controls. Step size is
/// 10 % per Cmd++/Cmd+- press; the clamp range matches Tauri's own
/// `zoom_hotkeys_enabled` polyfill semantics (0.25-5.0).
const ZOOM_STEP: f64 = 0.10;
const ZOOM_MIN: f64 = 0.25;
const ZOOM_MAX: f64 = 5.0;

/// Read the current zoom level for `label` from process state,
/// defaulting to 1.0 (chan-desktop's initial zoom). Pure read; the
/// IPC handlers compute the next level locally and write back.
fn current_zoom(state: &AppState, label: &str) -> f64 {
    state
        .live_window_zooms
        .lock()
        .unwrap()
        .get(label)
        .copied()
        .unwrap_or(1.0)
}

fn apply_zoom(window: &tauri::WebviewWindow, state: &AppState, next: f64) -> Result<(), String> {
    let clamped = next.clamp(ZOOM_MIN, ZOOM_MAX);
    window
        .set_zoom(clamped)
        .map_err(|e| format!("setting webview zoom on {}: {e}", window.label()))?;
    state
        .live_window_zooms
        .lock()
        .unwrap()
        .insert(window.label().to_string(), clamped);
    Ok(())
}

/// Zoom the calling webview one step up (Cmd++ / Ctrl++).
#[tauri::command]
fn zoom_in(window: tauri::WebviewWindow, state: State<Arc<AppState>>) -> Result<(), String> {
    let current = current_zoom(&state, window.label());
    apply_zoom(&window, &state, current + ZOOM_STEP)
}

/// Zoom the calling webview one step down (Cmd+- / Ctrl+-).
#[tauri::command]
fn zoom_out(window: tauri::WebviewWindow, state: State<Arc<AppState>>) -> Result<(), String> {
    let current = current_zoom(&state, window.label());
    apply_zoom(&window, &state, current - ZOOM_STEP)
}

/// Reset the calling webview to 100 % (Cmd+0 / Ctrl+0).
#[tauri::command]
fn zoom_reset(window: tauri::WebviewWindow, state: State<Arc<AppState>>) -> Result<(), String> {
    apply_zoom(&window, &state, 1.0)
}

/// Canonical-path key used for desktop config, serve identity, and
/// the displayed path. `canonicalize` falls back to the input on
/// error so we still produce a stable key for not-yet-existing or
/// asleep paths.
fn canonical_key(p: &Path) -> String {
    p.canonicalize()
        .unwrap_or_else(|_| PathBuf::from(p))
        .display()
        .to_string()
}

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn emit_chan_busy(app: &tauri::AppHandle, busy: bool, op: &str, path: &str) {
    let _ = app.emit(
        CHAN_BUSY_CHANGED,
        serde_json::json!({ "busy": busy, "op": op, "path": path }),
    );
}

fn emit_system_notice(app: &tauri::AppHandle, level: &str, message: impl Into<String>) {
    let _ = app.emit(
        SYSTEM_NOTICE,
        serde_json::json!({ "level": level, "message": message.into() }),
    );
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("CHAN_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn,chan_desktop=info")),
        )
        .with_writer(std::io::stderr)
        .init();
}

#[cfg(unix)]
fn run_hidden_mcp_proxy_if_requested() -> Result<bool, String> {
    let mut args = std::env::args_os();
    let _program = args.next();
    if args.next().as_deref() != Some(OsStr::new("__mcp-proxy")) {
        return Ok(false);
    }
    let socket = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "__mcp-proxy requires a socket path".to_string())?;
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("building MCP proxy runtime: {e}"))?;
    rt.block_on(run_mcp_proxy(socket))?;
    Ok(true)
}

#[cfg(not(unix))]
fn run_hidden_mcp_proxy_if_requested() -> Result<bool, String> {
    Ok(false)
}

/// When chan-desktop is invoked through a `cs` name (a `~/.local/bin/cs`
/// wrapper or symlink, argv[0] stem == "cs"), behave as the `cs` control
/// client and EXIT instead of launching the GUI. This is what lets desktop
/// users get `cs` (and the MCP discovery it carries) without a separate
/// `chan` binary on PATH. Mirrors `run_hidden_mcp_proxy_if_requested`: a
/// pre-GUI argv probe that short-circuits `main`. Returns `Ok(true)` when
/// it handled the invocation (caller returns), `Ok(false)` for a normal
/// GUI launch.
fn run_as_cs_if_requested() -> Result<bool, String> {
    let mut argv = std::env::args_os();
    let Some(arg0) = argv.next() else {
        return Ok(false);
    };
    if !chan_shell::invoked_as_cs(&arg0) {
        return Ok(false);
    }
    // The `cs` client is a single round-trip over the control socket, so a
    // current-thread runtime is enough (matches the `chan` binary's `cs`
    // path). clap parses + dispatches; it prints help/usage and exits on a
    // parse error, so a bad `cs` invocation never falls through to the GUI.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("building cs runtime: {e}"))?;
    let args = std::iter::once(arg0).chain(argv);
    rt.block_on(chan_shell::run_cs(args))
        .map_err(|e| format!("{e:#}"))?;
    Ok(true)
}

#[cfg(unix)]
async fn run_mcp_proxy(socket: PathBuf) -> Result<(), String> {
    chan_server::run_mcp_stdio_proxy(socket)
        .await
        .map_err(|e| format!("running MCP proxy: {e}"))
}

fn main() {
    match run_hidden_mcp_proxy_if_requested() {
        Ok(true) => return,
        Ok(false) => {}
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
    // `cs` alias dispatch (argv[0] stem == "cs"): run the control client
    // and exit, before any GUI / runtime / config setup below.
    match run_as_cs_if_requested() {
        Ok(true) => return,
        Ok(false) => {}
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
    // Linux AppImage only: prefer the host GTK/WebKit/EGL stack over the
    // bundled one and re-exec once before the webview is created, so it does
    // not abort with EGL_BAD_PARAMETER against a rolling-distro Mesa. No-op
    // off Linux/AppImage and once already applied.
    linux_gui_stack::prefer_system_gui_stack();
    init_tracing();
    // AppImage-only, best-effort: drop a `~/.local/bin/cs` wrapper so a
    // desktop-only Linux user gets the `cs` control client without a
    // separate `chan` binary. No-op off an AppImage; never fatal.
    match cs_install::install_appimage_cs_wrapper() {
        Ok(true) => tracing::info!("installed cs wrapper into ~/.local/bin"),
        Ok(false) => {}
        Err(e) => tracing::warn!(error = %e, "installing cs wrapper failed"),
    }
    let default_workspace_boot = match default_workspace::ensure_fresh_default_workspace() {
        Ok(created) => created,
        Err(e) => {
            tracing::warn!(error = %e, "first-launch default workspace setup failed");
            None
        }
    };
    let store = ConfigStore::new().expect("failed to init config store");
    let state = Arc::new(AppState {
        store: Mutex::new(store),
        serves: Mutex::new(HashMap::new()),
        embedded: OnceLock::new(),
        tunnel: TunnelState::new(),
        live_window_zooms: Mutex::new(HashMap::new()),
        window_numbers: Mutex::new(HashMap::new()),
        buried_windows: Mutex::new(Vec::new()),
        remote_reopen: Mutex::new(HashMap::new()),
    });
    let state_for_exit = Arc::clone(&state);
    let state_for_setup = Arc::clone(&state);

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(state)
        .setup(move |app| {
            install_app_menu(app.handle())?;

            match tauri::async_runtime::block_on(embedded::EmbeddedServer::start()) {
                Ok(server) => {
                    if state_for_setup.embedded.set(server).is_err() {
                        tracing::warn!("embedded local server initialized more than once");
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "embedded local server disabled");
                }
            }

            // Deep-link callbacks from the system browser
            // (`chan://auth/callback#...`). Cold-start URLs and
            // runtime URLs both flow through the same handler so the
            // sign-in completes whether the user clicked "Open with
            // chan-desktop" before or after the app was running.
            use tauri_plugin_deep_link::DeepLinkExt;
            let app_for_links = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    auth::handle_callback(&app_for_links, url.as_str());
                }
            });
            if let Ok(Some(urls)) = app.deep_link().get_current() {
                for url in urls {
                    auth::handle_callback(app.handle(), url.as_str());
                }
            }

            // Closing the main window via the red traffic light or
            // Cmd+W should hide it, not destroy it: hidden serve
            // children can still keep the process alive, and
            // reopening via Dock click or the Window > Workspaces menu
            // item should be instant. Without this, a closed main
            // window cannot be brought back without quitting and
            // relaunching.
            if let Some(main) = app.get_webview_window("main") {
                // Number the singleton launcher ("Chan Desktop Window 1") so
                // it disambiguates from extra `main-N` launchers in the OS
                // Window menu. It hides rather than destroys on close, so the
                // number is stable for the process lifetime.
                let _ = main.set_title(&launcher_window_title("main"));
                let main_for_event = main.clone();
                main.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = main_for_event.hide();
                    }
                });
                let _ = main.show();
                let _ = main.set_focus();
            }

            // Registry watcher. Leaked: we want it alive for the
            // process lifetime and the inner Watcher type is
            // unnameable through `manage`.
            match watcher::spawn(app.handle().clone(), &registry::path()) {
                Ok(d) => {
                    Box::leak(Box::new(d));
                }
                Err(e) => {
                    tracing::warn!(error = %e, "registry watcher disabled");
                    emit_system_notice(
                        app.handle(),
                        "warning",
                        "Auto-refresh disabled; close and reopen the window after running chan add.",
                    );
                }
            }

            // Tunnel listener is OFF until the user explicitly
            // clicks "Attach" in the Workspaces window. We just
            // construct the empty TunnelState during boot; binding
            // 127.0.0.1 happens on the IPC `tunnel_start` call.
            let _ = state_for_setup.tunnel.clone();

            // macOS CLI-to-desktop handoff listener (ratified Option
            // B). Binds the well-known per-user UDS so a `chan serve
            // <workspace>` in a terminal hands the workspace to this desktop
            // window instead of failing on the per-workspace flock. Leaked
            // for the process lifetime (the registry watcher above uses
            // the same Box::leak pattern; the handle's Drop unlinks the
            // socket but we want it live until exit, and RunEvent::Exit
            // tears the process down anyway). A bind failure is
            // non-fatal: the CLI just falls back to its own server.
            #[cfg(unix)]
            if let Some(sock) = chan_server::handoff::well_known_socket_path() {
                let app_for_handoff = app.handle().clone();
                let state_for_handoff = Arc::clone(&state_for_setup);
                // `start_listener` binds a tokio `UnixListener` and
                // `tokio::spawn`s the accept loop, so it MUST run inside
                // a tokio runtime context. The Tauri `setup` closure runs
                // on the main thread OUTSIDE any runtime, so calling it
                // directly panics ("there is no reactor running"), which
                // aborts the whole desktop on launch. Enter the Tauri-
                // managed runtime via `block_on` (the same runtime the
                // embedded server above and every `async_runtime::spawn`
                // below use) so the bind + the spawned accept loop attach
                // to it and survive after this returns.
                let listener = tauri::async_runtime::block_on(async {
                    chan_server::handoff::start_listener(sock, move |path| {
                        open_workspace_from_handoff(
                            app_for_handoff.clone(),
                            Arc::clone(&state_for_handoff),
                            path,
                        )
                    })
                });
                match listener {
                    Ok(handle) => {
                        Box::leak(Box::new(handle));
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "CLI-to-desktop handoff listener disabled");
                    }
                }
            }

            if let Some(created) = default_workspace_boot.clone() {
                let app_for_default = app.handle().clone();
                let state_for_default = Arc::clone(&state_for_setup);
                tauri::async_runtime::spawn(async move {
                    let key = canonical_key(&created.root);
                    if let Err(e) =
                        serve::start(app_for_default.clone(), state_for_default, key).await
                    {
                        tracing::warn!(
                            root = %created.root.display(),
                            error = %e,
                            "starting first-launch default workspace failed",
                        );
                        emit_system_notice(
                            &app_for_default,
                            "warning",
                            format!(
                                "Created the default Chan workspace at {}, but opening it failed: {e}",
                                created.root.display(),
                            ),
                        );
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_workspaces,
            add_workspace,
            remove_workspace,
            set_workspace_on,
            get_config,
            home_dir,
            platform_os,
            read_clipboard_text,
            reveal_in_finder,
            reload_window,
            open_devtools,
            request_close_window,
            download::save_file_to_downloads,
            // Native vector PDF export. macOS-only: WKWebView's `createPDF`
            // has no Linux/Windows equivalent wired, and the SPA hides the
            // "Export to PDF" button off-macOS so this is never invoked
            // there.
            #[cfg(target_os = "macos")]
            pdf::export_pdf_macos,
            zoom_in,
            zoom_out,
            zoom_reset,
            tunnel_status,
            tunnel_start,
            tunnel_stop,
            default_workspace_status,
            choose_default_workspace,
            create_default_workspace,
            factory_reset_default_workspace,
            open_local_workspace,
            open_tunneled_workspace,
            probe_url,
            add_outbound_workspace,
            open_outbound_workspace,
            remove_outbound_workspace,
            auth::auth_status,
            auth::open_signin,
            auth::signout,
        ])
        .build(tauri::generate_context!())
        .expect("error building tauri application");

    app.run(move |_app, event| {
        match event {
            RunEvent::Exit => {
                // Best-effort: unmount every embedded local workspace
                // before the desktop runtime exits.
                serve::stop_all(&state_for_exit);
                // Cancel the tunnel listener (if active) and every
                // per-tenant listener. Tasks exit when their cancel
                // token fires; the process is on its way out, so we
                // don't await them.
                tunnel::shutdown(&state_for_exit.tunnel);
            }
            // macOS: Dock click or `open -a` while the process is
            // still alive. If no windows are visible (main has been
            // hidden / closed and the user has no workspace windows
            // open), bring the main window back.
            #[cfg(target_os = "macos")]
            RunEvent::Reopen {
                has_visible_windows: false,
                ..
            } => {
                let _ = show_window(_app, "main");
            }
            _ => {}
        }
    });
}

/// Build and install the application menu.
///
/// The Window submenu carries Workspaces / New Window so a closed main
/// window stays reachable by name. There is no Settings menu item: Cmd+,
/// is the SPA's Hybrid-flip chord (`app.settings.toggle`), bound by the
/// SPA itself, so no menu accelerator may claim Comma or the keydown
/// never reaches the webview.
///
/// macOS starts from Tauri's `Menu::default` (the system menubar already
/// carries the App menu's About / Quit). Off macOS `Menu::default` has no
/// File menu - Linux shows only Edit/Window/Help - so the bar is built
/// explicitly: File (About, Exit), Edit, Window; no Help.
fn install_app_menu(app: &tauri::AppHandle) -> tauri::Result<()> {
    // Window-navigation items shared by both menu shapes.
    //
    // Workspaces keeps no accelerator: Cmd+1..9 is reserved for
    // jump-to-tab in workspace windows (handled by the per-workspace key
    // bridge script in serve.rs). The menu entry still surfaces the
    // window by name.
    let workspace_manager = MenuItemBuilder::with_id("win-main", "Workspaces").build(app)?;
    // `fullstack-83`: Cmd+N spawns a fresh launcher window. The
    // existing "main" window stays untouched (singleton label);
    // additional launchers land on `main-<N>` so each carries its
    // own state independently. Convention for future chan-desktop
    // shortcuts: declare a MenuItemBuilder here with the
    // `CmdOrCtrl+<key>` accelerator, add it to the Window submenu, and
    // add a matching `on_menu_event` branch.
    // `fullstack-b-27`: moved from `CmdOrCtrl+N` to
    // `CmdOrCtrl+Shift+N` so the SPA's New Draft handler (per
    // `fullstack-a-66`) can claim plain Cmd+N without the menu
    // accelerator intercepting first. Menu label stays "New Window".
    // `phase-13 r2` (B-slice 3): the handler now opens a new window of
    // the FOCUSED window's workspace (open_new_window_for_focused_workspace)
    // instead of the workspace picker; the picker stays on the
    // "Workspaces" (win-main) item.
    let new_window = MenuItemBuilder::with_id("app-new-window", "New Window")
        .accelerator("CmdOrCtrl+Shift+N")
        .build(app)?;
    // `phase-20`: File ▸ New Terminal. Cmd+T, ALWAYS enabled on both
    // platforms (no dynamic enable/disable: a disabled menu item still
    // swallows the accelerator on macOS, so a launcher-focused Cmd+T would
    // dead-end). The single handler routes by the FOCUSED window's kind: a
    // launcher (main / main-*) opens a new standalone terminal window; any
    // embedded SPA window (workspace-* / tunnel-* / outbound-* / terminal-*)
    // gets `app.terminal.toggle` dispatched, which the SPA interprets per
    // its mode (workspace: toggle a pane terminal; terminal: add a tab).
    let new_terminal = MenuItemBuilder::with_id("app-new-terminal", "New Terminal")
        .accelerator("CmdOrCtrl+T")
        .build(app)?;
    // macOS: inject the window-nav items into the system menubar's Window
    // submenu. The App menu already owns About <app> and Quit, so File ▸
    // About / Exit are macOS-implicit.
    #[cfg(target_os = "macos")]
    let menu = {
        let menu = Menu::default(app)?;
        // Strip muda's predefined "Close Window" from a submenu. We replace it
        // with our own Cmd+W-bound item (see `close_window` below) so a single
        // accelerator can route by the focused window's kind; leaving the
        // predefined one would either double-bind Cmd+W or natively close the
        // window unconditionally. Match by text since muda assigns predefined
        // items an opaque generated id.
        let strip_close = |submenu: &tauri::menu::Submenu<tauri::Wry>| {
            if let Ok(items) = submenu.items() {
                for item in items {
                    if let MenuItemKind::Predefined(p) = &item {
                        if let Ok(text) = p.text() {
                            if text.to_lowercase().contains("close") {
                                let _ = submenu.remove(&item);
                            }
                        }
                    }
                }
            }
        };
        // `phase-21 fix`: File ▸ Close Window. A CUSTOM item (not the predefined
        // close_window) carrying Cmd+W, routed by `handle_close_window`: a
        // focused workspace webview closes the active TAB (dispatching the same
        // `app.tab.close` the KEY_BRIDGE_JS KeyW case fires), while the launcher
        // (`main`) and other plain windows close natively. The accelerator
        // pre-empts the webview on macOS, so the KEY_BRIDGE_JS KeyW case is
        // harmlessly shadowed here (same arrangement as New Terminal's Cmd+T).
        let close_window = MenuItemBuilder::with_id("app-close-window", "Close Window")
            .accelerator("CmdOrCtrl+W")
            .build(app)?;
        // `phase-20` / `phase-21 fix`: macOS `Menu::default` ALREADY ships a
        // File submenu (carrying the predefined Close Window) alongside App /
        // Edit / View / Window / Help. Reuse it rather than inserting a second
        // one (which produced a duplicate "File" menu): strip the predefined
        // Close Window, then rebuild File as New Terminal, a separator, and our
        // routed Close Window. Match the submenu by title.
        if let Some(file_submenu) = menu.items().ok().and_then(|items| {
            items.into_iter().find_map(|k| {
                k.as_submenu()
                    .filter(|sm| sm.text().ok().as_deref() == Some("File"))
                    .cloned()
            })
        }) {
            strip_close(&file_submenu);
            let sep = PredefinedMenuItem::separator(app)?;
            file_submenu.prepend_items(&[&new_terminal, &sep, &close_window])?;
        }
        if let Some(window_submenu) = menu
            .get(WINDOW_SUBMENU_ID)
            .and_then(|k| k.as_submenu().cloned())
        {
            let sep = PredefinedMenuItem::separator(app)?;
            window_submenu.prepend_items(&[&workspace_manager, &new_window, &sep])?;
            // Drop the Window submenu's own Close Window so Cmd+W is owned
            // solely by File's routed item above (no double accelerator).
            strip_close(&window_submenu);
        }
        // Redirect the system "About Chan" item to our bundled About window
        // so macOS shows the same About content as Linux/Windows (the
        // Dashboard About slide). The App menu is the first submenu in the
        // default macOS menubar: prepend a custom (non-predefined) About
        // item routed to `chan-about`, then strip the predefined system
        // About. The Predefined-only match below leaves our custom item.
        if let Some(app_submenu) = menu
            .items()
            .ok()
            .and_then(|items| items.into_iter().next())
            .and_then(|k| k.as_submenu().cloned())
        {
            let about = MenuItemBuilder::with_id("chan-about", "About Chan").build(app)?;
            app_submenu.prepend_items(&[&about])?;
            if let Ok(items) = app_submenu.items() {
                for item in items {
                    if let MenuItemKind::Predefined(p) = &item {
                        if let Ok(text) = p.text() {
                            if text.to_lowercase().contains("about") {
                                let _ = app_submenu.remove(&item);
                            }
                        }
                    }
                }
            }
        }
        menu
    };

    // Linux / Windows: build the bar by hand. "About Chan" opens a version
    // dialog that also offers a manual update check - the only manual
    // self-update entry point off macOS (the launcher window otherwise
    // auto-checks once per launch). No Help submenu.
    //
    // Quit is a CUSTOM item, not PredefinedMenuItem::quit: muda has no GTK
    // handler for the predefined Quit (it is wired only on macOS / Windows),
    // so on Linux the predefined item is silently dropped and File showed no
    // Exit at all. A custom item with an explicit app.exit(0) handler renders
    // and works. Undo/Redo are likewise GTK-unsupported (dropped, and they
    // would orphan a leading separator), so Edit sticks to the four clipboard
    // items muda does implement on GTK.
    #[cfg(not(target_os = "macos"))]
    let menu = {
        use tauri::menu::{MenuBuilder, SubmenuBuilder};
        let about = MenuItemBuilder::with_id("chan-about", "About Chan").build(app)?;
        let quit = MenuItemBuilder::with_id("chan-quit", "Quit")
            .accelerator("CmdOrCtrl+Q")
            .build(app)?;
        let file = SubmenuBuilder::new(app, "File")
            .item(&new_terminal)
            .separator()
            .item(&about)
            .separator()
            .item(&quit)
            .build()?;
        let edit = SubmenuBuilder::new(app, "Edit")
            .cut()
            .copy()
            .paste()
            .select_all()
            .build()?;
        let window = SubmenuBuilder::with_id(app, LINUX_WINDOW_SUBMENU_ID, "Window")
            .item(&workspace_manager)
            .item(&new_window)
            .build()?;
        MenuBuilder::new(app)
            .item(&file)
            .item(&edit)
            .item(&window)
            .build()?
    };

    app.set_menu(menu)?;
    app.on_menu_event(|app, event| {
        let id = event.id().as_ref();
        // Dynamic Window-menu entries (buried windows) carry their
        // window label in the id; route by prefix before the static
        // match.
        if let Some(label) = id.strip_prefix(BURIED_MENU_ID_PREFIX) {
            if !unbury_window(app, label) {
                tracing::warn!(label, "buried window menu entry pointed at a dead window");
            }
            return;
        }
        if let Some(label) = id.strip_prefix(REMOTE_MENU_ID_PREFIX) {
            open_remote_window_from_menu(app, label);
            return;
        }
        match id {
            "win-main" => {
                let _ = show_window(app, "main");
            }
            "app-new-window" => {
                if let Err(e) = open_new_window_for_focused_workspace(app) {
                    tracing::warn!(error = %e, "open new window for focused workspace failed");
                }
            }
            "app-new-terminal" => {
                handle_new_terminal(app);
            }
            #[cfg(target_os = "macos")]
            "app-close-window" => {
                handle_close_window(app);
            }
            "chan-about" => {
                if let Err(e) = open_about_window(app) {
                    tracing::warn!(error = %e, "open about window failed");
                }
            }
            #[cfg(not(target_os = "macos"))]
            "chan-quit" => {
                app.exit(0);
            }
            _ => {}
        }
    });
    Ok(())
}

/// Window-menu item id namespace for buried-window entries: the id is
/// this prefix + the Tauri window label, so the menu handler recovers
/// the label with a `strip_prefix`. The constant doubles as the marker
/// `rebuild_window_menu` uses to find (and replace) its own entries.
const BURIED_MENU_ID_PREFIX: &str = "buried:";
/// Disabled section header above the buried entries.
const BURIED_MENU_HEADER_ID: &str = "buried-header";
/// Window-menu id namespace for reopenable remote windows (same
/// prefix+label scheme as `buried:`).
const REMOTE_MENU_ID_PREFIX: &str = "remote:";
/// Disabled section header above the remote entries.
const REMOTE_MENU_HEADER_ID: &str = "remote-header";
/// Linux/Windows Window-submenu id (macOS uses the system
/// `WINDOW_SUBMENU_ID` from `Menu::default`).
#[cfg(not(target_os = "macos"))]
const LINUX_WINDOW_SUBMENU_ID: &str = "chan-window-submenu";

/// The app menubar's Window submenu, on any platform. `None` before
/// `install_app_menu` ran (impossible in practice) or if the platform
/// menu lost it.
fn window_submenu(app: &tauri::AppHandle) -> Option<Submenu<tauri::Wry>> {
    let menu = app.menu()?;
    #[cfg(target_os = "macos")]
    let key = WINDOW_SUBMENU_ID;
    #[cfg(not(target_os = "macos"))]
    let key = LINUX_WINDOW_SUBMENU_ID;
    menu.get(key).and_then(|k| k.as_submenu().cloned())
}

/// Re-sync the Window submenu's dynamic tail: remove every
/// previously-appended `buried:*` / `remote:*` entry (and the section
/// headers), then append the current snapshots — buried windows most
/// recent first, then reopenable remote windows sorted by title. Runs
/// on the main thread — muda requires menu mutation there on macOS —
/// and is best-effort throughout: a menu glitch must never take down a
/// close/destroy handler.
pub fn rebuild_window_menu(app: &tauri::AppHandle) {
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        let Some(submenu) = window_submenu(&app) else {
            return;
        };
        if let Ok(items) = submenu.items() {
            for item in items {
                let id = item.id().as_ref();
                if id == BURIED_MENU_HEADER_ID
                    || id == REMOTE_MENU_HEADER_ID
                    || id.starts_with(BURIED_MENU_ID_PREFIX)
                    || id.starts_with(REMOTE_MENU_ID_PREFIX)
                {
                    let _ = submenu.remove(&item);
                }
            }
        }
        let state = app.state::<Arc<AppState>>();
        let buried = state.buried_snapshot();
        let mut remote: Vec<(String, String)> = state
            .remote_reopen
            .lock()
            .unwrap()
            .iter()
            .map(|(label, entry)| (label.clone(), entry.menu_title.clone()))
            .collect();
        remote.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

        let append_section = |header_id: &str, header: &str, rows: &[(String, String)], id_prefix: &str| {
            if rows.is_empty() {
                return;
            }
            if let Ok(item) = MenuItemBuilder::with_id(header_id, header)
                .enabled(false)
                .build(&app)
            {
                let _ = submenu.append(&item);
            }
            for (label, title) in rows {
                match MenuItemBuilder::with_id(format!("{id_prefix}{label}"), title).build(&app) {
                    Ok(item) => {
                        let _ = submenu.append(&item);
                    }
                    Err(e) => {
                        tracing::warn!(label, error = %e, "building dynamic window menu item failed");
                    }
                }
            }
        };
        append_section(
            BURIED_MENU_HEADER_ID,
            "Hidden Windows",
            &buried,
            BURIED_MENU_ID_PREFIX,
        );
        append_section(
            REMOTE_MENU_HEADER_ID,
            "Remote Windows",
            &remote,
            REMOTE_MENU_ID_PREFIX,
        );
    });
}

/// Re-poll every remote connection's `GET /api/windows` and replace the
/// reopenable-remote-windows snapshot (then rebuild the menu). Spawned
/// async: each remote gets a short timeout and a failed poll just
/// leaves that connection out this round. Triggers: a tunnel/outbound
/// window opening or being destroyed, and a `remote:` menu click.
/// Tauri 2 exposes no menu-will-open hook, so event-driven refresh
/// with tolerable staleness is the design.
pub fn refresh_remote_windows_menu(app: &tauri::AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<Arc<AppState>>();

        /// One remote connection to poll.
        struct Conn {
            family: String,
            url: String,
            base_title: String,
            config_key: String,
            connecting: bool,
        }
        let mut conns: Vec<Conn> = Vec::new();
        let cfg = {
            let store = state.store.lock().unwrap();
            store.get().ok()
        };
        if let Some(cfg) = cfg {
            for o in &cfg.outbound {
                conns.push(Conn {
                    family: format!("{}-", serve::outbound_window_prefix(&o.id)),
                    url: o.url.clone(),
                    base_title: serve::outbound_window_title(&o.url),
                    config_key: config::outbound_window_key(&o.id),
                    connecting: true,
                });
            }
        }
        for t in state.tunnel.snapshot() {
            conns.push(Conn {
                family: format!("{}-", serve::tunnel_window_prefix(&t.label, &t.workspace)),
                url: t.url.clone(),
                base_title: serve::tunnel_window_title(&t.url),
                config_key: config::tunnel_window_key(&t.label, &t.workspace),
                connecting: false,
            });
        }

        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()
        {
            Ok(client) => client,
            Err(e) => {
                tracing::warn!(error = %e, "remote windows poll: building http client failed");
                return;
            }
        };
        let mut map: HashMap<String, RemoteReopen> = HashMap::new();
        for conn in conns {
            let rows = match fetch_remote_windows(&client, &conn.url).await {
                Some(rows) => rows,
                None => continue, // remote down / unparsable; skip this round
            };
            for row in rows {
                // Reopenable = the remote has restore state for the label
                // and no live socket holds it anywhere, and the label
                // belongs to THIS connection (filters out browser-session
                // ids and other desktops' families).
                if !(row.saved && !row.connected && row.id.starts_with(&conn.family)) {
                    continue;
                }
                map.insert(
                    row.id.clone(),
                    RemoteReopen {
                        url: conn.url.clone(),
                        base_title: conn.base_title.clone(),
                        menu_title: format!(
                            "{} — {}",
                            conn.base_title,
                            remote_window_tail(&row.id)
                        ),
                        config_key: conn.config_key.clone(),
                        connecting: conn.connecting,
                    },
                );
            }
        }
        *state.remote_reopen.lock().unwrap() = map;
        rebuild_window_menu(&app);
    });
}

/// Row shape of the remote `GET /api/windows` response. Field names are
/// the wire contract pinned server-side
/// (`routes::windows::WindowInfo`).
#[derive(serde::Deserialize)]
struct RemoteWindowRow {
    id: String,
    connected: bool,
    saved: bool,
}

/// GET `<base>/api/windows` preserving the base URL's query (`?t=`
/// token rides there for outbound attachments). `None` on any failure
/// — the caller skips that connection for this refresh round.
async fn fetch_remote_windows(
    client: &reqwest::Client,
    base: &str,
) -> Option<Vec<RemoteWindowRow>> {
    let base = tauri::Url::parse(base).ok()?;
    let mut api = base.clone();
    let mut path = base.path().to_string();
    if !path.ends_with('/') {
        path.push('/');
    }
    api.set_path(&format!("{path}api/windows"));
    api.set_fragment(None);
    let resp = client.get(api.as_str()).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.json::<Vec<RemoteWindowRow>>().await.ok()
}

/// Human tail for a remote window label in the menu:
/// `outbound-<16hex>-7` -> "window 7". Falls back to the raw label for
/// anything unexpected.
fn remote_window_tail(label: &str) -> String {
    match label.rsplit('-').next().and_then(|n| n.parse::<u64>().ok()) {
        Some(seq) => format!("window {seq}"),
        None => label.to_string(),
    }
}

/// `remote:` menu click: open a webview for the remote-known label.
/// On success the label becomes `connected` remote-side, so a refresh
/// drops it from the menu.
fn open_remote_window_from_menu(app: &tauri::AppHandle, label: &str) {
    let entry = {
        let state = app.state::<Arc<AppState>>();
        let map = state.remote_reopen.lock().unwrap();
        map.get(label).cloned()
    };
    let Some(entry) = entry else {
        tracing::warn!(label, "remote window menu entry has no stored connection");
        return;
    };
    if let Err(e) = serve::reopen_remote_window(app, label, &entry) {
        tracing::warn!(label, error = %e, "reopening remote window failed");
    }
}

/// Re-show a buried window and drop it from the registry + menu.
/// Returns `false` when the label no longer names a live window (it
/// was destroyed underneath; the registry entry is cleaned up either
/// way).
pub fn unbury_window(app: &tauri::AppHandle, label: &str) -> bool {
    let removed = app.state::<Arc<AppState>>().remove_buried(label);
    let shown = match app.get_webview_window(label) {
        Some(w) => {
            let _ = w.show();
            let _ = w.set_focus();
            true
        }
        None => false,
    };
    if removed {
        rebuild_window_menu(app);
    }
    shown
}

/// Open the bundled About window. Same content on every platform (mirrors
/// the SPA Dashboard About slide: version, license, links, donation QR, and
/// third-party attributions), replacing both the macOS system About panel
/// and the old Linux/Windows version dialog with its manual update check.
/// Singleton: focus an existing About window instead of stacking copies.
/// The desktop version is passed as a query param so `about.html` needs no
/// `app`-plugin capability just to render it.
fn open_about_window(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("about") {
        let _ = win.set_focus();
        return Ok(());
    }
    let version = app.package_info().version.to_string();
    WebviewWindowBuilder::new(
        app,
        "about",
        WebviewUrl::App(format!("about.html?v={version}").into()),
    )
    .title("About Chan Desktop")
    // Sized to fit the content with equal top/bottom margin: app head,
    // links, the Fund-the-work card, separator, and the credits line.
    // Fixed, since the window is non-resizable.
    .inner_size(420.0, 426.0)
    .min_inner_size(420.0, 380.0)
    .resizable(false)
    .build()
    .map_err(|e| format!("building about window: {e}"))?;
    Ok(())
}

/// `fullstack-83`: spawn a fresh launcher (workspace-picker) window via
/// `WebviewWindowBuilder`. The label is picked from the next free
/// `main-N` slot so each launcher carries its own per-window state
/// (mirrors the `workspace-N` / `tunnel-N` convention). New windows use
/// the same `index.html` entry as the singleton `main`, so the
/// SPA's `boot()` path runs and the user lands on the workspace
/// picker — never inheriting any existing launcher's runtime
/// state.
fn open_new_launcher_window(app: &tauri::AppHandle) -> Result<(), String> {
    let label = next_launcher_label(app);
    if app.get_webview_window(&label).is_some() {
        // Defensive: the slot picker scans existing windows so a
        // collision shouldn't happen. If it ever does, surface a
        // clear error rather than panicking on `build`.
        return Err(format!("launcher label {label} already exists"));
    }
    WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
        .title(launcher_window_title(&label))
        .inner_size(960.0, 600.0)
        .min_inner_size(720.0, 400.0)
        .resizable(true)
        .build()
        .map_err(|e| format!("building launcher window {label}: {e}"))?;
    Ok(())
}

/// `phase-13 r2` (B-slice 3): open a new window of the workspace that
/// owns the currently focused window. Replaces the old Cmd+Shift+N
/// behaviour (which always opened the workspace-picker launcher) per
/// @@Alex: "open a new window of the currently open workspace".
///
/// Window labels are `workspace-<hash(key)>-<seq>` and the hash is
/// one-way, so we recover the workspace key by matching
/// `serve::workspace_window_prefix(key)` against the focused window's
/// label across the running `serves` map, then reuse the same
/// `spawn_local_workspace_window` path `open_local_workspace` uses.
///
/// Falls back to the launcher picker when no LOCAL `workspace-*` window
/// is focused (the launcher itself, a `tunnel-*` / `outbound-*` window,
/// or no running match), so the menu item never dead-ends. The
/// "Workspaces" picker stays reachable via the `win-main` menu item.
fn open_new_window_for_focused_workspace(app: &tauri::AppHandle) -> Result<(), String> {
    // Buried windows take precedence in every family: Cmd+Shift+N on a
    // window whose family has a hidden sibling REOPENS that sibling
    // (most recent first) instead of spawning a fresh window — the
    // hide-on-close counterpart of the old "reopens the last closed
    // window" LRU behaviour, now with the live window state intact.
    //
    // `phase-20`: a focused standalone terminal window opens ANOTHER
    // terminal window (its workspace-less analogue of "new window of this
    // workspace"), not the launcher. Checked first because a terminal-*
    // window has no entry in the `serves` map for the workspace-recovery
    // path below to match.
    if app
        .webview_windows()
        .values()
        .any(|w| w.label().starts_with("terminal-") && w.is_focused().unwrap_or(false))
    {
        let state = app.state::<Arc<AppState>>();
        if let Some(buried) = state.most_recent_buried("terminal-win-") {
            if unbury_window(app, &buried) {
                return Ok(());
            }
        }
        spawn_terminal_window(app);
        return Ok(());
    }
    let Some(focused) = app
        .webview_windows()
        .into_values()
        .find(|w| serve::is_workspace_webview_label(w.label()) && w.is_focused().unwrap_or(false))
    else {
        return open_new_launcher_window(app);
    };
    let focused_label = focused.label().to_string();
    let state = app.state::<Arc<AppState>>();
    // Family unbury first: workspace-, tunnel- and outbound- windows all
    // group by their `<kind>-<16hex>-` label prefix.
    if let Some(buried) = state.most_recent_buried(window_family_prefix(&focused_label)) {
        if unbury_window(app, &buried) {
            return Ok(());
        }
    }
    if !focused_label.starts_with("workspace-") {
        // No spawn path recovers a tunnel/outbound identity from its
        // label alone; with nothing buried, fall back to the launcher
        // (the pre-bury behaviour for these windows).
        return open_new_launcher_window(app);
    }
    let resolved = {
        let serves = state.serves.lock().unwrap();
        serves.iter().find_map(|(key, handle)| {
            let prefix = serve::workspace_window_prefix(key);
            if focused_label.starts_with(&format!("{prefix}-")) {
                handle.url.clone().map(|url| (key.clone(), url))
            } else {
                None
            }
        })
    };
    match resolved {
        Some((key, url)) => serve::spawn_local_workspace_window(app, &key, &url),
        None => open_new_launcher_window(app),
    }
}

/// Display number for a launcher window, derived from its label:
/// the singleton `main` is window 1; `main-N` is window N. Since
/// `next_launcher_label` hands out the lowest-free `main-N` slot, a
/// number freed by a closed launcher is already reused — so the
/// label IS the reusable display number (no separate allocator
/// needed; launchers never go through `build_workspace_window`).
fn launcher_window_number(label: &str) -> u32 {
    if label == "main" {
        return 1;
    }
    label
        .strip_prefix("main-")
        .and_then(|n| n.parse::<u32>().ok())
        .unwrap_or(1)
}

/// OS window title for a launcher window: `Chan Desktop Window N`, so
/// the macOS Window menu disambiguates multiple open launchers
/// (mirrors the `"{title} Window {N}"` scheme the workspace / terminal
/// windows use).
fn launcher_window_title(label: &str) -> String {
    format!("Chan Desktop Window {}", launcher_window_number(label))
}

/// Pick the next free `main-N` label. Launchers spawn from the
/// File → New Window menu item; the singleton `main` from
/// tauri.conf.json keeps its bare label so existing
/// `show_window(app, "main")` callers and the `Workspaces` menu
/// entry keep working.
fn next_launcher_label(app: &tauri::AppHandle) -> String {
    let existing: std::collections::HashSet<String> = app
        .webview_windows()
        .into_keys()
        .filter(|l| l == "main" || l.starts_with("main-"))
        .collect();
    for n in 2u32..u32::MAX {
        let candidate = format!("main-{n}");
        if !existing.contains(&candidate) {
            return candidate;
        }
    }
    // Practically unreachable; falls back to a UUID-ish suffix so
    // the menu action still does *something* if a hostile loop
    // saturates the integer range.
    format!(
        "main-{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    )
}

/// Eval a `chan:command` dispatch on the currently-focused workspace
/// webview. Used by menu items that should defer to chan's per-workspace
/// behavior (Settings). No-op when the focused window isn't a workspace,
/// matching the "each window owns its own settings" model.
fn dispatch_to_focused_workspace(app: &tauri::AppHandle, command: &str) {
    let Some(w) = app
        .webview_windows()
        .into_values()
        .find(|w| serve::is_workspace_webview_label(w.label()) && w.is_focused().unwrap_or(false))
    else {
        return;
    };
    let js = format!(
        "window.dispatchEvent(new CustomEvent('chan:command', {{detail: {{name: {}}}}}));",
        serde_json::to_string(command).unwrap_or_else(|_| "\"\"".into())
    );
    let _ = w.eval(&js);
}

/// `phase-20`: route File ▸ New Terminal (Cmd+T) by the focused window's
/// kind.
///
/// - An embedded SPA window (workspace-* / tunnel-* / outbound-* /
///   terminal-*) gets `app.terminal.toggle` dispatched. The SPA decides
///   what that means: a workspace window toggles a pane terminal (its
///   existing behaviour); a terminal window adds a terminal tab.
/// - Anything else (a focused launcher `main` / `main-*`, or no focused
///   window at all) opens a fresh standalone terminal window.
///
/// The single always-enabled menu accelerator pre-empts the webview, so
/// the KEY_BRIDGE_JS `KeyT` -> `app.terminal.toggle` case is harmlessly
/// shadowed in the desktop; this routing reproduces the same dispatch for
/// SPA windows while giving the launcher a working Cmd+T.
fn handle_new_terminal(app: &tauri::AppHandle) {
    let focused_spa = app
        .webview_windows()
        .into_values()
        .any(|w| serve::is_workspace_webview_label(w.label()) && w.is_focused().unwrap_or(false));
    if focused_spa {
        dispatch_to_focused_workspace(app, "app.terminal.toggle");
    } else {
        spawn_terminal_window(app);
    }
}

/// `phase-21`: route File ▸ Close Window (Cmd+W) by the focused window's
/// kind, mirroring `handle_new_terminal`.
///
/// - A focused workspace webview (workspace-* / tunnel-* / outbound-* /
///   terminal-*) gets `app.tab.close` dispatched — the same CustomEvent the
///   KEY_BRIDGE_JS KeyW case fires — so Cmd+W closes the active tab, not the
///   window.
/// - Any other focused window (the launcher `main` / `main-*`, the About
///   window) is closed natively. The launcher's `CloseRequested` handler
///   intercepts that to hide rather than destroy it, keeping reopen instant.
///
/// macOS-only: the File ▸ Close Window item exists only there. Off macOS the
/// platform mod is Ctrl and Ctrl+W stays a terminal readline chord, so no
/// menu accelerator claims it.
#[cfg(target_os = "macos")]
fn handle_close_window(app: &tauri::AppHandle) {
    let Some(window) = app
        .webview_windows()
        .into_values()
        .find(|w| w.is_focused().unwrap_or(false))
    else {
        return;
    };
    if serve::is_workspace_webview_label(window.label()) {
        dispatch_to_focused_workspace(app, "app.tab.close");
    } else {
        let _ = window.close();
    }
}

/// `phase-20`: open a standalone terminal-only window. Mounting the
/// embedded tenant is async (`EmbeddedServer::open_terminal`), so this
/// hands off to the Tauri async runtime; a failure surfaces as a system
/// notice rather than blocking the menu-event thread. Mirrors how the
/// IPC commands drive `serve::start`.
fn spawn_terminal_window(app: &tauri::AppHandle) {
    let app_for_task = app.clone();
    let state = Arc::clone(&app.state::<Arc<AppState>>());
    tauri::async_runtime::spawn(async move {
        if let Err(e) = serve::spawn_local_terminal_window(app_for_task.clone(), state).await {
            tracing::warn!(error = %e, "opening standalone terminal window failed");
            emit_system_notice(
                &app_for_task,
                "error",
                format!("Could not open terminal: {e}"),
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_binary_accepts_hidden_mcp_proxy_command() {
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(MAIN_RS.contains("\"__mcp-proxy\""));
        assert!(MAIN_RS.contains("run_hidden_mcp_proxy_if_requested"));
        assert!(MAIN_RS.contains("run_mcp_proxy(socket)"));
        assert!(MAIN_RS.contains("chan_server::run_mcp_stdio_proxy"));
    }

    #[test]
    fn normalize_outbound_url_accepts_http_and_strips_window_param() {
        let url = normalize_outbound_url(" http://127.0.0.1:4000/workspace/?t=abc&w=old#files ")
            .expect("valid url");
        assert_eq!(url, "http://127.0.0.1:4000/workspace/?t=abc#files");
    }

    #[test]
    fn normalize_outbound_url_rejects_non_http() {
        let err = normalize_outbound_url("file:///tmp/foo").expect_err("rejected");
        assert!(err.contains("http:// or https://"));
    }

    #[test]
    fn normalize_outbound_label_trims_and_caps() {
        assert_eq!(
            normalize_outbound_label("  Remote notes  ").expect("label"),
            "Remote notes",
        );
        let too_long = "x".repeat(OUTBOUND_LABEL_MAX_CHARS + 1);
        assert!(normalize_outbound_label(&too_long).is_err());
    }

    #[test]
    fn window_numbers_are_lowest_free_per_base_with_reuse() {
        let mut numbers: HashMap<String, (String, u64)> = HashMap::new();
        // Helper mirroring AppState::assign_window_number against the
        // local map (the method just locks + delegates to the same
        // free function).
        let assign = |numbers: &mut HashMap<String, (String, u64)>, label: &str, base: &str| {
            let n = lowest_free_window_number(numbers, label, base);
            numbers.insert(label.to_string(), (base.to_string(), n));
            n
        };

        // First two terminal windows get 1, 2.
        assert_eq!(assign(&mut numbers, "terminal-win-0", "Terminal"), 1);
        assert_eq!(assign(&mut numbers, "terminal-win-1", "Terminal"), 2);
        // A different base title starts its own sequence at 1.
        assert_eq!(assign(&mut numbers, "workspace-aa-0", "🏠 /w"), 1);

        // Free the first terminal; the next terminal reuses 1, not 3.
        numbers.remove("terminal-win-0");
        assert_eq!(assign(&mut numbers, "terminal-win-2", "Terminal"), 1);
        // The unrelated base is untouched by the terminal churn.
        assert_eq!(assign(&mut numbers, "workspace-aa-1", "🏠 /w"), 2);

        // Re-assigning a live label keeps its slot (ignores itself).
        assert_eq!(assign(&mut numbers, "terminal-win-1", "Terminal"), 2);
    }

    #[test]
    fn window_family_prefix_strips_the_seq_segment() {
        // All standalone terminals are one family.
        assert_eq!(window_family_prefix("terminal-win-0"), "terminal-win-");
        assert_eq!(window_family_prefix("terminal-win-12"), "terminal-win-");
        // Workspace / tunnel / outbound group per hash segment.
        assert_eq!(
            window_family_prefix("workspace-00deadbeef00aa11-3"),
            "workspace-00deadbeef00aa11-",
        );
        assert_eq!(
            window_family_prefix("outbound-00deadbeef00aa11-0"),
            "outbound-00deadbeef00aa11-",
        );
        // Degenerate label without a dash stays itself (never matches a
        // family-prefixed lookup, which always ends in '-').
        assert_eq!(window_family_prefix("main"), "main");
    }

    #[test]
    fn buried_lookup_is_most_recent_first_within_a_family() {
        let buried = vec![
            BuriedWindow {
                label: "terminal-win-0".into(),
                title: "Terminal Window 1".into(),
                buried_at: 100,
            },
            BuriedWindow {
                label: "workspace-aa-0".into(),
                title: "🏠 /w Window 1".into(),
                buried_at: 200,
            },
            BuriedWindow {
                label: "terminal-win-2".into(),
                title: "Terminal Window 3".into(),
                buried_at: 300,
            },
        ];
        // Most recently buried terminal wins; the workspace family is
        // untouched by terminal churn.
        assert_eq!(
            most_recent_buried_with_prefix(&buried, "terminal-win-"),
            Some("terminal-win-2"),
        );
        assert_eq!(
            most_recent_buried_with_prefix(&buried, "workspace-aa-"),
            Some("workspace-aa-0"),
        );
        // A family with nothing buried finds nothing — and a family
        // prefix never matches another family's labels.
        assert_eq!(
            most_recent_buried_with_prefix(&buried, "workspace-bb-"),
            None
        );
    }

    #[test]
    fn launcher_window_number_derives_from_label() {
        // The singleton launcher is window 1; `main-N` is window N.
        assert_eq!(launcher_window_number("main"), 1);
        assert_eq!(launcher_window_number("main-2"), 2);
        assert_eq!(launcher_window_number("main-3"), 3);
        // Malformed / unexpected labels fall back to 1 rather than panic.
        assert_eq!(launcher_window_number("main-"), 1);
        assert_eq!(launcher_window_number("main-x"), 1);
        assert_eq!(launcher_window_title("main"), "Chan Desktop Window 1");
        assert_eq!(launcher_window_title("main-4"), "Chan Desktop Window 4");
    }
}
