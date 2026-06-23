//! Binds the surface-agnostic window-watcher core ([`crate::window_watcher`])
//! to the live desktop: the local library's in-process window feed and the
//! Tauri native-window surface.
//!
//! [`spawn_local_window_watcher`] runs one [`watch_loop`] for the embedded
//! local library (`"local"`). The feed snapshots
//! [`EmbeddedServer::assemble_window_records`](crate::embedded::EmbeddedServer::assemble_window_records)
//! and wakes on its aggregate change `Notify`; the surface opens windows
//! through [`serve::open_watched_local_window`] (the shared SPA builder) and
//! closes them by destroying the Tauri window. The watcher is inert until a
//! local window is minted (an empty registry reconciles to nothing); routing
//! the window-creation paths through the registry mint makes it the SOLE driver
//! of local windows, so the reconnect/relaunch duplicate class (L0 Bug A) is
//! unreachable by construction.

use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chan_server::{WindowRecord, WindowSet};
use tauri::{AppHandle, Manager};
use tokio::sync::{watch, Notify};

use crate::devserver::DevserverConn;
use crate::window_watcher::{
    native_label, watch_loop, NativeSurface, WatcherViewState, WindowFeed,
};
use crate::{serve, AppState};

/// Library id of the embedded local-disk library (Seam-L scheme).
const LOCAL_LIBRARY_ID: &str = "local";

/// How a watched window opens its SPA — the only library-specific bit of the
/// otherwise surface-agnostic [`TauriNativeSurface`]. Local windows load the
/// in-process loopback library; remote windows load a connected devserver's SPA
/// at `host:port` (through the connecting screen, since the remote may be down).
enum WindowOpener {
    Local {
        addr: SocketAddr,
    },
    Remote {
        host: String,
        port: u16,
        /// Devserver display name for the window title (see `DevserverConn.name`).
        devserver_name: String,
    },
}

impl WindowOpener {
    fn open(&self, app: &AppHandle, record: &WindowRecord) -> Result<(), String> {
        match self {
            WindowOpener::Local { addr } => serve::open_watched_local_window(app, *addr, record),
            WindowOpener::Remote {
                host,
                port,
                devserver_name,
            } => serve::open_watched_remote_window(app, host, *port, devserver_name, record),
        }
    }
}

/// The local library's window-set feed, read in-process from the embedded host.
struct LocalWindowFeed {
    state: Arc<AppState>,
    /// The aggregate change signal, captured once at spawn (a stable `Arc` the
    /// host re-hands on every call, so capturing it once is sufficient).
    change: Arc<Notify>,
}

impl WindowFeed for LocalWindowFeed {
    fn snapshot(&self) -> Vec<WindowRecord> {
        // LOCAL records only (`local_window_records`): the merged set (seam #1)
        // includes devserver windows, but the LOCAL native watcher must only
        // reconcile LOCAL windows — devserver windows are driven by their own
        // per-devserver watcher — else the local reconcile would try to open
        // remote records via the local opener (and trip its same-library assert).
        self.state
            .embedded()
            .map(|embedded| embedded.local_window_records())
            .unwrap_or_default()
    }

    fn change_notify(&self) -> Arc<Notify> {
        self.change.clone()
    }
}

/// The Tauri native-window surface: opens windows via the shared SPA builder,
/// closes them by destroying the OS window, and enumerates the open native
/// windows for a library by their `{library_id}::` label prefix.
struct TauriNativeSurface {
    app: AppHandle,
    opener: WindowOpener,
    /// Labels whose build was dispatched to the Tauri main thread but may not yet
    /// be in `webview_windows()` — the build is async (`open` returns before
    /// `build_workspace_window`'s `run_on_main_thread` closure runs). Tracked and
    /// folded into `open_labels` so a reconcile in the dispatch→build gap treats
    /// the window as already open and does NOT double-`open` the same label (the
    /// TOCTOU that produced "webview label already exists" + a stuck/duplicate
    /// window during the multi-notify boot burst). Self-cleaning: a label that
    /// has landed in `webview_windows` is dropped from the set.
    in_flight: Arc<Mutex<HashSet<String>>>,
}

impl NativeSurface for TauriNativeSurface {
    fn open_labels(&self, library_id: &str) -> HashSet<String> {
        let prefix = format!("{library_id}::");
        let mut labels: HashSet<String> = self
            .app
            .webview_windows()
            .into_keys()
            .filter(|label| label.starts_with(&prefix))
            .collect();
        // A dispatched build that has now landed in `webview_windows` is no
        // longer in-flight; drop it, then fold the still-pending ones in so the
        // reconcile sees them as open.
        let mut in_flight = self.in_flight.lock().unwrap();
        in_flight.retain(|label| !labels.contains(label));
        labels.extend(in_flight.iter().filter(|l| l.starts_with(&prefix)).cloned());
        labels
    }

    fn open(&self, record: &WindowRecord) {
        // Mark the label in-flight BEFORE dispatching the (async) build, so a
        // reconcile that runs before the build lands won't re-open it.
        let label = native_label(record);
        self.in_flight.lock().unwrap().insert(label.clone());
        // The opener dispatches the actual build to the Tauri main thread
        // internally (local or remote builder), so this returns promptly.
        if let Err(e) = self.opener.open(&self.app, record) {
            self.in_flight.lock().unwrap().remove(&label);
            tracing::warn!(
                window = %record.window_id,
                error = %e,
                "window watcher: opening a window failed",
            );
        }
    }

    fn close(&self, label: &str) {
        // No longer in-flight (also covers a close before the build landed).
        self.in_flight.lock().unwrap().remove(label);
        // Destroying a window must run on the Tauri main thread.
        let app = self.app.clone();
        let dispatch = self.app.clone();
        let label_owned = label.to_string();
        let result = dispatch.run_on_main_thread(move || {
            if let Some(window) = app.get_webview_window(&label_owned) {
                let _ = window.destroy();
            }
        });
        if let Err(e) = result {
            tracing::warn!(window = %label, error = %e, "window watcher: closing a local window failed");
        }
    }
}

/// Spawn the local library's window watcher (one [`watch_loop`] for `"local"`),
/// living for the process lifetime. A no-op when the embedded server is not up.
pub(crate) fn spawn_local_window_watcher(app: AppHandle, state: Arc<AppState>) {
    let Some(embedded) = state.embedded() else {
        tracing::warn!("local window watcher not started: embedded server unavailable");
        return;
    };
    let addr = embedded.addr();
    let change = embedded.library_change_notify();
    let feed = LocalWindowFeed {
        state: Arc::clone(&state),
        change,
    };
    let surface = TauriNativeSurface {
        app,
        opener: WindowOpener::Local { addr },
        in_flight: Arc::new(Mutex::new(HashSet::new())),
    };
    let view = Arc::new(WatcherViewState::default());
    // Share the view state so the desktop close handlers can bury/unbury
    // through the watcher (L5), then hand the same Arc to the loop.
    state.set_local_watcher_view(Arc::clone(&view));
    // The local library lives for the whole process, so the watcher is never
    // cancelled — `cancel` is a future that only resolves at process exit
    // (which drops the spawned task).
    tauri::async_runtime::spawn(watch_loop(
        Some(LOCAL_LIBRARY_ID),
        feed,
        surface,
        view,
        std::future::pending::<()>(),
    ));
}

/// A connected devserver's window-set feed, pushed over the
/// `GET /api/library/windows/watch` WebSocket. A background task holds the
/// latest snapshot and wakes the watcher on every push; it reconnects on a
/// dropped socket (resubscribe + the idempotent reconcile self-heals).
struct DevserverWindowFeed {
    snapshot: Arc<Mutex<Vec<WindowRecord>>>,
    change: Arc<Notify>,
}

impl WindowFeed for DevserverWindowFeed {
    fn snapshot(&self) -> Vec<WindowRecord> {
        self.snapshot.lock().unwrap().clone()
    }

    fn change_notify(&self) -> Arc<Notify> {
        self.change.clone()
    }
}

/// The devserver window-feed WS URL. The devserver HTTP base is `http://`
/// (`devserver::base_origin`), so the WS scheme mirrors it (`ws://`; a future
/// `https://` base would yield `wss://`, which rustls covers).
fn watch_ws_url(host: &str, port: u16) -> String {
    format!("ws://{host}:{port}/api/library/windows/watch")
}

/// Stream a devserver's window-set feed into `snapshot` + wake `change` on every
/// push, reconnecting on a dropped socket until `cancel` fires. The server
/// pushes a full snapshot on connect, so a drop self-heals on the next reconcile.
async fn run_devserver_window_feed(
    conn: DevserverConn,
    snapshot: Arc<Mutex<Vec<WindowRecord>>>,
    change: Arc<Notify>,
    state: Arc<AppState>,
    mut cancel: watch::Receiver<bool>,
) {
    const RECONNECT_BACKOFF: Duration = Duration::from_secs(2);
    loop {
        // A `watch` (not a `Notify`) so the cancel PERSISTS: a disconnect that
        // flips it while we are between selects is still seen here, not missed.
        if *cancel.borrow_and_update() {
            return;
        }
        tokio::select! {
            _ = cancel.changed() => return,
            result = stream_window_feed(&conn, &snapshot, &change, &state) => {
                if let Err(e) = result {
                    tracing::debug!(
                        host = %conn.host,
                        error = %e,
                        "devserver window feed disconnected; reconnecting",
                    );
                }
            }
        }
        if *cancel.borrow_and_update() {
            return;
        }
        tokio::select! {
            _ = cancel.changed() => return,
            _ = tokio::time::sleep(RECONNECT_BACKOFF) => {}
        }
    }
}

/// One connection's lifetime: open the `/watch` WS (bearer in the Authorization
/// header — the desktop uses the header, not the `?t=` query a browser needs),
/// then push every `WindowSet` text frame into `snapshot` + wake `change`.
async fn stream_window_feed(
    conn: &DevserverConn,
    snapshot: &Arc<Mutex<Vec<WindowRecord>>>,
    change: &Arc<Notify>,
    state: &Arc<AppState>,
) -> Result<(), String> {
    use futures::StreamExt;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    use tokio_tungstenite::tungstenite::Message;
    let mut request = watch_ws_url(&conn.host, conn.port)
        .into_client_request()
        .map_err(|e| format!("bad watch url: {e}"))?;
    request.headers_mut().insert(
        "Authorization",
        format!("Bearer {}", conn.token)
            .parse()
            .map_err(|e| format!("bad bearer header: {e}"))?,
    );
    let (mut ws, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| format!("connect /watch: {e}"))?;
    while let Some(message) = ws.next().await {
        if let Message::Text(text) = message.map_err(|e| format!("watch stream: {e}"))? {
            if let Ok(set) = serde_json::from_str::<WindowSet>(&text) {
                // Refresh this library's active-transfer cache so the desktop
                // close guard can see a remote window's in-flight transfer (the
                // desktop sees no remote `/ws`; the feed bit is its only signal).
                // The library_id is constant per devserver; an empty snapshot
                // carries none, but then there are no windows to guard either.
                if let Some(library_id) = set.windows.first().map(|r| r.library_id.clone()) {
                    let active: Vec<String> = set
                        .windows
                        .iter()
                        .filter(|r| r.active_transfer)
                        .map(native_label)
                        .collect();
                    state.refresh_devserver_active_transfers(&library_id, &active);
                }
                *snapshot.lock().unwrap() = set.windows;
                // Re-push the launcher feed (seam #1): a devserver window change
                // shifts the merged launcher window set, so signal the embedded
                // host to re-assemble + re-push. The devserver only pushes on a
                // real change, so this is already change-gated.
                if let Some(embedded) = state.embedded() {
                    embedded.signal_library_change();
                }
                change.notify_one();
            }
        }
    }
    Ok(())
}

/// Subscribe to a connected devserver's pane-highlight COLOUR feed
/// (`GET /api/library/local-color/watch`, Theme 6): on each `{ color }` push,
/// refresh the launcher's per-devserver colour cache and — only on a real change
/// — re-push the library feed, so a NEW window of this devserver reads the fresh
/// `?pane=` colour at build. Push-based; replaces the old 5s colour poll (the
/// workspace list stays polled — there is no `workspaces/watch`). Reconnects on a
/// dropped socket until `cancel` flips true (disconnect), like the window feed.
pub(crate) fn spawn_devserver_color_watch(
    state: Arc<AppState>,
    id: String,
    conn: DevserverConn,
    mut cancel: watch::Receiver<bool>,
) {
    const RECONNECT_BACKOFF: Duration = Duration::from_secs(2);
    tauri::async_runtime::spawn(async move {
        loop {
            if *cancel.borrow_and_update() {
                return;
            }
            tokio::select! {
                _ = cancel.changed() => return,
                result = stream_color_feed(&state, &id, &conn) => {
                    if let Err(e) = result {
                        tracing::debug!(
                            devserver = %id,
                            error = %e,
                            "devserver colour feed disconnected; reconnecting",
                        );
                    }
                }
            }
            if *cancel.borrow_and_update() {
                return;
            }
            tokio::select! {
                _ = cancel.changed() => return,
                _ = tokio::time::sleep(RECONNECT_BACKOFF) => {}
            }
        }
    });
}

/// One `{ color }` frame of the devserver colour watch.
#[derive(serde::Deserialize)]
struct LocalColorFrame {
    color: Option<String>,
}

/// One connection's lifetime on the devserver colour watch: open the WS (bearer
/// in the Authorization header, like the window feed), then push every `{ color }`
/// frame into the per-devserver colour cache, re-pushing the launcher feed only on
/// a real change.
async fn stream_color_feed(
    state: &Arc<AppState>,
    id: &str,
    conn: &DevserverConn,
) -> Result<(), String> {
    use futures::StreamExt;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    use tokio_tungstenite::tungstenite::Message;
    let url = format!(
        "ws://{}:{}/api/library/local-color/watch",
        conn.host, conn.port
    );
    let mut request = url
        .into_client_request()
        .map_err(|e| format!("bad colour watch url: {e}"))?;
    request.headers_mut().insert(
        "Authorization",
        format!("Bearer {}", conn.token)
            .parse()
            .map_err(|e| format!("bad bearer header: {e}"))?,
    );
    let (mut ws, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| format!("connect colour watch: {e}"))?;
    while let Some(message) = ws.next().await {
        if let Message::Text(text) = message.map_err(|e| format!("colour watch stream: {e}"))? {
            if let Ok(frame) = serde_json::from_str::<LocalColorFrame>(&text) {
                if state.devserver_feed.set_color(id.to_string(), frame.color) {
                    if let Some(embedded) = state.embedded() {
                        embedded.signal_library_change();
                    }
                }
            }
        }
    }
    Ok(())
}

/// Spawn a connected devserver's window watcher: one [`watch_loop`] driven by the
/// devserver's `/api/library/windows/watch` feed, opening windows as remote SPA
/// webviews. Returns the `cancel` (a `watch::Sender`) — flip it to `true` on
/// disconnect to stop the watcher + its feed task; the watcher itself reconciles
/// its native windows away on cancel (detach, not reap).
///
/// The `library_id` (`lib-<hex>`) is NOT needed up front: an EMPTY feed is valid
/// (a devserver with no windows, or one the user emptied before disconnecting),
/// so the watcher learns the id LAZILY from the first record (`watch_loop`). The
/// initial seed is best-effort — an empty or failed fetch is fine; the `/watch`
/// WS pushes the authoritative snapshot on connect.
pub(crate) async fn spawn_devserver_window_watcher(
    app: AppHandle,
    conn: DevserverConn,
) -> Result<
    (
        watch::Sender<bool>,
        Arc<Mutex<Vec<WindowRecord>>>,
        Arc<WatcherViewState>,
    ),
    String,
> {
    let seed = crate::devserver::fetch_library_windows(&conn)
        .await
        .unwrap_or_default();
    let snapshot = Arc::new(Mutex::new(seed));
    // A handle on the snapshot for the caller's launcher feed (seam #1): the same
    // Arc the feed task mutates, so the launcher reads this devserver's live windows.
    let snapshot_handle = Arc::clone(&snapshot);
    let change = Arc::new(Notify::new());
    let (cancel_tx, cancel_rx) = watch::channel(false);
    let host = conn.host.clone();
    let port = conn.port;
    let devserver_name = conn.name.clone();
    // Shared state so the feed task can refresh the active-transfer cache the
    // close guard reads for this devserver's windows.
    let state = Arc::clone(app.state::<Arc<AppState>>().inner());
    // The WS feed task owns `conn`, pushes changes into `snapshot` + wakes
    // `change`, and stops when `cancel` flips true.
    tauri::async_runtime::spawn(run_devserver_window_feed(
        conn,
        Arc::clone(&snapshot),
        Arc::clone(&change),
        state,
        cancel_rx.clone(),
    ));
    let surface = TauriNativeSurface {
        app,
        opener: WindowOpener::Remote {
            host,
            port,
            devserver_name,
        },
        in_flight: Arc::new(Mutex::new(HashSet::new())),
    };
    let feed = DevserverWindowFeed { snapshot, change };
    let view = Arc::new(WatcherViewState::default());
    // A handle on the view for the caller so the close handler can bury THIS
    // devserver's windows through it (D2): a bury flips `should_show` false and
    // the reconcile CLOSES the webview (drops the `/ws`), so the launcher dot
    // reflects hidden — unlike a bare `window.hide()`, which keeps the `/ws` live.
    let view_handle = Arc::clone(&view);
    let mut cancel_loop = cancel_rx;
    tauri::async_runtime::spawn(watch_loop(None, feed, surface, view, async move {
        while !*cancel_loop.borrow_and_update() {
            if cancel_loop.changed().await.is_err() {
                break;
            }
        }
    }));
    Ok((cancel_tx, snapshot_handle, view_handle))
}
