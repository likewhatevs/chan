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

use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use axum::body::Body;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path as AxumPath, State};
use axum::http::{header, Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use chan_library::{allocate_workspace_prefix, ServeConfig};
use serde::{Deserialize, Serialize};
use tokio::sync::Notify;

use crate::devserver::bytes_eq;
use crate::static_assets::serve_launcher;
use crate::{CreateWindow, WindowRecord, WindowSet, WorkspaceHost};

/// State shared by the `/api/library/workspaces` handlers: the library host plus
/// the surface's serve address. `serve_addr` is the read-only/full discriminator
/// AND the mount enabler:
///   - `Some(cell)` — the desktop loopback (single-user, token-gated). Workspace
///     MUTATION (add/on/off/rm) is served; mounting needs the listen address,
///     which the embedder fills into the `OnceLock` after it binds (the install
///     happens before the bind, so the cell is read at request time).
///   - `None` — the tunnel-trust devserver/gateway surface. Workspaces are
///     READ-ONLY: a grantee holding a `devserver_gate` cookie must not mutate the
///     owner's library, and `bearer=None` can't enforce role. The mutation
///     handlers answer 403 there.
struct LauncherState {
    host: Arc<WorkspaceHost>,
    serve_addr: Option<Arc<OnceLock<SocketAddr>>>,
}

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
pub fn launcher_router(
    host: Arc<WorkspaceHost>,
    bearer: Option<&str>,
    serve_addr: Option<Arc<OnceLock<SocketAddr>>>,
) -> Router {
    // Read-only when there is no serve address (the tunnel-trust devserver/gateway
    // surface): the SPA shell is told to hide the mutation controls, and the
    // mutation handlers answer 403. A serve address marks the loopback surface.
    let read_only = serve_addr.is_none();
    // Windows: list/mint/discard on BOTH surfaces (per-view state, low-risk).
    let windows = Router::new()
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
        .with_state(host.clone());
    // Workspaces: list always; the mutation routes are always present but
    // refuse with 403 on the read-only surface (gated by `serve_addr` inside the
    // handlers), so a direct call can never escalate to mutation.
    let workspaces = Router::new()
        .route(
            "/api/library/workspaces",
            get(handle_list_workspaces).post(handle_add_workspace),
        )
        .route("/api/library/workspaces/:id/on", post(handle_workspace_on))
        .route(
            "/api/library/workspaces/:id/off",
            post(handle_workspace_off),
        )
        .route(
            "/api/library/workspaces/:id",
            delete(handle_remove_workspace),
        )
        .with_state(Arc::new(LauncherState { host, serve_addr }));
    let api = windows.merge(workspaces);
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
    // The static SPA shell is ALWAYS public (loads before it holds the token) and
    // carries the read-only hint so the SPA hides mutation controls on the
    // devserver surface rather than showing buttons that 403.
    Router::new()
        .merge(api)
        .fallback(move |uri| serve_launcher(uri, read_only))
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

// ---------------------------------------------------------------------------
// Workspaces (`/api/library/workspaces`). List today; add/on/off/rm next.
// ---------------------------------------------------------------------------

/// The launcher's workspace row. `workspace_id` is the route prefix without its
/// leading slash — a single legible segment the launcher addresses by
/// (`/api/library/workspaces/{id}/{on|off}`) and treats as opaque; the server
/// owns the scheme. `on` = currently mounted/served. No token: the launcher
/// opens a workspace's tenant separately (which carries its own per-tenant
/// token), so the workspace list never needs one.
#[derive(Serialize)]
struct LauncherWorkspace {
    workspace_id: String,
    path: String,
    label: String,
    on: bool,
}

/// `GET /api/library/workspaces`: one row per registered library workspace (the
/// set `chan list` shows, read live from the host library — the source of
/// truth), each stamped with whether it is currently served (`mounted_prefixes`
/// supplies the live on-state). Sorted by id for a stable list.
async fn handle_list_workspaces(State(state): State<Arc<LauncherState>>) -> Response {
    let host = &state.host;
    let mounted: HashSet<String> = match host.mounted_prefixes() {
        Ok(prefixes) => prefixes.into_iter().collect(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let mut rows: Vec<LauncherWorkspace> = host
        .library()
        .list_workspaces()
        .into_iter()
        .filter_map(|ws| {
            let prefix = allocate_workspace_prefix(&ws.root_path).ok()?;
            Some(LauncherWorkspace {
                workspace_id: prefix.trim_start_matches('/').to_string(),
                path: ws.root_path.to_string_lossy().into_owned(),
                label: workspace_label(&ws.root_path),
                on: mounted.contains(&prefix),
            })
        })
        .collect();
    rows.sort_by(|a, b| a.workspace_id.cmp(&b.workspace_id));
    Json(rows).into_response()
}

/// A workspace's display label: its directory basename. The launcher falls back
/// to the path basename when the label is empty, so the two agree.
fn workspace_label(root: &Path) -> String {
    root.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string()
}

/// The serve address for a mutating workspace handler, or the error response to
/// return instead. Mutation is loopback-only: `serve_addr=None` is the
/// read-only devserver/gateway surface → 403. A present-but-unfilled cell
/// means a request landed before the embedder finished binding → 503 (momentary).
/// The `Response` is boxed to keep the `Err` variant small (`clippy::result_large_err`).
fn require_mutable(state: &LauncherState) -> Result<SocketAddr, Box<Response>> {
    match state.serve_addr.as_ref() {
        None => Err(Box::new(
            (
                StatusCode::FORBIDDEN,
                "workspace mutation is available only on the desktop loopback; manage a devserver's \
                 workspaces from the desktop app or the CLI",
            )
                .into_response(),
        )),
        Some(cell) => match cell.get() {
            Some(addr) => Ok(*addr),
            None => Err(Box::new(
                (StatusCode::SERVICE_UNAVAILABLE, "launcher not ready").into_response(),
            )),
        },
    }
}

/// Resolve a launcher `workspace_id` (the route prefix without its leading slash)
/// to `(prefix, root)` against the live host library, or `None` when no
/// registered workspace maps to it. Mirrors the devserver's stable
/// `allocate_workspace_prefix` mapping.
fn resolve_workspace(host: &WorkspaceHost, id: &str) -> Option<(String, PathBuf)> {
    let prefix = format!("/{id}");
    host.library()
        .list_workspaces()
        .into_iter()
        .map(|ws| ws.root_path)
        .find(|root| allocate_workspace_prefix(root).ok().as_deref() == Some(prefix.as_str()))
        .map(|root| (prefix, root))
}

/// The per-tenant serve config the launcher mounts a workspace with. Mirrors the
/// devserver's `tenant_config`: a tokened tenant (`no_token:false`) at its stable
/// public slug, served in-process under the host's listener.
fn tenant_config(addr: SocketAddr, prefix: &str) -> ServeConfig {
    ServeConfig {
        addr,
        no_token: false,
        prefix: prefix.to_string(),
        idle_timeout: None,
        open_browser: false,
        search_aggression: None,
        settings_disabled: false,
        verbose: false,
    }
}

/// Record a workspace's on-state in the library-owned overlay (keyed by the
/// canonical root path the boot/restore path reads). No-op when no overlay is
/// installed (then on/off does not survive a restart, the host's existing
/// behavior).
fn set_overlay(host: &WorkspaceHost, root: &Path, on: bool) {
    if let Some(overlay) = host.workspace_overlay() {
        overlay.set(&root.to_string_lossy(), on);
    }
}

#[derive(Deserialize)]
struct AddWorkspace {
    path: String,
}

/// `POST /api/library/workspaces` `{path}`: register the local folder in the host
/// library and mount it (on), persisting its on-state. Returns the new row.
/// Loopback-only.
async fn handle_add_workspace(
    State(state): State<Arc<LauncherState>>,
    Json(req): Json<AddWorkspace>,
) -> Response {
    let addr = match require_mutable(&state) {
        Ok(addr) => addr,
        Err(resp) => return *resp,
    };
    let root = Path::new(&req.path);
    let prefix = match allocate_workspace_prefix(root) {
        Ok(prefix) => prefix,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    if let Err(e) = state.host.library().register_workspace(root) {
        return (StatusCode::BAD_REQUEST, e.to_string()).into_response();
    }
    match state
        .host
        .open_or_get_registered_workspace(root, tenant_config(addr, &prefix))
        .await
    {
        Ok(hosted) => {
            set_overlay(&state.host, &hosted.root, true);
            Json(LauncherWorkspace {
                workspace_id: hosted.prefix.trim_start_matches('/').to_string(),
                path: hosted.root.to_string_lossy().into_owned(),
                label: workspace_label(&hosted.root),
                on: true,
            })
            .into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

/// `POST /api/library/workspaces/{id}/on`: mount the registered workspace at its
/// SAME stable prefix (minting a fresh tenant token), persisting on. Loopback-only.
async fn handle_workspace_on(
    State(state): State<Arc<LauncherState>>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    let addr = match require_mutable(&state) {
        Ok(addr) => addr,
        Err(resp) => return *resp,
    };
    let Some((prefix, root)) = resolve_workspace(&state.host, &id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match state
        .host
        .open_or_get_registered_workspace(&root, tenant_config(addr, &prefix))
        .await
    {
        Ok(_) => {
            set_overlay(&state.host, &root, true);
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `POST /api/library/workspaces/{id}/off`: unmount (release the per-workspace
/// flock), keep the registration, persist off. Plain unmount — the
/// confirm-before-off is a launcher-UI concern, not a wire 409. Loopback-only.
async fn handle_workspace_off(
    State(state): State<Arc<LauncherState>>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    if let Err(resp) = require_mutable(&state) {
        return *resp;
    }
    let Some((prefix, root)) = resolve_workspace(&state.host, &id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match state.host.close_workspace(&prefix) {
        Ok(_) => {
            set_overlay(&state.host, &root, false);
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `DELETE /api/library/workspaces/{id}`: unmount if mounted, then UNREGISTER the
/// workspace from the host library (the single registry) so it disappears
/// everywhere. Loopback-only. 404 when no workspace maps to the id.
async fn handle_remove_workspace(
    State(state): State<Arc<LauncherState>>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    if let Err(resp) = require_mutable(&state) {
        return *resp;
    }
    let Some((prefix, root)) = resolve_workspace(&state.host, &id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    // Unmount first (releases the flock before the unregister's reset); a no-op
    // when the workspace is registered-but-off.
    let _ = state.host.close_workspace(&prefix);
    if let Err(e) = state.host.library().unregister_workspace(&root) {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    if let Some(overlay) = state.host.workspace_overlay() {
        overlay.forget(&root.to_string_lossy());
    }
    StatusCode::NO_CONTENT.into_response()
}
