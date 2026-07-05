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
//! of local windows, so reconnect/relaunch cannot duplicate windows and is
//! unreachable by construction.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chan_server::{WindowRecord, WindowSet};
use tauri::{AppHandle, Manager};
use tokio::sync::{watch, Notify};

use crate::devserver::DevserverConn;
use crate::window_watcher::{
    native_label, watch_loop, NativeSurface, WatchLoopStop, WatcherViewState, WindowFeed,
};
use crate::{serve, AppState};

/// Library id of the embedded local-disk library.
const LOCAL_LIBRARY_ID: &str = "local";

/// How a devserver watcher should stop. Disconnect closes that devserver's
/// native windows; token-rotation handoff retires only the old watcher because a
/// fresh watcher will refresh the same labels in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DevserverWatcherStop {
    Running,
    RetireKeepWindows,
    CloseWindows,
}

impl DevserverWatcherStop {
    pub(crate) fn is_stopped(self) -> bool {
        self != Self::Running
    }

    fn watch_loop_stop(self) -> Option<WatchLoopStop> {
        match self {
            Self::Running => None,
            Self::RetireKeepWindows => Some(WatchLoopStop::KeepWindows),
            Self::CloseWindows => Some(WatchLoopStop::CloseWindows),
        }
    }
}

/// How a watched window opens its SPA — the only library-specific bit of the
/// otherwise surface-agnostic [`TauriNativeSurface`]. Local windows load the
/// in-process loopback library; remote windows load a connected devserver's SPA
/// at `host:port` (through the connecting screen, since the remote may be down).
enum WindowOpener {
    Local {
        addr: SocketAddr,
    },
    Remote {
        base_origin: String,
        /// Devserver display name for the window title (see `DevserverConn.name`).
        devserver_name: String,
    },
}

impl WindowOpener {
    fn open(&self, app: &AppHandle, record: &WindowRecord) -> Result<(), String> {
        match self {
            WindowOpener::Local { addr } => serve::open_watched_local_window(app, *addr, record),
            WindowOpener::Remote {
                base_origin,
                devserver_name,
            } => serve::open_watched_remote_window(app, base_origin, devserver_name, record),
        }
    }

    fn is_remote(&self) -> bool {
        matches!(self, WindowOpener::Remote { .. })
    }

    fn retarget(&self, app: &AppHandle, record: &WindowRecord) -> Result<bool, String> {
        match self {
            WindowOpener::Local { .. } => Ok(false),
            WindowOpener::Remote { base_origin, .. } => {
                serve::retarget_watched_remote_window(app, base_origin, record)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RemoteLaunchKey {
    prefix: String,
    token: String,
    kind: chan_server::WindowKind,
    workspace_path: Option<String>,
    ordinal: u32,
}

impl RemoteLaunchKey {
    fn from_record(record: &WindowRecord) -> Self {
        Self {
            prefix: record.prefix.clone(),
            token: record.token.clone(),
            kind: record.kind,
            workspace_path: record.workspace_path.clone(),
            ordinal: record.ordinal,
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
        // LOCAL records only (`local_window_records`): the merged launcher set
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
    /// Last launch-only state used for remote devserver windows. A devserver
    /// restart keeps the same `{library_id}::{window_id}` label but rotates the
    /// tenant token in the URL, so an existing webview may need an in-place
    /// rebuild even though it is already "open" to the reconciler.
    remote_launches: Arc<Mutex<HashMap<String, RemoteLaunchKey>>>,
}

impl TauriNativeSurface {
    fn remember_remote_launch(&self, label: &str, record: &WindowRecord) {
        if self.opener.is_remote() {
            self.remote_launches
                .lock()
                .unwrap()
                .insert(label.to_string(), RemoteLaunchKey::from_record(record));
        }
    }
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
        } else {
            self.remember_remote_launch(&label, record);
        }
    }

    fn refresh(&self, record: &WindowRecord) {
        if !self.opener.is_remote() {
            return;
        }
        let label = native_label(record);
        let next = RemoteLaunchKey::from_record(record);
        let current = self.remote_launches.lock().unwrap().get(&label).cloned();
        if current.as_ref() != Some(&next) {
            match self.opener.retarget(&self.app, record) {
                Ok(true) => self.remember_remote_launch(&label, record),
                Ok(false) => self.open(record),
                Err(e) => {
                    tracing::warn!(
                        window = %record.window_id,
                        error = %e,
                        "window watcher: retargeting a devserver window failed; rebuilding",
                    );
                    self.open(record);
                }
            }
        }
    }

    fn close(&self, label: &str) {
        // No longer in-flight (also covers a close before the build landed).
        self.in_flight.lock().unwrap().remove(label);
        self.remote_launches.lock().unwrap().remove(label);
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
        remote_launches: Arc::new(Mutex::new(HashMap::new())),
    };
    let view = Arc::new(WatcherViewState::default());
    // Share the view state so the desktop close handlers can bury/unbury
    // through the watcher, then hand the same Arc to the loop.
    state.set_local_watcher_view(Arc::clone(&view));
    // The local library lives for the whole process, so the watcher is never
    // cancelled — `cancel` is a future that only resolves at process exit
    // (which drops the spawned task).
    tauri::async_runtime::spawn(watch_loop(
        Some(LOCAL_LIBRARY_ID),
        feed,
        surface,
        view,
        std::future::pending::<WatchLoopStop>(),
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

/// The raw devserver window-feed WS URL. Gateway-backed devservers use the
/// gateway proxy origin instead.
fn watch_ws_url(host: &str, port: u16) -> String {
    format!("ws://{host}:{port}/api/library/windows/watch")
}

type GatewayWs =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn gateway_ws_request(
    conn: &DevserverConn,
    path: &str,
) -> Result<tokio_tungstenite::tungstenite::handshake::client::Request, String> {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let url = crate::devserver::gateway_ws_url(conn, path)?;
    let mut request = url
        .into_client_request()
        .map_err(|e| format!("bad gateway watch url: {e}"))?;
    request.headers_mut().insert(
        "Cookie",
        crate::devserver::gateway_cookie_header(conn)
            .await?
            .parse()
            .map_err(|e| format!("bad gateway cookie header: {e}"))?,
    );
    Ok(request)
}

fn ws_auth_shaped(e: &tokio_tungstenite::tungstenite::Error) -> bool {
    matches!(
        e,
        tokio_tungstenite::tungstenite::Error::Http(resp)
            if matches!(resp.status().as_u16(), 401 | 404)
    )
}

async fn connect_gateway_ws(conn: &DevserverConn, path: &str) -> Result<GatewayWs, String> {
    let request = gateway_ws_request(conn, path).await?;
    match tokio_tungstenite::connect_async(request).await {
        Ok((ws, _)) => Ok(ws),
        Err(e) if ws_auth_shaped(&e) => {
            crate::devserver::refresh_gateway_session(conn).await?;
            let request = gateway_ws_request(conn, path).await?;
            tokio_tungstenite::connect_async(request)
                .await
                .map(|(ws, _)| ws)
                .map_err(|e| format!("connect gateway watch after refresh: {e}"))
        }
        Err(e) => Err(format!("connect gateway watch: {e}")),
    }
}

/// Stream a devserver's window-set feed into `snapshot` + wake `change` on every
/// push, reconnecting on a dropped socket until `cancel` fires. The server
/// pushes a full snapshot on connect, so a drop self-heals on the next reconcile.
async fn run_devserver_window_feed(
    conn: DevserverConn,
    snapshot: Arc<Mutex<Vec<WindowRecord>>>,
    change: Arc<Notify>,
    state: Arc<AppState>,
    mut cancel: watch::Receiver<DevserverWatcherStop>,
) {
    const RECONNECT_BACKOFF: Duration = Duration::from_secs(2);
    loop {
        // A `watch` (not a `Notify`) so the cancel PERSISTS: a disconnect that
        // flips it while we are between selects is still seen here, not missed.
        if (*cancel.borrow_and_update()).is_stopped() {
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
        if (*cancel.borrow_and_update()).is_stopped() {
            return;
        }
        tokio::select! {
            _ = cancel.changed() => return,
            _ = tokio::time::sleep(RECONNECT_BACKOFF) => {}
        }
    }
}

/// One connection's lifetime: open the `/watch` WS, then push every `WindowSet`
/// text frame into `snapshot` + wake `change`. Raw tunnel devservers auth with a
/// bearer header; gateway devservers auth with the devserver-gate cookie.
async fn stream_window_feed(
    conn: &DevserverConn,
    snapshot: &Arc<Mutex<Vec<WindowRecord>>>,
    change: &Arc<Notify>,
    state: &Arc<AppState>,
) -> Result<(), String> {
    use futures::StreamExt;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    use tokio_tungstenite::tungstenite::Message;
    let mut ws = if conn.gateway.is_some() {
        connect_gateway_ws(conn, "/api/library/windows/watch").await?
    } else {
        let url = watch_ws_url(&conn.host, conn.port);
        let mut request = url
            .into_client_request()
            .map_err(|e| format!("bad watch url: {e}"))?;
        request.headers_mut().insert(
            "Authorization",
            format!("Bearer {}", conn.token)
                .parse()
                .map_err(|e| format!("bad bearer header: {e}"))?,
        );
        tokio_tungstenite::connect_async(request)
            .await
            .map(|(ws, _)| ws)
            .map_err(|e| format!("connect /watch: {e}"))?
    };
    while let Some(message) = ws.next().await {
        if let Message::Text(text) = message.map_err(|e| format!("watch stream: {e}"))? {
            if let Ok(set) = serde_json::from_str::<WindowSet>(&text) {
                let mut windows = set.windows;
                crate::devserver::rewrite_gateway_window_tokens(conn, &mut windows).await?;
                // Refresh this library's active-transfer cache so the desktop
                // close guard can see a remote window's in-flight transfer (the
                // desktop sees no remote `/ws`; the feed bit is its only signal).
                // The library_id is constant per devserver; an empty snapshot
                // carries none, but then there are no windows to guard either.
                if let Some(library_id) = windows.first().map(|r| r.library_id.clone()) {
                    let active: Vec<String> = windows
                        .iter()
                        .filter(|r| r.active_transfer)
                        .map(native_label)
                        .collect();
                    state.refresh_devserver_active_transfers(&library_id, &active);
                }
                *snapshot.lock().unwrap() = windows;
                // Re-push the launcher feed: a devserver window change
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
/// (`GET /api/library/local-color/watch`): on each `{ color }` push,
/// refresh the launcher's per-devserver colour cache and — only on a real change
/// — re-push the library feed, so a NEW window of this devserver reads the fresh
/// `?pane=` colour at build. Push-based; replaces the old 5s colour poll (the
/// workspace list stays polled — there is no `workspaces/watch`). Reconnects on a
/// dropped socket until `cancel` flips true (disconnect), like the window feed.
pub(crate) fn spawn_devserver_color_watch(
    state: Arc<AppState>,
    id: String,
    conn: DevserverConn,
    mut cancel: watch::Receiver<DevserverWatcherStop>,
) {
    const RECONNECT_BACKOFF: Duration = Duration::from_secs(2);
    tauri::async_runtime::spawn(async move {
        loop {
            if (*cancel.borrow_and_update()).is_stopped() {
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
            if (*cancel.borrow_and_update()).is_stopped() {
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

/// One connection's lifetime on the devserver colour watch: raw devservers auth
/// with bearer; gateway devservers auth with the devserver-gate cookie. Each
/// `{ color }` frame refreshes the per-devserver colour cache, re-pushing the
/// launcher feed only on a real change.
async fn stream_color_feed(
    state: &Arc<AppState>,
    id: &str,
    conn: &DevserverConn,
) -> Result<(), String> {
    use futures::StreamExt;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    use tokio_tungstenite::tungstenite::Message;
    let mut ws = if conn.gateway.is_some() {
        connect_gateway_ws(conn, "/api/library/local-color/watch").await?
    } else {
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
        tokio_tungstenite::connect_async(request)
            .await
            .map(|(ws, _)| ws)
            .map_err(|e| format!("connect colour watch: {e}"))?
    };
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
/// webviews. Returns the `cancel` (a `watch::Sender`) — send
/// [`DevserverWatcherStop::CloseWindows`] on disconnect, or
/// [`DevserverWatcherStop::RetireKeepWindows`] for token-rotation handoff.
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
        watch::Sender<DevserverWatcherStop>,
        Arc<Mutex<Vec<WindowRecord>>>,
        Arc<WatcherViewState>,
    ),
    String,
> {
    let seed = crate::devserver::fetch_library_windows(&conn)
        .await
        .unwrap_or_default();
    let snapshot = Arc::new(Mutex::new(seed));
    // A handle on the snapshot for the caller's launcher feed: the same
    // Arc the feed task mutates, so the launcher reads this devserver's live windows.
    let snapshot_handle = Arc::clone(&snapshot);
    let change = Arc::new(Notify::new());
    let (cancel_tx, cancel_rx) = watch::channel(DevserverWatcherStop::Running);
    let base_origin = crate::devserver::conn_base_origin(&conn);
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
            base_origin,
            devserver_name,
        },
        in_flight: Arc::new(Mutex::new(HashSet::new())),
        remote_launches: Arc::new(Mutex::new(HashMap::new())),
    };
    let feed = DevserverWindowFeed { snapshot, change };
    let view = Arc::new(WatcherViewState::default());
    // A handle on the view for the caller so the close handler can bury THIS
    // devserver's windows through it: a bury flips `should_show` false and
    // the reconcile CLOSES the webview (drops the `/ws`), so the launcher dot
    // reflects hidden — unlike a bare `window.hide()`, which keeps the `/ws` live.
    let view_handle = Arc::clone(&view);
    let mut cancel_loop = cancel_rx;
    tauri::async_runtime::spawn(watch_loop(None, feed, surface, view, async move {
        loop {
            let stop = *cancel_loop.borrow_and_update();
            if let Some(stop) = stop.watch_loop_stop() {
                return stop;
            }
            if cancel_loop.changed().await.is_err() {
                return WatchLoopStop::CloseWindows;
            }
        }
    }));
    Ok((cancel_tx, snapshot_handle, view_handle))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec() -> WindowRecord {
        WindowRecord {
            window_id: "w-1".into(),
            library_id: "lib-test".into(),
            kind: chan_server::WindowKind::Terminal,
            title: "Terminal".into(),
            ordinal: 1,
            workspace_path: None,
            prefix: "/terminal".into(),
            token: "tok-1".into(),
            persisted: true,
            connected: false,
            active_transfer: false,
            control: false,
            hidden: false,
            origin: chan_server::WindowOrigin::Native,
        }
    }

    #[test]
    fn remote_launch_key_ignores_feed_status_fields() {
        let a = rec();
        let mut b = a.clone();
        b.connected = true;
        b.active_transfer = true;
        b.control = true;
        b.hidden = true;

        assert_eq!(
            RemoteLaunchKey::from_record(&a),
            RemoteLaunchKey::from_record(&b)
        );
    }

    #[test]
    fn remote_launch_key_tracks_url_and_window_shape_fields() {
        let base = rec();

        let mut token = base.clone();
        token.token = "tok-2".into();
        assert_ne!(
            RemoteLaunchKey::from_record(&base),
            RemoteLaunchKey::from_record(&token)
        );

        let mut prefix = base.clone();
        prefix.prefix = "/other".into();
        assert_ne!(
            RemoteLaunchKey::from_record(&base),
            RemoteLaunchKey::from_record(&prefix)
        );

        let mut workspace = base.clone();
        workspace.kind = chan_server::WindowKind::Workspace;
        workspace.workspace_path = Some("/repo".into());
        assert_ne!(
            RemoteLaunchKey::from_record(&base),
            RemoteLaunchKey::from_record(&workspace)
        );
    }
}
