//! The launcher SPA root surface + the `/api/library/*` serve handlers.
//!
//! `web-launcher/` is a pure `/api/library/*` HTTP client served at the
//! devserver/library root `/`. chan-library's `host_dispatch` 404s the root
//! (it only matches workspace-tenant prefixes); this module builds the router
//! the embedder installs as the host's root fallback
//! (`WorkspaceHost::install_root_fallback`) so `/` serves the launcher and
//! `/api/library/*` reaches the library handles. It lives in chan-server, not
//! chan-library, because it serves a frontend bundle and the crate dependency
//! only flows chan-server -> chan-library.
//!
//! ONE bundle, installed on BOTH surfaces — the headless devserver
//! (`build_devserver_app`) and the desktop loopback (`embedded.rs`) — over the
//! shared [`WorkspaceHost`]. The window handlers used to live only in
//! `build_devserver_app`, which the desktop loopback never got, so the desktop
//! launcher would have been blind to its own windows; unifying them here fixes
//! that and removes the double-registration.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path as AxumPath, State};
use axum::http::{header, Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get};
use axum::{Json, Router};
use tokio::sync::Notify;

use crate::devserver::bytes_eq;
use crate::static_assets::serve_launcher;
use crate::{CreateWindow, WindowRecord, WindowSet, WorkspaceHost};

/// The `windows/watch` WS path: the one `/api/library/*` route that accepts the
/// bearer as a `?t=` query param, because a browser WebSocket cannot set an
/// `Authorization` header. Every other route requires the header (a query token
/// leaks through URL logs and the SPA `fetch` can set the header).
const WATCH_WS_PATH: &str = "/api/library/windows/watch";

/// Build the launcher router installed as the [`WorkspaceHost`] root fallback:
/// the static launcher SPA ([`serve_launcher`]) plus the host-backed
/// `/api/library/*` data surface (windows today; workspaces next). One bundle,
/// installed on both surfaces so the launcher is functional everywhere.
///
/// `bearer` is the per-surface launcher token: `Some` gates `/api/library/*` on
/// `Authorization: Bearer <token>` (the watch WS additionally accepts
/// `?t=<token>`); `None` leaves the data surface public (tests / a
/// localhost-trust install). The static SPA shell is ALWAYS public so it can
/// load before it holds the token — the SPA then reads `?t=` from its URL and
/// presents it on every data call.
pub fn launcher_router(host: Arc<WorkspaceHost>, bearer: Option<&str>) -> Router {
    let api = Router::new()
        .route(
            "/api/library/windows",
            get(handle_list_library_windows).post(handle_create_library_window),
        )
        .route(
            "/api/library/windows/watch",
            get(handle_watch_library_windows),
        )
        .route(
            "/api/library/windows/:window_id",
            delete(handle_discard_library_window),
        )
        .with_state(host);
    let api = match bearer {
        Some(token) => {
            let token = token.to_string();
            api.route_layer(middleware::from_fn(move |req, next| {
                let token = token.clone();
                async move { require_launcher_bearer(token, req, next).await }
            }))
        }
        None => api,
    };
    Router::new().merge(api).fallback(serve_launcher)
}

/// Gate `/api/library/*` on the surface's launcher token. The token is accepted
/// in the `Authorization: Bearer` header on every route, and additionally as the
/// `?t=` query param on the watch WebSocket only (a browser WS can't header).
/// The comparison is constant-time so a wrong token leaks no position info.
async fn require_launcher_bearer(token: String, req: Request<Body>, next: Next) -> Response {
    let header_token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let query_token = (req.uri().path() == WATCH_WS_PATH)
        .then(|| req.uri().query().and_then(query_bearer))
        .flatten();
    let expected = token.as_bytes();
    let authorized = header_token.is_some_and(|t| bytes_eq(t.as_bytes(), expected))
        || query_token.is_some_and(|t| bytes_eq(t.as_bytes(), expected));
    if authorized {
        next.run(req).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            "missing or invalid launcher bearer token",
        )
            .into_response()
    }
}

/// The `t` bearer from a URL query string (`...?t=<token>`), for the watch WS
/// where the browser cannot set the `Authorization` header.
fn query_bearer(query: &str) -> Option<&str> {
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == "t").then_some(value)
    })
}

/// `GET /api/library/windows`: the full library window set every client
/// reconciles to. A thin wrapper over the host's shared `assemble_window_records`,
/// which the desktop watcher and `cs window list` also call in-process, so every
/// client reads one assembly with no divergence.
async fn handle_list_library_windows(
    State(host): State<Arc<WorkspaceHost>>,
) -> Json<Vec<WindowRecord>> {
    Json(host.assemble_window_records())
}

/// `GET /api/library/windows/watch`: a WebSocket that pushes the full window set
/// on connect and again on every change, so a client reconciles its surface to
/// the live library state without polling. Bearer-gated by
/// [`require_launcher_bearer`]; a browser WebSocket cannot send the
/// `Authorization` header, so it presents the bearer in the `?t=` query param,
/// while `cs` and the desktop use the header.
async fn handle_watch_library_windows(
    State(host): State<Arc<WorkspaceHost>>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| watch_library_windows(socket, host))
}

/// Push a fresh window-set snapshot on connect and on every change. Sending the
/// whole set rather than a delta keeps the client's reconcile idempotent: a
/// dropped frame self-heals on the next push. The change waiter is armed
/// (`enable`d) BEFORE each snapshot so a change that lands between the snapshot
/// and the await is never missed. The loop ends when the client disconnects.
async fn watch_library_windows(mut socket: WebSocket, host: Arc<WorkspaceHost>) {
    let notify: Arc<Notify> = host.library_change_notify();
    let changed = notify.notified();
    tokio::pin!(changed);
    loop {
        // Arm the change waiter BEFORE the snapshot. A `Notified` records the
        // `notify_waiters` count when it is created, so creating and `enable`-ing
        // it before the snapshot guarantees a change during the snapshot or the
        // `send().await` advances that count and wakes the `select!` below,
        // rather than being read into a snapshot the waiter was armed after. The
        // explicit `enable` also keeps this consumer's ordering identical to the
        // desktop's local watcher.
        changed.as_mut().enable();
        let set = WindowSet {
            windows: host.assemble_window_records(),
        };
        let frame = match serde_json::to_string(&set) {
            Ok(frame) => frame,
            Err(_) => break,
        };
        if socket.send(Message::Text(frame)).await.is_err() {
            break; // the client is gone
        }
        tokio::select! {
            _ = changed.as_mut() => {
                // A window-set change woke us: drop the consumed waiter and
                // re-arm a fresh one, which the next loop turn enables before
                // it reads the snapshot.
                changed.set(notify.notified());
            }
            msg = socket.recv() => match msg {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                _ => {} // ignore any other client frame
            },
        }
    }
}

/// `POST /api/library/windows` `{kind, workspace_path?}`: mint a window. The
/// library assigns the id and persists the record; the registry change bridge
/// fires the watch. Returns the assembled record in the feed shape.
async fn handle_create_library_window(
    State(host): State<Arc<WorkspaceHost>>,
    Json(req): Json<CreateWindow>,
) -> Response {
    match host.mint_window(req.kind, req.workspace_path) {
        Ok(record) => Json(record).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `DELETE /api/library/windows/{window_id}`: discard a window by dropping its
/// record; the change bridge fires the watch, and each client's reconcile then
/// closes the window. `discard_window` reaps the window's own session state
/// (its shared-terminal-tenant session, a workspace tenant's layout blob), so a
/// single registry discard is the authoritative cleanup. 404 when no window has
/// that id.
async fn handle_discard_library_window(
    State(host): State<Arc<WorkspaceHost>>,
    AxumPath(window_id): AxumPath<String>,
) -> Response {
    match host.discard_window(&window_id) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
