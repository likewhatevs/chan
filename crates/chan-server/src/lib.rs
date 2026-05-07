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

mod config;
mod preferences;
mod self_writes;

pub use config::ServerConfig;
pub use preferences::{EditorPrefs, FontPrefs, FontSpec, LineSpacing, PaneWidths, ThemeChoice};

use self_writes::SelfWrites;

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Multipart, Path as AxumPath, Query, State};
use axum::http::{header, HeaderMap, Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chan_core::{
    paths::DrivePaths, Drive, EdgeKind, Library, ResetMode, SearchOpts, WatchCallback, WatchEvent,
    WatchHandle,
};
use chan_llm::{
    BackendKind, Delta, LlmConfig, LlmError, LlmSession, Message as LlmMessage, Role as LlmRole,
    SessionListener, StopReason, ToolCall, ToolResult,
};
use rand::RngCore;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tower_http::trace::TraceLayer;

// Frontend bundle baked at compile time. The path is relative to
// this crate's manifest. In debug builds rust-embed reads files
// from disk on each request (so `npm run build` updates take
// effect without a cargo rebuild). In release builds the bundle
// is embedded; build.rs emits cargo:rerun-if-changed for every
// file under web/dist so a re-bundled frontend triggers a relink.
#[derive(RustEmbed)]
#[folder = "../../web/dist/"]
struct WebAssets;

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
    #[error("config: {0}")]
    Config(String),
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

    // Unified event stream: every /ws subscriber gets watcher
    // events AND assistant streaming events from the same channel.
    // Producers serialize to JSON strings (with a `type` field as
    // the discriminator); the WS pump just forwards strings as
    // text frames. Buffer of 256 is enough headroom for typical
    // bursts (mass rename, LLM token-stream); slow subscribers
    // see Lagged and skip ahead rather than blocking the sender.
    let (events_tx, _) = broadcast::channel::<String>(256);
    // Shared dedupe queue: server writes note their path here, the
    // watcher bridge consults it before forwarding so save->reload
    // echoes don't fire spurious external-edit prompts in the
    // editor.
    let self_writes = Arc::new(SelfWrites::new());
    let bridge = make_watch_bridge(&events_tx, &self_writes);
    let watch_handle = drive.watch(bridge)?;
    let drive_root = drive.root().to_path_buf();

    // LLM config: load once at boot. Falling back to defaults on
    // a malformed file keeps the server bootable; user fixes the
    // TOML and restarts.
    let llm_config = LlmConfig::load().unwrap_or_else(|e| {
        tracing::warn!("malformed llm config, falling back to defaults: {e}");
        LlmConfig::default()
    });

    // Server config: same fall-back-on-malformed policy as the
    // LLM config. Holds chan-server-specific paths
    // (attachments_dir, answers_dir).
    let server_config = ServerConfig::load().unwrap_or_else(|e| {
        tracing::warn!("malformed server config, falling back to defaults: {e}");
        ServerConfig::default()
    });

    // Editor preferences: fonts / theme / pane widths / line spacing /
    // date format. The remaining "preferences" surfaced by the
    // Settings UI live in LlmConfig (assistant) and ServerConfig
    // (attachments / answers dirs); the unified view returned over
    // /api/drive and /api/config joins all three.
    let editor_prefs = EditorPrefs::load().unwrap_or_else(|e| {
        tracing::warn!("malformed editor preferences, falling back to defaults: {e}");
        EditorPrefs::default()
    });

    let state = Arc::new(AppState {
        library,
        drive_root,
        drive_cell: RwLock::new(Some(DriveCell {
            drive,
            watch_handle: Some(watch_handle),
        })),
        token: handle.token,
        events_tx,
        llm_config: Mutex::new(llm_config),
        server_config: Mutex::new(server_config),
        editor_prefs: Mutex::new(editor_prefs),
        self_writes,
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
    /// Drive root resolved at boot. Stays stable for the server's
    /// lifetime even when `drive_cell` is swapped during a reset
    /// (the swap reopens against the same root).
    drive_root: PathBuf,
    /// Live drive + its watcher, behind an RwLock so /api/storage/
    /// reset can drop and reopen them without restarting the
    /// process. Always `Some` outside the brief swap window inside
    /// reset itself; handlers reach the inner Arc<Drive> via
    /// `state.drive()` which clones it under a read lock.
    drive_cell: RwLock<Option<DriveCell>>,
    token: Option<String>,
    /// Pre-serialized JSON-envelope frames: `{"type": "watch",
    /// "event": ...}`, `{"type": "llm.delta", "session_id": ...,
    /// "text": ...}`, etc. One channel; the `type` field tells
    /// the frontend what to do.
    events_tx: broadcast::Sender<String>,
    /// Loaded at boot; mutable for future PATCH /api/llm/config
    /// (backend selection, auto_apply_writes toggle). Currently
    /// only read by the status route and the complete handler.
    llm_config: Mutex<LlmConfig>,
    /// chan-server's own preferences (attachments_dir,
    /// answers_dir, etc). Mutable via PATCH /api/server/config;
    /// reads route through the get handler.
    server_config: Mutex<ServerConfig>,
    /// Editor preferences: fonts / theme / pane widths / line
    /// spacing / date format. Persisted to
    /// `<config>/chan/preferences.toml`; mutated through the
    /// /api/config PATCH path.
    editor_prefs: Mutex<EditorPrefs>,
    /// Recently-written paths for the watcher dedupe. Every server-
    /// side write notes its target here; WatchBroadcast checks the
    /// queue before forwarding so an editor save doesn't bounce
    /// back as an "external edit" event.
    self_writes: Arc<SelfWrites>,
}

/// Drive + its notify watcher. Replaced wholesale by /api/storage/
/// reset: drop the cell, run chan-core's reset_drive, reopen, store
/// a fresh cell. The watch_handle is `Option` only because reset
/// must take it out before dropping the inner Drive (the watcher
/// holds a callback that references the same broadcast channel; we
/// keep it tidy by dropping the handle first).
struct DriveCell {
    drive: Arc<Drive>,
    watch_handle: Option<WatchHandle>,
}

impl AppState {
    /// Snapshot the current drive Arc. Acquires the RwLock read
    /// guard for the duration of the clone (microseconds). The
    /// returned Arc keeps the drive alive even if a reset swaps
    /// the cell out a moment later, so callers don't need to hold
    /// the lock through their I/O.
    ///
    /// Panics if called while the cell is in the brief
    /// "between drop and reopen" state inside reset itself; the
    /// reset path holds the write lock end-to-end so handlers can
    /// never observe `None` (they wait on the read lock).
    fn drive(&self) -> Arc<Drive> {
        self.drive_cell
            .read()
            .expect("drive cell poisoned")
            .as_ref()
            .expect("drive cell missing outside reset window")
            .drive
            .clone()
    }
}

/// Construct a watcher bridge. Extracted so /api/storage/reset can
/// rebuild one cheaply when re-attaching the watcher to a fresh
/// Drive instance.
fn make_watch_bridge(
    events_tx: &broadcast::Sender<String>,
    self_writes: &Arc<SelfWrites>,
) -> Arc<dyn WatchCallback> {
    Arc::new(WatchBroadcast {
        tx: events_tx.clone(),
        self_writes: self_writes.clone(),
    })
}

/// Bridge from chan-core's callback-shaped watcher into the
/// broadcast channel that backs every /ws subscriber. Each event
/// goes out as a `{"type": "watch", "event": {...}}` envelope so
/// the frontend can multiplex with LLM events on the same socket.
///
/// Drops events that match a recent server-side write so the editor
/// doesn't see its own save as an external edit (the
/// "you wrote, OS told us, we tell you, you reload" loop). For
/// rename events both `from` and `to` are checked since both sides
/// land as separate notify events on most kernels.
struct WatchBroadcast {
    tx: broadcast::Sender<String>,
    self_writes: Arc<SelfWrites>,
}

impl WatchCallback for WatchBroadcast {
    fn on_event(&self, event: WatchEvent) {
        if event_is_self_echo(&event, &self.self_writes) {
            return;
        }
        let frame = serde_json::json!({"type": "watch", "event": event});
        if let Ok(s) = serde_json::to_string(&frame) {
            let _ = self.tx.send(s);
        }
    }
}

fn event_is_self_echo(event: &WatchEvent, sw: &SelfWrites) -> bool {
    if let Some(p) = event.path.as_deref() {
        if sw.should_suppress(p) {
            return true;
        }
    }
    if let Some(p) = event.to.as_deref() {
        if sw.should_suppress(p) {
            return true;
        }
    }
    false
}

/// Bridge from chan-llm's SessionListener into the same broadcast
/// channel. One listener instance per /api/llm/complete call;
/// dropped when the session emits `Done` or when the consumer
/// drops the `Arc` at the end of the request handler.
///
/// `session_id` is client-supplied so the frontend can correlate
/// streaming events to its in-flight assistant turn (multiple
/// turns can interleave on the same socket).
struct LlmBroadcastListener {
    tx: broadcast::Sender<String>,
    session_id: String,
}

impl LlmBroadcastListener {
    fn send(&self, ty: &str, body: serde_json::Value) {
        let mut frame = serde_json::Map::new();
        frame.insert("type".into(), ty.into());
        frame.insert("session_id".into(), self.session_id.clone().into());
        if let serde_json::Value::Object(map) = body {
            for (k, v) in map {
                frame.insert(k, v);
            }
        }
        if let Ok(s) = serde_json::to_string(&serde_json::Value::Object(frame)) {
            let _ = self.tx.send(s);
        }
    }
}

impl SessionListener for LlmBroadcastListener {
    fn on_delta(&self, d: Delta) {
        self.send("llm.delta", serde_json::json!({"text": d.text}));
    }
    fn on_tool_call(&self, c: ToolCall) {
        self.send("llm.tool_call", serde_json::json!({"call": c}));
    }
    fn on_tool_result(&self, r: ToolResult) {
        self.send("llm.tool_result", serde_json::json!({"result": r}));
    }
    fn on_done(&self, r: StopReason) {
        self.send("llm.done", serde_json::json!({"reason": r}));
    }
    fn on_error(&self, e: String) {
        self.send("llm.error", serde_json::json!({"error": e}));
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
        .route("/api/resolve-link", get(api_resolve_link))
        .route("/api/headings/*path", get(api_headings))
        .route("/api/links", get(api_links))
        .route("/api/graph", get(api_graph))
        .route("/api/backlinks/*path", get(api_backlinks))
        .route("/api/llm/status", get(api_llm_status))
        .route("/api/llm/tools", get(api_llm_tools))
        .route("/api/llm/complete", post(api_llm_complete))
        // Per-provider key writes. Path shape matches the frontend's
        // /api/llm/keys/<provider> (a stable contract across the
        // chan-writer surfaces); the older /api/llm/<provider>/key
        // form was a port artifact and is gone.
        .route(
            "/api/llm/keys/anthropic",
            axum::routing::put(api_llm_set_anthropic_key).delete(api_llm_clear_anthropic_key),
        )
        .route(
            "/api/llm/keys/gemini",
            axum::routing::put(api_llm_set_gemini_key).delete(api_llm_clear_gemini_key),
        )
        .route("/api/llm/anthropic/models", get(api_llm_anthropic_models))
        .route("/api/llm/gemini/models", get(api_llm_gemini_models))
        .route("/api/llm/ollama/models", get(api_llm_ollama_models))
        .route(
            "/api/server/config",
            get(api_get_server_config).patch(api_patch_server_config),
        )
        .route("/api/config", get(api_get_config).patch(api_patch_config))
        .route("/api/build-info", get(api_build_info))
        // Session blob keyed by window id (?w=<id>). The frontend
        // sends the window id as a query string (path-segment encode
        // would force special-character escaping for free-form ids);
        // the server matches that contract. GET on a missing key
        // returns 204, not 404, since "no session yet" is the normal
        // first-launch state.
        .route(
            "/api/session",
            get(api_get_session)
                .put(api_put_session)
                .delete(api_delete_session),
        )
        .route("/api/sessions", get(api_list_sessions))
        // Assistant per-conversation blob keyed by file path or group
        // key (?path=<key>). Same query-string contract as /api/session
        // for the same reason. The plural sibling endpoint covers
        // listing and clearing all conversations at once.
        .route(
            "/api/assistant/conversation",
            get(api_get_assistant)
                .put(api_put_assistant)
                .delete(api_delete_assistant),
        )
        .route(
            "/api/assistant/conversations",
            get(api_list_assistant).delete(api_clear_assistant),
        )
        .route("/api/answers", post(api_post_answer))
        .route("/api/attachments", post(api_post_attachment))
        .route("/api/storage/reset", post(api_storage_reset))
        .route("/api/health", get(api_health))
        .route("/ws", get(ws_upgrade));
    Router::new()
        .merge(api)
        .fallback(serve_static)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

// ----- static frontend ----------------------------------------------------
//
// Single-page-app fallback: any path that doesn't match an /api or
// /ws route, and doesn't correspond to a baked asset, returns
// index.html so client-side routes work. For unknown /api paths
// we return a real 404 instead of the SPA shell so callers don't
// silently get HTML when they expected JSON.

async fn serve_static(uri: axum::http::Uri) -> Response {
    let path = uri.path();
    // Refuse to serve the SPA shell for /api or /ws misses; those
    // are programmatic surfaces, not browser navigation.
    if path.starts_with("/api") || path == "/ws" {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }
    let candidate = path.trim_start_matches('/');
    let candidate = if candidate.is_empty() {
        "index.html"
    } else {
        candidate
    };
    if let Some(file) = WebAssets::get(candidate) {
        return (
            [(header::CONTENT_TYPE, content_type_for(candidate))],
            file.data.into_owned(),
        )
            .into_response();
    }
    // SPA fallback: route paths the frontend handles client-side.
    if let Some(file) = WebAssets::get("index.html") {
        return (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            file.data.into_owned(),
        )
            .into_response();
    }
    // No bundle baked / on disk yet (fresh clone, npm not run).
    (
        StatusCode::NOT_FOUND,
        "frontend bundle not built; run `cd web && npm install && npm run build`",
    )
        .into_response()
}

/// Conservative MIME map for the file types the SPA bundle ships:
/// hashed JS / CSS, source maps, fonts, images, and a couple of
/// well-known toplevel files. Falls back to
/// `application/octet-stream` so unknown extensions never get the
/// wrong type assigned.
fn content_type_for(path: &str) -> &'static str {
    let ext = match path.rsplit_once('.') {
        Some((_, e)) => e.to_ascii_lowercase(),
        None => return "application/octet-stream",
    };
    match ext.as_str() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "map" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "wasm" => "application/wasm",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "txt" | "md" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
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

/// Forward pre-serialized JSON envelope frames to one WebSocket
/// client until either side hangs up. Producers (WatchBroadcast,
/// LlmBroadcastListener) build the JSON once; this pump just
/// fans out. Lagged subscribers skip ahead rather than tearing
/// down the connection.
async fn ws_pump(mut socket: WebSocket, mut rx: broadcast::Receiver<String>) {
    loop {
        match rx.recv().await {
            Ok(frame) => {
                if socket.send(Message::Text(frame)).await.is_err() {
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
    name: Option<String>,
    /// Absolute drive root, POSIX-style on every platform so the
    /// JSON shape stays stable.
    root: String,
    /// Per-device preferences view. The frontend uses this to seed
    /// the editor (fonts, theme, line spacing) without a follow-up
    /// /api/config round-trip. Same shape as
    /// `GlobalConfig.preferences`; assembled by joining
    /// EditorPrefs + ServerConfig + LlmConfig.
    preferences: PreferencesView,
}

/// Unified Preferences shape returned over /api/drive and
/// /api/config. The fields are owned by three different stores:
///
/// - fonts / theme / pane_widths / line_spacing / date_format:
///   EditorPrefs (preferences.toml)
/// - attachments_dir: ServerConfig (server.toml; the answers_dir
///   field there is mirrored into the assistant subtree below)
/// - assistant: LlmConfig (llm.toml) + ServerConfig.answers_dir
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PreferencesView {
    fonts: FontPrefs,
    assistant: AssistantPrefsView,
    attachments_dir: String,
    theme: ThemeChoice,
    pane_widths: PaneWidths,
    line_spacing: LineSpacing,
    date_format: String,
}

/// Frontend's `AssistantPrefs` view. The subtables (claude / ollama /
/// gemini) carry only model overrides today; per-backend ollama URL
/// is stubbed out (Some(None)) since chan-llm doesn't persist it.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AssistantPrefsView {
    enabled: bool,
    backend: AssistantBackendKind,
    answers_dir: String,
    auto_apply_writes: bool,
    claude: ProviderPrefsView,
    ollama: OllamaPrefsView,
    gemini: ProviderPrefsView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProviderPrefsView {
    #[serde(default)]
    model: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct OllamaPrefsView {
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    model: Option<String>,
}

/// Frontend uses "claude" (display label) for what chan-llm types
/// internally as `BackendKind::Anthropic`. The "claude_cli" variant
/// covers the new shell-executor backend that wraps the local
/// `claude` CLI. The "embedded" variant is reserved for a future
/// on-device backend (qwen2.5 via candle); it has no chan-llm
/// counterpart yet, so PATCHing it is treated as a no-op when read
/// back the value falls through to the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AssistantBackendKind {
    Claude,
    Ollama,
    Gemini,
    ClaudeCli,
    Embedded,
}

impl AssistantBackendKind {
    fn from_chan_llm(kind: BackendKind) -> Self {
        match kind {
            BackendKind::Anthropic => AssistantBackendKind::Claude,
            BackendKind::Ollama => AssistantBackendKind::Ollama,
            BackendKind::Gemini => AssistantBackendKind::Gemini,
            BackendKind::ClaudeCli => AssistantBackendKind::ClaudeCli,
        }
    }

    fn to_chan_llm(self) -> Option<BackendKind> {
        match self {
            AssistantBackendKind::Claude => Some(BackendKind::Anthropic),
            AssistantBackendKind::Ollama => Some(BackendKind::Ollama),
            AssistantBackendKind::Gemini => Some(BackendKind::Gemini),
            AssistantBackendKind::ClaudeCli => Some(BackendKind::ClaudeCli),
            AssistantBackendKind::Embedded => None,
        }
    }
}

/// Build the unified Preferences view for the current state. Reads
/// each backing store under its own lock.
fn preferences_view(state: &AppState) -> PreferencesView {
    let editor = state.editor_prefs.lock().expect("editor prefs poisoned");
    let server = state.server_config.lock().expect("server config poisoned");
    let llm = state.llm_config.lock().expect("llm config poisoned");
    let backend_kind = llm.backend.unwrap_or(BackendKind::Anthropic);
    PreferencesView {
        fonts: editor.fonts.clone(),
        assistant: AssistantPrefsView {
            enabled: llm.backend.is_some(),
            backend: AssistantBackendKind::from_chan_llm(backend_kind),
            answers_dir: server.answers_dir.clone(),
            auto_apply_writes: llm.auto_apply_writes,
            claude: ProviderPrefsView {
                model: llm.models.anthropic.clone(),
            },
            ollama: OllamaPrefsView {
                url: llm.urls.ollama.clone(),
                model: llm.models.ollama.clone(),
            },
            gemini: ProviderPrefsView {
                model: llm.models.gemini.clone(),
            },
        },
        attachments_dir: server.attachments_dir.clone(),
        theme: editor.theme,
        pane_widths: editor.pane_widths,
        line_spacing: editor.line_spacing,
        date_format: editor.date_format.clone(),
    }
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
        if let Err(e) = state.library.rename_drive(state.drive().root(), new_name) {
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
        .find(|d| d.path.as_path() == state.drive().root());
    DriveInfo {
        name: entry.and_then(|e| e.name.clone()),
        root: state.drive().root().to_string_lossy().into_owned(),
        preferences: preferences_view(state),
    }
}

async fn api_list_files(State(state): State<Arc<AppState>>) -> Response {
    match state.drive().list_tree() {
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
    // Editable-text files (.md / .txt) come back as FileResponse
    // JSON since the frontend's editor wants the content as a
    // string. Anything else (images, attachments) comes back as
    // raw bytes with a sniffed Content-Type so `<img src=...>`
    // pointing at /api/files/<path> resolves correctly.
    if chan_core::fs_ops::is_editable_text(&path) {
        let content = match state.drive().read_text(&path) {
            Ok(c) => c,
            Err(e) => return err_from(&e),
        };
        let mtime = state.drive().stat(&path).ok().and_then(|s| s.mtime);
        return Json(FileResponse {
            path,
            content,
            mtime,
        })
        .into_response();
    }
    match state.drive().read(&path) {
        Ok(bytes) => ([(header::CONTENT_TYPE, content_type_for(&path))], bytes).into_response(),
        Err(e) => err_from(&e),
    }
}

#[derive(Deserialize)]
struct WriteBody {
    content: String,
    /// CAS token: the mtime the client thinks the file currently
    /// has on disk. When present, the server uses
    /// Drive::write_text_if_unchanged and rejects with 409 if the
    /// disk-side mtime differs. When absent, the write is
    /// last-write-wins (Drive::write_text), preserving the
    /// pre-CAS contract for callers that don't care
    /// (bulk imports, scripts).
    #[serde(default)]
    expected_mtime: Option<i64>,
}

#[derive(Serialize)]
struct WriteResponse {
    /// Mtime after the write. Frontend stores this as the next
    /// CAS token for subsequent saves so the client and disk stay
    /// in lock-step without an extra stat round-trip.
    mtime: Option<i64>,
}

#[derive(Serialize)]
struct WriteConflictBody {
    /// Mtime currently on disk, returned so the client knows what
    /// token to use on a follow-up "overwrite" attempt without a
    /// separate stat call. None when the file disappeared between
    /// the client's last fetch and now (rare; treat as "create
    /// fresh" on the retry).
    current_mtime: Option<i64>,
}

async fn api_write_file(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
    Json(body): Json<WriteBody>,
) -> Response {
    let result = match body.expected_mtime {
        Some(_) => state
            .drive()
            .write_text_if_unchanged(&path, body.expected_mtime, &body.content),
        None => state.drive().write_text(&path, &body.content),
    };
    if let Err(e) = result {
        if let chan_core::ChanError::WriteConflict { current_mtime } = e {
            return (
                StatusCode::CONFLICT,
                Json(WriteConflictBody { current_mtime }),
            )
                .into_response();
        }
        return err_from(&e);
    }
    state.self_writes.note(&path);
    let mtime = state.drive().stat(&path).ok().and_then(|s| s.mtime);
    Json(WriteResponse { mtime }).into_response()
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
    if state.drive().exists(&body.path) {
        return err(StatusCode::CONFLICT, "already exists".into());
    }
    if body.is_dir {
        match state.drive().create_dir(&body.path) {
            Ok(()) => {
                state.self_writes.note(&body.path);
                StatusCode::CREATED.into_response()
            }
            Err(e) => err_from(&e),
        }
    } else {
        let content = body.content.unwrap_or_default();
        match state.drive().write_text(&body.path, &content) {
            Ok(()) => {
                state.self_writes.note(&body.path);
                StatusCode::CREATED.into_response()
            }
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
    match state.drive().remove(&path) {
        Ok(()) => {
            state.self_writes.note(&path);
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => err_from(&e),
    }
}

#[derive(Deserialize)]
struct MoveBody {
    from: String,
    to: String,
}

async fn api_move(State(state): State<Arc<AppState>>, Json(body): Json<MoveBody>) -> Response {
    match state.drive().rename(&body.from, &body.to) {
        Ok(()) => {
            // Rename emits two notify events on most kernels (a
            // Removed at `from` and a Created at `to`); note both
            // so neither half of the pair fires an external-edit
            // prompt.
            state.self_writes.note(&body.from);
            state.self_writes.note(&body.to);
            StatusCode::NO_CONTENT.into_response()
        }
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
    let tree = match state.drive().list_tree() {
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

/// `/api/search/content` view. Frontend's `ContentSearchResponse`
/// is a flat hit list; chan-core's `SearchResults` wraps per-file
/// hits with a sub-array of snippets. We expand each snippet to its
/// own ContentHit so the result palette can show one row per
/// matching section. start_line isn't surfaced by chan-core today;
/// synthesized as 0 (the frontend sorts by score, not line).
#[derive(Serialize)]
struct ContentSearchResponse {
    /// True when the index is ready to serve queries. chan-core
    /// opens the index lazily and is always ready once a drive is
    /// open; kept as an explicit field so a future "rebuilding"
    /// state can land without a contract break.
    ready: bool,
    /// Mode actually used. "bm25" today (chan-core's tantivy
    /// search); "hybrid" / "semantic" reserved for the dense
    /// retrieval that lands with the embeddings feature.
    mode: &'static str,
    hits: Vec<ContentHit>,
}

#[derive(Serialize)]
struct ContentHit {
    path: String,
    chunk_id: String,
    heading: String,
    start_line: u32,
    snippet: String,
    score: f32,
}

fn search_mode_tag(m: chan_core::SearchMode) -> &'static str {
    match m {
        chan_core::SearchMode::Bm25 => "bm25",
        chan_core::SearchMode::Hybrid => "hybrid",
    }
}

async fn api_search_content(
    State(state): State<Arc<AppState>>,
    Query(p): Query<ContentSearchParams>,
) -> Response {
    if p.q.trim().is_empty() {
        return Json(ContentSearchResponse {
            ready: true,
            mode: "bm25",
            hits: Vec::new(),
        })
        .into_response();
    }
    let opts = SearchOpts {
        limit: p.limit,
        scope: p.scope.clone(),
        ..Default::default()
    };
    let results = match state.drive().search(&p.q, &opts) {
        Ok(r) => r,
        Err(e) => return err_from(&e),
    };
    let mode = search_mode_tag(results.mode_used);
    let mut flat: Vec<ContentHit> = Vec::new();
    for hit in results.hits {
        if hit.snippets.is_empty() {
            // No section-level snippets (older index entries, very
            // short files); emit one row per hit so the path still
            // shows up.
            flat.push(ContentHit {
                path: hit.path.clone(),
                chunk_id: format!("{}#0", hit.path),
                heading: String::new(),
                start_line: 0,
                snippet: String::new(),
                score: hit.score,
            });
            continue;
        }
        for (idx, sn) in hit.snippets.iter().enumerate() {
            flat.push(ContentHit {
                path: hit.path.clone(),
                chunk_id: format!("{}#{}", hit.path, idx),
                heading: sn.heading_path.join(" / "),
                start_line: 0,
                snippet: sn.text.clone(),
                score: hit.score,
            });
        }
    }
    Json(ContentSearchResponse {
        ready: true,
        mode,
        hits: flat,
    })
    .into_response()
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
    let drive = state.drive().clone();
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
    match state.drive().link_targets(&p.q, p.limit) {
        Ok(targets) => Json(targets).into_response(),
        Err(e) => err_from(&e),
    }
}

#[derive(Deserialize)]
struct ResolveLinkParams {
    /// Wiki-link target as written, e.g. `recipes/pasta` or
    /// `recipes/pasta#ingredients`. Pass through verbatim from
    /// the editor; chan-core handles the .md / .txt extension
    /// fallback and the anchor split.
    target: String,
}

/// Resolve a wiki-link target to an existing drive file. 404
/// when no file matches the candidates; this lets the editor's
/// click handler render a "broken link / create?" affordance.
async fn api_resolve_link(
    State(state): State<Arc<AppState>>,
    Query(p): Query<ResolveLinkParams>,
) -> Response {
    match state.drive().resolve_link(&p.target) {
        Some(resolved) => Json(resolved).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn api_headings(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
) -> Response {
    let drive = state.drive();
    let graph = match drive.graph() {
        Ok(g) => g,
        Err(e) => return err_from(&e),
    };
    match graph.headings_of(&path) {
        Ok(headings) => Json(headings).into_response(),
        Err(e) => err_from(&e),
    }
}

// ----- graph --------------------------------------------------------------
//
// chan-core's GraphView exposes per-file accessors (neighbors,
// backlinks, headings_of) and bulk reads (files, tags). It does
// NOT expose an "all edges" call, so /api/links and /api/graph
// walk the file list and accumulate. For typical drive sizes the
// O(n) sqlite round-trip is fine; if profiles show this hot we
// add a chan-core helper.

/// All link-kind edges in the drive. Mention and tag edges are
/// excluded; the graph view fetches those via /api/graph. The
/// shape is `[Edge]` so the frontend can render the link-only
/// view without a follow-up request.
async fn api_links(State(state): State<Arc<AppState>>) -> Response {
    let drive = state.drive();
    let graph = match drive.graph() {
        Ok(g) => g,
        Err(e) => return err_from(&e),
    };
    let files = match graph.files() {
        Ok(f) => f,
        Err(e) => return err_from(&e),
    };
    let mut edges = Vec::new();
    for f in &files {
        match graph.neighbors(f) {
            Ok(es) => edges.extend(es.into_iter().filter(|e| matches!(e.kind, EdgeKind::Link))),
            Err(e) => return err_from(&e),
        }
    }
    Json(edges).into_response()
}

/// Typed nodes + edges payload for the graph view.
///
///   files     [String]                file rel paths
///   tags      [{name, count}]         tag dst nodes with usage counts
///   mentions  [String]                distinct mention dst nodes
///   edges     [Edge]                  every edge in the drive
///
/// `/api/graph` view. Frontend's `GraphView` type is unified
/// `{ nodes, edges }`; chan-core exposes per-kind primitives
/// (files / tags / neighbors). This handler walks the graph DB and
/// emits the unified shape so the visualization can render without
/// per-kind glue on the frontend side.
///
/// Node kinds: file (one per indexed path), tag (#name), mention
/// (@@name). Date nodes from the typescript type aren't emitted;
/// chan-core's EdgeKind has no date variant today.
#[derive(Serialize)]
struct GraphViewResponse {
    nodes: Vec<GraphNodeView>,
    edges: Vec<GraphEdgeView>,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum GraphNodeView {
    File {
        id: String,
        label: String,
        path: String,
        /// True for ghost nodes synthesized as the target of a
        /// broken link. Frontend renders them muted.
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        missing: bool,
    },
    Tag {
        id: String,
        label: String,
    },
    Mention {
        id: String,
        label: String,
    },
}

#[derive(Serialize)]
struct GraphEdgeView {
    source: String,
    target: String,
    /// "link" | "tag" | "mention". Lowercase to match the
    /// frontend's GraphViewEdgeKind type.
    kind: &'static str,
    /// Only meaningful for link edges: true when the link resolves
    /// to a missing file. Other kinds skip the field.
    #[serde(skip_serializing_if = "Option::is_none")]
    broken: Option<bool>,
}

fn edge_kind_tag(k: EdgeKind) -> &'static str {
    match k {
        EdgeKind::Link => "link",
        EdgeKind::Tag => "tag",
        EdgeKind::Mention => "mention",
    }
}

/// Derive the file-node label from a drive-relative path. Strips
/// the `.md` / `.txt` extension and uses the basename so the graph
/// renders "recipes/pasta" as just "pasta" without losing the path
/// (the file node carries the full path on its `path` field).
fn file_label(rel: &str) -> String {
    let stem = std::path::Path::new(rel)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| rel.to_string());
    stem
}

async fn api_graph(State(state): State<Arc<AppState>>) -> Response {
    let drive = state.drive();
    let graph = match drive.graph() {
        Ok(g) => g,
        Err(e) => return err_from(&e),
    };
    let files = match graph.files() {
        Ok(f) => f,
        Err(e) => return err_from(&e),
    };
    let tags = match graph.tags() {
        Ok(t) => t,
        Err(e) => return err_from(&e),
    };
    let mut all_edges = Vec::new();
    for f in &files {
        match graph.neighbors(f) {
            Ok(es) => all_edges.extend(es),
            Err(e) => return err_from(&e),
        }
    }

    let file_set: std::collections::BTreeSet<&str> = files.iter().map(String::as_str).collect();

    // Build the node list. File nodes for every indexed path; tag
    // nodes per #tag; mention nodes per distinct @@name. Ghost
    // file nodes for unresolved link targets so the graph shows
    // broken links as dangling muted nodes.
    let mut nodes: Vec<GraphNodeView> = Vec::new();
    for path in &files {
        nodes.push(GraphNodeView::File {
            id: path.clone(),
            label: file_label(path),
            path: path.clone(),
            missing: false,
        });
    }
    for tag in &tags {
        nodes.push(GraphNodeView::Tag {
            id: format!("#{}", tag.name),
            label: format!("#{}", tag.name),
        });
    }
    let mut mention_set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut ghost_set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for e in &all_edges {
        match e.kind {
            EdgeKind::Mention => {
                mention_set.insert(e.dst.clone());
            }
            EdgeKind::Link => {
                if !file_set.contains(e.dst.as_str()) {
                    ghost_set.insert(e.dst.clone());
                }
            }
            EdgeKind::Tag => {}
        }
    }
    for m in &mention_set {
        nodes.push(GraphNodeView::Mention {
            id: m.clone(),
            label: m.clone(),
        });
    }
    for ghost in &ghost_set {
        nodes.push(GraphNodeView::File {
            id: ghost.clone(),
            label: file_label(ghost),
            path: ghost.clone(),
            missing: true,
        });
    }

    let edges: Vec<GraphEdgeView> = all_edges
        .iter()
        .map(|e| GraphEdgeView {
            source: e.src.clone(),
            target: match e.kind {
                EdgeKind::Tag => format!("#{}", e.dst),
                _ => e.dst.clone(),
            },
            kind: edge_kind_tag(e.kind),
            broken: match e.kind {
                EdgeKind::Link => Some(!file_set.contains(e.dst.as_str())),
                _ => None,
            },
        })
        .collect();

    Json(GraphViewResponse { nodes, edges }).into_response()
}

/// Incoming link edges for one file. The frontend uses this for
/// the "linked from" panel. chan-core's `backlinks` filters to
/// link-kind edges already; we just pass through.
async fn api_backlinks(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
) -> Response {
    let drive = state.drive();
    let graph = match drive.graph() {
        Ok(g) => g,
        Err(e) => return err_from(&e),
    };
    match graph.backlinks(&path) {
        Ok(edges) => Json(edges).into_response(),
        Err(e) => err_from(&e),
    }
}

// ----- llm ----------------------------------------------------------------
//
// Routes wrap chan_llm::LlmSession. Streaming events flow over the
// shared /ws so the frontend has one socket to read from. The
// route surface stays valid even though chan-llm's backends are
// stubs at this point: complete() emits an immediate
// llm.error + llm.done frame for the configured backend.

/// `/api/llm/status` view shape. Frontend's `LlmStatus` type is a
/// flat one-active-backend snapshot; the previous per-backend dict
/// shape didn't match (and threw at render time when SettingsPanel
/// reached for `key.set` on the missing field). One source of truth
/// per request: the configured backend, its effective model, and
/// the resolution status of its key.
#[derive(Serialize)]
struct LlmStatus {
    /// Frontend's display tag for the active backend.
    /// "claude" | "ollama" | "gemini". The "embedded" variant in
    /// the typescript type is reserved for a future on-device
    /// backend; not surfaced here yet.
    backend: &'static str,
    /// Effective model for the active backend (config override or
    /// the chan-llm default).
    model: Option<String>,
    /// Key resolution snapshot for the active backend.
    key: LlmKeyView,
    /// Whether a request would succeed today (active backend
    /// configured + key resolves, or Ollama which is keyless).
    ready: bool,
    /// Human-readable explanation when `ready = false`. Absent on
    /// the happy path so the UI knows there's nothing to surface.
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    /// Mirror of LlmConfig.backend.is_some(). Settings hides the
    /// assistant button when this flips off.
    enabled: bool,
    /// Backend supports tool use. All three current backends do;
    /// retained as a typed field so future "chat-only" variants
    /// don't break the contract.
    supports_tools: bool,
}

#[derive(Serialize)]
struct LlmKeyView {
    /// True iff the key resolved through any of env / keychain /
    /// file. Settings disables the "refresh models" buttons when
    /// this is false.
    set: bool,
    /// Lowercase tag for where the key came from. None when not
    /// set (the union with `set: false`).
    source: Option<&'static str>,
    /// Where the on-disk fallback would land. Constant per machine;
    /// surfaced so the Settings tab can point the user at the file
    /// to edit on a headless box.
    path: Option<String>,
    /// True when the OS keychain backend is reachable. Settings
    /// hides keychain controls on headless boxes (no Secret
    /// Service / DBus session, locked keychain, etc.).
    keychain_available: bool,
}

/// Map the active chan-llm BackendKind to the frontend's display
/// tag. Anthropic surfaces as "claude" because that's the brand the
/// user picks from the dropdown.
fn backend_tag(kind: BackendKind) -> &'static str {
    match kind {
        BackendKind::Anthropic => "claude",
        BackendKind::Ollama => "ollama",
        BackendKind::Gemini => "gemini",
        BackendKind::ClaudeCli => "claude_cli",
    }
}

fn key_status_tag(s: chan_llm::KeyStatus) -> Option<&'static str> {
    match s {
        chan_llm::KeyStatus::Env => Some("env"),
        chan_llm::KeyStatus::Keychain => Some("keychain"),
        chan_llm::KeyStatus::File => Some("file"),
        chan_llm::KeyStatus::Missing => None,
    }
}

/// Whether the OS keychain backend is reachable on this machine.
/// chan-llm doesn't expose a probe today; we report `true`
/// optimistically and let actual set / clear calls surface errors
/// through `/api/llm/keys/<provider>` when the backend isn't
/// usable. macOS Keychain, Windows Credential Manager, and
/// gnome-keyring / KWallet on a desktop Linux session all work
/// out of the box; the field is reserved as a future hook for
/// detecting headless boxes.
fn keychain_available() -> bool {
    true
}

async fn api_llm_status(State(state): State<Arc<AppState>>) -> Response {
    let cfg = state.llm_config.lock().unwrap().clone();
    let active = cfg.backend.unwrap_or(BackendKind::Anthropic);
    let model = cfg
        .models
        .for_backend(active)
        .map(str::to_owned)
        .or_else(|| Some(active.default_model().to_string()));
    let (active_key, status) = chan_llm::keys::resolve(active, &cfg);
    let key_set = active_key.is_some();
    let enabled = cfg.backend.is_some();
    // Ollama and ClaudeCli are keyless from chan-llm's view (Ollama
    // is local; ClaudeCli inherits auth from the user's installed
    // `claude` install), so a missing-key status doesn't block
    // ready. Anthropic and Gemini need a key to issue a request.
    let ready = enabled
        && match active {
            BackendKind::Ollama | BackendKind::ClaudeCli => true,
            BackendKind::Anthropic | BackendKind::Gemini => key_set,
        };
    let reason = if !enabled {
        Some("no backend selected; pick one in Settings".to_string())
    } else if !ready {
        // Per-backend env var so the message matches the active
        // selection (the previous "ANTHROPIC_API_KEY / GEMINI_API_KEY"
        // dual-string was confusing when only one of them was the
        // active backend).
        let env = match active {
            BackendKind::Anthropic => "ANTHROPIC_API_KEY",
            BackendKind::Gemini => "GEMINI_API_KEY",
            // Ollama and ClaudeCli are keyless from chan-llm's
            // perspective (Ollama is local; ClaudeCli inherits auth
            // from the user's installed `claude` install). The
            // !ready branch shouldn't fire for them; keep a
            // sensible env var so the exhaustive match compiles.
            BackendKind::Ollama => "OLLAMA_HOST",
            BackendKind::ClaudeCli => "CLAUDE_CLI",
        };
        Some(format!(
            "{} key not configured. Set {env} in your shell, or save the \
             key from this Settings panel.",
            backend_tag(active),
        ))
    } else {
        None
    };
    Json(LlmStatus {
        backend: backend_tag(active),
        model,
        key: LlmKeyView {
            set: key_set,
            source: key_status_tag(status),
            path: Some(api_keys_path_string()),
            keychain_available: keychain_available(),
        },
        ready,
        reason,
        enabled,
        supports_tools: true,
    })
    .into_response()
}

/// `<config>/chan/api-keys.toml`-style path the on-disk fallback
/// uses. Hardcoded here because chan-llm doesn't expose a public
/// path helper; the Settings UI surfaces this so users on headless
/// boxes know which file to edit. Stays in lockstep with chan-llm's
/// internal `default_path()` for keys.
fn api_keys_path_string() -> String {
    dirs::config_dir()
        .map(|p| {
            p.join("chan")
                .join("api-keys.toml")
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| "<config>/chan/api-keys.toml".to_string())
}

#[derive(Serialize)]
struct LlmToolSchema {
    name: &'static str,
    description: &'static str,
}

async fn api_llm_tools() -> Response {
    Json([
        LlmToolSchema {
            name: "read_file",
            description: chan_llm::prompts::READ_FILE_DESC,
        },
        LlmToolSchema {
            name: "write_file",
            description: chan_llm::prompts::WRITE_FILE_DESC,
        },
        LlmToolSchema {
            name: "list_files",
            description: chan_llm::prompts::LIST_FILES_DESC,
        },
        LlmToolSchema {
            name: "search_content",
            description: chan_llm::prompts::SEARCH_CONTENT_DESC,
        },
    ])
    .into_response()
}

#[derive(Deserialize)]
struct CompleteBody {
    /// Client-generated correlation id. Echoed on every llm.* WS
    /// frame so the frontend can match streaming events to its
    /// pending turn. Optional: when absent, the server generates
    /// a random one (callers that consume only the synchronous
    /// JSON response don't need to track an id at all).
    #[serde(default)]
    session_id: Option<String>,
    /// Convenience: a single user message. The server wraps this
    /// in a one-element Vec<Message> for the LLM. Use
    /// `messages` instead when the host needs to pass full
    /// transcript / tool-result turns.
    #[serde(default)]
    message: Option<String>,
    /// Full conversation transcript. Wins over `message` when
    /// both are set. The frontend builds this from its persisted
    /// assistant blob (see /api/assistant/conversation) and
    /// passes the full history each turn so chan-llm stays
    /// stateless.
    #[serde(default)]
    messages: Vec<ApiMessage>,
    /// Tools the caller wants to expose to this turn. Optional;
    /// chan-llm prepends its own standard tool schemas
    /// internally. Today this field is observed for forward
    /// compatibility but not actually plumbed (chan-llm's send()
    /// uses standard_tool_schemas unconditionally).
    #[serde(default)]
    #[allow(dead_code)]
    tools: Option<serde_json::Value>,
    /// Output cap. Per-backend defaults are sane; passed through
    /// for forward compatibility but currently ignored.
    #[serde(default)]
    #[allow(dead_code)]
    max_tokens: Option<u32>,
    /// Sampling temperature. Ignored today (extended-thinking
    /// models reject explicit values; we let backends pick).
    #[serde(default)]
    #[allow(dead_code)]
    temperature: Option<f32>,
}

#[derive(Deserialize)]
struct ApiMessage {
    role: ApiRole,
    content: String,
    #[serde(default)]
    tool_call_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum ApiRole {
    System,
    User,
    Assistant,
    Tool,
}

impl From<ApiMessage> for LlmMessage {
    fn from(m: ApiMessage) -> Self {
        let role = match m.role {
            ApiRole::System => LlmRole::System,
            ApiRole::User => LlmRole::User,
            ApiRole::Assistant => LlmRole::Assistant,
            ApiRole::Tool => LlmRole::Tool,
        };
        LlmMessage {
            role,
            content: m.content,
            tool_call_id: m.tool_call_id,
            tool_calls: Vec::new(),
        }
    }
}

/// Frontend's expected response shape for /api/llm/complete:
/// the full assistant turn returned synchronously. Streaming
/// deltas still flow over /ws as a side channel, but the simple
/// non-streaming caller (InlineAssist's submit path) just awaits
/// the JSON body and reads it once.
#[derive(Serialize)]
struct LlmCompletionResponse {
    content: String,
    tool_calls: Vec<LlmToolCallView>,
    /// Frontend's LlmStopReason: "end_turn" | "max_tokens" |
    /// "tool_use" | "stop_sequence" | "other".
    stop_reason: &'static str,
    model: String,
}

#[derive(Serialize)]
struct LlmToolCallView {
    id: String,
    name: String,
    /// chan-llm's struct uses `args`; the frontend types the same
    /// JSON value as `input` per Anthropic's convention. Renamed
    /// at the seam so neither side needs to know about the other.
    input: serde_json::Value,
}

fn stop_reason_tag(r: chan_llm::StopReason) -> &'static str {
    match r {
        chan_llm::StopReason::EndOfTurn => "end_turn",
        chan_llm::StopReason::MaxTokens => "max_tokens",
        chan_llm::StopReason::StopSequence => "stop_sequence",
        chan_llm::StopReason::ToolUse => "tool_use",
        chan_llm::StopReason::Error => "other",
    }
}

/// Listener that forwards events to the broadcast channel (so /ws
/// subscribers see them live) AND collects the final assistant
/// text + tool calls in memory so the HTTP handler can return
/// them synchronously. Completion is signalled via a Notify that
/// the handler awaits before responding.
struct CollectListener {
    forward: LlmBroadcastListener,
    state: Mutex<CollectState>,
    done: tokio::sync::Notify,
}

struct CollectState {
    text: String,
    tool_calls: Vec<chan_llm::ToolCall>,
    stop_reason: Option<chan_llm::StopReason>,
    error: Option<String>,
    finished: bool,
}

impl CollectListener {
    fn new(forward: LlmBroadcastListener) -> Self {
        Self {
            forward,
            state: Mutex::new(CollectState {
                text: String::new(),
                tool_calls: Vec::new(),
                stop_reason: None,
                error: None,
                finished: false,
            }),
            done: tokio::sync::Notify::new(),
        }
    }
}

impl SessionListener for CollectListener {
    fn on_delta(&self, delta: chan_llm::Delta) {
        self.state
            .lock()
            .expect("collect state poisoned")
            .text
            .push_str(&delta.text);
        self.forward.on_delta(delta);
    }
    fn on_tool_call(&self, call: chan_llm::ToolCall) {
        self.state
            .lock()
            .expect("collect state poisoned")
            .tool_calls
            .push(call.clone());
        self.forward.on_tool_call(call);
    }
    fn on_tool_result(&self, result: chan_llm::ToolResult) {
        self.forward.on_tool_result(result);
    }
    fn on_done(&self, reason: chan_llm::StopReason) {
        {
            let mut s = self.state.lock().expect("collect state poisoned");
            s.stop_reason = Some(reason);
            s.finished = true;
        }
        self.done.notify_waiters();
        self.forward.on_done(reason);
    }
    fn on_error(&self, error: String) {
        {
            let mut s = self.state.lock().expect("collect state poisoned");
            if s.error.is_none() {
                s.error = Some(error.clone());
            }
        }
        self.forward.on_error(error);
    }
}

async fn api_llm_complete(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CompleteBody>,
) -> Response {
    let config = state.llm_config.lock().unwrap().clone();
    // Active backend determines the model echoed back in the
    // response. Falls through the same way /api/llm/status does
    // (config override > backend default).
    let active = config.backend.unwrap_or(BackendKind::Anthropic);
    let model = config
        .models
        .for_backend(active)
        .map(str::to_owned)
        .unwrap_or_else(|| active.default_model().to_string());

    // session_id is optional now; generate one when absent so the
    // /ws side channel still has a correlatable id without
    // requiring the simple sync caller to track one.
    let session_id = body.session_id.clone().unwrap_or_else(random_session_id);

    let session = LlmSession::new(state.drive().clone(), config);
    let collect = Arc::new(CollectListener::new(LlmBroadcastListener {
        tx: state.events_tx.clone(),
        session_id,
    }));

    // Prefer the full `messages` array; fall back to wrapping a
    // single `message` string as one user turn. The frontend
    // sends the array form once it has chat history; the simpler
    // form is for one-shot prompts without context.
    let messages: Vec<LlmMessage> = if !body.messages.is_empty() {
        body.messages.into_iter().map(LlmMessage::from).collect()
    } else if let Some(text) = body.message {
        vec![LlmMessage::user(text)]
    } else {
        return err(
            StatusCode::BAD_REQUEST,
            "either `message` or `messages` is required".into(),
        );
    };

    // chan-llm's send is fire-and-forget (spawns the run_loop on
    // the ambient runtime); we wait on CollectListener's Notify
    // for the on_done signal. Events still fan out to /ws live;
    // this handler just blocks until the turn completes.
    let listener: Arc<dyn SessionListener> = collect.clone();
    session.send(messages, listener);
    collect.done.notified().await;

    let snapshot = collect.state.lock().expect("collect state poisoned");
    if let Some(err_msg) = snapshot.error.clone() {
        // chan-llm reports backend / network failures via on_error
        // before on_done(Error). Surface the original message at
        // 502 so the chat UI can show "anthropic 401: ..." instead
        // of a vague "other".
        return err(StatusCode::BAD_GATEWAY, err_msg);
    }
    let stop = snapshot
        .stop_reason
        .unwrap_or(chan_llm::StopReason::EndOfTurn);
    let tool_calls = snapshot
        .tool_calls
        .iter()
        .map(|c| LlmToolCallView {
            id: c.id.clone(),
            name: c.name.clone(),
            input: c.args.clone(),
        })
        .collect();
    Json(LlmCompletionResponse {
        content: snapshot.text.clone(),
        tool_calls,
        stop_reason: stop_reason_tag(stop),
        model,
    })
    .into_response()
}

/// Random session id for the WS correlation channel. Used when the
/// caller didn't supply one. Same alphabet as the auth token; the
/// id is opaque so the exact shape doesn't matter as long as it's
/// unlikely to collide on the same socket.
fn random_session_id() -> String {
    let mut bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
        .iter()
        .map(|b| {
            const A: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
            A[(*b as usize) % A.len()] as char
        })
        .collect()
}

#[derive(Deserialize)]
struct SetKeyBody {
    key: String,
}

async fn api_llm_set_anthropic_key(Json(body): Json<SetKeyBody>) -> Response {
    match chan_llm::keys::set(BackendKind::Anthropic, &body.key) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_llm(&e),
    }
}

async fn api_llm_clear_anthropic_key() -> Response {
    match chan_llm::keys::clear(BackendKind::Anthropic) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_llm(&e),
    }
}

async fn api_llm_set_gemini_key(Json(body): Json<SetKeyBody>) -> Response {
    match chan_llm::keys::set(BackendKind::Gemini, &body.key) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_llm(&e),
    }
}

async fn api_llm_clear_gemini_key() -> Response {
    match chan_llm::keys::clear(BackendKind::Gemini) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_llm(&e),
    }
}

/// One model entry in a catalog response. `supports_tools` is
/// hardcoded true for Anthropic and Gemini today (their entire
/// chat catalog supports function calling); future non-tool
/// variants would narrow this by name.
#[derive(Serialize)]
struct LlmModelEntry {
    name: &'static str,
    supports_tools: bool,
}

#[derive(Serialize)]
struct LlmModelEntryOwned {
    name: String,
    supports_tools: bool,
}

#[derive(Serialize)]
struct CatalogResponse {
    models: Vec<LlmModelEntryOwned>,
    /// Provenance tag for the Settings UI's "why is this list
    /// short" copy. live = fetched from upstream, curated = no
    /// key set so we returned a static shortlist, fallback = key
    /// set but live fetch failed.
    source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Curated Anthropic shortlist. Used when no key is configured
/// (so the dropdown isn't empty) and as the fallback when the
/// `/v1/models` call fails. Sorted newest-first; trim when older
/// generations stop being recommended.
const CURATED_ANTHROPIC: &[LlmModelEntry] = &[
    LlmModelEntry {
        name: "claude-opus-4-7",
        supports_tools: true,
    },
    LlmModelEntry {
        name: "claude-sonnet-4-6",
        supports_tools: true,
    },
    LlmModelEntry {
        name: "claude-haiku-4-5",
        supports_tools: true,
    },
];

/// Curated Gemini shortlist. Same purpose as the Anthropic one.
const CURATED_GEMINI: &[LlmModelEntry] = &[
    LlmModelEntry {
        name: "gemini-2.5-pro",
        supports_tools: true,
    },
    LlmModelEntry {
        name: "gemini-2.5-flash",
        supports_tools: true,
    },
];

fn curated_to_owned(curated: &[LlmModelEntry]) -> Vec<LlmModelEntryOwned> {
    curated
        .iter()
        .map(|e| LlmModelEntryOwned {
            name: e.name.to_string(),
            supports_tools: e.supports_tools,
        })
        .collect()
}

async fn api_llm_anthropic_models(State(state): State<Arc<AppState>>) -> Response {
    let cfg = state.llm_config.lock().unwrap().clone();
    let (key, _) = chan_llm::keys::resolve(BackendKind::Anthropic, &cfg);
    let Some(key) = key else {
        return Json(CatalogResponse {
            models: curated_to_owned(CURATED_ANTHROPIC),
            source: "curated",
            error: None,
        })
        .into_response();
    };
    match chan_llm::backends::anthropic::list_models(&key).await {
        Ok(models) => Json(CatalogResponse {
            models: models
                .into_iter()
                .map(|name| LlmModelEntryOwned {
                    name,
                    supports_tools: true,
                })
                .collect(),
            source: "live",
            error: None,
        })
        .into_response(),
        Err(e) => Json(CatalogResponse {
            models: curated_to_owned(CURATED_ANTHROPIC),
            source: "fallback",
            error: Some(e.to_string()),
        })
        .into_response(),
    }
}

async fn api_llm_gemini_models(State(state): State<Arc<AppState>>) -> Response {
    let cfg = state.llm_config.lock().unwrap().clone();
    let (key, _) = chan_llm::keys::resolve(BackendKind::Gemini, &cfg);
    let Some(key) = key else {
        return Json(CatalogResponse {
            models: curated_to_owned(CURATED_GEMINI),
            source: "curated",
            error: None,
        })
        .into_response();
    };
    match chan_llm::backends::gemini::list_models(&key).await {
        Ok(models) => Json(CatalogResponse {
            models: models
                .into_iter()
                .map(|name| LlmModelEntryOwned {
                    name,
                    supports_tools: true,
                })
                .collect(),
            source: "live",
            error: None,
        })
        .into_response(),
        Err(e) => Json(CatalogResponse {
            models: curated_to_owned(CURATED_GEMINI),
            source: "fallback",
            error: Some(e.to_string()),
        })
        .into_response(),
    }
}

/// Ollama URL probe query: the Settings UI passes the user's typed
/// URL so the dropdown can refresh against a remote daemon without
/// persisting the URL first. Empty / absent falls through to the
/// same precedence chan-llm uses at request time
/// (env OLLAMA_HOST > config > hardcoded default).
#[derive(Deserialize)]
struct OllamaModelsQuery {
    #[serde(default)]
    url: Option<String>,
}

async fn api_llm_ollama_models(
    State(state): State<Arc<AppState>>,
    Query(q): Query<OllamaModelsQuery>,
) -> Response {
    let cfg = state.llm_config.lock().unwrap().clone();
    // Resolution mirrors backends::build's Ollama branch:
    //   1. ?url= query (the user's typed value in Settings)
    //   2. OLLAMA_HOST env (per-shell override)
    //   3. config.urls.ollama (Settings UI persistence)
    //   4. hardcoded default
    let url = q
        .url
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("OLLAMA_HOST").ok().filter(|s| !s.is_empty()))
        .or_else(|| cfg.urls.ollama.clone())
        .unwrap_or_else(|| chan_llm::backends::ollama::DEFAULT_URL.to_string());
    match chan_llm::backends::ollama::list_models(&url).await {
        Ok(models) => Json(
            models
                .into_iter()
                .map(|name| LlmModelEntryOwned {
                    name,
                    supports_tools: true,
                })
                .collect::<Vec<_>>(),
        )
        .into_response(),
        // The frontend types ollamaModels as `LlmModelEntry[]` (no
        // wrapper) and treats request errors as "daemon unreachable".
        // Surface a 503 so the Settings UI's catch arm fires the
        // standard error toast with the upstream message.
        Err(e) => err(StatusCode::SERVICE_UNAVAILABLE, e.to_string()),
    }
}

// ----- server preferences -------------------------------------------------
//
// Holds chan-server-specific paths and toggles that aren't user
// content (those live in the drive) and aren't LLM-shaped (those
// live in chan-llm). See `config.rs`. The split:
//
//   /api/drive             chan-core registry: name, root
//   /api/llm/status        chan-llm config: backend, model, keys
//   /api/server/config     this: attachments_dir, answers_dir

async fn api_get_server_config(State(state): State<Arc<AppState>>) -> Response {
    let cfg = state.server_config.lock().unwrap().clone();
    Json(cfg).into_response()
}

#[derive(Deserialize)]
struct PatchServerConfigBody {
    /// Drive-relative POSIX path. Empty string is rejected
    /// because the path is used as a prefix; an empty prefix
    /// would land attachments in the drive root, surprising
    /// the user.
    #[serde(default)]
    attachments_dir: Option<String>,
    #[serde(default)]
    answers_dir: Option<String>,
}

async fn api_patch_server_config(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PatchServerConfigBody>,
) -> Response {
    let mut cfg = state.server_config.lock().unwrap();
    if let Some(p) = body.attachments_dir {
        if p.is_empty() {
            return err(
                StatusCode::BAD_REQUEST,
                "attachments_dir must be non-empty".into(),
            );
        }
        cfg.attachments_dir = p;
    }
    if let Some(p) = body.answers_dir {
        if p.is_empty() {
            return err(
                StatusCode::BAD_REQUEST,
                "answers_dir must be non-empty".into(),
            );
        }
        cfg.answers_dir = p;
    }
    if let Err(e) = cfg.save() {
        return err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }
    Json(cfg.clone()).into_response()
}

// ----- unified preferences (/api/config) ---------------------------------
//
// Frontend treats Settings as a single round-trip surface: GET the
// whole GlobalConfig (preferences + drives + default_drive_root),
// PATCH the same shape on save. We assemble the view from three
// underlying stores (EditorPrefs, ServerConfig, LlmConfig) plus the
// chan-core registry and route the writes back the same way.

#[derive(Serialize)]
struct GlobalConfigView {
    preferences: PreferencesView,
    /// Empty string serializes as None (the resolver falls back to
    /// the platform default).
    default_drive_root: Option<String>,
    drives: Vec<KnownDriveView>,
}

#[derive(Serialize)]
struct KnownDriveView {
    path: String,
    name: Option<String>,
    /// RFC3339 timestamp.
    last_opened: String,
}

#[derive(Deserialize)]
struct PatchConfigBody {
    /// Whole-block replacement. Frontend sends the entire
    /// GlobalConfig on every save.
    #[serde(default)]
    preferences: Option<PreferencesView>,
    #[serde(default)]
    default_drive_root: Option<Option<String>>,
    /// Read-only on PATCH: drives are managed via /api/drive PATCH
    /// (rename) and the CLI (`chan add` / `remove`). Frontend sends
    /// the field for round-tripping; we just ignore it.
    #[serde(default)]
    #[allow(dead_code)]
    drives: Option<serde_json::Value>,
}

fn global_config_view(state: &AppState) -> GlobalConfigView {
    let drives = state
        .library
        .list_drives()
        .into_iter()
        .map(|d| KnownDriveView {
            path: d.path.to_string_lossy().into_owned(),
            name: d.name,
            last_opened: d.last_opened.to_rfc3339(),
        })
        .collect();
    GlobalConfigView {
        preferences: preferences_view(state),
        default_drive_root: state
            .library
            .default_drive_root()
            .map(|p| p.to_string_lossy().into_owned()),
        drives,
    }
}

async fn api_get_config(State(state): State<Arc<AppState>>) -> Response {
    Json(global_config_view(&state)).into_response()
}

async fn api_patch_config(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PatchConfigBody>,
) -> Response {
    if let Some(prefs) = body.preferences {
        if let Err(e) = apply_preferences(&state, prefs) {
            return err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        }
    }
    if let Some(opt) = body.default_drive_root {
        let trimmed = opt.as_ref().map(|s| s.trim().to_string());
        let value = match trimmed {
            Some(s) if s.is_empty() => None,
            other => other,
        };
        if let Err(e) = state
            .library
            .set_default_drive_root(value.map(std::path::PathBuf::from))
        {
            return err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        }
    }
    Json(global_config_view(&state)).into_response()
}

/// Split the unified Preferences body across the three backing
/// stores. Each store saves itself; a partial failure leaves the
/// caller with whatever wrote successfully (no two-phase commit).
fn apply_preferences(state: &AppState, view: PreferencesView) -> Result<(), Error> {
    {
        let mut editor = state.editor_prefs.lock().expect("editor prefs poisoned");
        editor.fonts = view.fonts;
        editor.theme = view.theme;
        editor.pane_widths = view.pane_widths;
        editor.line_spacing = view.line_spacing;
        editor.date_format = view.date_format;
        editor.save()?;
    }
    {
        let mut server = state.server_config.lock().expect("server config poisoned");
        if !view.attachments_dir.is_empty() {
            server.attachments_dir = view.attachments_dir;
        }
        if !view.assistant.answers_dir.is_empty() {
            server.answers_dir = view.assistant.answers_dir;
        }
        server.save()?;
    }
    {
        let mut llm = state.llm_config.lock().expect("llm config poisoned");
        // The "embedded" backend has no chan-llm counterpart yet; a
        // PATCH carrying it is a no-op (the field round-trips as
        // the previous backend on the next read).
        if let Some(kind) = view.assistant.backend.to_chan_llm() {
            llm.backend = if view.assistant.enabled {
                Some(kind)
            } else {
                None
            };
        } else if !view.assistant.enabled {
            llm.backend = None;
        }
        llm.auto_apply_writes = view.assistant.auto_apply_writes;
        llm.models.anthropic = view.assistant.claude.model;
        llm.models.gemini = view.assistant.gemini.model;
        llm.models.ollama = view.assistant.ollama.model;
        // Empty string from the form clears the override (back to
        // env or the hardcoded default). Trim before storing so a
        // copy-pasted URL with whitespace doesn't break the http
        // client.
        llm.urls.ollama = view
            .assistant
            .ollama
            .url
            .map(|u| u.trim().to_string())
            .filter(|u| !u.is_empty());
        llm.save()
            .map_err(|e| Error::Config(format!("save llm config: {e}")))?;
    }
    Ok(())
}

// ----- build identity -----------------------------------------------------
//
// Compile-time identity for the running chan binary. The frontend's
// Settings "About" footer reads this so users can tell at a glance
// which version they're on and whether semantic search is available.
// The values come from CARGO_PKG_VERSION and cfg!(feature = ...) at
// build time; nothing is computed at runtime.

#[derive(Serialize)]
struct BuildInfo {
    version: &'static str,
    features: BuildFeatures,
}

#[derive(Serialize)]
struct BuildFeatures {
    /// Hybrid (BM25 + dense) search depends on the embeddings cargo
    /// feature being on at build time. When false, search falls back
    /// to BM25-only and the Settings "Search" section reflects that.
    /// chan-server itself doesn't gate on this feature; we forward
    /// chan-core's compile-time flag as exposed through the
    /// `chan_core::has_embeddings` helper.
    embeddings: bool,
}

async fn api_build_info() -> Response {
    Json(BuildInfo {
        version: env!("CARGO_PKG_VERSION"),
        features: BuildFeatures {
            // chan-core today is BM25-only (tantivy gated by the
            // `search` feature). Dense embeddings are a future v0.2
            // feature; the field stays in the contract so the
            // frontend's "Settings -> Search" section can render
            // accurate copy without a v0.2-incompatible refactor.
            embeddings: false,
        },
    })
    .into_response()
}

// ----- sessions / assistant blobs / answers ------------------------------
//
// chan-core owns the I/O (Drive::{put,get,list,delete}_session +
// _assistant + clear_assistant). chan-server is a thin HTTP shell;
// the JSON schema of session blobs (window/pane layout) and
// assistant blobs (chat turns) lives in the frontend, not here.
//
// Answers are different: the user picks a directory inside the
// drive (`server.toml` -> answers_dir) and we land each saved
// answer as a `.md` file there via Drive::write_text. Same path
// sandbox + special-file refusal apply.

/// Window id query param (`?w=<id>`) for session routes.
#[derive(Deserialize)]
struct SessionQuery {
    w: String,
}

/// Conversation key query param (`?path=<key>`) for the assistant
/// blob routes. The key is either a file path (per-file
/// conversation) or a synthetic group key (per-window-pane group);
/// the server treats it as opaque since the chunking is the
/// frontend's concern.
#[derive(Deserialize)]
struct ConversationQuery {
    path: String,
}

async fn api_get_session(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SessionQuery>,
) -> Response {
    match state.drive().get_session(&q.w) {
        Ok(Some(bytes)) => raw_json_response(bytes),
        // 204 NO_CONTENT, not 404: "no session yet" is the normal
        // first-launch state. transport.ts treats an empty 2xx body
        // as `undefined`; the api wrapper coerces that to `null`.
        Ok(None) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_from(&e),
    }
}

async fn api_put_session(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SessionQuery>,
    body: axum::body::Bytes,
) -> Response {
    match state.drive().put_session(&q.w, &body) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_from(&e),
    }
}

async fn api_delete_session(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SessionQuery>,
) -> Response {
    match state.drive().delete_session(&q.w) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_from(&e),
    }
}

async fn api_list_sessions(State(state): State<Arc<AppState>>) -> Response {
    match state.drive().list_sessions() {
        Ok(keys) => Json(keys).into_response(),
        Err(e) => err_from(&e),
    }
}

async fn api_get_assistant(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ConversationQuery>,
) -> Response {
    match state.drive().get_assistant(&q.path) {
        Ok(Some(bytes)) => raw_json_response(bytes),
        // 204 NO_CONTENT, not 404: same reasoning as get_session.
        Ok(None) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_from(&e),
    }
}

async fn api_put_assistant(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ConversationQuery>,
    body: axum::body::Bytes,
) -> Response {
    match state.drive().put_assistant(&q.path, &body) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_from(&e),
    }
}

async fn api_delete_assistant(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ConversationQuery>,
) -> Response {
    match state.drive().delete_assistant(&q.path) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_from(&e),
    }
}

async fn api_list_assistant(State(state): State<Arc<AppState>>) -> Response {
    match state.drive().list_assistant() {
        Ok(keys) => Json(keys).into_response(),
        Err(e) => err_from(&e),
    }
}

async fn api_clear_assistant(State(state): State<Arc<AppState>>) -> Response {
    match state.drive().clear_assistant() {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_from(&e),
    }
}

#[derive(Deserialize)]
struct AnswerBody {
    /// Markdown content to save. Becomes a new `.md` file under
    /// the configured `answers_dir`. Filename is derived from the
    /// body's first heading or, failing that, a timestamp slug.
    content: String,
    /// Optional override for the filename stem (no extension; the
    /// server appends `.md`). Useful when the frontend generates
    /// its own stable id for a saved answer.
    #[serde(default)]
    name: Option<String>,
}

#[derive(Serialize)]
struct AnswerSaved {
    /// Drive-relative POSIX path the answer landed at.
    path: String,
}

/// POST /api/attachments. Multipart upload from the editor's
/// `![...]` picker / drag-and-drop / clipboard paste. The
/// frontend sends one part named `file`; we slugify the original
/// filename, prefix with the unix timestamp (collision
/// resistance), and write under `attachments_dir` via
/// Drive::write_bytes (so the path sandbox + special-file
/// refusal apply). Returns the drive-relative path the file
/// landed at, matching the frontend's `uploadAttachment`
/// contract.
async fn api_post_attachment(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Response {
    let dir = state.server_config.lock().unwrap().attachments_dir.clone();

    // First part named "file" wins; later parts (extra form
    // fields the frontend may add for captions etc.) are ignored
    // for now. Errors from the multipart stream become 400 since
    // they typically mean the client framed the request wrong.
    let mut chosen: Option<(String, Vec<u8>)> = None;
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                if field.name() != Some("file") {
                    continue;
                }
                let filename = field.file_name().unwrap_or("").to_owned();
                let bytes = match field.bytes().await {
                    Ok(b) => b.to_vec(),
                    Err(e) => {
                        return err(StatusCode::BAD_REQUEST, format!("multipart read: {e}"));
                    }
                };
                chosen = Some((filename, bytes));
                break;
            }
            Ok(None) => break,
            Err(e) => {
                return err(StatusCode::BAD_REQUEST, format!("multipart parse: {e}"));
            }
        }
    }

    let Some((original, bytes)) = chosen else {
        return err(
            StatusCode::BAD_REQUEST,
            "missing `file` part in multipart body".into(),
        );
    };

    if bytes.is_empty() {
        return err(StatusCode::BAD_REQUEST, "empty file".into());
    }

    // Filename: <unix_ts>-<slugified-stem>.<ext>. Keeping the
    // unix timestamp at the front gives natural sort + collision
    // resistance without committing to a date format the frontend
    // would parse. Extension is preserved (lowercased) so the
    // browser's content-type sniffer agrees with what the editor
    // wrote.
    let (stem, ext) = split_filename(&original);
    let stem_slug = slugify_for_filename(stem);
    let stem_or_default = if stem_slug.is_empty() {
        "file"
    } else {
        &stem_slug
    };
    let ext = ext.map(|e| e.to_ascii_lowercase()).unwrap_or_default();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let saved = if ext.is_empty() {
        format!("{ts}-{stem_or_default}")
    } else {
        format!("{ts}-{stem_or_default}.{ext}")
    };
    let rel = format!("{dir}/{saved}");

    if let Err(e) = state.drive().write_bytes(&rel, &bytes) {
        return err_from(&e);
    }
    state.self_writes.note(&rel);
    Json(serde_json::json!({ "path": rel })).into_response()
}

/// Split `foo.bar.PNG` into (`"foo.bar"`, Some("PNG")). Bare
/// names with no `.` return (input, None). Hidden files like
/// `.gitignore` are treated as having no extension (`.gitignore`,
/// None) so we don't produce a garbage extension.
fn split_filename(name: &str) -> (&str, Option<&str>) {
    if name.starts_with('.') {
        return (name, None);
    }
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => (stem, Some(ext)),
        _ => (name, None),
    }
}

async fn api_post_answer(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AnswerBody>,
) -> Response {
    let dir = state.server_config.lock().unwrap().answers_dir.clone();
    let stem = body
        .name
        .as_deref()
        .map(slugify_for_filename)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            body.content
                .lines()
                .find_map(extract_h1)
                .map(|s| slugify_for_filename(&s))
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(timestamp_slug);
    let rel = format!("{dir}/{stem}.md");
    match state.drive().write_text(&rel, &body.content) {
        Ok(()) => {
            state.self_writes.note(&rel);
            Json(AnswerSaved { path: rel }).into_response()
        }
        Err(e) => err_from(&e),
    }
}

// ----- storage reset ------------------------------------------------------
//
// Drops the drive's writer lock by replacing the active DriveCell,
// runs chan-core's Library::reset_drive (which acquires the lock
// briefly to verify exclusive access), then reopens the drive and
// re-attaches the watcher in a fresh cell. Frontend reloads the
// window after a successful reset, so any in-flight handler clones
// of the old Arc<Drive> drain naturally.

/// Body of `POST /api/storage/reset`. Two modes mirror the chan-
/// core enum; the JSON tag is lowercased for the frontend's
/// `ResetMode` type.
#[derive(Deserialize)]
struct ResetBody {
    mode: ResetModeView,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum ResetModeView {
    /// Map -> chan-core ResetMode::State (keep the registry entry).
    Drive,
    /// Map -> chan-core ResetMode::Everything.
    Everything,
}

impl From<ResetModeView> for ResetMode {
    fn from(m: ResetModeView) -> Self {
        match m {
            ResetModeView::Drive => ResetMode::State,
            ResetModeView::Everything => ResetMode::Everything,
        }
    }
}

#[derive(Serialize)]
struct ResetResponse {
    removed_entries: usize,
}

/// How long the reset path waits for outstanding `Arc<Drive>` clones
/// (in-flight handler tasks) to drop before giving up. Editor-side
/// I/O is fast (markdown reads / writes); 5 s is comfortable
/// headroom without making a misclick feel like a hang.
const RESET_DRAIN_DEADLINE: Duration = Duration::from_secs(5);

async fn api_storage_reset(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ResetBody>,
) -> Response {
    let mode: ResetMode = body.mode.into();
    // Run the reset on a blocking-thread: the drain spin-wait sleeps
    // and the chan-core wipe walks the filesystem; neither belongs
    // on the async runtime's worker thread.
    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || perform_reset(&state_clone, mode)).await;
    match result {
        Ok(Ok(report)) => Json(ResetResponse {
            removed_entries: report.removed_entries,
        })
        .into_response(),
        Ok(Err(e)) => err_from_reset(&e),
        Err(e) => err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("reset task: {e}"),
        ),
    }
}

#[derive(Debug)]
enum ResetError {
    Busy,
    Core(chan_core::ChanError),
}

fn err_from_reset(e: &ResetError) -> Response {
    match e {
        ResetError::Busy => err(
            StatusCode::CONFLICT,
            "drive busy: in-flight requests still hold the writer lock; \
             retry in a moment"
                .into(),
        ),
        ResetError::Core(c) => err_from(c),
    }
}

/// Replace `state.drive_cell` end-to-end. Holds the write lock the
/// entire time so handlers waiting on the read lock see exactly one
/// transition (old drive -> new drive); they never observe the
/// `None` middle state.
fn perform_reset(state: &AppState, mode: ResetMode) -> Result<chan_core::ResetReport, ResetError> {
    let mut cell_guard = state.drive_cell.write().expect("drive cell poisoned");
    let mut cell = cell_guard
        .take()
        .expect("drive cell missing outside reset window");
    // Stop the watcher first so nothing notify-side keeps a Drive
    // ref alive past our drop.
    cell.watch_handle.take();
    let drive_weak = Arc::downgrade(&cell.drive);
    drop(cell);
    // Wait for in-flight handler tasks to drop their Arc<Drive>
    // clones; the cell can't be reborrowed while we hold the write
    // lock so the count strictly decreases. Spin-sleep on a
    // blocking thread (we're in spawn_blocking).
    let deadline = Instant::now() + RESET_DRAIN_DEADLINE;
    while drive_weak.upgrade().is_some() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(25));
    }
    if drive_weak.upgrade().is_some() {
        // Outstanding clones never dropped. Restore the original
        // cell so handlers can proceed; reset has to be retried.
        // We need the strong ref back; the simplest path is to
        // reopen against the same root, since the cell was already
        // dropped from our side. The user retry is the same UX
        // either way.
        let drive = state
            .library
            .open_drive(&state.drive_root)
            .map_err(ResetError::Core)?;
        let bridge = make_watch_bridge(&state.events_tx, &state.self_writes);
        let watch_handle = drive.watch(bridge).map_err(ResetError::Core)?;
        *cell_guard = Some(DriveCell {
            drive,
            watch_handle: Some(watch_handle),
        });
        return Err(ResetError::Busy);
    }
    // Clean. Run the actual wipe, reopen, restart watcher.
    let report = state
        .library
        .reset_drive(&state.drive_root, mode)
        .map_err(ResetError::Core)?;
    let drive = state
        .library
        .open_drive(&state.drive_root)
        .map_err(ResetError::Core)?;
    let bridge = make_watch_bridge(&state.events_tx, &state.self_writes);
    let watch_handle = drive.watch(bridge).map_err(ResetError::Core)?;
    *cell_guard = Some(DriveCell {
        drive,
        watch_handle: Some(watch_handle),
    });
    Ok(report)
}

/// Wrap an opaque blob in an `application/json` response. We don't
/// re-parse + re-serialize because the blob may be large and we
/// trust whoever wrote it (Drive::put_*) handed back exactly what
/// they got. If the blob isn't JSON the client sees the raw bytes
/// with the wrong content-type, which is acceptable for opaque
/// storage that the frontend writes itself.
fn raw_json_response(bytes: Vec<u8>) -> Response {
    (
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        bytes,
    )
        .into_response()
}

fn extract_h1(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let stripped = trimmed.strip_prefix("# ")?;
    let s = stripped.trim().trim_end_matches('#').trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Strip a string into a filesystem-safe slug. Keeps ASCII alnum,
/// '-', '_'; collapses everything else to '-'; trims leading and
/// trailing dashes; clamps to 80 chars (safe under chan-core's
/// blob key length and most filesystems' name limits).
fn slugify_for_filename(s: &str) -> String {
    let mut out = String::with_capacity(s.len().min(80));
    let mut last_dash = true;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
        if out.len() >= 80 {
            break;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    while out.starts_with('-') {
        out.remove(0);
    }
    out
}

/// Fallback name when no header / explicit name was provided:
/// `answer-YYYYMMDD-HHMMSS`. Uses the system clock; tests should
/// pass `name` to keep filenames deterministic.
fn timestamp_slug() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("answer-{secs}")
}

fn err_llm(e: &LlmError) -> Response {
    let status = match e {
        LlmError::MissingApiKey(_) => StatusCode::BAD_REQUEST,
        LlmError::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
        LlmError::BackendError { status, .. } => {
            StatusCode::from_u16(*status).unwrap_or(StatusCode::BAD_GATEWAY)
        }
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    err(status, e.to_string())
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
