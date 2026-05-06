//! HTTP + WebSocket surface for chan.
//!
//! Phase 1 ports the files cluster (`/api/files`, `/api/move`) plus
//! per-launch bearer-token auth from the old `chan-core/src/server.rs`
//! in `fiorix/chan`. Subsequent phases add drive metadata, search,
//! graph, watcher WS, LLM, and the embedded frontend.
//!
//! Auth: every `/api/*` route is gated by a per-launch token. The
//! token is persisted at `<state>/tokens/<drive-key>` (mode 0600 on
//! Unix) so a `cargo build && chan serve` cycle does not invalidate
//! the browser's cached sessionStorage token. Clients pass it as
//! `?t=TOKEN` query string or `Authorization: Bearer TOKEN` header.
//! Pass `--no-token` to disable; loopback bind is the only check
//! left in that mode (test / desktop-shell only).

#![forbid(unsafe_code)]

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{header, HeaderMap, Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chan_core::{
    paths::DrivePaths, Drive, Library, SearchOpts, WatchCallback, WatchEvent, WatchHandle,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tower_http::trace::TraceLayer;

/// Configuration the binary hands the server at boot. Kept terse on
/// purpose; expand only when a route demands it.
#[derive(Debug, Clone)]
pub struct ServeConfig {
    pub addr: SocketAddr,
    /// When false, the server skips the per-launch token gate. Used
    /// by tests and by the desktop shell embedding the server in the
    /// same process. Loopback bind is the only check left; do not
    /// flip this in production.
    pub no_token: bool,
}

/// Resolved at boot for the launch banner / browser handoff.
#[derive(Debug, Clone)]
pub struct ServeHandle {
    pub addr: SocketAddr,
    pub token: Option<String>,
}

impl ServeHandle {
    pub fn launch_url(&self) -> String {
        match &self.token {
            Some(t) => format!("http://{}/?t={}", self.addr, t),
            None => format!("http://{}/", self.addr),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("chan-core: {0}")]
    Core(#[from] chan_core::ChanError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// Spawn the listener, build the router, and serve forever.
/// Returns when the server stops (e.g. on SIGINT).
///
/// `library` is held alongside `drive` so handlers that mutate
/// the registry (rename, etc.) operate against the same state the
/// CLI sees. Both are `Arc`-able and cheap to clone.
pub async fn serve(library: Library, drive: Arc<Drive>, config: ServeConfig) -> Result<(), Error> {
    let listener = TcpListener::bind(config.addr).await?;
    let addr = listener.local_addr()?;
    let token = if config.no_token {
        None
    } else {
        Some(load_or_create_token(drive.paths())?)
    };
    let handle = ServeHandle { addr, token };
    eprintln!("chan listening on {}", handle.launch_url());

    // Filesystem watcher: chan-core's Drive::watch is callback-
    // shaped, so we bridge into a tokio broadcast channel that any
    // number of WebSocket subscribers can read from. The handle
    // must stay alive for the watcher's life; we park it in
    // AppState. Buffer of 256 is enough headroom for typical
    // bursts (mass rename, save-all). Slow subscribers see Lagged
    // and skip ahead rather than blocking the sender.
    let (events_tx, _) = broadcast::channel::<WatchEvent>(256);
    let bridge: Arc<dyn WatchCallback> = Arc::new(WatchBroadcast {
        tx: events_tx.clone(),
    });
    let watch_handle = drive.watch(bridge)?;

    let state = Arc::new(AppState {
        library,
        drive,
        token: handle.token,
        events_tx,
        _watch_handle: watch_handle,
    });
    let app = router(state);
    axum::serve(listener, app)
        .await
        .map_err(|e| Error::Io(std::io::Error::other(e.to_string())))?;
    Ok(())
}

/// Server state shared across all handlers.
struct AppState {
    library: Library,
    drive: Arc<Drive>,
    token: Option<String>,
    events_tx: broadcast::Sender<WatchEvent>,
    /// Held so the underlying notify watcher keeps running for the
    /// server's lifetime. Field is `_`-prefixed because nothing
    /// reads it; dropping AppState drops the handle, which stops
    /// the watcher.
    _watch_handle: WatchHandle,
}

/// Bridge from chan-core's callback-shaped watcher into the
/// broadcast channel that backs every /ws subscriber. Send errors
/// (no current subscribers) are intentionally swallowed; broadcast
/// returns `Err(SendError)` only when the channel is closed, which
/// can't happen while AppState is alive.
struct WatchBroadcast {
    tx: broadcast::Sender<WatchEvent>,
}

impl WatchCallback for WatchBroadcast {
    fn on_event(&self, event: WatchEvent) {
        let _ = self.tx.send(event);
    }
}

fn router(state: Arc<AppState>) -> Router {
    let api = Router::new()
        .route("/api/drive", get(api_get_drive).patch(api_patch_drive))
        .route("/api/cloud-drives", get(api_cloud_drives))
        .route("/api/files", get(api_list_files).post(api_create_file))
        .route(
            "/api/files/*path",
            get(api_read_file)
                .put(api_write_file)
                .delete(api_delete_file),
        )
        .route("/api/move", post(api_move))
        .route("/api/search/files", get(api_search_files))
        .route("/api/search/content", get(api_search_content))
        .route("/api/index/status", get(api_index_status))
        .route("/api/index/rebuild", post(api_index_rebuild))
        .route("/api/link-targets", get(api_link_targets))
        .route("/api/headings/*path", get(api_headings))
        .route("/api/health", get(api_health))
        .route("/ws", get(ws_upgrade));
    Router::new()
        .merge(api)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

// ----- token + auth -------------------------------------------------------

const TOKEN_LEN: usize = 32;
const TOKEN_ALPHABET: &[u8] = b"abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";

fn random_token() -> String {
    let mut bytes = [0u8; TOKEN_LEN];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
        .iter()
        .map(|b| TOKEN_ALPHABET[(*b as usize) % TOKEN_ALPHABET.len()] as char)
        .collect()
}

/// Load the persisted server token, generating one on first run.
/// Lives at `<paths.tokens>/token` (mode 0600 on Unix). The token
/// survives a binary rebuild so the browser's cached sessionStorage
/// token stays valid across `cargo build && chan serve` cycles.
fn load_or_create_token(paths: &DrivePaths) -> std::io::Result<String> {
    std::fs::create_dir_all(&paths.tokens)?;
    let token_path = paths.tokens.join("token");
    if let Ok(s) = std::fs::read_to_string(&token_path) {
        let s = s.trim();
        if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Ok(s.to_owned());
        }
    }
    let token = random_token();
    write_token_atomic(&token_path, &token)?;
    Ok(token)
}

fn write_token_atomic(token_path: &Path, token: &str) -> std::io::Result<()> {
    use std::io::Write;
    let parent = token_path
        .parent()
        .ok_or_else(|| std::io::Error::other("token_path has no parent"))?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(token.as_bytes())?;
    tmp.as_file().sync_all()?;
    tmp.persist(token_path)
        .map_err(|e| std::io::Error::other(e.error.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(token_path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Reject requests that don't carry the right token.
///
/// Auth scope: only `/api/*` routes are gated. Static assets (when
/// they land in a later phase) stay open: the browser issues those
/// via `<script src>` / `<link href>` before our JS runs and they
/// can't carry the token. The data plane is what needs protecting.
async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let Some(expected) = state.token.as_deref() else {
        return next.run(req).await;
    };
    let path = req.uri().path();
    if !(path.starts_with("/api") || path == "/ws") {
        return next.run(req).await;
    }
    if extract_token(req.uri().query(), req.headers()) == Some(expected) {
        return next.run(req).await;
    }
    (StatusCode::UNAUTHORIZED, "missing or invalid token").into_response()
}

fn extract_token<'a>(query: Option<&'a str>, headers: &'a HeaderMap) -> Option<&'a str> {
    if let Some(q) = query {
        for pair in q.split('&') {
            if let Some(rest) = pair.strip_prefix("t=") {
                return Some(rest);
            }
        }
    }
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
}

// ----- handlers -----------------------------------------------------------

async fn api_health() -> Response {
    Json(serde_json::json!({"status": "ok"})).into_response()
}

async fn ws_upgrade(State(state): State<Arc<AppState>>, ws: WebSocketUpgrade) -> Response {
    let rx = state.events_tx.subscribe();
    ws.on_upgrade(move |socket| ws_pump(socket, rx))
}

/// Forward filesystem events to one WebSocket client until either
/// side hangs up. Lagged subscribers (the server fills the buffer
/// faster than the client drains it) skip ahead and continue
/// rather than tearing down the connection.
async fn ws_pump(mut socket: WebSocket, mut rx: broadcast::Receiver<WatchEvent>) {
    loop {
        match rx.recv().await {
            Ok(event) => {
                let json = match serde_json::to_string(&event) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!("ws event serialize error: {e}");
                        continue;
                    }
                };
                if socket.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Closed) => break,
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
        }
    }
}

#[derive(Serialize)]
struct DriveInfo {
    /// User-facing display name from the registry. None when the
    /// drive has no name set; the frontend falls back to the
    /// basename of `root` for display.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// Absolute drive root, POSIX-style on every platform so the
    /// JSON shape stays stable.
    root: String,
    // Future: a `preferences` field carrying app-level prefs (font,
    // theme, attachments dir) once the chan-server preference layer
    // lands. Held off here so we don't ship a partial schema.
}

async fn api_get_drive(State(state): State<Arc<AppState>>) -> Response {
    Json(drive_info(&state)).into_response()
}

#[derive(Deserialize)]
struct PatchDriveBody {
    /// Empty string clears the name (the basename takes over for
    /// display). Field absent in the body is a no-op so the same
    /// PATCH endpoint can grow other fields later without each
    /// caller having to pass them.
    #[serde(default)]
    name: Option<String>,
}

async fn api_patch_drive(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PatchDriveBody>,
) -> Response {
    if let Some(name) = body.name {
        let new_name = if name.is_empty() { None } else { Some(name) };
        if let Err(e) = state.library.rename_drive(state.drive.root(), new_name) {
            return err_from(&e);
        }
    }
    Json(drive_info(&state)).into_response()
}

#[derive(Serialize)]
struct CloudDriveJson {
    provider: String,
    provider_root: String,
    suggested_root: String,
}

async fn api_cloud_drives() -> Response {
    let out: Vec<CloudDriveJson> = chan_core::paths::detected_cloud_drives()
        .into_iter()
        .map(|c| CloudDriveJson {
            provider: c.provider,
            provider_root: c.provider_root.to_string_lossy().into_owned(),
            suggested_root: c.suggested_root.to_string_lossy().into_owned(),
        })
        .collect();
    Json(out).into_response()
}

/// Build a `DriveInfo` from current registry state. Re-reads the
/// registry on every call so a CLI-side `chan rename` immediately
/// reflects in the next /api/drive response.
fn drive_info(state: &AppState) -> DriveInfo {
    let drives = state.library.list_drives();
    let entry = drives
        .iter()
        .find(|d| d.path.as_path() == state.drive.root());
    DriveInfo {
        name: entry.and_then(|e| e.name.clone()),
        root: state.drive.root().to_string_lossy().into_owned(),
    }
}

async fn api_list_files(State(state): State<Arc<AppState>>) -> Response {
    match state.drive.list_tree() {
        Ok(tree) => Json(tree).into_response(),
        Err(e) => err_from(&e),
    }
}

#[derive(Serialize)]
struct FileResponse {
    path: String,
    content: String,
    mtime: Option<i64>,
}

async fn api_read_file(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
) -> Response {
    let content = match state.drive.read_text(&path) {
        Ok(c) => c,
        Err(e) => return err_from(&e),
    };
    let mtime = state.drive.stat(&path).ok().and_then(|s| s.mtime);
    Json(FileResponse {
        path,
        content,
        mtime,
    })
    .into_response()
}

#[derive(Deserialize)]
struct WriteBody {
    content: String,
}

async fn api_write_file(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
    Json(body): Json<WriteBody>,
) -> Response {
    if let Err(e) = state.drive.write_text(&path, &body.content) {
        return err_from(&e);
    }
    StatusCode::NO_CONTENT.into_response()
}

#[derive(Deserialize)]
struct CreateBody {
    path: String,
    is_dir: bool,
    /// Optional initial contents for files. Ignored for directories.
    content: Option<String>,
}

async fn api_create_file(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateBody>,
) -> Response {
    if state.drive.exists(&body.path) {
        return err(StatusCode::CONFLICT, "already exists".into());
    }
    if body.is_dir {
        match state.drive.create_dir(&body.path) {
            Ok(()) => StatusCode::CREATED.into_response(),
            Err(e) => err_from(&e),
        }
    } else {
        let content = body.content.unwrap_or_default();
        match state.drive.write_text(&body.path, &content) {
            Ok(()) => StatusCode::CREATED.into_response(),
            Err(e) => err_from(&e),
        }
    }
}

async fn api_delete_file(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
) -> Response {
    // chan-core's Drive::remove handles files and EMPTY directories.
    // Recursive deletion of a non-empty directory is a deliberate
    // foot-gun guard; supporting it here would require either a new
    // chan-core API (`Drive::remove_recursive`) or a server-side walk
    // that issues per-leaf removes. Tracked for a follow-up; current
    // behavior is "error out, frontend resolves the leaves itself".
    match state.drive.remove(&path) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_from(&e),
    }
}

#[derive(Deserialize)]
struct MoveBody {
    from: String,
    to: String,
}

async fn api_move(State(state): State<Arc<AppState>>, Json(body): Json<MoveBody>) -> Response {
    match state.drive.rename(&body.from, &body.to) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_from(&e),
    }
}

// ----- search + index -----------------------------------------------------

/// Filename search params. Empty `q` returns the first `limit`
/// files in the tree, mirroring the [[ picker's empty state.
#[derive(Deserialize)]
struct FileSearchParams {
    #[serde(default)]
    q: String,
    #[serde(default = "default_search_limit")]
    limit: usize,
}

fn default_search_limit() -> usize {
    50
}

/// Server-side filename match: walk the tree, keep regular files
/// whose basename contains `q` (case-insensitive). chan-core has
/// no built-in filename index since the cost (scan list_tree) is
/// linear and the drive size budget is small. Revisit if profiles
/// show this hot.
async fn api_search_files(
    State(state): State<Arc<AppState>>,
    Query(p): Query<FileSearchParams>,
) -> Response {
    let tree = match state.drive.list_tree() {
        Ok(t) => t,
        Err(e) => return err_from(&e),
    };
    let needle = p.q.to_lowercase();
    let mut hits = Vec::new();
    for entry in tree {
        if entry.is_dir {
            continue;
        }
        let basename = std::path::Path::new(&entry.path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if needle.is_empty() || basename.contains(&needle) {
            hits.push(entry);
            if hits.len() >= p.limit {
                break;
            }
        }
    }
    Json(hits).into_response()
}

#[derive(Deserialize)]
struct ContentSearchParams {
    #[serde(default)]
    q: String,
    #[serde(default = "default_content_limit")]
    limit: u32,
    /// Optional subdir scope (POSIX rel path under the drive root).
    /// Mirrors chan-core's `SearchOpts::scope`.
    #[serde(default)]
    scope: Option<String>,
}

fn default_content_limit() -> u32 {
    20
}

async fn api_search_content(
    State(state): State<Arc<AppState>>,
    Query(p): Query<ContentSearchParams>,
) -> Response {
    if p.q.trim().is_empty() {
        return Json(serde_json::json!({"hits": [], "total": 0, "mode_used": "Bm25"}))
            .into_response();
    }
    let opts = SearchOpts {
        limit: p.limit,
        scope: p.scope.clone(),
        ..Default::default()
    };
    match state.drive.search(&p.q, &opts) {
        Ok(res) => Json(res).into_response(),
        Err(e) => err_from(&e),
    }
}

/// Minimal index-status placeholder. chan-core's index opens
/// lazily and is always ready once a drive is open; meaningful
/// progress reporting (files indexed, last rebuild time) requires
/// chan-core surfacing the IndexStats from the most recent
/// reindex, which it does not today. Returning a small JSON shape
/// so the frontend can show a "ready" / "rebuilding" state.
async fn api_index_status() -> Response {
    Json(serde_json::json!({"ready": true})).into_response()
}

/// Trigger a full reindex of the drive (search + graph). chan-core's
/// reindex is synchronous and blocking, so we run it on the blocking
/// thread pool and return when it completes. For very large drives
/// this can take seconds; consider adding a job-handle abstraction
/// to chan-core if the wait becomes painful in practice.
async fn api_index_rebuild(State(state): State<Arc<AppState>>) -> Response {
    let drive = state.drive.clone();
    let result = tokio::task::spawn_blocking(move || drive.reindex()).await;
    match result {
        Ok(Ok(stats)) => Json(stats).into_response(),
        Ok(Err(e)) => err_from(&e),
        Err(e) => err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("rebuild task: {e}"),
        ),
    }
}

// ----- [[ typeahead -------------------------------------------------------
//
// Two-phase UX. Phase 1: as the user types `[[Re...`, the picker
// hits /api/link-targets to surface candidate files. Phase 2:
// after the user picks a file (`[[recipes/pasta.md`), they may
// type `#` to jump to a heading; the picker hits
// /api/headings/<rel> to enumerate the file's anchors.

#[derive(Deserialize)]
struct LinkTargetsParams {
    #[serde(default)]
    q: String,
    #[serde(default = "default_link_limit")]
    limit: u32,
}

fn default_link_limit() -> u32 {
    20
}

async fn api_link_targets(
    State(state): State<Arc<AppState>>,
    Query(p): Query<LinkTargetsParams>,
) -> Response {
    match state.drive.link_targets(&p.q, p.limit) {
        Ok(targets) => Json(targets).into_response(),
        Err(e) => err_from(&e),
    }
}

async fn api_headings(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
) -> Response {
    let graph = match state.drive.graph() {
        Ok(g) => g,
        Err(e) => return err_from(&e),
    };
    match graph.headings_of(&path) {
        Ok(headings) => Json(headings).into_response(),
        Err(e) => err_from(&e),
    }
}

// ----- error mapping ------------------------------------------------------

fn err(status: StatusCode, msg: String) -> Response {
    (status, Json(serde_json::json!({"error": msg}))).into_response()
}

/// Map chan-core errors to HTTP statuses. The shape of the JSON
/// matches the old server so frontend error handling stays unchanged.
fn err_from(e: &chan_core::ChanError) -> Response {
    use chan_core::ChanError as C;
    let (status, msg) = match e {
        C::PathEmpty | C::PathEscape | C::SymlinkEscape(_) => {
            (StatusCode::BAD_REQUEST, e.to_string())
        }
        C::NotEditableText(_) => (StatusCode::UNSUPPORTED_MEDIA_TYPE, e.to_string()),
        C::SpecialFile { .. } => (StatusCode::UNSUPPORTED_MEDIA_TYPE, e.to_string()),
        C::DriveNotRegistered(_) | C::DriveRootMissing(_) => (StatusCode::NOT_FOUND, e.to_string()),
        C::DriveLocked => (StatusCode::CONFLICT, e.to_string()),
        C::Io(s) if s.contains("No such file") || s.contains("not found") => {
            (StatusCode::NOT_FOUND, e.to_string())
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    err(status, msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_token_is_alphanumeric_and_long() {
        let t = random_token();
        assert_eq!(t.len(), TOKEN_LEN);
        assert!(t.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn extract_token_query_param() {
        let h = HeaderMap::new();
        assert_eq!(
            extract_token(Some("foo=bar&t=secret&x=y"), &h),
            Some("secret")
        );
    }

    #[test]
    fn extract_token_authorization_header() {
        let mut h = HeaderMap::new();
        h.insert(header::AUTHORIZATION, "Bearer secret".parse().unwrap());
        assert_eq!(extract_token(None, &h), Some("secret"));
    }

    #[test]
    fn extract_token_missing() {
        let h = HeaderMap::new();
        assert_eq!(extract_token(None, &h), None);
    }
}
