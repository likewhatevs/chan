//! GET /api/terminal/ws - interactive PTY-backed terminal sessions.

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Json, Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use portable_pty::PtySize;
use serde::{Deserialize, Serialize};

use crate::error::err_tunnel_public_locked;
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

struct TerminalWsOptions {
    session_id: Option<String>,
    since: Option<u64>,
    size: PtySize,
    tab_name: Option<String>,
    window_id: Option<String>,
    mcp_env: bool,
    cwd: Option<PathBuf>,
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
    use std::fs;
    use std::process::Command;
    use std::time::{Duration, Instant};

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
