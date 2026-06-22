#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod config;
mod cs_install;
mod devserver;
mod download;
mod dropped_paths;
mod embedded;
mod linux_gui_stack;
#[cfg(target_os = "macos")]
mod pdf;
mod registry;
mod serve;
mod watcher;
mod window_ops;
mod window_watcher;
mod window_watcher_wiring;

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;
// MenuItemKind is only NAMED by the macOS menu surgery (strip-close /
// About matching); the dynamic Window-menu rebuild iterates items
// without naming the kind, so off-macOS the import is unused and
// `-D warnings` fails the Linux build (caught by CI, not the local
// macOS gate, which never compiles the other cfg branch).
#[cfg(target_os = "macos")]
use tauri::menu::{Menu, MenuItemKind, PredefinedMenuItem, WINDOW_SUBMENU_ID};
use tauri::menu::{MenuItemBuilder, Submenu};
use tauri::{Emitter, Manager, RunEvent, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use config::{Config, ConfigStore, Devserver, OutboundWorkspace, WindowConfig};
use serve::ServeHandle;

const CHAN_BUSY_CHANGED: &str = "chan-busy";
const SYSTEM_NOTICE: &str = "system-notice";

/// Process-wide state. Shared via `Arc` because Tauri commands and
/// background runtime owners need the same state handle.
pub struct AppState {
    /// Shared config handle. An `Arc<Mutex<…>>` (not a bare `Mutex`) so the
    /// launcher's [`DevserverConfigRegistry`](config::DevserverConfigRegistry),
    /// installed into the embedded host, writes the SAME config the desktop's
    /// own commands and the window-config LRU do — every full-file rewrite
    /// serializes through one lock, so a devserver CRUD can't lose an update to
    /// a concurrent window-config save.
    store: Arc<Mutex<ConfigStore>>,
    /// Live embedded local workspaces keyed by canonical workspace path.
    serves: Mutex<HashMap<String, ServeHandle>>,
    /// In-process chan-server host for normal local workspaces.
    /// Initialized during Tauri setup, after the async runtime is
    /// available for Tokio listener registration.
    embedded: OnceLock<embedded::EmbeddedServer>,
    /// The local window watcher's desktop-local view state (the L5 bury set),
    /// shared so the close handlers can bury/unbury through the watcher rather
    /// than the legacy hide path. Set once when the watcher spawns.
    local_watcher_view: OnceLock<Arc<window_watcher::WatcherViewState>>,
    /// Per-live-window zoom level. Tracks the
    /// current zoom for every open webview keyed by window label so
    /// `zoom_in` / `zoom_out` / `zoom_reset` can compute the next
    /// level without spawning a JS eval round-trip to read the
    /// current. Drained into `WindowConfig.zoom_level` by the close
    /// handler so the LRU restore picks the level up on
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
    /// Custom window titles set via `cs window title <id> <title>`, keyed
    /// by window label. Consulted by `build_workspace_window` so the
    /// override survives the bury/reopen cycle (the auto "{base} Window
    /// {N}" scheme applies only when there's no override). Session-scoped:
    /// not persisted across an app restart, like the display numbers.
    pub window_title_overrides: Mutex<HashMap<String, String>>,
    /// Windows hidden ("buried") by the OS close button instead of
    /// destroyed, in bury order (most recent last). The webview stays
    /// alive — live terminals keep running, layout state stays warm —
    /// and the Window menu lists each entry for reopening (also
    /// Cmd/Ctrl+Shift+N, which unburies the most recent of the focused
    /// family). Entries leave the list on unbury or window destroy.
    pub buried_windows: Mutex<Vec<BuriedWindow>>,
    /// Reopenable REMOTE windows, keyed by remote window label: the
    /// `saved && !connected` rows from each remote connection's
    /// (outbound attachment) `GET /api/windows`,
    /// refreshed by `refresh_remote_windows_menu`. The Window menu
    /// lists them under `remote:` ids; clicking one opens a webview
    /// with that exact label so the remote restores its session blob.
    pub remote_reopen: Mutex<HashMap<String, RemoteReopen>>,
    /// Live connections to devservers, keyed by `Devserver.id`. A devserver
    /// present here is connected (the launcher polls its workspace list and
    /// can open its tenants); absent means disconnected. In memory only:
    /// the bearer token rotates, so it is re-acquired on each connect.
    pub devservers: devserver::DevserverConns,
    /// Windows the desktop opened for each devserver (its standalone terminal
    /// and workspace tenants), keyed by `Devserver.id`. Tracked so a
    /// disconnect tears down exactly its windows, and a reconnect re-opens its
    /// workspace windows with a fresh token under the same label.
    pub devserver_windows: Mutex<HashMap<String, Vec<DevserverWindow>>>,
    /// Per connected devserver (`Devserver.id`), the `cancel` handle for its
    /// window watcher. The watcher drives that devserver's native windows as a
    /// pure reconcile of its `/api/library/windows` feed; flipping `cancel` on
    /// disconnect stops it AND makes it reconcile its windows away (detach, not
    /// reap — it learned its `library_id` lazily from the feed). Supersedes the
    /// imperative `devserver_windows` tracking for the watcher-driven path.
    pub devserver_watchers: Mutex<HashMap<String, tokio::sync::watch::Sender<bool>>>,
    /// The embedded control-terminal tenant prefix (`/control-N`) running each
    /// scripted devserver's connect script, keyed by `Devserver.id`. Kept
    /// separate from `devserver_windows` because this is a LOCAL embedded
    /// tenant prefix, not a remote workspace prefix; teardown closes the tenant
    /// (reaping the script PTY) on disconnect/forget, and reconnect must never
    /// mistake it for a workspace window. Absent for a no-script devserver.
    pub control_terminal_prefixes: Mutex<HashMap<String, String>>,
    /// Teardown hook the launcher's [`DevserverConfigRegistry`] fires after an
    /// HTTP `DELETE /api/library/devservers/{id}` drops a row, so that path
    /// reaps a live connection/windows the same way the Tauri `remove_devserver`
    /// command does. The registry (chan-server side) can't see the `AppHandle`,
    /// so it's installed with this shared cell and the desktop fills it (with a
    /// closure over the `AppHandle`) once Tauri setup runs.
    pub devserver_remove_hook: Arc<OnceLock<config::DevserverRemoveHook>>,
    /// Set when the user confirmed the quit dialog: the re-fired
    /// `ExitRequested` (from `app.exit(0)` in the dialog callback)
    /// must pass instead of prompting again.
    pub quit_confirmed: std::sync::atomic::AtomicBool,
    /// True while the quit-confirmation dialog is showing, so a
    /// repeated Cmd+Q doesn't stack a second dialog.
    pub quit_prompt_open: std::sync::atomic::AtomicBool,
}

/// One reopenable remote window: see `AppState::remote_reopen`.
#[derive(Debug, Clone)]
pub struct RemoteReopen {
    /// The connection's webview URL (outbound URL with its token).
    pub url: String,
    /// Base window title (`📤 <url>`); the build
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
    /// Set when this is a CLOSED devserver-tenant window enumerated for the
    /// Window menu (L10) rather than an outbound attachment: the reopen
    /// re-creates it at its label AND re-tracks it under the devserver so a
    /// later disconnect tears it down. `None` for outbound reopens.
    pub devserver: Option<DevserverReopen>,
}

/// The devserver context a menu-reopened workspace window needs (see
/// `RemoteReopen`): which devserver owns it and how to re-create + track it.
#[derive(Debug, Clone)]
pub struct DevserverReopen {
    /// Devserver id — the teardown key (a disconnect closes this window).
    pub id: String,
    /// Tenant route prefix — the tracking `window_id` for a workspace window.
    pub prefix: String,
}

/// One buried (hidden, not closed) window: see `AppState::buried_windows`.
#[derive(Debug, Clone)]
pub struct BuriedWindow {
    /// Tauri window label (`workspace-<16hex>-<seq>` / `terminal-win-<seq>` /
    /// outbound). Also the Window-menu item id suffix.
    pub label: String,
    /// OS display title at bury time ("🏠 /path Window 2",
    /// "Terminal Window 1") — shown verbatim in the Window menu.
    pub title: String,
    /// Wall-clock millis at bury time; diagnostics only (the Vec's
    /// push order is the recency authority).
    pub buried_at: u64,
}

/// One window the desktop opened for a devserver: see
/// `AppState::devserver_windows`.
#[derive(Debug, Clone)]
pub struct DevserverWindow {
    /// Outbound spawn id (the workspace tenant prefix, or the standalone
    /// terminal id). Teardown closes the window by this.
    pub window_id: String,
    /// The actual Tauri window label. Reconnect re-opens the window under the
    /// SAME label so the remote hydrates its `?w=<label>` session.
    pub label: String,
    /// Workspace tenant prefix for a workspace window (`None` for the
    /// standalone terminal). Reconnect re-assembles a fresh tenant URL from
    /// this and the rotated token.
    pub prefix: Option<String>,
}

/// Family prefix for unbury matching: the label with its trailing
/// `-<seq>` segment removed (everything through the LAST dash).
/// `terminal-win-3` -> `terminal-win-` (all standalone terminals are
/// one family); `workspace-<16hex>-2` -> `workspace-<16hex>-` (one
/// family per workspace; same shape for outbound labels).
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
    /// The embedded local server, once `.setup()` has started it. The
    /// window-watcher wiring reads the library's window feed through this.
    pub(crate) fn embedded(&self) -> Option<&embedded::EmbeddedServer> {
        self.embedded.get()
    }

    /// The window watcher's view state (the L5 bury set), once the watcher has
    /// spawned. Close handlers bury/unbury local windows through it.
    pub(crate) fn local_watcher_view(&self) -> Option<&Arc<window_watcher::WatcherViewState>> {
        self.local_watcher_view.get()
    }

    /// Record the watcher's view state so close handlers can reach it. Set once.
    pub(crate) fn set_local_watcher_view(&self, view: Arc<window_watcher::WatcherViewState>) {
        let _ = self.local_watcher_view.set(view);
    }

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

    /// The custom OS-title override for `label`, if any (read by
    /// `build_workspace_window`).
    pub fn window_title_override(&self, label: &str) -> Option<String> {
        self.window_title_overrides
            .lock()
            .unwrap()
            .get(label)
            .cloned()
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
/// * `kind = "outbound"`: a remote `chan open` explicitly attached
///   by URL. No desktop-owned lifecycle; `id` points at the stored
///   attachment row.
///
/// `id` / `label` are specific to outbound rows and optional so the JSON
/// shape is a strict superset of the local row; the renderer reads `kind`
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
            }
        })
        .collect();

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
        });
    }

    Ok(merged)
}

/// Register a local workspace folder and open it. Registration is
/// lean (BM25-only, no reports): the SPA's onboarding card enables
/// the optional Semantic / Reports layers post-boot.
#[tauri::command]
async fn add_workspace(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    path: String,
) -> Result<(), String> {
    let path = canonical_key(Path::new(&path));
    let Some(embedded) = state.embedded.get() else {
        return Err("embedded local server is unavailable".to_string());
    };
    // Route through the SINGLE embedded Library so the in-memory
    // registry the host opens workspaces against learns about the new
    // row immediately. A subprocess `chan workspace add` would mutate only
    // the on-disk registry, leaving the host's boot-time snapshot
    // stale, which is the "workspace not registered" bug this replaces.
    let library = embedded.library().clone();
    let path_for_block = path.clone();

    emit_chan_busy(&app, true, "add", &path);
    // register_workspace writes the registry on disk; run it off the
    // async executor.
    let result =
        tokio::task::spawn_blocking(move || register_workspace_path(&library, &path_for_block))
            .await;
    emit_chan_busy(&app, false, "add", &path);
    match result {
        Ok(inner) => inner?,
        Err(e) => return Err(format!("registering workspace panicked: {e}")),
    }

    // Auto-start: opening a workspace from the desktop is the user's
    // way of saying "make this workspace usable now". Spinning up the
    // serve immediately is what they expect; otherwise the freshly
    // added row sits there with On=off and Launch disabled, which
    // looks broken.
    serve::start(app, Arc::clone(&state), path).await?;
    Ok(())
}

/// Register `path` with the shared embedded Library, creating the
/// directory for a fresh path. No workspace handle is held when this
/// returns, so the immediately-following `serve::start` can mount the
/// workspace without tripping `WorkspaceAlreadyOpen` against the
/// lifetime flock. Blocking: `register_workspace` writes the registry,
/// so callers invoke it via `spawn_blocking`.
fn register_workspace_path(library: &chan_workspace::Library, path: &str) -> Result<(), String> {
    let root = Path::new(path);
    if !root.exists() {
        std::fs::create_dir_all(root)
            .map_err(|e| format!("creating workspace root {path}: {e}"))?;
    }
    library
        .register_workspace(root)
        .map_err(|e| format!("registering workspace {path}: {e}"))?;
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
        // `serve::stop` → `close_workspace` busy-waits up to 5s for the flock
        // release (host.rs `wait_for_workspace_release`), so run it off the
        // runtime — this is an async command (async-audit A3).
        let state_owned = Arc::clone(&state);
        tokio::task::spawn_blocking(move || serve::stop(Some(&app), &state_owned, &key))
            .await
            .map_err(|e| format!("stopping workspace {path}: {e}"))?;
    }
    persist_workspaces(&state);
    Ok(())
}

/// Snapshot every currently-on local workspace into the library-owned workspace
/// overlay (`~/.chan/workspaces.json`) as `on` rows, so the next boot re-serves
/// them (the §3.2 boot matrix). Off workspaces are simply absent — the CLI
/// registry surfaces them off; a workspace that fails to re-serve at boot is not
/// in `serves` and so drops out of this snapshot on the next clean shutdown.
/// Called after each on/off toggle and on clean shutdown. Best-effort: a no-op
/// when the embedded host / overlay is unavailable, never fatal to the toggle or
/// the exit.
fn persist_workspaces(state: &AppState) {
    let Some(embedded) = state.embedded.get() else {
        return;
    };
    let Some(overlay) = embedded.workspace_overlay() else {
        return;
    };
    let mut keys: Vec<String> = state.serves.lock().unwrap().keys().cloned().collect();
    keys.sort();
    let rows: Vec<chan_server::PersistedWorkspace> = keys
        .into_iter()
        .map(|path| chan_server::PersistedWorkspace { path, on: true })
        .collect();
    overlay.replace(rows);
}

#[tauri::command]
fn get_config(state: State<Arc<AppState>>) -> Result<Config, String> {
    state.store.lock().unwrap().get().map_err(err)
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
    serve::spawn_remote_workspace_window(&app, &id, &stored_url)?;
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
    serve::spawn_remote_workspace_window(&app, &id, &url).map(|_| ())
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
    serve::close_remote_workspace_windows(&app, &id);
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

const DEVSERVER_LABEL_MAX_CHARS: usize = 120;

/// Persist a devserver connection recipe (the New / Devserver form) and
/// return its desktop-local id. A devserver is a multi-workspace
/// aggregator the desktop dials out to; this records the connection
/// recipe so it renders as a `[DEVSERVER {host}]` launcher section.
///
/// Idempotent on `url`: re-adding the same endpoint updates its
/// script/label instead of stacking a duplicate, mirroring
/// `add_outbound_workspace`'s URL dedup.
#[tauri::command]
fn add_devserver(
    app: tauri::AppHandle,
    state: State<Arc<AppState>>,
    url: String,
    script: String,
    label: String,
) -> Result<String, String> {
    let url = url.trim().to_string();
    // Validate the URL parses as scheme://host[:port] (rejects bare host:port).
    devserver::parse_devserver_url(&url)?;
    let script = script.trim().to_string();
    let label = normalize_devserver_label(&label)?;
    let id = {
        let mut store = state.store.lock().unwrap();
        let mut cfg = store.get().map_err(err)?;
        let id = match cfg.devservers.iter_mut().find(|d| d.url == url) {
            Some(existing) => {
                existing.script = script;
                if !label.is_empty() {
                    existing.label = label;
                }
                existing.id.clone()
            }
            None => {
                // Keep this id a UUID, never a bare numeric counter: it rides in
                // the control terminal's window label (`control-terminal-<id>`),
                // and the SPA's cross-window drag scope strips a trailing
                // `-<digits>` from the `?w=` label. A `control-terminal-3` would
                // collapse to the bare `control-terminal` scope, letting tabs
                // drag between two devservers' control terminals; a UUID keeps
                // each scope distinct (the F6 d&d isolation).
                let entry = Devserver {
                    id: uuid::Uuid::new_v4().to_string(),
                    url,
                    script,
                    label,
                    token: String::new(),
                    added_at: config::current_millis(),
                };
                let id = entry.id.clone();
                cfg.devservers.push(entry);
                id
            }
        };
        store.save(&cfg).map_err(err)?;
        id
    };
    let _ = app.emit(serve::SERVES_CHANGED, ());
    Ok(id)
}

/// A configured devserver plus whether the desktop is currently connected
/// to it. The launcher groups by these: a connected devserver shows its live
/// workspace rows, a disconnected one shows the connect affordance.
#[derive(Debug, Clone, Serialize)]
struct DevserverView {
    id: String,
    url: String,
    script: String,
    label: String,
    added_at: u64,
    connected: bool,
}

/// The configured devservers, for the launcher's `[DEVSERVER {host}]`
/// grouping, each tagged with its live connection state. The per-devserver
/// workspace rows come from `list_devserver_workspaces` once connected.
#[tauri::command]
fn list_devservers(state: State<Arc<AppState>>) -> Result<Vec<DevserverView>, String> {
    let devservers = state.store.lock().unwrap().get().map_err(err)?.devservers;
    Ok(devservers
        .into_iter()
        .map(|d| DevserverView {
            connected: state.devservers.is_connected(&d.id),
            id: d.id,
            url: d.url,
            script: d.script,
            label: d.label,
            added_at: d.added_at,
        })
        .collect())
}

/// Display name for a devserver in the Window menu: its user label, or its
/// host when unlabelled.
fn devserver_display(d: &Devserver) -> String {
    let label = d.label.trim();
    if !label.is_empty() {
        return label.to_string();
    }
    // No label: fall back to the URL host (the `[DEVSERVER {host}]` identity),
    // or the raw URL if it somehow doesn't parse.
    devserver::parse_devserver_url(&d.url)
        .map(|(host, _)| host)
        .unwrap_or_else(|_| d.url.clone())
}

/// Record a window the desktop opened for a devserver, so a later disconnect
/// can tear it down and a reconnect can re-open it.
fn track_devserver_window(state: &AppState, id: &str, window: DevserverWindow) {
    state
        .devserver_windows
        .lock()
        .unwrap()
        .entry(id.to_string())
        .or_default()
        .push(window);
}

/// Close every window the desktop opened for a devserver (its standalone
/// terminal, its workspace tenants, and its control terminal) and forget the
/// tracking. Best-effort: a window the user already closed is a no-op.
fn teardown_devserver_windows(app: &tauri::AppHandle, state: &AppState, id: &str) {
    let windows = state
        .devserver_windows
        .lock()
        .unwrap()
        .remove(id)
        .unwrap_or_default();
    for window in windows {
        serve::close_remote_workspace_windows(app, &window.window_id);
    }
    serve::close_window_by_label(app, &serve::control_terminal_label(id));
    // Closing the control-terminal WINDOW doesn't stop the connect script: its
    // tenant outlives the window. Reap the tenant (kills the script PTY) so a
    // scripted devserver's disconnect/forget leaves nothing running.
    let control_prefix = state.control_terminal_prefixes.lock().unwrap().remove(id);
    if let Some(prefix) = control_prefix {
        if let Some(embedded) = state.embedded.get() {
            if let Err(e) = embedded.close_control_terminal(&prefix) {
                tracing::warn!(devserver = %id, error = %e, "closing control terminal tenant failed");
            }
        }
    }
}

/// Fully tear down a devserver's live connection: drop the in-memory
/// connection, stop its window watcher (which reconciles its windows away), reap
/// its windows and control-terminal tenant, then signal the launcher to refresh.
/// Shared by `disconnect_devserver`, `remove_devserver`, and the registry's
/// HTTP-DELETE teardown hook so all three reap a connected devserver
/// identically. Idempotent and a no-op when the devserver wasn't connected.
fn teardown_devserver_connection(app: &tauri::AppHandle, state: &AppState, id: &str) {
    state.devservers.remove(id);
    // Cancel the window watcher (it detaches its windows, not reap — the
    // devserver keeps its set server-side); the imperative windows + control
    // terminal are reaped by `teardown_devserver_windows` below.
    if let Some(cancel) = state.devserver_watchers.lock().unwrap().remove(id) {
        let _ = cancel.send(true);
    }
    teardown_devserver_windows(app, state, id);
    let _ = app.emit(serve::SERVES_CHANGED, ());
}

/// Poll a devserver's info endpoint until it answers or the budget runs out.
/// The connect script may take a moment to bring the devserver up, or prompt
/// for credentials in the control terminal, so the wait is generous; a
/// refused connection fails fast, so most attempts cost only the backoff.
async fn wait_for_devserver(host: &str, port: u16) -> Result<devserver::DevserverInfo, String> {
    const MAX_ATTEMPTS: usize = 20;
    const BACKOFF: std::time::Duration = std::time::Duration::from_millis(1500);
    let mut last_err = String::new();
    for attempt in 1..=MAX_ATTEMPTS {
        match devserver::fetch_info(host, port).await {
            Ok(info) => return Ok(info),
            Err(e) => {
                last_err = e;
                if attempt < MAX_ATTEMPTS {
                    tokio::time::sleep(BACKOFF).await;
                }
            }
        }
    }
    Err(format!(
        "devserver {host}:{port} did not come up in time ({last_err})"
    ))
}

/// Poll a control terminal's output until the connect script's devserver
/// prints its `token=` line, or the budget runs out. The script may take a
/// moment, or prompt for credentials in the terminal, so the wait is
/// generous.
async fn scrape_control_terminal_token(
    app: &tauri::AppHandle,
    state: &AppState,
    control_label: &str,
    prefix: &str,
) -> Result<String, String> {
    let Some(embedded) = state.embedded.get() else {
        return Err("embedded local server is unavailable".to_string());
    };
    const MAX_ATTEMPTS: usize = 40;
    const BACKOFF: std::time::Duration = std::time::Duration::from_millis(1500);
    // `build_workspace_window` registers the control window on the main thread
    // AFTER its spawn returns, so the first poll(s) here can run before the
    // window exists. Latch once we've seen it, so a later disappearance reads as
    // a user close (below) rather than the build race.
    let mut window_seen = false;
    for _ in 0..MAX_ATTEMPTS {
        // Scrape the token FIRST: a script that prints the token then exits
        // cleanly is a success, and the scrollback survives the exit, so a
        // token found this pass wins over the exit / close checks below.
        if let Some(token) = devserver::scrape_token(&embedded.read_control_terminal_output(prefix))
        {
            return Ok(token);
        }
        // No token yet, and the connect script's PTY has exited: a failed
        // connect (bad credentials, script error, a ^C-killed script). Fail fast
        // instead of waiting out the full backoff budget, so the launcher
        // surveys (abandon/edit/retry) promptly rather than sticking on
        // "connecting". The exit status is the tenant's, independent of the
        // control window, so this also catches the script dying in place.
        if let Some(code) = embedded.control_terminal_exit(prefix) {
            return Err(format!(
                "the devserver connect script exited (status {code}) before printing its token"
            ));
        }
        // The user closed the control terminal (^W / red button) before it
        // connected. A window close does NOT reap the tenant — the PTY outlives
        // it (client WS detach keeps it warm), so `control_terminal_exit` above
        // stays None and we'd otherwise strand on "connecting" until the budget
        // runs out. Abort so the SAME failure survey fires at once. Gated on
        // `window_seen` to ride out the build race above.
        match app.get_webview_window(control_label) {
            Some(_) => window_seen = true,
            None if window_seen => {
                return Err(
                    "the control terminal was closed before the devserver connected".to_string(),
                );
            }
            None => {}
        }
        tokio::time::sleep(BACKOFF).await;
    }
    Err("the devserver did not print its token in the control terminal in time".to_string())
}

/// Watch a connected scripted devserver's control-terminal PTY for a
/// connected-phase exit — the connect script returning on its own, or a ^C in
/// the control window. The connect flow's scrape loop only watches the PTY
/// until the token lands; once connected nothing else does, so a script that
/// dies leaves the devserver unreachable with no signal (the user's "^C and no
/// dialog shows"). On such an exit, fire the same `devserver-control-closed`
/// launcher survey (re-run vs abandon) the empty-window close path emits — but,
/// unlike that path, WITHOUT closing the control window: the user didn't ask to
/// close it, so it stays showing "process exited; press Ctrl+D", same as a
/// non-control terminal. The fire is desktop-side (a poll of the tenant's exit
/// status), independent of the control window's SPA and so robust to a
/// buried/throttled WKWebView (frozen Seam C contract, rule a).
///
/// Stops without firing once this watcher's control terminal is no longer the
/// devserver's current one: a disconnect/forget removes the prefix (and reaps
/// the tenant), and a fresh connect replaces it — either way that exit is not a
/// surprise THIS watcher owns, so it must not double-survey or fire against a
/// reconnected session.
fn spawn_control_terminal_exit_watcher(
    app: tauri::AppHandle,
    state: Arc<AppState>,
    id: String,
    prefix: String,
) {
    tauri::async_runtime::spawn(async move {
        const POLL: std::time::Duration = std::time::Duration::from_millis(1000);
        loop {
            // Still this devserver's live control terminal? A disconnect drops
            // the prefix from the map (and reaps the tenant); a fresh connect
            // overwrites it. Either way, stop — this watcher's job is done.
            let current = state
                .control_terminal_prefixes
                .lock()
                .unwrap()
                .get(&id)
                .cloned();
            if current.as_deref() != Some(prefix.as_str()) {
                return;
            }
            let exited = state
                .embedded
                .get()
                .and_then(|e| e.control_terminal_exit(&prefix));
            if let Some(code) = exited {
                // Fire only while still connected: a disconnect-driven reap is
                // expected teardown, not a surprise loss to survey.
                if state.devservers.is_connected(&id) {
                    tracing::info!(
                        devserver = %id,
                        status = code,
                        "control terminal exited while connected; surveying re-run vs abandon"
                    );
                    let _ = app.emit("devserver-control-closed", id);
                }
                return;
            }
            tokio::time::sleep(POLL).await;
        }
    });
}

/// Connect to a configured devserver: run its connect script in a control
/// terminal (when one is set), acquire its bearer token, confirm it answers,
/// record the connection, open a standalone terminal on it, then tuck the
/// control terminal away. When a script ran it, the token is scraped from the
/// control terminal's output (so a remote devserver whose config the desktop
/// cannot read still works); with no script, the devserver runs locally and
/// the token comes from its `~/.chan/devserver/config.json`. Once connected
/// the launcher polls the devserver's workspace list.
///
/// Driven over the desktop bridge: the launcher's Connect button fires
/// `POST /api/library/devservers/{id}/connect` → `DesktopWindowOp::ConnectDevserver`
/// → `window_ops`, which calls this. There is no `#[tauri::command]` wrapper —
/// the launcher is pure HTTP, never a Tauri invoke.
async fn connect_devserver_impl(
    app: tauri::AppHandle,
    state: Arc<AppState>,
    id: String,
) -> Result<(), String> {
    let (url, script) = {
        let cfg = state.store.lock().unwrap().get().map_err(err)?;
        let ds = cfg
            .devservers
            .iter()
            .find(|d| d.id == id)
            .ok_or_else(|| format!("no devserver {id}"))?;
        (ds.url.clone(), ds.script.clone())
    };
    // Parse the stored URL into the (host, port) the raw-tunnel dial uses
    // (the port defaults from the scheme when omitted).
    let (host, port) = devserver::parse_devserver_url(&url)?;
    // A configured script runs in a control terminal that brings the
    // devserver up; with no script the devserver is expected to be running
    // already.
    let control = if script.trim().is_empty() {
        None
    } else {
        let ct = serve::spawn_control_terminal_window(app.clone(), Arc::clone(&state), &id, script)
            .await?;
        // Track the control tenant prefix NOW, before the fallible scrape /
        // wait / open below: a connect that fails partway leaves the script
        // PTY running, and the failure survey's Retry / Edit / Abandon reap it
        // through teardown_devserver_windows (which reads this map).
        state
            .control_terminal_prefixes
            .lock()
            .unwrap()
            .insert(id.clone(), ct.prefix.clone());
        Some(ct)
    };
    let token = match &control {
        Some(ct) => {
            scrape_control_terminal_token(
                &app,
                &state,
                &serve::control_terminal_label(&id),
                &ct.prefix,
            )
            .await?
        }
        None => devserver::read_local_token()?,
    };
    let info = wait_for_devserver(&host, port).await?;
    if info.protocol != devserver::DEVSERVER_API_PROTOCOL {
        return Err(format!(
            "devserver speaks management protocol {} but this desktop speaks {}; update whichever is older",
            info.protocol,
            devserver::DEVSERVER_API_PROTOCOL
        ));
    }
    tracing::info!(
        version = %info.devserver_version,
        label = %info.host_label,
        "connected to devserver"
    );
    // Window-title display name: the server's host_label, else the dialed host
    // (a bare tunnel host like 127.0.0.1 is a poor title, but better than blank).
    let name = if info.host_label.trim().is_empty() {
        host.clone()
    } else {
        info.host_label.clone()
    };
    let conn = devserver::DevserverConn {
        host,
        port,
        token,
        name,
    };
    state.devservers.set(id.clone(), conn.clone());
    // The window watcher is the SOLE driver of this devserver's native windows:
    // spawn it over the library feed (`/api/library/windows/watch`), and its
    // snapshots reconcile open whatever the devserver persisted. An EMPTY feed is
    // valid (a fresh devserver, or one the user emptied before disconnecting).
    let cancel =
        window_watcher_wiring::spawn_devserver_window_watcher(app.clone(), conn.clone()).await?;
    state
        .devserver_watchers
        .lock()
        .unwrap()
        .insert(id.clone(), cancel);
    // The desktop does not mint a boot terminal on connect: the headless
    // devserver runs the library's own first-open rule when it opens (one
    // terminal the very first time, never re-minted once the user closes it), so
    // the desktop just reconciles whatever the feed reports.
    //
    // The control terminal stays open after connect. It runs the connect
    // script, which may keep streaming or prompt for ssh credentials, so
    // burying it on connect hid live output and read as a flash. The user
    // closes it with the native red dot (which buries it; reopen from the
    // [DEVSERVER] row dropdown via open_window_by_label), and disconnect/forget
    // reaps both the window and its tenant through teardown_devserver_windows.
    // The tenant prefix was tracked at spawn time (control is read above for
    // the token scrape).
    //
    // Rule (a) of the control-terminal dialog: the scrape loop above stopped
    // watching the PTY once the token landed, so a connected-phase exit (the
    // script returning, or a ^C in the control window) would otherwise go
    // unnoticed. Watch it from here so it surveys re-run/abandon.
    if let Some(ct) = &control {
        spawn_control_terminal_exit_watcher(
            app.clone(),
            Arc::clone(&state),
            id.clone(),
            ct.prefix.clone(),
        );
    }
    let _ = app.emit(serve::SERVES_CHANGED, ());
    Ok(())
}

/// Disconnect from a devserver: drop the live connection and tear down the
/// windows it opened (standalone terminal, workspace tenants, control
/// terminal), so its section returns to the connect affordance with no orphan
/// windows left pointing at a server the desktop no longer talks to.
#[tauri::command]
fn disconnect_devserver(
    app: tauri::AppHandle,
    state: State<Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    teardown_devserver_connection(&app, &state, &id);
    Ok(())
}

/// The live workspace rows for a connected devserver, each with an assembled
/// tenant URL. Empty when the devserver is not connected. The launcher polls
/// this on an interval to track serve-driven additions and removals.
#[tauri::command]
async fn list_devserver_workspaces(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<Vec<devserver::DevserverWorkspaceRow>, String> {
    let Some(conn) = state.devservers.get(&id) else {
        return Ok(Vec::new());
    };
    devserver::fetch_workspaces(&conn).await
}

/// Open a devserver workspace window by MINTING it on the devserver's library
/// (`POST /api/library/windows {Workspace, path}`). The window watcher then
/// reconciles the new record open, so the window is feed-driven: it persists
/// server-side and reopens on reconnect, and disconnect closes it via the
/// watcher's reconcile-to-empty — unlike the old imperative `outbound-` spawn,
/// which lived outside the feed and vanished on reconnect. The SPA Open button
/// turns the workspace ON first, so the minted record resolves a live token (an
/// off workspace mints an empty token the watcher skips).
#[tauri::command]
async fn open_devserver_workspace(
    state: State<'_, Arc<AppState>>,
    id: String,
    path: String,
) -> Result<(), String> {
    let conn = state
        .devservers
        .get(&id)
        .ok_or_else(|| "devserver is not connected".to_string())?;
    devserver::mint_library_window(&conn, chan_server::WindowKind::Workspace, Some(path)).await?;
    Ok(())
}

/// Mint a standalone terminal window on a connected devserver's library (the
/// launcher's per-devserver New Terminal button). The library assigns the window
/// id, persists the record, and fires the watch, so the desktop's window watcher
/// opens it as a `lib-` terminal on the devserver's shared `/terminal` tenant —
/// the same terminal family as the connect-time boot terminal, not an isolated
/// per-window tenant.
#[tauri::command]
async fn open_devserver_terminal(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    let conn = state
        .devservers
        .get(&id)
        .ok_or_else(|| format!("devserver {id} is not connected"))?;
    devserver::mint_library_window(&conn, chan_server::WindowKind::Terminal, None).await?;
    Ok(())
}

/// Re-open a devserver's open workspace windows with fresh tenant URLs after
/// the devserver rotated its token, each under its original label so the
/// remote restores its `?w=<label>` session (the rebuild replaces the stale
/// webview in place). The standalone terminal is not re-opened here (its
/// tenant is gone after a restart); the user reopens one from the recovered
/// section.
// Imperative reconnect orchestrator superseded by the devserver watcher (the
// reconcile re-surfaces workspace windows); deleted in S2-DEVSERVER D3.
#[allow(dead_code)]
fn reopen_devserver_workspace_windows(
    app: &tauri::AppHandle,
    state: &AppState,
    id: &str,
    rows: &[devserver::DevserverWorkspaceRow],
) {
    let windows = state
        .devserver_windows
        .lock()
        .unwrap()
        .get(id)
        .cloned()
        .unwrap_or_default();
    let fresh: HashMap<&str, &str> = rows
        .iter()
        .map(|r| (r.prefix.as_str(), r.url.as_str()))
        .collect();
    for window in windows {
        let Some(prefix) = window.prefix.as_deref() else {
            continue;
        };
        let Some(url) = fresh.get(prefix) else {
            continue; // tenant no longer mounted
        };
        let entry = RemoteReopen {
            url: url.to_string(),
            base_title: serve::remote_window_title(url),
            menu_title: String::new(),
            config_key: config::remote_window_key(&window.window_id),
            connecting: true,
            devserver: None,
        };
        let _ = serve::reopen_remote_window(app, &window.label, &entry);
    }
}

/// Try to recover a connected devserver that went unreachable: re-acquire its
/// (possibly rotated) token, confirm it answers, and if the token changed,
/// re-open its workspace windows with fresh URLs. Returns true on recovery,
/// false if it is still unreachable. The launcher calls this when a workspace
/// poll fails.
#[tauri::command]
async fn reconnect_devserver(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<bool, String> {
    let Some(conn) = state.devservers.get(&id) else {
        return Ok(false);
    };
    // Try the current token first (a transient network blip keeps it valid),
    // then the local devserver's config token (a local restart rotates it). A
    // remote devserver's token is not in the local config, so it stays
    // unreachable until its control terminal re-runs the connect script (which
    // re-emits the CHAN_DEVSERVER_TOKEN= marker the desktop scrapes fresh).
    let mut candidates = vec![conn.token.clone()];
    if let Ok(local) = devserver::read_local_token() {
        if local != conn.token {
            candidates.push(local);
        }
    }
    for token in candidates {
        let mut probe = conn.clone();
        probe.token = token.clone();
        // `fetch_workspaces` is the connectivity probe (does this token auth?);
        // its rows are no longer consumed (the watcher re-surfaces the windows).
        if devserver::fetch_workspaces(&probe).await.is_ok() {
            // A disconnect that landed mid-probe already tore the windows
            // down; do not resurrect the connection or re-open them.
            if !state.devservers.is_connected(&id) {
                return Ok(false);
            }
            let rotated = token != conn.token;
            state.devservers.set(id.clone(), probe.clone());
            if rotated {
                // A rotated token means the devserver restarted: its old tenants
                // are gone AND the running watcher's feed task can't auth with
                // the stale token. RESPAWN the watcher on the fresh conn — cancel
                // the old one + reconcile its windows away, then spawn anew so its
                // first snapshot re-opens the restarted devserver's persisted set.
                // (A non-rotated reconnect needs nothing: the feed task's own
                // reconnect-on-drop self-heals with the same token.)
                if let Some(cancel) = state.devserver_watchers.lock().unwrap().remove(&id) {
                    let _ = cancel.send(true); // the watcher reconciles its windows away
                }
                let cancel =
                    window_watcher_wiring::spawn_devserver_window_watcher(app.clone(), probe)
                        .await?;
                state
                    .devserver_watchers
                    .lock()
                    .unwrap()
                    .insert(id.clone(), cancel);
            }
            let _ = app.emit(serve::SERVES_CHANGED, ());
            return Ok(true);
        }
    }
    Ok(false)
}

/// Update a devserver's connection recipe (host/port/script/label), from the
/// Edit form. Rejected while connected: a live connection's parameters must
/// not change underneath it. The new host:port must not collide with another
/// configured devserver.
#[tauri::command]
fn update_devserver(
    app: tauri::AppHandle,
    state: State<Arc<AppState>>,
    id: String,
    url: String,
    script: String,
    label: String,
) -> Result<(), String> {
    if state.devservers.is_connected(&id) {
        return Err("disconnect this devserver before editing it".to_string());
    }
    let url = url.trim().to_string();
    devserver::parse_devserver_url(&url)?;
    let script = script.trim().to_string();
    let label = normalize_devserver_label(&label)?;
    {
        let mut store = state.store.lock().unwrap();
        let mut cfg = store.get().map_err(err)?;
        if cfg.devservers.iter().any(|d| d.id != id && d.url == url) {
            return Err(format!("another devserver is already at {url}"));
        }
        let ds = cfg
            .devservers
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or_else(|| format!("no devserver {id}"))?;
        // Leave `ds.token` untouched: an edit keeps the stored credential
        // (it's write-only, so the form can't resubmit it), mirroring the
        // registry's keep-on-blank token semantics.
        ds.url = url;
        ds.script = script;
        ds.label = label;
        store.save(&cfg).map_err(err)?;
    }
    let _ = app.emit(serve::SERVES_CHANGED, ());
    Ok(())
}

/// Forget (unmount) a workspace on a connected devserver via its management
/// API. The devserver stops serving that workspace; its files on the box are
/// untouched and it can be re-mounted later.
#[tauri::command]
async fn forget_devserver_workspace(
    state: State<'_, Arc<AppState>>,
    id: String,
    prefix: String,
) -> Result<(), String> {
    let conn = state
        .devservers
        .get(&id)
        .ok_or_else(|| format!("devserver {id} is not connected"))?;
    devserver::forget_workspace(&conn, &prefix).await
}

/// Set a registered devserver workspace on (mount + mint a fresh tenant token)
/// or off (unmount, keep registered) — the on/off toggle on a devserver row,
/// distinct from Forget (`forget_devserver_workspace`). An unforced off with
/// live terminals fails with [`SetWorkspaceOnError::ActiveTerminals`] so the SPA
/// confirms then retries with `force: true`. The launcher reflects the new state
/// via the next poll (the row is re-fetched, not returned).
#[tauri::command]
async fn set_devserver_workspace_on(
    state: State<'_, Arc<AppState>>,
    id: String,
    prefix: String,
    on: bool,
    force: bool,
) -> Result<(), devserver::SetWorkspaceOnError> {
    let conn = state.devservers.get(&id).ok_or_else(|| {
        devserver::SetWorkspaceOnError::other(format!("devserver {id} is not connected"))
    })?;
    devserver::set_workspace_on(&conn, &prefix, on, force).await
}

/// Forget a devserver: drops any live connection, tears down its windows, and
/// removes the persisted connection recipe so its launcher section disappears.
#[tauri::command]
fn remove_devserver(
    app: tauri::AppHandle,
    state: State<Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    {
        let mut store = state.store.lock().unwrap();
        let mut cfg = store.get().map_err(err)?;
        let before = cfg.devservers.len();
        cfg.devservers.retain(|d| d.id != id);
        if cfg.devservers.len() != before {
            store.save(&cfg).map_err(err)?;
        }
    }
    // Reap the live connection/windows (no-op when not connected). Shared with
    // the launcher's HTTP-DELETE path so both Remove routes behave identically.
    teardown_devserver_connection(&app, &state, &id);
    Ok(())
}

fn normalize_devserver_label(raw: &str) -> Result<String, String> {
    let label = raw.trim().to_string();
    if label.chars().count() > DEVSERVER_LABEL_MAX_CHARS {
        return Err(format!(
            "devserver label must be {DEVSERVER_LABEL_MAX_CHARS} characters or fewer",
        ));
    }
    Ok(label)
}

/// Open an additional in-app Tauri webview for a running local
/// workspace. The first window is auto-opened by the serve supervisor
/// when chan prints its URL; subsequent clicks on Launch reach
/// here and add new windows alongside it. Errors if the workspace is
/// not currently running (no URL captured yet).
#[tauri::command]
fn open_local_workspace(state: State<Arc<AppState>>, path: String) -> Result<(), String> {
    let key = canonical_key(Path::new(&path));
    // Mint the window into the library registry; the watcher opens it (the
    // registry is the sole window-creation authority, so a reconnect/relaunch
    // can never duplicate it). Require the workspace running so the minted
    // record resolves a live tenant to attach to.
    if !state.serves.lock().unwrap().contains_key(&key) {
        return Err(format!("workspace {key} is not running"));
    }
    state
        .embedded()
        .ok_or_else(|| "embedded local server is unavailable".to_string())?
        .mint_window(chan_server::WindowKind::Workspace, Some(key))?;
    Ok(())
}

/// Register (and persist) a devserver from a `chan open {url}` CLI handoff.
/// Writes the `{url, name, script}` entry through the same
/// [`DevserverConfigRegistry`](config::DevserverConfigRegistry) the launcher's
/// `/api/library/devservers` routes use (the shared config handle), so the new
/// row shows up in the launcher. The handoff carries no token (the desktop owns
/// credentials — a tokened devserver is set up from the launcher dialog), so
/// this registers it untokened; the user connects it from its launcher row.
#[cfg(any(unix, windows))]
fn register_devserver_from_handoff(
    state: &Arc<AppState>,
    url: String,
    name: Option<String>,
    script: Option<String>,
) -> Result<(), String> {
    use chan_server::{DevserverInput, DevserverRegistry};
    let registry = config::DevserverConfigRegistry::new(
        Arc::clone(&state.store),
        Arc::clone(&state.devserver_remove_hook),
    );
    registry.add(DevserverInput {
        url,
        label: name,
        script,
        token: None,
    })?;
    Ok(())
}

/// Open a workspace in a native window in response to a CLI handoff
/// request (`chan open <workspace>` while this desktop is running).
///
/// Mirrors the `add_workspace` flow: register + boot the workspace through the
/// shared embedded Library, then `serve::start` (mount + mint the
/// first window). If the workspace is ALREADY running, `serve::start`
/// returns early without minting, so we mint an additional window (the
/// watcher opens it) to match the user's intent
/// ("show me this workspace now").
///
/// The slow work (registry write, boot scan, mount) runs on a spawned
/// task so the callback returns promptly and the CLI doesn't block on
/// the handshake. The synchronous return therefore reports only that
/// the request was accepted, not that the window is fully up; on a
/// genuine mount failure the desktop emits a system notice rather than
/// blocking the CLI.
#[cfg(any(unix, windows))]
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
    if running_url.is_some() {
        // Already running: mint another window; the watcher opens it.
        return state
            .embedded()
            .ok_or_else(|| "embedded local server is unavailable".to_string())?
            .mint_window(chan_server::WindowKind::Workspace, Some(key.clone()))
            .map(|_| ());
    }

    // Not running: register (creating the dir for a fresh path)
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
            register_workspace_path(&library_for_register, &key_for_register)
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

/// Drive `tauri-plugin-updater` in response to a `chan upgrade` from the
/// desktop-dispatched `chan` binary (handoff `Upgrade` request).
///
/// With `check_only` we report availability synchronously (the CLI prints
/// it) without installing. Otherwise we kick off check -> download -> install
/// on a background task and return `UpgradeStarted` at once (fire-and-return:
/// the multi-MB download can't be awaited from the CLI socket round-trip);
/// when it finishes we re-affirm the `~/.local/bin/{chan,cs}` shims and
/// relaunch into the new version.
/// Windows: `chan upgrade` is not wired over the hand-off this phase (no Windows
/// updater feed), and no Windows `chan` sends an `Upgrade` request, so this is
/// effectively dead — it exists only so the (now cross-platform) hand-off
/// listener's `Upgrade` arm compiles. Returns a clear error rather than
/// pretending to upgrade.
#[cfg(windows)]
async fn desktop_handle_upgrade(
    _app: tauri::AppHandle,
    _check_only: bool,
) -> chan_server::handoff::Response {
    chan_server::handoff::Response::Error {
        message: "desktop upgrade over hand-off is not supported on Windows yet".into(),
    }
}

#[cfg(unix)]
async fn desktop_handle_upgrade(
    app: tauri::AppHandle,
    check_only: bool,
) -> chan_server::handoff::Response {
    use chan_server::handoff::{Response, CHAN_VERSION};
    use tauri_plugin_updater::UpdaterExt;

    let updater = match app.updater() {
        Ok(u) => u,
        Err(e) => {
            return Response::Error {
                message: format!("updater unavailable: {e}"),
            }
        }
    };

    match updater.check().await {
        Ok(Some(update)) => {
            let version = update.version.clone();
            if check_only {
                return Response::UpgradeChecked {
                    desktop_version: CHAN_VERSION.into(),
                    available: Some(version),
                };
            }
            // Fire-and-return: install in the background; the CLI already has
            // its `UpgradeStarted` ack.
            let app_bg = app.clone();
            tauri::async_runtime::spawn(async move {
                match update
                    .download_and_install(|_chunk, _total| {}, || {})
                    .await
                {
                    Ok(()) => {
                        // Re-affirm the shims to the (possibly relocated)
                        // binary before relaunching into the new version.
                        match cs_install::install_bin_shims() {
                            Ok(n) => {
                                tracing::info!(shims = n, "re-affirmed bin shims after update")
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "re-affirming bin shims after update failed")
                            }
                        }
                        tracing::info!(%version, "chan-desktop update installed; relaunching");
                        app_bg.restart();
                    }
                    Err(e) => {
                        emit_system_notice(
                            &app_bg,
                            "warning",
                            format!("chan-desktop update failed: {e}"),
                        );
                    }
                }
            });
            Response::UpgradeStarted {
                desktop_version: CHAN_VERSION.into(),
            }
        }
        Ok(None) => Response::UpgradeChecked {
            desktop_version: CHAN_VERSION.into(),
            available: None,
        },
        Err(e) => Response::Error {
            message: format!("update check failed: {e}"),
        },
    }
}

/// On-launch background self-update check. The desktop registers
/// `tauri-plugin-updater`, but only the hand `chan upgrade` (the hand-off
/// `desktop_handle_upgrade` path) drives it — a running desktop never checks on
/// its own, so it stays on its installed version until the user upgrades by
/// hand. Spawn a background check on launch so a stale desktop updates itself.
///
/// Opt-out mirrors the CLI's `CHAN_UPDATE_CHECK=0` (`chan::update` `ENV_DISABLE`)
/// so one env silences both the CLI banner probe and this desktop check. The new
/// bundle is downloaded + installed in the background, then the user is asked
/// whether to relaunch now (the bundle applies on the next launch either way, so
/// "Later" is non-destructive). Windows has no updater feed (see
/// `desktop_handle_upgrade`), so this is a no-op there.
#[cfg(unix)]
fn spawn_launch_update_check(app: tauri::AppHandle) {
    use tauri_plugin_updater::UpdaterExt;

    // Mirror the CLI opt-out exactly: only the literal "0" disables it.
    if matches!(std::env::var("CHAN_UPDATE_CHECK"), Ok(v) if v == "0") {
        tracing::info!("on-launch update check disabled by CHAN_UPDATE_CHECK=0");
        return;
    }
    tauri::async_runtime::spawn(async move {
        let updater = match app.updater() {
            Ok(u) => u,
            Err(e) => {
                tracing::warn!(error = %e, "on-launch update check: updater unavailable");
                return;
            }
        };
        match updater.check().await {
            Ok(Some(update)) => {
                let version = update.version.clone();
                tracing::info!(%version, "on-launch update available; downloading");
                match update
                    .download_and_install(|_chunk, _total| {}, || {})
                    .await
                {
                    Ok(()) => {
                        // Re-affirm the `~/.local/bin/{chan,cs}` shims to the
                        // (possibly relocated) new binary, mirroring the
                        // `chan upgrade` install path.
                        match cs_install::install_bin_shims() {
                            Ok(n) => {
                                tracing::info!(shims = n, "re-affirmed bin shims after update")
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "re-affirming bin shims after update failed")
                            }
                        }
                        tracing::info!(%version, "on-launch update installed; prompting to restart");
                        prompt_restart_for_update(&app, &version);
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "on-launch update download/install failed");
                    }
                }
            }
            Ok(None) => tracing::info!("on-launch update check: already up to date"),
            Err(e) => tracing::warn!(error = %e, "on-launch update check failed"),
        }
    });
}

#[cfg(windows)]
fn spawn_launch_update_check(_app: tauri::AppHandle) {
    // No Windows updater feed (see `desktop_handle_upgrade`); nothing to check.
}

/// After an on-launch update installs, ask whether to relaunch now. The new
/// bundle is already on disk, so "Later" simply applies it on the next launch;
/// "Restart" relaunches into the new version immediately. Tauri dialogs must run
/// on the main thread.
#[cfg(unix)]
fn prompt_restart_for_update(app: &tauri::AppHandle, version: &str) {
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

    let app_owned = app.clone();
    let message = format!(
        "chan-desktop {version} has been downloaded. Restart now to use it, \
         or it will apply the next time you open chan-desktop."
    );
    let scheduled = app.run_on_main_thread(move || {
        app_owned
            .clone()
            .dialog()
            .message(message)
            .title("Update ready")
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Restart".into(),
                "Later".into(),
            ))
            .show(move |restart| {
                if restart {
                    app_owned.restart();
                }
            });
    });
    if let Err(e) = scheduled {
        tracing::warn!(error = %e, "scheduling the update-ready restart prompt failed");
    }
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

/// Reload the calling webview window. Backs the SPA's tab
/// context-menu "Reload" entry AND the
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
/// the SPA's "Open Inspector" context-menu entry
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

/// Close-cascade tail. The SPA
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
    // A connected devserver's control terminal emptying (its connect-script
    // tab was closed, or the script exited) takes the devserver connection
    // down with it: that terminal IS the connection endpoint. Don't silently
    // destroy into a dead state. Raise the launcher and survey the user
    // (re-run the script / abandon), reusing the connect-failure pattern. Only
    // while the devserver is still connected; a control terminal closed as
    // part of a normal disconnect/forget goes through teardown's destroy, not
    // this SPA close-cascade.
    if let Some(id) = closing.strip_prefix("control-terminal-") {
        let state = app.state::<Arc<AppState>>();
        if state.devservers.is_connected(id) {
            let id = id.to_string();
            let _ = show_window(&app, "main");
            let _ = app.emit("devserver-control-closed", id);
            return window.destroy().map_err(err);
        }
    }
    let others_remain = app
        .webview_windows()
        .keys()
        .any(|label| label != closing && serve::is_workspace_webview_label(label));
    if !others_remain {
        let _ = show_window(&app, "main");
    }
    // A watcher-managed local window (`local::<window_id>`) emptied (last
    // pane/tab closed, ^W/^D/Cmd+W): DISCARD its registry record — which reaps
    // its sessions and fires the feed — so the watcher reconciles the native
    // window closed. The record is gone, so it can NEVER reopen (the boomerang
    // bug a bare destroy hit: the record stayed live and reconcile reopened it).
    if let Some(window_id) = closing.strip_prefix("local::") {
        if let Some(embedded) = app.state::<Arc<AppState>>().embedded() {
            match embedded.discard_window(window_id) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    tracing::warn!(window = %window_id, error = %e, "discarding an emptied window failed; destroying");
                }
            }
        }
    }
    // A watcher-managed DEVSERVER window (`lib-<library_id>::<window_id>`) emptied:
    // discard its record on the owning devserver — the async analog of the
    // `local::` discard above. The server drops + PERSISTS the removal and fires
    // the watch, so the close survives a restart instead of the record reopening
    // empty. The DELETE is an HTTP round-trip, so fire-and-forget it (logging a
    // failure) and destroy the native window now for an instant close.
    if closing.starts_with("lib-") {
        let label = closing.to_string();
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = discard_devserver_window(&app, &label).await {
                tracing::warn!(label = %label, error = %e, "discarding a closed devserver window failed");
            }
        });
        return window.destroy().map_err(err);
    }
    // `destroy()`, not `close()`: this is the SPA's DELIBERATE close-cascade
    // (last tab, then last pane, just closed — the window is empty). `close()`
    // would fire `CloseRequested`, where the bury-on-close handler hides SPA
    // windows instead of closing them; an empty window is worthless buried.
    // Destroy skips the request phase and goes straight to `Destroyed` cleanup.
    window.destroy().map_err(err)
}

/// Browser-style zoom controls. Step size is
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

/// Cross-platform MCP-proxy short-circuit: when chan-desktop is invoked as
/// `<exe> __mcp-proxy <socket>` (the `cs` / `chan` MCP discovery hands this
/// off), bridge stdio to the chan-server MCP socket and EXIT instead of
/// launching the GUI. The transport underneath (`run_mcp_stdio_proxy`) is
/// cross-platform — a Unix-domain socket on unix, a named pipe on Windows — so
/// the desktop carries MCP on every platform. Returns `Ok(true)` when it
/// handled the invocation, `Ok(false)` for a normal GUI launch.
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

/// When chan-desktop is invoked through a `cs` name (a `~/.local/bin/cs`
/// wrapper or symlink, argv[0] stem == "cs"), behave as the `cs` control
/// client and EXIT instead of launching the GUI. This is what lets desktop
/// users get `cs` (and the MCP discovery it carries) without a separate
/// `chan` binary on PATH. Mirrors `run_hidden_mcp_proxy_if_requested`: a
/// pre-GUI argv probe that short-circuits `main`. Returns `Ok(true)` when
/// it handled the invocation (caller returns), `Ok(false)` for a normal
/// GUI launch.
fn run_as_cs_if_requested() -> Result<bool, String> {
    // Stem detection prefers `$ARGV0` (see `chan_shell::invoked_arg0`): a
    // packaged AppImage invoked via `exec -a cs "$APPIMAGE"` loses argv[0] to
    // AppRun, so keying on `args_os().next()` alone would launch the GUI
    // instead of dispatching `cs`. The args we PASS keep the real argv (clap
    // ignores the program-name slot).
    if !chan_shell::invoked_as_cs(&chan_shell::invoked_arg0()) {
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
    rt.block_on(chan_shell::run_cs(std::env::args_os()))
        .map_err(|e| format!("{e:#}"))?;
    Ok(true)
}

/// When chan-desktop is invoked through a `chan` name (a `~/.local/bin/chan`
/// symlink or AppImage wrapper, argv[0] stem == "chan"), run the whole `chan`
/// CLI in-process with the Desktop personality and EXIT instead of launching
/// the GUI. This is what makes a desktop install also provide `chan` with no
/// separate download. Mirrors `run_as_cs_if_requested`: a pre-GUI argv probe
/// that short-circuits `main`. The Desktop personality makes `chan open`
/// integrate with the running desktop (handoff / GUI launch) and `chan
/// upgrade` drive the desktop updater rather than replacing a CLI tarball.
/// Returns `Ok(true)` when it handled the invocation, `Ok(false)` for a
/// normal GUI launch.
fn run_as_chan_if_requested() -> Result<bool, String> {
    // Stem detection prefers `$ARGV0` (see `chan_shell::invoked_arg0`) so a
    // packaged AppImage invoked via `exec -a chan "$APPIMAGE"` (AppRun drops
    // argv[0]) still dispatches the CLI instead of launching the GUI. The args
    // passed to `chan::run` keep the real argv (clap ignores arg[0]).
    if !chan_shell::invoked_as_chan(&chan_shell::invoked_arg0()) {
        return Ok(false);
    }
    // `chan open` needs a multi-threaded runtime; everything else runs fine
    // on it too. shutdown_background() detaches chan-workspace's uncancellable
    // reindex pool on exit, matching the standalone `chan` binary's shim.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("building chan runtime: {e}"))?;
    let res = rt.block_on(chan::run(std::env::args_os(), chan::Personality::Desktop));
    rt.shutdown_background();
    res.map_err(|e| format!("{e:#}"))?;
    Ok(true)
}

async fn run_mcp_proxy(socket: PathBuf) -> Result<(), String> {
    chan_server::run_mcp_stdio_proxy(socket)
        .await
        .map_err(|e| format!("running MCP proxy: {e}"))
}

/// Windows console attach for the `chan` / `cs` CLI dispatch.
///
/// A release `chan-desktop.exe` is built `windows_subsystem = "windows"` (GUI
/// subsystem) so a normal double-click never flashes a console window. The cost:
/// when the SAME exe is invoked through a `chan` / `cs` shim from a terminal and
/// runs as a CLI (see `run_as_chan_if_requested` / `run_as_cs_if_requested`),
/// the process starts with NO console and its standard handles are null, so
/// every `println!` is silently discarded — `chan --version` "returns empty".
/// Re-attaching to the parent shell's console (and binding any null std handle
/// to it) is what routes the CLI output back to the terminal.
///
/// Gated on the CLI invocation: a normal GUI launch (argv[0] stem
/// "chan-desktop") returns early and stays console-free.
#[cfg(windows)]
fn attach_parent_console_for_cli() {
    let arg0 = chan_shell::invoked_arg0();
    if !chan_shell::invoked_as_chan(&arg0) && !chan_shell::invoked_as_cs(&arg0) {
        return;
    }
    win_console::attach_parent();
}

/// Win32 console-attach mechanics for the `chan` / `cs` CLI dispatch, kept in
/// one place beside the dispatch probes. Raw FFI (no higher-level wrapper) like
/// `cs_install`'s `WM_SETTINGCHANGE` broadcast.
#[cfg(windows)]
mod win_console {
    use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Console::{
        AttachConsole, GetStdHandle, SetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE,
        STD_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    };

    /// Attach to the parent process's console and bind any unset standard handle
    /// to it so the `chan` / `cs` CLI output reaches the terminal. Best-effort:
    /// `AttachConsole` fails when there is no parent console (a GUI launch from
    /// Explorer) and we leave everything alone. An already-valid std handle (a
    /// shell redirection like `chan ... > out.txt`, or one AttachConsole itself
    /// wired up) is preserved, never clobbered.
    pub(super) fn attach_parent() {
        // SAFETY: standard Win32 console FFI. AttachConsole is guarded on its
        // own return before any handle work; each std handle is validated before
        // use; the CONOUT$/CONIN$ names are valid NUL-terminated UTF-16 buffers
        // that outlive the synchronous CreateFileW call.
        unsafe {
            if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
                return; // no parent console — a normal GUI launch
            }
            bind(STD_OUTPUT_HANDLE, "CONOUT$", GENERIC_WRITE);
            bind(STD_ERROR_HANDLE, "CONOUT$", GENERIC_WRITE);
            bind(STD_INPUT_HANDLE, "CONIN$", GENERIC_READ);
        }
    }

    /// Bind one standard handle to the console device `dev` (`CONOUT$` /
    /// `CONIN$`) when it is currently unset (null / invalid). A valid handle — a
    /// shell redirection, or one AttachConsole already populated — is left
    /// untouched so redirection to a file/pipe still works. Best-effort: a
    /// CreateFileW / SetStdHandle failure is ignored (nothing more we can do).
    unsafe fn bind(std_id: STD_HANDLE, dev: &str, access: u32) {
        let cur = GetStdHandle(std_id);
        if !cur.is_null() && cur != INVALID_HANDLE_VALUE {
            return; // already wired (redirection, or AttachConsole set it)
        }
        let wide: Vec<u16> = dev.encode_utf16().chain(std::iter::once(0)).collect();
        let h = CreateFileW(
            wide.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        );
        if h != INVALID_HANDLE_VALUE {
            SetStdHandle(std_id, h);
        }
    }
}

/// macOS GUI launches (Finder / Dock / Spotlight) inherit a restricted launchd
/// `$PATH` that misses the user's interactive dirs (`~/.local/bin`,
/// `/opt/homebrew/bin`, and custom dirs). Resolve the login+interactive shell's
/// `$PATH` and merge it into this process's `$PATH`, so in-process checks (the
/// `cs` alias detection, which scans `$PATH`) and spawned subprocesses
/// (terminals) see binaries wherever the user actually has them — the general
/// fix for the launchd restricted-PATH gotcha, not `cs`-specific. Best-effort:
/// any failure leaves the inherited PATH untouched (status quo, no regression).
#[cfg(target_os = "macos")]
fn fix_macos_login_path() {
    let Some(shell_path) = resolve_login_shell_path() else {
        return;
    };
    let inherited = std::env::var("PATH").unwrap_or_default();
    let merged = merge_path_dirs(&shell_path, &inherited);
    if !merged.is_empty() {
        std::env::set_var("PATH", merged);
    }
}

/// Keep the interactive shell PATH first, then any inherited (launchd) dirs not
/// already present — deduped, order-stable, empty segments dropped.
#[cfg(target_os = "macos")]
fn merge_path_dirs(shell_path: &str, inherited: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    shell_path
        .split(':')
        .chain(inherited.split(':'))
        .filter(|dir| !dir.is_empty() && seen.insert(dir.to_string()))
        .collect::<Vec<_>>()
        .join(":")
}

/// Run the user's login shell (`$SHELL`) as a login + interactive shell to
/// capture the `$PATH` it exports — the dirs the user has on their REAL
/// interactive PATH (their profile / rc files), which the GUI launchd PATH
/// lacks. Markers delimit the value so a chatty rc (banners) can't corrupt it;
/// stdin is `/dev/null` so an interactive shell can't block on input, and
/// stderr is discarded. Bounded by a ~3s timeout so a pathological / hanging
/// rc can't block app launch (a hang is worse than the no-op fallback). `None`
/// on any failure, timeout, or empty result.
#[cfg(target_os = "macos")]
fn resolve_login_shell_path() -> Option<String> {
    use std::io::Read;
    use std::time::Duration;
    const MARK: &str = "__CHAN_PATH__";
    const TIMEOUT: Duration = Duration::from_secs(3);
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let mut child = std::process::Command::new(shell)
        .args([
            "-l",
            "-i",
            "-c",
            &format!("printf '{MARK}%s{MARK}' \"$PATH\""),
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    // Poll for exit with a timeout; on timeout kill the shell and fall back to
    // the inherited PATH. (stdin=/dev/null already stops the common read-hang;
    // this is belt-and-suspenders for a broken rc.)
    if !wait_for_child(&mut child, TIMEOUT) {
        return None;
    }
    // The output is tiny (a PATH between markers), so the pipe never fills and
    // the child exits before this read.
    let mut out = String::new();
    child.stdout.take()?.read_to_string(&mut out).ok()?;
    let begin = out.find(MARK)? + MARK.len();
    let end = out[begin..].find(MARK)? + begin;
    let path = &out[begin..end];
    (!path.is_empty()).then(|| path.to_string())
}

/// Wait for `child` to exit within `timeout`. Returns `true` if it exited on
/// its own; on timeout, kill + reap it and return `false`. Polls rather than
/// blocking so a broken interactive rc can't hang app launch. Extracted from
/// `resolve_login_shell_path` so the timeout/kill branch is unit-testable
/// without a real login shell.
#[cfg(target_os = "macos")]
fn wait_for_child(child: &mut std::process::Child, timeout: std::time::Duration) -> bool {
    use std::time::{Duration, Instant};
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) if started.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => return false,
        }
    }
}

fn main() {
    // Windows: a release chan-desktop.exe is GUI-subsystem (no console). When
    // invoked as the `chan` / `cs` CLI through a shim, reattach to the parent
    // shell's console FIRST so the CLI's stdout/stderr reach the terminal
    // instead of vanishing. No-op for a GUI launch and off Windows.
    #[cfg(windows)]
    attach_parent_console_for_cli();

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
    // `chan` alias dispatch (argv[0] stem == "chan"): run the whole chan CLI
    // in-process with the Desktop personality and exit, before any GUI /
    // runtime / config setup below. Same pre-GUI argv probe as `cs`.
    match run_as_chan_if_requested() {
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
    // Best-effort on boot: own `~/.local/bin/{chan,cs}` so a desktop install
    // also provides the `chan` + `cs` CLI without a separate download. Real
    // symlinks / AppImage wrappers / deb-rpm symlinks per package kind,
    // idempotent + marker-guarded + never clobbers a user-written shim. No-op
    // for a dev build / unrecognized layout; never fatal to boot.
    match cs_install::install_bin_shims() {
        Ok(0) => {}
        Ok(n) => tracing::info!(shims = n, "installed chan/cs bin shims into ~/.local/bin"),
        Err(e) => tracing::warn!(error = %e, "installing bin shims failed"),
    }
    let store = Arc::new(Mutex::new(
        ConfigStore::new().expect("failed to init config store"),
    ));
    let state = Arc::new(AppState {
        store,
        serves: Mutex::new(HashMap::new()),
        embedded: OnceLock::new(),
        local_watcher_view: OnceLock::new(),
        live_window_zooms: Mutex::new(HashMap::new()),
        window_numbers: Mutex::new(HashMap::new()),
        window_title_overrides: Mutex::new(HashMap::new()),
        buried_windows: Mutex::new(Vec::new()),
        remote_reopen: Mutex::new(HashMap::new()),
        devservers: devserver::DevserverConns::default(),
        devserver_windows: Mutex::new(HashMap::new()),
        devserver_watchers: Mutex::new(HashMap::new()),
        control_terminal_prefixes: Mutex::new(HashMap::new()),
        devserver_remove_hook: Arc::new(OnceLock::new()),
        quit_confirmed: std::sync::atomic::AtomicBool::new(false),
        quit_prompt_open: std::sync::atomic::AtomicBool::new(false),
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

            // Fix the restricted launchd `$PATH` of a macOS GUI launch BEFORE
            // the embedded server starts, so its in-process `cs` detection (and
            // the terminals it spawns) scan the user's real interactive PATH.
            #[cfg(target_os = "macos")]
            fix_macos_login_path();

            // Share the desktop's config handle + devserver-remove hook cell with
            // the embedded host so the launcher's devserver registry persists
            // through the same lock and its HTTP DELETE can reap a live connection.
            let config_store = Arc::clone(&state_for_setup.store);
            let remove_hook = Arc::clone(&state_for_setup.devserver_remove_hook);
            match tauri::async_runtime::block_on(embedded::EmbeddedServer::start(
                config_store,
                remove_hook,
            )) {
                Ok(server) => {
                    if state_for_setup.embedded.set(server).is_err() {
                        tracing::warn!("embedded local server initialized more than once");
                    }
                    // Fill the registry's remove hook now that the AppHandle
                    // exists: the launcher's HTTP DELETE then reaps a live
                    // devserver's connection/windows like `remove_devserver`.
                    // The closure holds only the AppHandle (no Arc cycle) and
                    // resolves the AppState from it at call time.
                    let app_for_teardown = app.handle().clone();
                    let _ = state_for_setup.devserver_remove_hook.set(Arc::new(
                        move |id: &str| {
                            let state = app_for_teardown.state::<Arc<AppState>>();
                            teardown_devserver_connection(&app_for_teardown, &state, id);
                        },
                    ));
                    // Spawn the `cs window <op>` consumer now that the
                    // AppHandle exists: it owns the bridge receiver and
                    // turns lifecycle requests into Tauri window actions.
                    // The task lives until the channel closes at exit.
                    if let Some(rx) = state_for_setup
                        .embedded
                        .get()
                        .and_then(|e| e.take_window_ops_rx())
                    {
                        let app_for_ops = app.handle().clone();
                        let state_for_ops = Arc::clone(&state_for_setup);
                        tauri::async_runtime::spawn(window_ops::run(
                            app_for_ops,
                            state_for_ops,
                            rx,
                        ));
                    }
                    // Spawn the local window watcher: native windows become a
                    // pure idempotent reconcile of the local library's window
                    // set (Seam W), so reconnect / relaunch can never spawn a
                    // duplicate. Inert until a local window is minted (an empty
                    // registry reconciles to nothing); converting the creation
                    // paths to mint makes it the sole driver.
                    window_watcher_wiring::spawn_local_window_watcher(
                        app.handle().clone(),
                        Arc::clone(&state_for_setup),
                    );
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

            // The launcher window loads the embedded loopback's root `/`, where
            // the same web-launcher SPA is served as on every other surface
            // (replacing the former native `main.js` launcher). Its `?t=` token
            // authorizes the launcher's `/api/library/*` calls. Built here rather
            // than declared statically because the loopback address is dynamic and
            // is only known after the embedded server starts above.
            //
            // Closing it via the red traffic light or Cmd+W hides, not destroys:
            // hidden serve children keep the process alive, and reopening via Dock
            // click or the Window > Workspaces menu item is instant.
            if let Some(embedded) = state_for_setup.embedded.get() {
                let launcher_url =
                    format!("http://{}/?t={}", embedded.addr(), embedded.launcher_token());
                match launcher_url.parse::<tauri::Url>() {
                    Ok(url) => {
                        match WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url))
                            .title(LAUNCHER_WINDOW_TITLE)
                            .inner_size(960.0, 600.0)
                            .min_inner_size(720.0, 400.0)
                            .resizable(true)
                            .build()
                        {
                            Ok(main) => {
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
                            Err(e) => {
                                tracing::warn!(error = %e, "building the launcher window failed")
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, url = %launcher_url, "bad launcher window URL")
                    }
                }
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
                        "Auto-refresh disabled; close and reopen the window after running chan workspace add.",
                    );
                }
            }

            // CLI-to-desktop handoff listener (ratified Option B). Binds the
            // well-known per-user endpoint (a UDS on unix, a named pipe on
            // Windows) so a `chan open <workspace>` in a terminal hands the
            // workspace to this desktop window instead of failing on the
            // per-workspace flock. Leaked for the process lifetime (the registry
            // watcher above uses the same Box::leak pattern; the handle's Drop
            // tears down the listener, but we want it live until exit, and
            // RunEvent::Exit tears the process down anyway). A bind failure is
            // non-fatal: the CLI just falls back to its own server.
            #[cfg(any(unix, windows))]
            if let Some(sock) = chan_server::handoff::well_known_socket_path() {
                let app_for_handoff = app.handle().clone();
                let state_for_handoff = Arc::clone(&state_for_setup);
                // `start_listener` binds a tokio listener (UnixListener /
                // named-pipe server) and `tokio::spawn`s the accept loop, so it
                // MUST run inside a tokio runtime context. The Tauri `setup` runs
                // on the main thread OUTSIDE any runtime, so calling it
                // directly panics ("there is no reactor running"), which
                // aborts the whole desktop on launch. Enter the Tauri-
                // managed runtime via `block_on` (the same runtime the
                // embedded server above and every `async_runtime::spawn`
                // below use) so the bind + the spawned accept loop attach
                // to it and survive after this returns.
                let listener = tauri::async_runtime::block_on(async {
                    chan_server::handoff::start_listener(sock, move |req| {
                        let app = app_for_handoff.clone();
                        let state = Arc::clone(&state_for_handoff);
                        async move {
                            use chan_server::handoff::{Capabilities, Request, Response, CHAN_VERSION};
                            match req {
                                Request::OpenWorkspace { workspace_path, .. } => {
                                    match open_workspace_from_handoff(
                                        app,
                                        state,
                                        PathBuf::from(workspace_path),
                                    ) {
                                        Ok(()) => Response::Opened {
                                            desktop_version: CHAN_VERSION.into(),
                                            capabilities: Capabilities {
                                                open_local_workspace: true,
                                            },
                                        },
                                        Err(message) => Response::Error { message },
                                    }
                                }
                                Request::Upgrade { check_only, .. } => {
                                    desktop_handle_upgrade(app, check_only).await
                                }
                                Request::OpenDevserver {
                                    url, name, script, ..
                                } => match register_devserver_from_handoff(
                                    &state, url, name, script,
                                ) {
                                    Ok(()) => {
                                        let _ = app.emit(serve::SERVES_CHANGED, ());
                                        Response::DevserverRegistered {
                                            desktop_version: CHAN_VERSION.into(),
                                        }
                                    }
                                    Err(message) => Response::Error { message },
                                },
                            }
                        }
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

            // Boot matrix. Every window is a library registry row, so boot is
            // uniform: mount the shared terminal tenant (so persisted terminal
            // windows resolve a live prefix/token and the watcher reopens them),
            // re-serve the workspaces the user left on (each mount lets the watcher
            // reopen that workspace's persisted windows at their stable window_id),
            // and let the library's first-open rule mint one boot terminal only on
            // a truly fresh library (empty registry, marker unset).
            let handle = app.handle().clone();
            let state_for_restore = Arc::clone(&state_for_setup);
            tauri::async_runtime::spawn(async move {
                // Mount the shared terminal tenant FIRST so persisted terminal
                // windows resolve and the watcher reopens them on relaunch.
                if let Some(embedded) = state_for_restore.embedded() {
                    if let Err(e) = embedded.open_terminal().await {
                        tracing::warn!(error = %e, "mounting the shared terminal tenant on boot failed");
                    }
                }
                // Re-serve each workspace that was on at the last clean shutdown,
                // read from the library-owned workspace overlay. Serial so
                // concurrent opens can't race the shared embedded host; on a
                // failure surface a notice and leave it off (the key drops out of
                // the overlay on the next clean shutdown).
                let enabled: Vec<String> = state_for_restore
                    .embedded()
                    .and_then(|embedded| embedded.workspace_overlay())
                    .map(|overlay| overlay.on_paths())
                    .unwrap_or_default();
                for key in enabled {
                    if let Err(e) =
                        serve::start(handle.clone(), Arc::clone(&state_for_restore), key.clone())
                            .await
                    {
                        tracing::warn!(key = %key, error = %e, "restoring enabled workspace failed");
                        emit_system_notice(
                            &handle,
                            "warning",
                            format!("Could not re-open workspace {key}: {e}"),
                        );
                    }
                }
                // First-open rule (library-owned): the very first time this local
                // library is opened with an empty registry, mint one boot terminal
                // and persist a marker. Once set, an emptied registry never
                // re-mints — the user who closed their only terminal reopens to
                // none. Persisted windows restore via the watcher independently.
                if let Some(embedded) = state_for_restore.embedded() {
                    if let Err(e) = embedded.ensure_first_open_terminal() {
                        tracing::warn!(error = %e, "ensuring the boot terminal failed");
                    }
                }
            });

            // On-launch self-update check: a running stale desktop updates
            // itself instead of only on a hand `chan upgrade`. Spawns its own
            // background task; honors CHAN_UPDATE_CHECK=0.
            spawn_launch_update_check(app.handle().clone());

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
            // Registered on every platform; returns [] off macOS so the
            // SPA's terminal drop handler needs no platform branching.
            // ACL-scoped to locally-served windows (capabilities/
            // local-drop.json) — the drag pasteboard is system-wide.
            dropped_paths::read_dropped_paths,
            // Native vector PDF export. macOS-only: WKWebView's `createPDF`
            // has no Linux/Windows equivalent wired, and the SPA hides the
            // "Export to PDF" button off-macOS so this is never invoked
            // there.
            #[cfg(target_os = "macos")]
            pdf::export_pdf_macos,
            zoom_in,
            zoom_out,
            zoom_reset,
            open_local_workspace,
            probe_url,
            add_outbound_workspace,
            open_outbound_workspace,
            remove_outbound_workspace,
            add_devserver,
            list_devservers,
            remove_devserver,
            disconnect_devserver,
            list_devserver_workspaces,
            open_devserver_workspace,
            open_devserver_terminal,
            reconnect_devserver,
            update_devserver,
            forget_devserver_workspace,
            set_devserver_workspace_on,
            auth::auth_status,
            auth::open_signin,
            auth::signout,
        ])
        .build(tauri::generate_context!())
        .expect("error building tauri application");

    app.run(move |_app, event| {
        match event {
            // Backstop for exit paths that do not come through the
            // chan-quit menu item (which already confirmed via
            // `request_quit`). NOTE this arm alone proved insufficient
            // for Cmd+Q: the macOS PREDEFINED Quit item exits through a
            // flow `prevent_exit` cannot reliably stop, so the menu now
            // carries a custom Quit item that asks BEFORE any exit is
            // requested. Kept for the code-None flows (e.g. last window
            // destroyed) where prevention does work.
            RunEvent::ExitRequested { api, .. } => {
                use std::sync::atomic::Ordering;
                if state_for_exit.quit_confirmed.load(Ordering::SeqCst) {
                    return; // user already confirmed; let the exit run
                }
                let open = _app
                    .webview_windows()
                    .into_keys()
                    .filter(|l| serve::is_workspace_webview_label(l))
                    .count();
                if open == 0 {
                    return; // nothing worth guarding; quit as before
                }
                api.prevent_exit();
                request_quit(_app);
            }
            RunEvent::Exit => {
                // Persist the on-set BEFORE teardown drains it, so the
                // next boot re-serves exactly the workspaces that were
                // on at this clean shutdown (the §3.2 boot matrix).
                persist_workspaces(&state_for_exit);
                // Best-effort: unmount every embedded local workspace
                // before the desktop runtime exits.
                serve::stop_all(&state_for_exit);
                // Explicitly reap any devserver connect-script tenants (the
                // control terminals): stop_all only unmounts workspaces, so
                // without this their PTYs would ride process-death SIGHUP
                // rather than a deterministic kill. Mirrors the
                // disconnect/forget teardown. Best-effort; the prefixes are
                // collected before the close calls so the lock isn't held
                // across them.
                if let Some(embedded) = state_for_exit.embedded.get() {
                    let prefixes: Vec<String> = state_for_exit
                        .control_terminal_prefixes
                        .lock()
                        .unwrap()
                        .values()
                        .cloned()
                        .collect();
                    for prefix in prefixes {
                        let _ = embedded.close_control_terminal(&prefix);
                    }
                }
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
    // New Window opens another window of the FOCUSED window's
    // connection (open_new_window_for_focused_workspace): local
    // workspace or outbound remote, or another standalone
    // terminal window; with the launcher (or nothing) focused it opens
    // a standalone terminal window — the launcher itself is a
    // singleton and is never multiplied. Convention for future
    // chan-desktop shortcuts: declare a MenuItemBuilder here with the
    // `CmdOrCtrl+<key>` accelerator, add it to the Window submenu, and
    // add a matching `on_menu_event` branch.
    // `CmdOrCtrl+Shift+N` (not plain Cmd+N) so the
    // SPA's New Draft handler can claim Cmd+N without the menu
    // accelerator intercepting first. Menu label stays "New Window".
    let new_window = MenuItemBuilder::with_id("app-new-window", "New Window")
        .accelerator("CmdOrCtrl+Shift+N")
        .build(app)?;
    // File ▸ New Terminal. Cmd+T, ALWAYS enabled on both
    // platforms (no dynamic enable/disable: a disabled menu item still
    // swallows the accelerator on macOS, so a launcher-focused Cmd+T would
    // dead-end). The single handler routes by the FOCUSED window's kind: a
    // launcher (main / main-*) opens a new standalone terminal window; any
    // embedded SPA window (workspace-* / outbound-* / terminal-*)
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
        // File ▸ Close Window. A CUSTOM item (not the predefined
        // close_window) carrying Cmd+W, routed by `handle_close_window`: a
        // focused workspace webview closes the active TAB (dispatching the same
        // `app.tab.close` the KEY_BRIDGE_JS KeyW case fires), while the launcher
        // (`main`) and other plain windows close natively. The accelerator
        // pre-empts the webview on macOS, so the KEY_BRIDGE_JS KeyW case is
        // harmlessly shadowed here (same arrangement as New Terminal's Cmd+T).
        let close_window = MenuItemBuilder::with_id("app-close-window", "Close Window")
            .accelerator("CmdOrCtrl+W")
            .build(app)?;
        // macOS `Menu::default` ALREADY ships a
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
        //
        // The predefined QUIT is replaced the same way: it exits through
        // a flow `ExitRequested` + `prevent_exit` cannot reliably stop,
        // so the v0.31.0 quit-confirmation dialog never appeared. Our
        // custom item keeps Cmd+Q but routes through `request_quit`,
        // which asks BEFORE any exit is requested. Appended (not
        // prepended) so Quit stays at the App menu's bottom.
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
                            let text = text.to_lowercase();
                            if text.contains("about") || text.contains("quit") {
                                let _ = app_submenu.remove(&item);
                            }
                        }
                    }
                }
            }
            let quit = MenuItemBuilder::with_id("chan-quit", "Quit Chan")
                .accelerator("CmdOrCtrl+Q")
                .build(app)?;
            app_submenu.append(&quit)?;
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
    // Exit at all. The custom item routes through `request_quit` (confirm
    // while windows exist). Undo/Redo are likewise GTK-unsupported (dropped, and they
    // would orphan a leading separator), so Edit sticks to the four clipboard
    // items muda does implement on GTK.
    #[cfg(not(target_os = "macos"))]
    let menu = {
        use tauri::menu::{MenuBuilder, SubmenuBuilder};
        let about = MenuItemBuilder::with_id("chan-about", "About Chan").build(app)?;
        let quit = MenuItemBuilder::with_id("chan-quit", "Quit")
            .accelerator("CmdOrCtrl+Q")
            .build(app)?;
        // Close Window on Linux/Windows rides Ctrl+Shift+W (plain
        // Ctrl+W stays a terminal readline chord). Same routed handler
        // as macOS's Cmd+W item: tab-close in SPA windows,
        // cancel-close on the connecting screen, native close
        // elsewhere. KEY_BRIDGE_JS claims the same chord inside SPA
        // webviews, mirroring the macOS menu/bridge shadow pair.
        let close_window = MenuItemBuilder::with_id("app-close-window", "Close Window")
            .accelerator("CmdOrCtrl+Shift+W")
            .build(app)?;
        let file = SubmenuBuilder::new(app, "File")
            .item(&new_terminal)
            .item(&close_window)
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
            "app-close-window" => {
                handle_close_window(app);
            }
            "chan-about" => {
                if let Err(e) = open_about_window(app) {
                    tracing::warn!(error = %e, "open about window failed");
                }
            }
            // Cross-platform: the custom Quit item (Cmd/Ctrl+Q) asks
            // BEFORE exiting while SPA windows are open or hidden.
            "chan-quit" => {
                request_quit(app);
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
                // Buried headers are now one per group (local + each
                // devserver), so match the header id by prefix.
                if id.starts_with(BURIED_MENU_HEADER_ID)
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
        // Group the hidden windows by the devserver that opened them, so a
        // user with several devservers can tell their windows apart; a window
        // tracked under no devserver is local. The devserver's tracked window
        // labels (plus its control terminal) are the membership test.
        let cfg = state.store.lock().unwrap().get().ok();
        let mut devservers: Vec<(String, String, std::collections::HashSet<String>)> = {
            let tracked = state.devserver_windows.lock().unwrap();
            tracked
                .iter()
                .map(|(ds_id, windows)| {
                    let display = cfg
                        .as_ref()
                        .and_then(|c| c.devservers.iter().find(|d| &d.id == ds_id))
                        .map(devserver_display)
                        .unwrap_or_else(|| ds_id.clone());
                    let mut labels: std::collections::HashSet<String> =
                        windows.iter().map(|w| w.label.clone()).collect();
                    labels.insert(serve::control_terminal_label(ds_id));
                    (ds_id.clone(), display, labels)
                })
                .collect()
        };
        devservers.sort_by(|a, b| a.1.cmp(&b.1));

        let mut local: Vec<(String, String)> = Vec::new();
        let mut grouped: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for (label, title) in buried {
            match devservers.iter().find(|(_, _, labels)| labels.contains(&label)) {
                Some((ds_id, _, _)) => grouped
                    .entry(ds_id.clone())
                    .or_default()
                    .push((label, title)),
                None => local.push((label, title)),
            }
        }

        // Count + cost hint in the header: buried webviews stay live (warm
        // layout, running terminals), which is memory the user can't see.
        if !local.is_empty() {
            append_section(
                BURIED_MENU_HEADER_ID,
                &format!("Hidden Windows ({}, kept warm in memory)", local.len()),
                &local,
                BURIED_MENU_ID_PREFIX,
            );
        }
        for (ds_id, display, _) in &devservers {
            if let Some(rows) = grouped.get(ds_id) {
                append_section(
                    &format!("{BURIED_MENU_HEADER_ID}-{ds_id}"),
                    &format!("{display} hidden windows ({})", rows.len()),
                    rows,
                    BURIED_MENU_ID_PREFIX,
                );
            }
        }
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
/// leaves that connection out this round. Triggers: an outbound
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
        // (id, display name, live conn) for each CONNECTED devserver, so its
        // persisted-but-closed windows become reopen entries below (L10).
        let mut devserver_targets: Vec<(String, String, devserver::DevserverConn)> = Vec::new();
        let cfg = {
            let store = state.store.lock().unwrap();
            store.get().ok()
        };
        if let Some(cfg) = cfg {
            for o in &cfg.outbound {
                conns.push(Conn {
                    family: format!("{}-", serve::outbound_window_prefix(&o.id)),
                    url: o.url.clone(),
                    base_title: serve::remote_window_title(&o.url),
                    config_key: config::remote_window_key(&o.id),
                    connecting: true,
                });
            }
            for d in &cfg.devservers {
                // `devservers.get` returns Some only for a CONNECTED devserver.
                if let Some(conn) = state.devservers.get(&d.id) {
                    devserver_targets.push((d.id.clone(), devserver_display(d), conn));
                }
            }
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
                        devserver: None,
                    },
                );
            }
        }
        // L10: a connected devserver's CLOSED-but-persisted windows
        // (`saved && !connected`) are reopenable from the Window menu. The URL
        // is re-minted with the devserver's CURRENT per-mount token
        // (`assemble_tenant_url`); an OFF tenant (empty token) is not
        // menu-reopenable here — its launcher row turns it back on. The reopen
        // (`open_remote_window_from_menu`) re-creates AND re-tracks the window so
        // a later disconnect tears it down.
        for (id, display, conn) in devserver_targets {
            let rows = match devserver::fetch_devserver_windows(&conn).await {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::warn!(devserver = %id, error = %e, "remote windows poll: listing devserver windows failed");
                    continue;
                }
            };
            for row in rows {
                if !(row.saved && !row.connected && !row.token.is_empty()) {
                    continue;
                }
                let url = match devserver::assemble_tenant_url(
                    &conn.host,
                    conn.port,
                    &row.prefix,
                    &row.token,
                ) {
                    Ok(url) => url,
                    Err(e) => {
                        tracing::warn!(devserver = %id, label = %row.label, error = %e, "assembling a devserver reopen url failed");
                        continue;
                    }
                };
                let tail = row
                    .title
                    .clone()
                    .unwrap_or_else(|| remote_window_tail(&row.label));
                map.insert(
                    row.label.clone(),
                    RemoteReopen {
                        url,
                        base_title: row.title.unwrap_or_else(|| display.clone()),
                        menu_title: format!("{display} — {tail}"),
                        config_key: config::remote_window_key(&row.prefix),
                        // Workspace reopen routes through the connecting screen,
                        // like the reconnect path.
                        connecting: true,
                        devserver: Some(DevserverReopen {
                            id: id.clone(),
                            prefix: row.prefix,
                        }),
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
    // A CLOSED devserver WORKSPACE window (L10): re-create it at its label AND
    // re-track it under the devserver so a later disconnect tears it down. It
    // reuses the outbound reopen (connecting screen, like reconnect).
    if let Some(ds) = entry.devserver.clone() {
        let tracked = serve::reopen_remote_window(app, label, &entry).map(|()| DevserverWindow {
            window_id: ds.prefix.clone(),
            label: label.to_string(),
            prefix: Some(ds.prefix.clone()),
        });
        match tracked {
            Ok(window) => {
                let state = app.state::<Arc<AppState>>();
                track_devserver_window(&state, &ds.id, window);
            }
            Err(e) => tracing::warn!(label, error = %e, "reopening devserver window failed"),
        }
        return;
    }
    if let Err(e) = serve::reopen_remote_window(app, label, &entry) {
        tracing::warn!(label, error = %e, "reopening remote window failed");
    }
}

/// Re-show a buried window and drop it from the registry + menu.
/// Returns `false` when the label no longer names a live window (it
/// was destroyed underneath; the registry entry is cleaned up either
/// way).
pub fn unbury_window(app: &tauri::AppHandle, label: &str) -> bool {
    let state = app.state::<Arc<AppState>>();
    let removed = state.remove_buried(label);
    // A watcher-managed local window: un-bury through the view state. The bury
    // destroyed the native window (the reconcile closed it), so there is nothing
    // to show() — the reconcile reopens it at its window_id. Counts as shown.
    if label.starts_with("local::") {
        if let Some(view) = state.local_watcher_view() {
            view.unbury(label);
        }
        if removed {
            rebuild_window_menu(app);
        }
        return true;
    }
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
/// third-party attributions); the macOS system About panel is redirected
/// here so all platforms share one surface.
/// Singleton: focus an existing About window instead of stacking copies.
/// The desktop version is passed as a query param so `about.html` needs no
/// `app`-plugin capability just to render it.
fn open_about_window(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("about") {
        let _ = win.set_focus();
        return Ok(());
    }
    let version = app.package_info().version.to_string();
    let win = WebviewWindowBuilder::new(
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
    // Off macOS the app menu renders as a per-window GTK menubar, and a
    // File/Edit/Window bar on a fixed-size About dialog is noise (and
    // eats its height). macOS keeps the global menubar — nothing to
    // remove there. Best-effort: a failure just leaves the bar.
    #[cfg(not(target_os = "macos"))]
    let _ = win.remove_menu();
    #[cfg(target_os = "macos")]
    let _ = win;
    Ok(())
}

/// Open a new window of the workspace that owns the currently
/// focused window (the Cmd/Ctrl+Shift+N "New Window" semantics).
///
/// Window labels are `workspace-<hash(key)>-<seq>` and the hash is
/// one-way, so we recover the workspace key by matching
/// `serve::workspace_window_prefix(key)` against the focused window's
/// label across the running `serves` map, then mint another window for
/// it (the watcher opens it), like `open_local_workspace`.
///
/// A focused `outbound-*` window opens a new window on the
/// SAME remote (the connection is recovered from the label's hash
/// prefix against the outbound attachments). With the
/// launcher (or nothing) focused, Cmd/Ctrl+Shift+N opens a standalone
/// terminal window instead — the launcher is a singleton, never
/// multiplied. The "Workspaces" picker stays reachable via the
/// `win-main` menu item, which is also the fallback surface when a
/// focused window's backing connection can't be resolved (stale
/// window for a forgotten attachment).
fn open_new_window_for_focused_workspace(app: &tauri::AppHandle) -> Result<(), String> {
    // Buried workspace-/outbound- windows take precedence in their family:
    // Cmd+Shift+N on a window whose family has a hidden sibling REOPENS that
    // sibling (most recent first) instead of spawning a fresh window. Local
    // `local::` windows are independent registry records — no family unbury;
    // a focused one mints/opens a fresh window (branched on kind below), and a
    // focused launcher (or nothing) opens a standalone terminal.
    let Some(focused) = app
        .webview_windows()
        .into_values()
        .find(|w| serve::is_workspace_webview_label(w.label()) && w.is_focused().unwrap_or(false))
    else {
        // Launcher (or nothing) focused: New Window means a standalone terminal.
        spawn_terminal_window(app);
        return Ok(());
    };
    let focused_label = focused.label().to_string();
    let state = app.state::<Arc<AppState>>();
    // A watcher-opened local window (`local::<window_id>`): branch on the
    // focused window's KIND. A terminal opens ANOTHER standalone terminal; a
    // workspace mints another window for the same workspace (the watcher opens
    // it). Each minted window is an independent registry record, so there is no
    // `<kind>-<hash>-<seq>` family to unbury (unlike the schemes below).
    // (A Terminal record carries no `workspace_path`, so keying on that — the
    // old code — fell through to the launcher: the #2 bug.)
    if focused_label.starts_with("local::") {
        let record = state.embedded().and_then(|embedded| {
            embedded
                .assemble_window_records()
                .into_iter()
                .find(|r| crate::window_watcher::native_label(r) == focused_label)
        });
        return match record {
            Some(r) if r.kind == chan_server::WindowKind::Terminal => {
                spawn_terminal_window(app);
                Ok(())
            }
            Some(r) => state
                .embedded()
                .ok_or_else(|| "embedded local server is unavailable".to_string())?
                .mint_window(chan_server::WindowKind::Workspace, r.workspace_path)
                .map(|_| ()),
            None => show_window(app, "main"),
        };
    }
    // A watcher-opened devserver window (`lib-<library_id>::<window_id>`): mint
    // ANOTHER window of the SAME kind on the same devserver, mirroring the
    // `local::` branch (a Terminal opens another standalone terminal; a Workspace
    // another window for its workspace). There is no stored library_id->devserver
    // map, so the async helper matches the focused label against each connected
    // devserver's feed (which hands back the focused window's kind +
    // workspace_path). It is an HTTP round-trip, so fire-and-forget — a failure
    // surfaces as a warning, not a blocked menu handler. (Without this a `lib-`
    // label matches no branch below and falls through to `show_window("main")` —
    // Cmd+Shift+N on a devserver window jumps focus back to the launcher.)
    if focused_label.starts_with("lib-") {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = mint_another_devserver_window(&app, &focused_label).await {
                tracing::warn!(label = %focused_label, error = %e, "Cmd+Shift+N on a devserver window failed");
            }
        });
        return Ok(());
    }
    // Family unbury first: workspace- and outbound- windows all
    // group by their `<kind>-<16hex>-` label prefix.
    if let Some(buried) = state.most_recent_buried(window_family_prefix(&focused_label)) {
        if unbury_window(app, &buried) {
            return Ok(());
        }
    }
    if focused_label.starts_with("outbound-") {
        let cfg = state.store.lock().unwrap().get().map_err(err)?;
        // New window on the SAME outbound remote: recover the attachment
        // by matching the focused label's hash prefix (labels are
        // `outbound-<hash(id)>-<seq>`; the hash is one-way).
        for o in &cfg.outbound {
            let prefix = serve::outbound_window_prefix(&o.id);
            if focused_label.starts_with(&format!("{prefix}-")) {
                return serve::spawn_remote_workspace_window(app, &o.id, &o.url).map(|_| ());
            }
        }
        // Stale window for a forgotten attachment: surface the picker.
        return show_window(app, "main");
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
        // Mint another window for the workspace; the watcher opens it.
        Some((key, _url)) => state
            .embedded()
            .ok_or_else(|| "embedded local server is unavailable".to_string())?
            .mint_window(chan_server::WindowKind::Workspace, Some(key))
            .map(|_| ()),
        // Workspace runtime gone under a live window: surface the picker.
        None => show_window(app, "main"),
    }
}

/// Mint another window for the devserver window that owns `focused_label`
/// (a `lib-<library_id>::<window_id>` watcher window), for Cmd+Shift+N. No stored
/// `library_id -> devserver` map exists, so match the focused label against each
/// connected devserver's library feed; the matching record yields the conn AND
/// the focused window's kind + `workspace_path`. Mint the SAME kind on that conn —
/// the watcher opens it — mirroring the `local::` New-Window behavior. A stale
/// window whose devserver is gone falls back to the picker.
async fn mint_another_devserver_window(
    app: &tauri::AppHandle,
    focused_label: &str,
) -> Result<(), String> {
    let state = app.state::<Arc<AppState>>();
    // Snapshot the ids under the lock, then release it before the awaits.
    let devserver_ids: Vec<String> = {
        let cfg = state.store.lock().unwrap().get().map_err(err)?;
        cfg.devservers.iter().map(|ds| ds.id.clone()).collect()
    };
    for id in devserver_ids {
        let Some(conn) = state.devservers.get(&id) else {
            continue;
        };
        let Ok(windows) = devserver::fetch_library_windows(&conn).await else {
            continue;
        };
        if let Some(record) = windows
            .iter()
            .find(|r| crate::window_watcher::native_label(r) == focused_label)
        {
            // Mirror the focused window's kind. A Terminal record carries no
            // `workspace_path`, so minting a Workspace would make a blank,
            // path-less window; branch on the kind and carry the path only for a
            // Workspace.
            let workspace_path = match record.kind {
                chan_server::WindowKind::Terminal => None,
                chan_server::WindowKind::Workspace => record.workspace_path.clone(),
            };
            return devserver::mint_library_window(&conn, record.kind, workspace_path)
                .await
                .map(|_| ());
        }
    }
    // Stale window for a disconnected/forgotten devserver: surface the picker.
    show_window(app, "main")
}

/// Discard a closed devserver window's record on its owning devserver — the
/// `DELETE` the empty-window close-cascade sends for `lib-` windows (the
/// devserver analog of `embedded.discard_window`). There is no stored
/// library_id->devserver map, so feed-match the focused/closing label to find the
/// conn AND the bare `window_id`, then DELETE. A no-op if the devserver is gone
/// or the record already left the feed.
async fn discard_devserver_window(app: &tauri::AppHandle, label: &str) -> Result<(), String> {
    let state = app.state::<Arc<AppState>>();
    let devserver_ids: Vec<String> = {
        let cfg = state.store.lock().unwrap().get().map_err(err)?;
        cfg.devservers.iter().map(|ds| ds.id.clone()).collect()
    };
    for id in devserver_ids {
        let Some(conn) = state.devservers.get(&id) else {
            continue;
        };
        let Ok(windows) = devserver::fetch_library_windows(&conn).await else {
            continue;
        };
        if let Some(record) = windows
            .iter()
            .find(|r| crate::window_watcher::native_label(r) == label)
        {
            return devserver::discard_library_window(&conn, &record.window_id).await;
        }
    }
    Ok(())
}

/// OS window title for the singleton launcher. Launchers are never
/// multiplied anymore (Cmd/Ctrl+Shift+N on the launcher opens a
/// standalone terminal window instead), so there is no `Window N`
/// suffix to disambiguate.
const LAUNCHER_WINDOW_TITLE: &str = "Chan Desktop";

/// Quit, asking first while ANY SPA window is alive — visible or
/// buried (a buried window is a live hidden webview, so one
/// `webview_windows()` scan covers both): quitting silently kills
/// standalone-terminal shells and stops local workspaces. A bare
/// launcher (or About) quits without ceremony.
///
/// The confirmation lives HERE, before any exit is requested, because
/// the macOS predefined Quit item exits through a flow
/// `RunEvent::ExitRequested` + `prevent_exit` cannot reliably stop
/// (the v0.31.0 dialog never appeared). The custom chan-quit menu item
/// (Cmd/Ctrl+Q) routes here on every platform; on Quit the
/// `quit_confirmed` flag lets the resulting `ExitRequested` pass.
fn request_quit(app: &tauri::AppHandle) {
    use std::sync::atomic::Ordering;
    let state = Arc::clone(&app.state::<Arc<AppState>>());
    let open = app
        .webview_windows()
        .into_keys()
        .filter(|l| serve::is_workspace_webview_label(l))
        .count();
    if open == 0 {
        state.quit_confirmed.store(true, Ordering::SeqCst);
        app.exit(0);
        return;
    }
    // One dialog at a time: a second Cmd+Q while the ask is up must
    // not stack another.
    if state.quit_prompt_open.swap(true, Ordering::SeqCst) {
        return;
    }
    let hidden = state.buried_windows.lock().unwrap().len();
    let message = if hidden > 0 {
        format!(
            "Chan has {open} window(s) ({hidden} hidden). Quitting stops their terminals and local workspaces; remote servers keep running."
        )
    } else {
        format!(
            "Chan has {open} window(s) open. Quitting stops their terminals and local workspaces; remote servers keep running."
        )
    };
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
    let app_for_reply = app.clone();
    app.dialog()
        .message(message)
        .title("Quit Chan?")
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Quit".into(),
            "Cancel".into(),
        ))
        .show(move |quit| {
            state.quit_prompt_open.store(false, Ordering::SeqCst);
            if quit {
                state.quit_confirmed.store(true, Ordering::SeqCst);
                app_for_reply.exit(0);
            }
        });
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

/// Route File ▸ New Terminal (Cmd+T) by the focused window's
/// kind.
///
/// - An embedded SPA window (workspace-* / outbound-* /
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

/// Route File ▸ Close Window (Cmd+W) by the focused window's
/// kind, mirroring `handle_new_terminal`.
///
/// - A focused workspace webview (workspace-* / outbound-* /
///   terminal-*) gets `app.tab.close` dispatched — the same CustomEvent the
///   KEY_BRIDGE_JS KeyW case fires — so Cmd+W closes the active tab, not the
///   window.
/// - Any other focused window (the launcher `main` / `main-*`, the About
///   window) is closed natively. The launcher's `CloseRequested` handler
///   intercepts that to hide rather than destroy it, keeping reopen instant.
///
/// Cross-platform: macOS binds Cmd+W; Linux/Windows bind Ctrl+Shift+W
/// (plain Ctrl+W stays a terminal readline chord there).
fn handle_close_window(app: &tauri::AppHandle) {
    let Some(window) = app
        .webview_windows()
        .into_values()
        .find(|w| w.is_focused().unwrap_or(false))
    else {
        return;
    };
    // A control terminal's Cmd+W must fire the abandon/re-run dialog (a
    // connected devserver's control window IS the connection), not bury or
    // close a tab. `control-terminal-*` is not a workspace webview label, so
    // without this it would fall to `window.close()` → the CloseRequested
    // bury-when-connected branch. Route it through `request_close_window`
    // instead: connected → emits `devserver-control-closed` (the survey) +
    // destroys; connecting → destroys (the scrape loop then surveys). This is
    // the macOS half of the control-window close model — the menu accelerator
    // pre-empts the webview, so the SPA's own Mod+W handler never sees it on
    // macOS (it covers web/linux/windows).
    if window.label().starts_with("control-terminal-") {
        let _ = request_close_window(app.clone(), window);
        return;
    }
    if serve::is_workspace_webview_label(window.label()) {
        // A window still on the connecting/retry screen has no tabs to
        // close and nothing to bury: Cmd+W means cancel — destroy for
        // real (destroy skips the bury-on-close handler).
        if serve::window_on_connecting_screen(app, window.label()) {
            let _ = window.destroy();
            return;
        }
        dispatch_to_focused_workspace(app, "app.tab.close");
    } else {
        let _ = window.close();
    }
}

/// Open a standalone terminal-only window. Mounting the
/// embedded tenant is async (`EmbeddedServer::open_terminal`), so this
/// hands off to the Tauri async runtime; a failure surfaces as a system
/// notice rather than blocking the menu-event thread. Mirrors how the
/// IPC commands drive `serve::start`.
fn spawn_terminal_window(app: &tauri::AppHandle) {
    let app_for_task = app.clone();
    let state = Arc::clone(&app.state::<Arc<AppState>>());
    tauri::async_runtime::spawn(async move {
        if let Err(e) = serve::spawn_local_terminal_window(state).await {
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

    #[cfg(target_os = "macos")]
    #[test]
    fn merge_path_dirs_keeps_shell_first_and_dedups() {
        // Shell PATH first, inherited dirs appended once, deduped.
        assert_eq!(merge_path_dirs("/a:/b", "/b:/c"), "/a:/b:/c");
        // Empty segments dropped.
        assert_eq!(merge_path_dirs("/a::/b", ""), "/a:/b");
        assert_eq!(merge_path_dirs("", "/x"), "/x");
        assert_eq!(merge_path_dirs("", ""), "");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn wait_for_child_times_out_and_kills_a_hung_process() {
        use std::time::Duration;
        // A shell that never exits stands in for a broken interactive rc.
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .spawn()
            .expect("spawn sleep");
        // 100ms << 30s, so this deterministically takes the timeout branch.
        assert!(!wait_for_child(&mut child, Duration::from_millis(100)));
        // The child was killed + reaped, so its status is already available.
        assert!(matches!(child.try_wait(), Ok(Some(_))));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn wait_for_child_reports_a_fast_exit() {
        use std::time::Duration;
        let mut child = std::process::Command::new("true")
            .spawn()
            .expect("spawn true");
        assert!(wait_for_child(&mut child, Duration::from_secs(5)));
    }

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
        // Workspace / outbound group per hash segment.
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
    fn quit_is_gated_behind_a_confirmation_while_windows_exist() {
        // Cmd+Q / Quit must prompt while any SPA window (open or
        // buried) exists. The confirmation runs BEFORE any exit is
        // requested (`request_quit` behind the custom chan-quit item):
        // the macOS PREDEFINED Quit exits through a flow prevent_exit
        // cannot reliably stop, so it must be stripped and replaced.
        // concat! so the pins don't match this test's source.
        const MAIN_RS: &str = include_str!("main.rs");
        // The custom item exists with the Cmd+Q accelerator and routes
        // to request_quit; the predefined one is stripped by text.
        assert!(MAIN_RS.contains(concat!("fn request", "_quit(app: &tauri::AppHandle)")));
        assert!(MAIN_RS.contains(r#"accelerator("CmdOrCtrl+Q")"#));
        assert!(MAIN_RS.contains(concat!("text.contains(", "\"quit\")")));
        // The ExitRequested backstop still guards non-menu exit paths.
        assert!(MAIN_RS.contains(concat!("RunEvent::Exit", "Requested { api, .. }")));
        assert!(MAIN_RS.contains(concat!("api.prevent", "_exit();")));
        assert!(MAIN_RS.contains(concat!("quit_", "confirmed.load")));
    }

    #[test]
    fn launcher_is_a_singleton_with_an_unsuffixed_title() {
        // Launchers are never multiplied (Cmd/Ctrl+Shift+N on the
        // launcher opens a standalone terminal window), so the title
        // carries no "Window N" suffix and no main-N spawner exists.
        assert_eq!(LAUNCHER_WINDOW_TITLE, "Chan Desktop");
        // concat! so the pin doesn't match its own assertion source.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(!MAIN_RS.contains(concat!("fn ", "open_new_launcher_window")));
        assert!(!MAIN_RS.contains(concat!("fn ", "next_launcher_label")));
    }
}
