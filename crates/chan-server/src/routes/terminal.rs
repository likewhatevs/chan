//! Interactive PTY-backed terminal sessions and terminal control APIs.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Json, Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use portable_pty::PtySize;
use serde::{Deserialize, Serialize};

use crate::error::err_tunnel_public_locked;
use crate::event_watcher::{SurveyAnswer, SurveyScope};
use crate::signal::now_unix_secs;
use crate::state::AppState;
use crate::terminal_sessions::{
    CloseReason, CreateError, CreateOptions, SessionEvent, ALT_SCREEN_ATTACH_PRELUDE,
};

const DEFAULT_COLS: u16 = 80;
const DEFAULT_ROWS: u16 = 24;
const MAX_COLS: u16 = 500;
const MAX_ROWS: u16 = 200;

#[derive(Debug, Deserialize)]
pub struct TerminalQuery {
    session: Option<String>,
    since: Option<u64>,
    cols: Option<u16>,
    rows: Option<u16>,
    tab_name: Option<String>,
    window_id: Option<String>,
    mcp_env: Option<TerminalMcpEnv>,
    cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WatcherBody {
    path: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTerminalBody {
    name: String,
    command: String,
    #[serde(default)]
    env: BTreeMap<String, String>,
    orchestrator_session: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateTerminalResponse {
    session: String,
    tab_label: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EventReplyBody {
    id: String,
    #[serde(rename = "type")]
    event_type: EventReplyType,
    from: String,
    to: String,
    answers: Vec<SurveyAnswer>,
    scope_grant: SurveyScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum EventReplyType {
    SurveyReply,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum TerminalMcpEnv {
    #[default]
    On,
    Off,
}

impl TerminalMcpEnv {
    fn enabled(self) -> bool {
        matches!(self, Self::On)
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientFrame {
    #[serde(rename = "input")]
    Input { data: String },
    #[serde(rename = "resize")]
    Resize { cols: u16, rows: u16 },
    #[serde(rename = "cwd")]
    Cwd,
    #[serde(rename = "close")]
    Close,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ServerFrame {
    #[serde(rename = "session")]
    Session {
        id: String,
        seq: u64,
        missed_bytes: u64,
    },
    #[serde(rename = "ready")]
    Ready {
        cols: u16,
        rows: u16,
        #[serde(skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
    },
    #[serde(rename = "cwd")]
    Cwd {
        #[serde(skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
    },
    #[serde(rename = "resize")]
    Resize { cols: u16, rows: u16 },
    #[serde(rename = "exit")]
    Exit { code: u32 },
    #[serde(rename = "closed")]
    Closed { reason: CloseReason },
    #[serde(rename = "error")]
    Error {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<&'static str>,
    },
}

pub async fn api_terminal_ws(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TerminalQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    if state.tunnel_public {
        return err_tunnel_public_locked();
    }

    let size = pty_size(query.cols, query.rows);
    let tab_name = query.tab_name.as_deref().and_then(normalize_tab_name);
    let window_id = query.window_id.as_deref().and_then(normalize_window_id);
    let mcp_env = query.mcp_env.unwrap_or_default().enabled();
    let cwd = if query.session.is_some() {
        None
    } else {
        match resolve_terminal_cwd(&state.drive_root, query.cwd.as_deref()) {
            Ok(cwd) => cwd,
            Err(message) => return (StatusCode::BAD_REQUEST, message).into_response(),
        }
    };
    let opts = TerminalWsOptions {
        session_id: query.session,
        since: query.since,
        size,
        tab_name,
        window_id,
        mcp_env,
        cwd,
    };
    ws.on_upgrade(move |socket| terminal_ws(socket, state, opts))
        .into_response()
}

pub async fn api_set_terminal_watcher(
    State(state): State<Arc<AppState>>,
    AxumPath(session): AxumPath<String>,
    Json(body): Json<WatcherBody>,
) -> Response {
    if state.tunnel_public {
        return err_tunnel_public_locked();
    }
    let dir = match resolve_watcher_dir(&state.drive_root, &body.path) {
        Ok(dir) => dir,
        Err(message) => return (StatusCode::BAD_REQUEST, message).into_response(),
    };
    match state.terminal_sessions.set_watcher(&session, dir) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "terminal session not found").into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            format!("failed to start terminal watcher: {e}"),
        )
            .into_response(),
    }
}

pub async fn api_unset_terminal_watcher(
    State(state): State<Arc<AppState>>,
    AxumPath(session): AxumPath<String>,
) -> Response {
    if state.tunnel_public {
        return err_tunnel_public_locked();
    }
    if state.terminal_sessions.clear_watcher(&session) {
        StatusCode::NO_CONTENT.into_response()
    } else {
        (StatusCode::NOT_FOUND, "terminal watcher not found").into_response()
    }
}

pub async fn api_create_terminal(
    State(state): State<Arc<AppState>>,
    body: Result<Json<CreateTerminalBody>, JsonRejection>,
) -> Response {
    if state.tunnel_public {
        return err_tunnel_public_locked();
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("invalid terminal create: {e}"),
            )
                .into_response()
        }
    };
    let name = match normalize_terminal_name(&body.name) {
        Some(name) => name,
        None => return (StatusCode::BAD_REQUEST, "terminal name is required").into_response(),
    };
    let command = match normalize_terminal_command(&body.command) {
        Some(command) => command,
        None => return (StatusCode::BAD_REQUEST, "terminal command is required").into_response(),
    };
    if let Err(message) = validate_terminal_env(&body.env) {
        return (StatusCode::BAD_REQUEST, message).into_response();
    }
    let preflight = body.orchestrator_session.as_deref().and_then(|id| {
        state
            .terminal_sessions
            .watcher_preflight_config(id, name.clone())
    });
    let opts = CreateOptions {
        size: pty_size(None, None),
        tab_name: Some(name.clone()),
        window_id: None,
        mcp_env: true,
        cwd: None,
        command: Some(command),
        env: body.env,
        preflight,
    };
    match state.terminal_sessions.create(opts) {
        Ok(handle) => (
            StatusCode::CREATED,
            Json(CreateTerminalResponse {
                session: handle.id().to_string(),
                tab_label: name,
            }),
        )
            .into_response(),
        Err(CreateError::Capped) => {
            (StatusCode::CONFLICT, "terminal session cap reached").into_response()
        }
        Err(CreateError::Spawn(e)) => (
            StatusCode::BAD_REQUEST,
            format!("failed to start terminal: {e}"),
        )
            .into_response(),
    }
}

pub async fn api_restart_terminal(
    State(state): State<Arc<AppState>>,
    AxumPath(session): AxumPath<String>,
) -> Response {
    if state.tunnel_public {
        return err_tunnel_public_locked();
    }
    match state.terminal_sessions.restart(&session) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "terminal session not found").into_response(),
        Err(CreateError::Capped) => {
            (StatusCode::CONFLICT, "terminal session cap reached").into_response()
        }
        Err(CreateError::Spawn(e)) => (
            StatusCode::BAD_REQUEST,
            format!("failed to restart terminal: {e}"),
        )
            .into_response(),
    }
}

pub async fn api_delete_terminal(
    State(state): State<Arc<AppState>>,
    AxumPath(session): AxumPath<String>,
) -> Response {
    if state.tunnel_public {
        return err_tunnel_public_locked();
    }
    if state
        .terminal_sessions
        .close(&session, CloseReason::Explicit)
    {
        StatusCode::NO_CONTENT.into_response()
    } else {
        (StatusCode::NOT_FOUND, "terminal session not found").into_response()
    }
}

pub async fn api_terminal_event_reply(
    State(state): State<Arc<AppState>>,
    AxumPath(session): AxumPath<String>,
    body: Result<Json<EventReplyBody>, JsonRejection>,
) -> Response {
    if state.tunnel_public {
        return err_tunnel_public_locked();
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, format!("invalid event reply: {e}")).into_response()
        }
    };
    if let Err(message) = validate_event_reply(&body) {
        return (StatusCode::BAD_REQUEST, message).into_response();
    }
    let Some(dir) = state.terminal_sessions.watcher_dir(&session) else {
        return (
            StatusCode::CONFLICT,
            "terminal watcher is not attached".to_string(),
        )
            .into_response();
    };
    match write_event_reply_atomic(&dir, &body).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            format!("failed to write event reply: {e}"),
        )
            .into_response(),
    }
}

struct TerminalWsOptions {
    session_id: Option<String>,
    since: Option<u64>,
    size: PtySize,
    tab_name: Option<String>,
    window_id: Option<String>,
    mcp_env: bool,
    cwd: Option<PathBuf>,
}

fn validate_event_reply(body: &EventReplyBody) -> Result<(), String> {
    if body.id.trim().is_empty() {
        return Err("event reply id is required".into());
    }
    if body.from.trim().is_empty() {
        return Err("event reply from is required".into());
    }
    if body.to.trim().is_empty() {
        return Err("event reply to is required".into());
    }
    Ok(())
}

fn normalize_terminal_name(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.chars().take(128).collect())
}

fn normalize_terminal_command(command: &str) -> Option<String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

fn validate_terminal_env(env: &BTreeMap<String, String>) -> Result<(), String> {
    for key in env.keys() {
        if key.trim().is_empty() || key.contains('=') || key.contains('\0') {
            return Err(format!("invalid terminal env key: {key:?}"));
        }
    }
    for value in env.values() {
        if value.contains('\0') {
            return Err("invalid terminal env value: contains NUL".into());
        }
    }
    Ok(())
}

async fn write_event_reply_atomic(dir: &Path, body: &EventReplyBody) -> std::io::Result<()> {
    let file_id = event_reply_file_id(&body.id);
    let final_path = dir.join(format!("event-reply-{file_id}.md"));
    let tmp_path = dir.join(format!(
        ".event-reply-{file_id}-{:016x}.tmp",
        rand::random::<u64>()
    ));
    let bytes = serde_json::to_vec(body)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    let result = async {
        tokio::fs::write(&tmp_path, bytes).await?;
        tokio::fs::rename(&tmp_path, &final_path).await
    }
    .await;
    let _ = tokio::fs::remove_file(&tmp_path).await;
    result
}

fn event_reply_file_id(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    for ch in id.trim().chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch);
        } else {
            out.push('-');
        }
    }
    if out.is_empty() {
        "reply".into()
    } else {
        out
    }
}

async fn terminal_ws(mut socket: WebSocket, state: Arc<AppState>, opts: TerminalWsOptions) {
    state
        .last_activity
        .store(now_unix_secs(), Ordering::Relaxed);

    let create_opts = CreateOptions {
        size: opts.size,
        tab_name: opts.tab_name,
        window_id: opts.window_id,
        mcp_env: opts.mcp_env,
        cwd: opts.cwd,
        command: None,
        env: Default::default(),
        preflight: None,
    };
    let mut session = match state.terminal_sessions.get_or_create(
        opts.session_id.as_deref(),
        opts.since,
        create_opts,
    ) {
        Ok(session) => session,
        Err(CreateError::Capped) => {
            let _ = send_frame(
                &mut socket,
                ServerFrame::Error {
                    message: "terminal session cap reached".into(),
                    reason: Some(CloseReason::Capped.as_str()),
                },
            )
            .await;
            let _ = socket
                .send(Message::Close(Some(CloseFrame {
                    code: 1013,
                    reason: "terminal session cap reached".into(),
                })))
                .await;
            return;
        }
        Err(CreateError::Spawn(e)) => {
            let _ = send_frame(
                &mut socket,
                ServerFrame::Error {
                    message: format!("failed to start terminal: {e}"),
                    reason: None,
                },
            )
            .await;
            return;
        }
    };
    let mut shutdown_rx = state.shutdown_rx.clone();

    let _ = send_frame(
        &mut socket,
        ServerFrame::Session {
            id: session.id().to_owned(),
            seq: session.seq,
            missed_bytes: session.missed_bytes,
        },
    )
    .await;
    for chunk in &session.replay {
        if socket.send(Message::Binary(chunk.clone())).await.is_err() {
            return;
        }
    }
    if session.alt_screen
        && socket
            .send(Message::Binary(ALT_SCREEN_ATTACH_PRELUDE.to_vec()))
            .await
            .is_err()
    {
        return;
    }
    session.request_redraw();
    let _ = send_frame(
        &mut socket,
        ServerFrame::Ready {
            cols: opts.size.cols,
            rows: opts.size.rows,
            cwd: session.cwd().map(path_to_wire),
        },
    )
    .await;

    loop {
        tokio::select! {
            biased;
            _ = shutdown_rx.changed() => {
                let _ = socket
                    .send(Message::Close(Some(CloseFrame {
                        code: 1001,
                        reason: "server shutdown".into(),
                    })))
                    .await;
                break;
            }
            msg = socket.recv() => {
                let Some(msg) = msg else {
                    break;
                };
                match msg {
                    Ok(Message::Text(text)) => {
                        match serde_json::from_str::<ClientFrame>(&text) {
                            Ok(ClientFrame::Input { data }) => {
                                session.send_input(data.as_bytes());
                                state.last_activity.store(now_unix_secs(), Ordering::Relaxed);
                            }
                            Ok(ClientFrame::Resize { cols, rows }) => {
                                session.resize(pty_size(Some(cols), Some(rows)));
                                state.last_activity.store(now_unix_secs(), Ordering::Relaxed);
                            }
                            Ok(ClientFrame::Cwd) => {
                                let cwd = session.cwd().map(path_to_wire);
                                let _ = send_frame(&mut socket, ServerFrame::Cwd { cwd }).await;
                            }
                            Ok(ClientFrame::Close) => {
                                let id = session.id().to_owned();
                                state.terminal_sessions.close(&id, CloseReason::Explicit);
                            }
                            Err(e) => {
                                let _ = send_frame(
                                    &mut socket,
                                    ServerFrame::Error {
                                        message: format!("invalid terminal frame: {e}"),
                                        reason: None,
                                    },
                                )
                                .await;
                            }
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Ok(Message::Ping(_)) | Ok(Message::Pong(_)) | Ok(Message::Binary(_)) => {}
                    Err(_) => break,
                }
            }
            event = session.rx.recv() => {
                match event {
                    Ok(SessionEvent::Output(data)) => {
                        if socket.send(Message::Binary(data)).await.is_err() {
                            break;
                        }
                        state.last_activity.store(now_unix_secs(), Ordering::Relaxed);
                    }
                    Ok(SessionEvent::Resize(size)) => {
                        if send_frame(&mut socket, ServerFrame::Resize { cols: size.cols, rows: size.rows }).await.is_err() {
                            break;
                        }
                    }
                    Ok(SessionEvent::Exit(code)) => {
                        let id = session.id().to_owned();
                        state.terminal_sessions.remove(&id);
                        let _ = send_frame(&mut socket, ServerFrame::Exit { code }).await;
                        break;
                    }
                    Ok(SessionEvent::Error(message)) => {
                        if send_frame(&mut socket, ServerFrame::Error { message, reason: None }).await.is_err() {
                            break;
                        }
                    }
                    Ok(SessionEvent::Closed(reason)) => {
                        let _ = send_frame(&mut socket, ServerFrame::Closed { reason }).await;
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        let _ = send_frame(
                            &mut socket,
                            ServerFrame::Error {
                                message: "terminal output lagged; reconnect to replay retained scrollback".into(),
                                reason: None,
                            },
                        )
                        .await;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}

async fn send_frame(socket: &mut WebSocket, frame: ServerFrame) -> Result<(), axum::Error> {
    socket
        .send(Message::Text(serde_json::to_string(&frame).unwrap_or_else(
            |e| format!(r#"{{"type":"error","message":"serialize failed: {e}"}}"#),
        )))
        .await
}

fn path_to_wire(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

fn pty_size(cols: Option<u16>, rows: Option<u16>) -> PtySize {
    PtySize {
        cols: cols.unwrap_or(DEFAULT_COLS).clamp(1, MAX_COLS),
        rows: rows.unwrap_or(DEFAULT_ROWS).clamp(1, MAX_ROWS),
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn normalize_tab_name(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.chars().take(128).collect())
}

fn normalize_window_id(id: &str) -> Option<String> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.chars().take(256).collect())
}

fn resolve_terminal_cwd(drive_root: &Path, cwd: Option<&str>) -> Result<Option<PathBuf>, String> {
    let Some(raw) = cwd else {
        return Ok(None);
    };
    let rel = raw.trim();
    let abs = if rel.is_empty() {
        drive_root.to_path_buf()
    } else {
        chan_drive::fs_ops::resolve_safe_strict(drive_root, rel)
            .map_err(|e| format!("invalid terminal cwd: {e}"))?
    };
    let meta = std::fs::metadata(&abs).map_err(|e| format!("invalid terminal cwd: {e}"))?;
    if !meta.is_dir() {
        return Err("invalid terminal cwd: path is not a directory".into());
    }
    Ok(Some(abs))
}

fn resolve_watcher_dir(drive_root: &Path, raw: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("watcher path is required".into());
    }
    let path = Path::new(trimmed);
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        chan_drive::fs_ops::resolve_safe_strict(drive_root, trimmed)
            .map_err(|e| format!("invalid watcher path: {e}"))?
    };
    let meta = std::fs::metadata(&abs).map_err(|e| format!("invalid watcher path: {e}"))?;
    if !meta.is_dir() {
        return Err("invalid watcher path: path is not a directory".into());
    }
    Ok(abs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TerminalConfig;
    use crate::terminal_sessions::{AttachHandle, Registry, RegistryConfig};
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use axum::routing::post;
    use axum::Router;
    use std::fs;
    use std::process::Command;
    use std::time::{Duration, Instant};
    use tower::ServiceExt;

    struct TestTerminal {
        _registry: Registry,
        handle: AttachHandle,
    }

    impl TestTerminal {
        fn spawn(
            cwd: std::path::PathBuf,
            size: PtySize,
            tab_name: Option<String>,
            mcp_socket_path: Option<std::path::PathBuf>,
        ) -> Self {
            Self::spawn_with_mcp_env(cwd, size, tab_name, mcp_socket_path, true)
        }

        fn spawn_with_mcp_env(
            cwd: std::path::PathBuf,
            size: PtySize,
            tab_name: Option<String>,
            mcp_socket_path: Option<std::path::PathBuf>,
            mcp_env: bool,
        ) -> Self {
            let registry = Registry::new(RegistryConfig {
                drive_root: cwd,
                mcp_socket_path,
                control_socket_path: Some(std::path::PathBuf::from("/tmp/chan-control-test.sock")),
                terminal: TerminalConfig::default(),
            });
            let handle = registry
                .create(CreateOptions {
                    size,
                    tab_name,
                    window_id: Some("window-test".into()),
                    mcp_env,
                    cwd: None,
                    command: None,
                    env: Default::default(),
                    preflight: None,
                })
                .expect("spawn pty");
            Self {
                _registry: registry,
                handle,
            }
        }
    }

    fn command_available(name: &str) -> bool {
        Command::new("sh")
            .arg("-lc")
            .arg(format!("command -v {name} >/dev/null 2>&1"))
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[test]
    fn resolve_terminal_cwd_allows_drive_relative_directory() {
        let tmp = tempfile::tempdir().expect("temp drive");
        fs::create_dir_all(tmp.path().join("notes/work")).expect("create dir");

        let cwd = resolve_terminal_cwd(tmp.path(), Some("notes/work"))
            .expect("valid cwd")
            .expect("cwd set");

        assert_eq!(cwd, tmp.path().join("notes/work"));
    }

    #[test]
    fn resolve_terminal_cwd_rejects_escape_and_files() {
        let tmp = tempfile::tempdir().expect("temp drive");
        fs::create_dir_all(tmp.path().join("notes")).expect("create dir");
        fs::write(tmp.path().join("notes/today.md"), "x").expect("create file");

        assert!(resolve_terminal_cwd(tmp.path(), Some("../outside")).is_err());
        assert!(resolve_terminal_cwd(tmp.path(), Some("notes/today.md")).is_err());
    }

    #[test]
    fn resolve_watcher_dir_allows_absolute_and_drive_relative_directories() {
        let tmp = tempfile::tempdir().expect("temp drive");
        fs::create_dir_all(tmp.path().join("events")).expect("create dir");

        assert_eq!(
            resolve_watcher_dir(tmp.path(), "events").expect("relative dir"),
            tmp.path().join("events")
        );
        assert_eq!(
            resolve_watcher_dir(tmp.path(), &tmp.path().join("events").display().to_string())
                .expect("absolute dir"),
            tmp.path().join("events")
        );
    }

    #[test]
    fn resolve_watcher_dir_rejects_empty_escape_and_files() {
        let tmp = tempfile::tempdir().expect("temp drive");
        fs::create_dir_all(tmp.path().join("events")).expect("create dir");
        fs::write(tmp.path().join("events/event.json"), "{}").expect("create file");

        assert!(resolve_watcher_dir(tmp.path(), "").is_err());
        assert!(resolve_watcher_dir(tmp.path(), "../outside").is_err());
        assert!(resolve_watcher_dir(tmp.path(), "events/event.json").is_err());
    }

    fn reply_body(id: &str, note: &str) -> EventReplyBody {
        EventReplyBody {
            id: id.into(),
            event_type: EventReplyType::SurveyReply,
            from: "@@Alex".into(),
            to: "@@Systacean".into(),
            answers: vec![SurveyAnswer {
                question_index: 0,
                key: "1".into(),
            }],
            scope_grant: SurveyScope::OneShot,
            note: Some(note.into()),
        }
    }

    fn tmp_reply_files(dir: &Path) -> Vec<String> {
        std::fs::read_dir(dir)
            .expect("read event dir")
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let name = entry.file_name().to_string_lossy().into_owned();
                name.starts_with(".event-reply-").then_some(name)
            })
            .collect()
    }

    async fn response_json(response: Response) -> serde_json::Value {
        let (parts, body) = response.into_parts();
        assert!(
            parts.status.is_success(),
            "response was not success: {}",
            parts.status
        );
        let bytes = to_bytes(body, 8192).await.expect("read body");
        serde_json::from_slice(&bytes).expect("response json")
    }

    fn create_terminal_body(command: &str) -> CreateTerminalBody {
        CreateTerminalBody {
            name: "@@Spawned".into(),
            command: command.into(),
            env: BTreeMap::new(),
            orchestrator_session: None,
        }
    }

    #[test]
    fn validate_terminal_env_rejects_bad_keys_and_values() {
        let mut env = BTreeMap::new();
        env.insert("OK".into(), "1".into());
        assert!(validate_terminal_env(&env).is_ok());
        env.insert("BAD=KEY".into(), "x".into());
        assert!(validate_terminal_env(&env).is_err());

        let mut env = BTreeMap::new();
        env.insert("BAD_VALUE".into(), "x\0y".into());
        assert!(validate_terminal_env(&env).is_err());
    }

    #[tokio::test]
    async fn api_create_terminal_spawns_command_and_returns_session() {
        let state = crate::state::test_support::make_test_state(false, false);
        let response = api_create_terminal(
            State(state.clone()),
            Ok(Json(create_terminal_body("printf 'hi from spawn\\n'"))),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response_json(response).await;
        assert_eq!(body["tab_label"], "@@Spawned");
        let session = body["session"].as_str().expect("session id");
        let mut handle = state
            .terminal_sessions
            .attach(session, Some(0))
            .expect("spawned session");
        let out = collect_until(&mut handle, "hi from spawn", Duration::from_secs(5)).await;
        assert!(out.contains("hi from spawn"), "missing output: {out:?}");
        state
            .terminal_sessions
            .close(session, CloseReason::Explicit);
    }

    #[tokio::test]
    async fn api_create_terminal_rejects_missing_command() {
        let state = crate::state::test_support::make_test_state(false, false);
        let response = api_create_terminal(
            State(state),
            Ok(Json(CreateTerminalBody {
                name: "@@Spawned".into(),
                command: " ".into(),
                env: BTreeMap::new(),
                orchestrator_session: None,
            })),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn terminal_control_endpoints_return_not_found_for_missing_session() {
        let state = crate::state::test_support::make_test_state(false, false);

        let restart = api_restart_terminal(State(state.clone()), AxumPath("missing".into())).await;
        let delete = api_delete_terminal(State(state), AxumPath("missing".into())).await;

        assert_eq!(restart.status(), StatusCode::NOT_FOUND);
        assert_eq!(delete.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn api_restart_terminal_respawns_same_session_command() {
        let state = crate::state::test_support::make_test_state(false, false);
        let mut body = create_terminal_body("printf \"restart-$SYSTACEAN_RESTART\\n\"; sleep 1");
        body.env.insert("SYSTACEAN_RESTART".into(), "one".into());
        let response = api_create_terminal(State(state.clone()), Ok(Json(body))).await;
        let json = response_json(response).await;
        let session = json["session"].as_str().expect("session id").to_string();
        let mut handle = state
            .terminal_sessions
            .attach(&session, Some(0))
            .expect("spawned session");
        let out = collect_until(&mut handle, "restart-one", Duration::from_secs(5)).await;
        assert!(out.contains("restart-one"), "missing first output: {out:?}");

        let response = api_restart_terminal(State(state.clone()), AxumPath(session.clone())).await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let mut restarted = state
            .terminal_sessions
            .attach(&session, Some(0))
            .expect("restarted session");
        let out = collect_until(&mut restarted, "restart-one", Duration::from_secs(5)).await;
        assert!(
            out.contains("restart-one"),
            "missing restarted output: {out:?}"
        );
        state
            .terminal_sessions
            .close(&session, CloseReason::Explicit);
    }

    #[tokio::test]
    async fn api_delete_terminal_closes_session() {
        let state = crate::state::test_support::make_test_state(false, false);
        let response = api_create_terminal(
            State(state.clone()),
            Ok(Json(create_terminal_body("sleep 5"))),
        )
        .await;
        let json = response_json(response).await;
        let session = json["session"].as_str().expect("session id").to_string();

        let response = api_delete_terminal(State(state.clone()), AxumPath(session.clone())).await;

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(state.terminal_sessions.attach(&session, None).is_none());
    }

    #[tokio::test]
    async fn api_create_terminal_writes_preflight_event_to_orchestrator_watcher() {
        let state = crate::state::test_support::make_test_state(false, false);
        let orchestrator = state
            .terminal_sessions
            .create(CreateOptions {
                size: pty_size(None, None),
                tab_name: Some("@@Architect".into()),
                window_id: None,
                mcp_env: true,
                cwd: None,
                command: Some("sleep 5".into()),
                env: Default::default(),
                preflight: None,
            })
            .expect("orchestrator terminal");
        let orchestrator_id = orchestrator.id().to_string();
        let dir = tempfile::tempdir().expect("watch dir");
        state
            .terminal_sessions
            .set_watcher(&orchestrator_id, dir.path().to_path_buf())
            .expect("set watcher");
        let body = CreateTerminalBody {
            name: "@@Spawned".into(),
            command: "printf 'please log in first\\n'; sleep 1".into(),
            env: BTreeMap::new(),
            orchestrator_session: Some(orchestrator_id.clone()),
        };

        let response = api_create_terminal(State(state.clone()), Ok(Json(body))).await;

        assert_eq!(response.status(), StatusCode::CREATED);
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut found = None;
        while Instant::now() < deadline {
            found = std::fs::read_dir(dir.path())
                .expect("read event dir")
                .filter_map(|entry| entry.ok())
                .find(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with("pre-flight-")
                });
            if found.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        let entry = found.expect("pre-flight event file");
        let text = std::fs::read_to_string(entry.path()).expect("read event");
        assert!(text.contains(r#""type":"pre-flight""#));
        assert!(text.contains("please log in first"));
        state
            .terminal_sessions
            .close(&orchestrator_id, CloseReason::Explicit);
    }

    #[tokio::test]
    async fn write_event_reply_atomic_writes_json_and_cleans_tmp() {
        let dir = tempfile::tempdir().expect("event dir");

        write_event_reply_atomic(dir.path(), &reply_body("survey-alpha", "go"))
            .await
            .expect("write reply");

        let path = dir.path().join("event-reply-survey-alpha.md");
        let text = tokio::fs::read_to_string(path).await.expect("read reply");
        assert!(text.contains(r#""type":"survey-reply""#));
        assert!(text.contains(r#""note":"go""#));
        assert!(tmp_reply_files(dir.path()).is_empty());
    }

    #[tokio::test]
    async fn write_event_reply_atomic_concurrent_calls_leave_valid_destination() {
        let dir = tempfile::tempdir().expect("event dir");
        let body_a = reply_body("survey-alpha", "a");
        let body_b = reply_body("survey-alpha", "b");
        let a = write_event_reply_atomic(dir.path(), &body_a);
        let b = write_event_reply_atomic(dir.path(), &body_b);

        let (ra, rb) = tokio::join!(a, b);
        ra.expect("write a");
        rb.expect("write b");

        let path = dir.path().join("event-reply-survey-alpha.md");
        let text = tokio::fs::read_to_string(path).await.expect("read reply");
        let parsed: EventReplyBody = serde_json::from_str(&text).expect("valid reply json");
        assert_eq!(parsed.id, "survey-alpha");
        assert!(matches!(parsed.note.as_deref(), Some("a" | "b")));
        assert!(tmp_reply_files(dir.path()).is_empty());
    }

    #[tokio::test]
    async fn write_event_reply_atomic_cleans_tmp_on_failure() {
        let dir = tempfile::tempdir().expect("event dir");
        std::fs::create_dir(dir.path().join("event-reply-survey-alpha.md"))
            .expect("block final path with dir");

        let err = write_event_reply_atomic(dir.path(), &reply_body("survey-alpha", "go"))
            .await
            .expect_err("rename over dir should fail");

        assert_eq!(err.kind(), std::io::ErrorKind::IsADirectory);
        assert!(tmp_reply_files(dir.path()).is_empty());
    }

    #[tokio::test]
    async fn api_terminal_event_reply_refuses_without_attached_watcher() {
        let state = crate::state::test_support::make_test_state(false, false);

        let response = api_terminal_event_reply(
            State(state),
            AxumPath("missing-session".into()),
            Ok(Json(reply_body("survey-alpha", "go"))),
        )
        .await;

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn api_terminal_event_reply_maps_schema_rejection_to_bad_request() {
        let state = crate::state::test_support::make_test_state(false, false);
        let app = Router::new()
            .route(
                "/api/terminal/:session/event-reply",
                post(api_terminal_event_reply),
            )
            .with_state(state);
        let req = Request::builder()
            .method("POST")
            .uri("/api/terminal/session-a/event-reply")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{
                  "id": "survey-alpha",
                  "type": "survey",
                  "from": "@@Alex",
                  "to": "@@Systacean",
                  "answers": [{"question_index": 0, "key": "1"}],
                  "scope_grant": "one-shot"
                }"#,
            ))
            .expect("request");

        let response = app.oneshot(req).await.expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn validate_event_reply_rejects_missing_required_text() {
        let mut body = reply_body("survey-alpha", "go");
        assert!(validate_event_reply(&body).is_ok());
        body.id = " ".into();
        assert!(validate_event_reply(&body).is_err());
    }

    async fn collect_until(session: &mut AttachHandle, needle: &str, timeout: Duration) -> String {
        let deadline = Instant::now() + timeout;
        let mut out = String::new();
        loop {
            if out.contains(needle) || Instant::now() >= deadline {
                return out;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            match tokio::time::timeout(remaining, session.rx.recv()).await {
                Ok(Ok(SessionEvent::Output(data))) => out.push_str(&String::from_utf8_lossy(&data)),
                Ok(Ok(SessionEvent::Error(message))) => {
                    out.push_str(&format!("\n__ERROR__{message}\n"));
                }
                Ok(Ok(_)) => {}
                Ok(Err(_)) | Err(_) => return out,
            }
        }
    }

    async fn collect_until_idle(
        session: &mut AttachHandle,
        timeout: Duration,
        idle: Duration,
    ) -> String {
        let deadline = Instant::now() + timeout;
        let mut out = String::new();
        let mut last_output = Instant::now();
        loop {
            if !out.is_empty() && Instant::now().duration_since(last_output) >= idle {
                return out;
            }
            if Instant::now() >= deadline {
                return out;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            let wait = remaining.min(Duration::from_millis(100));
            match tokio::time::timeout(wait, session.rx.recv()).await {
                Ok(Ok(SessionEvent::Output(data))) => {
                    out.push_str(&String::from_utf8_lossy(&data));
                    last_output = Instant::now();
                }
                Ok(Ok(SessionEvent::Error(message))) => {
                    out.push_str(&format!("\n__ERROR__{message}\n"));
                    last_output = Instant::now();
                }
                Ok(Ok(_)) => {}
                Ok(Err(_)) => return out,
                Err(_) => {}
            }
        }
    }

    async fn run_shell_probe(command: &str, end: &str) -> String {
        let tmp = tempfile::tempdir().expect("temp drive");
        let mut terminal = TestTerminal::spawn(
            tmp.path().to_path_buf(),
            pty_size(Some(100), Some(31)),
            None,
            None,
        );
        let _ = collect_until_idle(
            &mut terminal.handle,
            Duration::from_millis(300),
            Duration::from_millis(100),
        )
        .await;
        terminal
            .handle
            .send_input("stty -echo 2>/dev/null\r".as_bytes());
        let _ = collect_until_idle(
            &mut terminal.handle,
            Duration::from_millis(300),
            Duration::from_millis(100),
        )
        .await;
        terminal
            .handle
            .send_input(format!("{command}\r").as_bytes());
        let mut out = collect_until(&mut terminal.handle, end, Duration::from_secs(5)).await;
        out.push_str(
            &collect_until_idle(
                &mut terminal.handle,
                Duration::from_millis(300),
                Duration::from_millis(100),
            )
            .await,
        );
        out
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn terminal_session_reports_live_cwd() {
        let tmp = tempfile::tempdir().expect("temp drive");
        let subdir = tmp.path().join("work");
        fs::create_dir_all(&subdir).expect("create subdir");
        let root = tmp.path().canonicalize().expect("canonical root");
        let subdir = subdir.canonicalize().expect("canonical subdir");
        let mut terminal =
            TestTerminal::spawn(root.clone(), pty_size(Some(100), Some(31)), None, None);
        let _ = collect_until_idle(
            &mut terminal.handle,
            Duration::from_millis(300),
            Duration::from_millis(100),
        )
        .await;

        assert_eq!(
            terminal
                .handle
                .cwd()
                .as_ref()
                .and_then(|p| p.canonicalize().ok()),
            Some(root),
            "fresh terminal should report drive root cwd"
        );

        terminal.handle.send_input(b"cd work\r");
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if terminal.handle.cwd().and_then(|p| p.canonicalize().ok()) == Some(subdir.clone()) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "terminal cwd did not update to {}",
                subdir.display()
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    #[tokio::test]
    async fn conditional_pty_programs_validate_real_terminal() {
        let mut ran = 0usize;
        let mut passed = 0usize;

        if command_available("tty") {
            ran += 1;
            let out = run_shell_probe(
                "printf '\\n__TTY_BEGIN__\\n'; tty; printf '\\n__TTY_END__\\n'",
                "__TTY_END__",
            )
            .await;
            assert!(
                out.contains("/dev/"),
                "tty should report a device path, got {out:?}"
            );
            passed += 1;
        }

        if command_available("stty") {
            ran += 1;
            let out = run_shell_probe(
                "printf '\\n__STTY_BEGIN__\\n'; stty size; printf '\\n__STTY_END__\\n'",
                "__STTY_END__",
            )
            .await;
            assert!(
                out.contains("31 100"),
                "stty should see resized PTY as 31x100, got {out:?}"
            );
            passed += 1;
        }

        if command_available("tput") {
            ran += 1;
            let out = run_shell_probe(
                "printf '\\n__TPUT_BEGIN__\\n'; tput cols; tput lines; printf '\\n__TPUT_END__\\n'",
                "__TPUT_END__",
            )
            .await;
            assert!(
                out.contains("100") && out.contains("31"),
                "tput should see resized PTY dimensions, got {out:?}"
            );
            passed += 1;
        }

        if command_available("pwd") {
            ran += 1;
            let tmp = tempfile::tempdir().expect("temp drive");
            let cwd = tmp.path().to_path_buf();
            let mut terminal = TestTerminal::spawn(
                cwd.clone(),
                pty_size(Some(100), Some(31)),
                Some("build".to_string()),
                Some(std::path::PathBuf::from("/tmp/chan-test.sock")),
            );
            let _ = collect_until_idle(
                &mut terminal.handle,
                Duration::from_millis(300),
                Duration::from_millis(100),
            )
            .await;
            terminal
                .handle
                .send_input("stty -echo 2>/dev/null\r".as_bytes());
            let _ = collect_until_idle(
                &mut terminal.handle,
                Duration::from_millis(300),
                Duration::from_millis(100),
            )
            .await;
            terminal.handle.send_input(
                b"printf '\\n__CWD_HOME_BEGIN__\\n'; pwd; printf '<HOME=%s>\\n' \"$HOME\"; printf '<CHAN_TAB_NAME=%s>\\n' \"$CHAN_TAB_NAME\"; printf '<CHAN_WINDOW_ID=%s>\\n' \"$CHAN_WINDOW_ID\"; printf '<CHAN_CONTROL_SOCKET=%s>\\n' \"$CHAN_CONTROL_SOCKET\"; env | grep -E '^(CHAN|CLAUDE|CODEX|GEMINI)_MCP_' | sort; printf '\\n__CWD_HOME_END__\\n'\r",
            );
            let out = collect_until(
                &mut terminal.handle,
                "__CWD_HOME_END__",
                Duration::from_secs(5),
            )
            .await;
            assert!(
                out.contains(&cwd.display().to_string()),
                "terminal should start at drive root cwd, got {out:?}"
            );
            assert!(
                !out.contains(&format!("<HOME={}>", cwd.display())),
                "terminal HOME should not be rewritten to drive root, got {out:?}"
            );
            assert!(
                out.contains("<CHAN_TAB_NAME=build>"),
                "terminal should expose the tab name env var, got {out:?}"
            );
            assert!(
                out.contains("<CHAN_WINDOW_ID=window-test>"),
                "terminal should expose the window id env var, got {out:?}"
            );
            assert!(
                out.contains("<CHAN_CONTROL_SOCKET=/tmp/chan-control-test.sock>"),
                "terminal should expose the control socket env var, got {out:?}"
            );
            assert!(
                out.contains("CHAN_MCP_SOCKET=/tmp/chan-test.sock"),
                "terminal should expose the MCP socket env var, got {out:?}"
            );
            assert!(
                out.contains("CHAN_MCP_SERVER_NAME=chan"),
                "terminal should expose the MCP server name env var, got {out:?}"
            );
            assert!(
                out.contains("CHAN_MCP_SERVER_JSON=")
                    && out.contains("CHAN_MCP_COMMAND=")
                    && out.contains("CHAN_MCP_COMMAND_JSON="),
                "terminal should expose only chan MCP discovery env vars, got {out:?}"
            );
            assert!(
                !out.contains("CLAUDE_MCP_SERVER_JSON=")
                    && !out.contains("CODEX_MCP_SERVER_JSON=")
                    && !out.contains("GEMINI_MCP_SERVER_JSON="),
                "terminal should not expose third-party MCP aliases, got {out:?}"
            );
            passed += 1;
        }

        if command_available("sh") {
            ran += 1;
            let out = run_shell_probe("printf '\\n__READ_BEGIN__\\n'; sh -lc 'read x; printf \"<%s>\\\\n\" \"$x\"' <<'EOF'\nchan-term\nEOF\nprintf '\\n__READ_END__\\n'", "__READ_END__").await;
            assert!(
                out.contains("<chan-term>"),
                "shell read/write probe should roundtrip input, got {out:?}"
            );
            passed += 1;
        }

        if command_available("less") {
            ran += 1;
            let tmp = tempfile::tempdir().expect("temp drive");
            let mut terminal = TestTerminal::spawn(
                tmp.path().to_path_buf(),
                pty_size(Some(100), Some(31)),
                None,
                None,
            );
            let _ = collect_until_idle(
                &mut terminal.handle,
                Duration::from_millis(300),
                Duration::from_millis(100),
            )
            .await;
            terminal
                .handle
                .send_input("stty -echo 2>/dev/null\r".as_bytes());
            let _ = collect_until_idle(
                &mut terminal.handle,
                Duration::from_millis(300),
                Duration::from_millis(100),
            )
            .await;
            terminal
                .handle
                .send_input(b"printf 'alpha\\nbeta\\n' | less\r");
            let out = collect_until(&mut terminal.handle, "alpha", Duration::from_secs(5)).await;
            assert!(
                out.contains("alpha"),
                "less should render piped text, got {out:?}"
            );
            terminal.handle.send_input(b"q");
            terminal.handle.send_input(b"printf '\\n__LESS_END__\\n'\r");
            let out =
                collect_until(&mut terminal.handle, "__LESS_END__", Duration::from_secs(5)).await;
            assert!(
                out.contains("__LESS_END__"),
                "shell should remain usable after quitting less, got {out:?}"
            );
            passed += 1;
        }

        assert!(
            ran > 0,
            "no conditional PTY validation commands were available"
        );
        assert!(passed > 0, "no conditional PTY validation passed");
    }

    #[tokio::test]
    async fn mcp_env_off_omits_chan_mcp_vars() {
        if !command_available("env") {
            return;
        }
        let tmp = tempfile::tempdir().expect("temp drive");
        let mut terminal = TestTerminal::spawn_with_mcp_env(
            tmp.path().to_path_buf(),
            pty_size(Some(100), Some(31)),
            Some("plain".to_string()),
            Some(std::path::PathBuf::from("/tmp/chan-test.sock")),
            false,
        );
        let _ = collect_until_idle(
            &mut terminal.handle,
            Duration::from_millis(300),
            Duration::from_millis(100),
        )
        .await;
        terminal
            .handle
            .send_input("stty -echo 2>/dev/null\r".as_bytes());
        let _ = collect_until_idle(
            &mut terminal.handle,
            Duration::from_millis(300),
            Duration::from_millis(100),
        )
        .await;
        terminal.handle.send_input(
            b"printf '\\n__MCP_ENV_OFF_BEGIN__\\n'; env | grep '^CHAN_MCP_' || true; printf '<CHAN_TAB_NAME=%s>\\n' \"$CHAN_TAB_NAME\"; printf '<CHAN_WINDOW_ID=%s>\\n' \"$CHAN_WINDOW_ID\"; printf '<CHAN_CONTROL_SOCKET=%s>\\n' \"$CHAN_CONTROL_SOCKET\"; printf '\\n__MCP_ENV_OFF_END__\\n'\r",
        );
        let out = collect_until(
            &mut terminal.handle,
            "__MCP_ENV_OFF_END__",
            Duration::from_secs(5),
        )
        .await;
        assert!(
            !out.contains("CHAN_MCP_"),
            "mcp_env=false should omit CHAN_MCP_* env vars, got {out:?}"
        );
        assert!(
            out.contains("<CHAN_TAB_NAME=plain>"),
            "mcp_env=false should not affect CHAN_TAB_NAME, got {out:?}"
        );
        assert!(
            out.contains("<CHAN_WINDOW_ID=window-test>")
                && out.contains("<CHAN_CONTROL_SOCKET=/tmp/chan-control-test.sock>"),
            "mcp_env=false should not affect chan control env vars, got {out:?}"
        );
    }
}
