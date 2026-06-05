//! Interactive PTY-backed terminal sessions and terminal control APIs.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Json, Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chan_shell::{submit_writes, SubmitAgent};
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
    tab_group: Option<String>,
    window_id: Option<String>,
    mcp_env: Option<TerminalMcpEnv>,
    cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTerminalBody {
    name: String,
    command: String,
    #[serde(default)]
    env: BTreeMap<String, String>,
    /// Broadcast group the new session joins (sets `$CHAN_TAB_GROUP` and
    /// the registry's per-session `tab_group`). Absent / "default" leaves
    /// it ungrouped. Used by the Team Work bootstrap so every team
    /// terminal joins the team's group.
    #[serde(default)]
    group: Option<String>,
    /// Owning window for the new session. Team-dialog terminals are
    /// created through this POST and only ATTACHED over `/ws` afterwards,
    /// and attach does not rebind `window_id`. Without binding it here the
    /// session keeps `window_id = None`, so `cs terminal survey` (which
    /// resolves its target by window) reports "no live terminal session
    /// matched". The Team Work orchestrator passes the dialog window's
    /// `sessionWindowId()` so the survey overlay lands in the right window.
    #[serde(default)]
    window_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateTerminalResponse {
    session: String,
    tab_label: String,
}

#[derive(Debug, Deserialize)]
pub struct RestartTerminalBody {
    name: Option<String>,
    /// Broadcast group for the respawned shell. Sets `$CHAN_TAB_GROUP`
    /// and the registry's per-session `tab_group`. Absent / "default"
    /// resolves to the default group.
    group: Option<String>,
    window_id: Option<String>,
    /// Optional command override. When supplied, the restarted PTY
    /// runs this command instead of the original spawn command.
    /// Used by the team-bootstrap orchestrator to flip the host's
    /// terminal into the lead's session (e.g. `bash` -> `claude`).
    command: Option<String>,
    /// Optional env override. Merged into the restart options' env
    /// so the lead's CHAN_TAB_NAME and any other per-member env
    /// land before the new PTY spawns. Existing entries with the
    /// same key are replaced.
    #[serde(default)]
    env: Option<std::collections::BTreeMap<String, String>>,
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
    #[serde(rename = "focus")]
    Focus { focused: bool },
    #[serde(rename = "close")]
    Close,
    /// Rich Prompt bubble submit (@@LaneB). Unlike `Input` (raw keystrokes
    /// straight to the PTY), this ENQUEUES `data` onto this session's write
    /// queue -- the SAME FIFO the control socket's `cs terminal write` feeds
    /// -- so bubble prompts and CLI pokes serialize through one drain and
    /// submit one after another when the agent is idle. The server appends
    /// the submit chord for `agent` (claude / codex / gemini); `agent` is
    /// optional and DEFAULTS to claude (the round's primary agent) when the
    /// SPA does not know the terminal's launch command.
    #[serde(rename = "prompt")]
    Prompt {
        data: String,
        #[serde(default)]
        agent: Option<String>,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ServerFrame {
    #[serde(rename = "session")]
    Session {
        id: String,
        seq: u64,
        missed_bytes: u64,
        bytes_since_focus: u64,
    },
    #[serde(rename = "activity")]
    Activity { bytes_since_focus: u64 },
    #[serde(rename = "ready")]
    Ready {
        cols: u16,
        rows: u16,
        #[serde(skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cwd_rel: Option<String>,
    },
    #[serde(rename = "cwd")]
    Cwd {
        #[serde(skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cwd_rel: Option<String>,
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
    let tab_group = query.tab_group.as_deref().and_then(normalize_tab_group);
    let window_id = query.window_id.as_deref().and_then(normalize_window_id);
    // B5: MCP env is off by default. An explicit `?mcp_env=on|off` query
    // wins (the SPA can force a per-terminal choice); when absent we fall
    // back to the non-team server-config default, which itself defaults
    // off. Team spawns don't reach here -- they read the team config's
    // own `mcp_env` toggle in control_socket::spawn_team.
    let mcp_env = match query.mcp_env {
        Some(choice) => choice.enabled(),
        None => state
            .server_config
            .lock()
            .map(|c| c.terminal.mcp_env)
            .unwrap_or(false),
    };
    let cwd = if query.session.is_some() {
        None
    } else {
        let workspace = state.workspace();
        let cwd = query.cwd.clone();
        let result =
            tokio::task::spawn_blocking(move || resolve_terminal_cwd(&workspace, cwd.as_deref()))
                .await;
        match result {
            Ok(Ok(cwd)) => cwd,
            Ok(Err(message)) => return (StatusCode::BAD_REQUEST, message).into_response(),
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("terminal cwd task panicked: {e}"),
                )
                    .into_response()
            }
        }
    };
    let opts = TerminalWsOptions {
        session_id: query.session,
        since: query.since,
        size,
        tab_name,
        tab_group,
        window_id,
        mcp_env,
        cwd,
    };
    ws.on_upgrade(move |socket| terminal_ws(socket, state, opts))
        .into_response()
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
    let opts = CreateOptions {
        size: pty_size(None, None),
        tab_name: Some(name.clone()),
        tab_group: body.group.as_deref().and_then(normalize_tab_group),
        window_id: body.window_id.as_deref().and_then(normalize_window_id),
        // B5: off by default; honor the non-team server-config opt-in.
        mcp_env: state
            .server_config
            .lock()
            .map(|c| c.terminal.mcp_env)
            .unwrap_or(false),
        cwd: None,
        command: Some(command),
        env: body.env,
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
        Err(CreateError::FdPressure(e)) => {
            (StatusCode::SERVICE_UNAVAILABLE, e.to_string()).into_response()
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
    body: Option<Json<RestartTerminalBody>>,
) -> Response {
    if state.tunnel_public {
        return err_tunnel_public_locked();
    }
    let (tab_name, tab_group, window_id, command, env) = if let Some(Json(body)) = body {
        let tab_name = match body.name.as_deref() {
            Some(name) => match normalize_tab_name(name) {
                Some(name) => Some(name),
                None => {
                    return (StatusCode::BAD_REQUEST, "terminal name is required").into_response()
                }
            },
            None => None,
        };
        // Three-way: outer None (no `group` field) keeps the existing
        // group; `Some(None)` (blank / "default") sets the default group;
        // `Some(Some(g))` sets group g.
        let tab_group = body.group.as_deref().map(normalize_tab_group);
        let window_id = match body.window_id.as_deref() {
            Some(id) => match normalize_window_id(id) {
                Some(id) => Some(id),
                None => {
                    return (StatusCode::BAD_REQUEST, "terminal window id is required")
                        .into_response()
                }
            },
            None => None,
        };
        (tab_name, tab_group, window_id, body.command, body.env)
    } else {
        (None, None, None, None, None)
    };
    match state
        .terminal_sessions
        .restart(&session, tab_name, tab_group, window_id, command, env)
    {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "terminal session not found").into_response(),
        Err(CreateError::Capped) => {
            (StatusCode::CONFLICT, "terminal session cap reached").into_response()
        }
        Err(CreateError::FdPressure(e)) => {
            (StatusCode::SERVICE_UNAVAILABLE, e.to_string()).into_response()
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

struct TerminalWsOptions {
    session_id: Option<String>,
    since: Option<u64>,
    size: PtySize,
    tab_name: Option<String>,
    tab_group: Option<String>,
    window_id: Option<String>,
    mcp_env: bool,
    cwd: Option<PathBuf>,
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

async fn terminal_ws(mut socket: WebSocket, state: Arc<AppState>, opts: TerminalWsOptions) {
    state
        .last_activity
        .store(now_unix_secs(), Ordering::Relaxed);

    let create_opts = CreateOptions {
        size: opts.size,
        tab_name: opts.tab_name,
        tab_group: opts.tab_group,
        window_id: opts.window_id,
        mcp_env: opts.mcp_env,
        cwd: opts.cwd,
        command: None,
        env: Default::default(),
    };
    let mut session = match state.terminal_sessions.get_or_create_for_ws(
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
        Err(CreateError::FdPressure(e)) => {
            let message = e.to_string();
            let _ = send_frame(
                &mut socket,
                ServerFrame::Error {
                    message: message.clone(),
                    reason: Some("fd_pressure"),
                },
            )
            .await;
            let _ = socket
                .send(Message::Close(Some(CloseFrame {
                    code: 1013,
                    reason: message.into(),
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
            bytes_since_focus: session.bytes_since_focus(),
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
    let (cwd, cwd_rel) = terminal_cwd_payload(&state.workspace(), session.cwd());
    let _ = send_frame(
        &mut socket,
        ServerFrame::Ready {
            cols: opts.size.cols,
            rows: opts.size.rows,
            cwd,
            cwd_rel,
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
                                let (cwd, cwd_rel) =
                                    terminal_cwd_payload(&state.workspace(), session.cwd());
                                let _ = send_frame(&mut socket, ServerFrame::Cwd { cwd, cwd_rel }).await;
                            }
                            Ok(ClientFrame::Focus { focused }) => {
                                session.set_focused(focused);
                                state.last_activity.store(now_unix_secs(), Ordering::Relaxed);
                            }
                            Ok(ClientFrame::Close) => {
                                let id = session.id().to_owned();
                                state.terminal_sessions.close(&id, CloseReason::Explicit);
                            }
                            Ok(ClientFrame::Prompt { data, agent }) => {
                                // Rich Prompt bubble: append the target agent's
                                // submit chord (default claude when omitted),
                                // then ENQUEUE onto the shared write queue so it
                                // serializes with `cs terminal write` pokes and
                                // submits when the agent is idle.
                                let submit = SubmitAgent::from_agent_name(
                                    agent.as_deref().unwrap_or("claude"),
                                );
                                // gemini needs its submit chord as a SEPARATE
                                // queue item (it coalesces a bulk text+CR into
                                // a newline); submit_writes returns two writes
                                // for gemini, one for everyone else. Each
                                // enqueued item drains idle-gated, so the CR
                                // lands as a distinct keypress.
                                for write in submit_writes(data, submit) {
                                    let _ = session.enqueue_write(write.as_bytes());
                                }
                                state.last_activity.store(now_unix_secs(), Ordering::Relaxed);
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
                    Ok(SessionEvent::Activity { bytes_since_focus }) => {
                        if send_frame(&mut socket, ServerFrame::Activity { bytes_since_focus }).await.is_err() {
                            break;
                        }
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

fn terminal_cwd_payload(
    workspace: &chan_workspace::Workspace,
    cwd: Option<PathBuf>,
) -> (Option<String>, Option<String>) {
    match cwd {
        Some(path) => {
            let rel = workspace.physical_path_to_virtual(&path);
            (Some(path_to_wire(path)), rel)
        }
        None => (None, None),
    }
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

/// Normalize a broadcast group. Blank / "default" resolve to `None` so the
/// server treats the default group as the absence of an explicit group
/// (mirrors the SPA wire, which omits the default).
fn normalize_tab_group(group: &str) -> Option<String> {
    let trimmed = group.trim();
    if trimmed.is_empty() || trimmed == "default" {
        return None;
    }
    Some(trimmed.chars().take(128).collect())
}

fn resolve_terminal_cwd(
    workspace: &chan_workspace::Workspace,
    cwd: Option<&str>,
) -> Result<Option<PathBuf>, String> {
    let Some(raw) = cwd else {
        return Ok(None);
    };
    let rel = raw.trim();
    workspace
        .resolve_physical_dir(rel)
        .map(Some)
        .map_err(|e| format!("invalid terminal cwd: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TerminalConfig;
    use crate::terminal_sessions::{AttachHandle, Registry, RegistryConfig};
    use axum::body::to_bytes;
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
                workspace_root: cwd,
                mcp_socket_path,
                control_socket_path: Some(std::path::PathBuf::from("/tmp/chan-control-test.sock")),
                terminal: TerminalConfig::default(),
            });
            let handle = registry
                .create(CreateOptions {
                    size,
                    tab_name,
                    tab_group: None,
                    window_id: Some("window-test".into()),
                    mcp_env,
                    cwd: None,
                    command: None,
                    env: Default::default(),
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

    /// Well-known lock-file name (under the OS temp dir) for the
    /// cross-process FS-timing test gate. MUST stay identical to
    /// `chan_workspace::test_gate::GATE_FILE` and the copy in the indexer
    /// test module so every FS-timing test across both crates' separate
    /// test binaries contends on the same OS advisory lock.
    const FS_TIMING_GATE: &str = "chan-fs-timing-test.gate";

    /// Cross-process serial gate for the real-PTY shell-probe tests.
    /// Each spawns a real shell on a PTY, sends commands, and asserts on
    /// the shell's output within a bounded window. Under the FULL
    /// parallel `cargo test` run (CI) every core is saturated, so the
    /// shell's startup, `stty -echo` settling, and command output all
    /// slip past a tight window; the probe then returns only the echoed
    /// command line (which itself contains tokens like `CHAN_MCP_`),
    /// tripping the assertions.
    ///
    /// WHY a FILE lock and not a `static`/`tokio` Mutex: a `static` lock
    /// serializes only tests WITHIN this test binary, but `cargo test`
    /// runs each crate's test binary as a SEPARATE PROCESS concurrently,
    /// so these PTY tests still race chan-workspace's FS-watcher tests and
    /// this crate's indexer boot-walk tests for the CPU. An OS advisory
    /// lock on a well-known temp path spans process boundaries; opening
    /// the SAME `FS_TIMING_GATE` path here, in the indexer test module,
    /// and in chan-workspace (`crate::test_gate`) makes one named gate
    /// serialize the entire FS-timing class workspace-wide. The
    /// `std::fs::File` guard is `Send` (held across `.await` on the
    /// multi-thread runtime is fine) and releases on drop / process
    /// exit.
    fn pty_test_lock() -> std::fs::File {
        let path = std::env::temp_dir().join(FS_TIMING_GATE);
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .expect("open FS-timing test gate file");
        file.lock().expect("acquire FS-timing test gate");
        file
    }

    /// Wait budget for a real-PTY shell probe to emit its end marker. On
    /// an idle host the shell echoes the marker in well under a second,
    /// so this ceiling is never approached; it only governs the worst
    /// case under the full parallel suite, where shell scheduling slips
    /// by seconds. The cross-process `pty_test_lock` gate is the primary
    /// fix (it removes the competing FS-timing load); this budget is the
    /// backstop and should rarely be approached now.
    const PROBE_BUDGET: Duration = Duration::from_secs(30);

    fn terminal_workspace_fixture() -> (
        tempfile::TempDir,
        tempfile::TempDir,
        Arc<chan_workspace::Workspace>,
    ) {
        let cfg = tempfile::TempDir::new().expect("temp config");
        let root = tempfile::TempDir::new().expect("temp workspace");
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        (cfg, root, workspace)
    }

    #[test]
    fn client_frame_prompt_decodes_with_optional_agent() {
        // The @@LaneB Rich Prompt contract: { type: "prompt", data, agent? }.
        // A Rust rename of the tag / fields would break the bubble's wire at
        // runtime with a green build, so pin the decode.
        let with_agent: ClientFrame =
            serde_json::from_str(r#"{"type":"prompt","data":"hi","agent":"codex"}"#).unwrap();
        match with_agent {
            ClientFrame::Prompt { data, agent } => {
                assert_eq!(data, "hi");
                assert_eq!(agent.as_deref(), Some("codex"));
            }
            other => panic!("expected Prompt, got {other:?}"),
        }
        // agent omitted -> None (the handler defaults the chord to claude).
        let no_agent: ClientFrame =
            serde_json::from_str(r#"{"type":"prompt","data":"yo"}"#).unwrap();
        match no_agent {
            ClientFrame::Prompt { data, agent } => {
                assert_eq!(data, "yo");
                assert!(agent.is_none());
            }
            other => panic!("expected Prompt, got {other:?}"),
        }
    }

    #[test]
    fn resolve_terminal_cwd_allows_workspace_relative_directory() {
        let (_cfg, root, workspace) = terminal_workspace_fixture();
        fs::create_dir_all(root.path().join("notes/work")).expect("create dir");

        let cwd = resolve_terminal_cwd(&workspace, Some("notes/work"))
            .expect("valid cwd")
            .expect("cwd set");

        assert_eq!(cwd, root.path().canonicalize().unwrap().join("notes/work"));
    }

    #[test]
    fn resolve_terminal_cwd_maps_drafts_dir_to_in_root_path() {
        let (_cfg, _root, workspace) = terminal_workspace_fixture();
        workspace.create_draft_dir("untitled-1").unwrap();

        let cwd = resolve_terminal_cwd(&workspace, Some(".Drafts/untitled-1"))
            .expect("valid cwd")
            .expect("cwd set");

        assert_eq!(cwd, workspace.drafts_dir().join("untitled-1"));
        assert_eq!(
            workspace.physical_path_to_virtual(&cwd),
            Some(".Drafts/untitled-1".to_string())
        );

        let (cwd_abs, cwd_rel) = terminal_cwd_payload(&workspace, Some(cwd));
        assert!(cwd_abs
            .as_deref()
            .is_some_and(|path| path.ends_with("untitled-1")));
        assert_eq!(cwd_rel.as_deref(), Some(".Drafts/untitled-1"));
    }

    #[test]
    fn resolve_terminal_cwd_rejects_escape_and_files() {
        let (_cfg, root, workspace) = terminal_workspace_fixture();
        fs::create_dir_all(root.path().join("notes")).expect("create dir");
        fs::write(root.path().join("notes/today.md"), "x").expect("create file");

        assert!(resolve_terminal_cwd(&workspace, Some("../outside")).is_err());
        assert!(resolve_terminal_cwd(&workspace, Some("notes/today.md")).is_err());
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
            group: None,
            window_id: None,
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
        let out = collect_until(&mut handle, "hi from spawn", PROBE_BUDGET).await;
        assert!(out.contains("hi from spawn"), "missing output: {out:?}");
        state
            .terminal_sessions
            .close(session, CloseReason::Explicit);
    }

    #[tokio::test]
    async fn api_create_terminal_joins_the_requested_group() {
        // The Team Work bootstrap spawns each team terminal with a group;
        // it must land on the session's registry tab_group (and so
        // $CHAN_TAB_GROUP + cs terminal list grouping).
        let state = crate::state::test_support::make_test_state(false, false);
        let mut body = create_terminal_body("sleep 1");
        body.group = Some("team-x".into());
        let response = api_create_terminal(State(state.clone()), Ok(Json(body))).await;
        assert_eq!(response.status(), StatusCode::CREATED);

        let summaries = state.terminal_sessions.session_summaries();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].tab_group, "team-x");
        state
            .terminal_sessions
            .close(&summaries[0].session_id, CloseReason::Explicit);
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
                group: None,
                window_id: None,
            })),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn terminal_control_endpoints_return_not_found_for_missing_session() {
        let state = crate::state::test_support::make_test_state(false, false);

        let restart =
            api_restart_terminal(State(state.clone()), AxumPath("missing".into()), None).await;
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
        let out = collect_until(&mut handle, "restart-one", PROBE_BUDGET).await;
        assert!(out.contains("restart-one"), "missing first output: {out:?}");

        let response =
            api_restart_terminal(State(state.clone()), AxumPath(session.clone()), None).await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let mut restarted = state
            .terminal_sessions
            .attach(&session, Some(0))
            .expect("restarted session");
        let out = collect_until(&mut restarted, "restart-one", PROBE_BUDGET).await;
        assert!(
            out.contains("restart-one"),
            "missing restarted output: {out:?}"
        );
        state
            .terminal_sessions
            .close(&session, CloseReason::Explicit);
    }

    #[tokio::test]
    async fn api_restart_terminal_updates_chan_tab_name_env() {
        let state = crate::state::test_support::make_test_state(false, false);
        let mut body =
            create_terminal_body("printf '<CHAN_TAB_NAME=%s>\\n' \"$CHAN_TAB_NAME\"; sleep 1");
        body.name = "@@First".into();
        let response = api_create_terminal(State(state.clone()), Ok(Json(body))).await;
        let json = response_json(response).await;
        let session = json["session"].as_str().expect("session id").to_string();
        let mut handle = state
            .terminal_sessions
            .attach(&session, Some(0))
            .expect("spawned session");
        let out = collect_until(&mut handle, "<CHAN_TAB_NAME=@@First>", PROBE_BUDGET).await;
        assert!(
            out.contains("<CHAN_TAB_NAME=@@First>"),
            "missing first tab name: {out:?}"
        );

        let response = api_restart_terminal(
            State(state.clone()),
            AxumPath(session.clone()),
            Some(Json(RestartTerminalBody {
                name: Some("@@Second".into()),
                group: None,
                window_id: None,
                command: None,
                env: None,
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let mut restarted = state
            .terminal_sessions
            .attach(&session, Some(0))
            .expect("restarted session");
        let out = collect_until(&mut restarted, "<CHAN_TAB_NAME=@@Second>", PROBE_BUDGET).await;
        assert!(
            out.contains("<CHAN_TAB_NAME=@@Second>"),
            "missing restarted tab name: {out:?}"
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
        let tmp = tempfile::tempdir().expect("temp workspace");
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
        let mut out = collect_until(&mut terminal.handle, end, PROBE_BUDGET).await;
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
        // Serialize against the sibling real-PTY tests so they do not
        // stack real-shell load on each other under the full parallel run.
        let _serial = pty_test_lock();
        let tmp = tempfile::tempdir().expect("temp workspace");
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
            "fresh terminal should report workspace root cwd"
        );

        terminal.handle.send_input(b"cd work\r");
        let deadline = Instant::now() + PROBE_BUDGET;
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
        // Serialize against the sibling real-PTY tests so they do not
        // stack real-shell load on each other under the full parallel run.
        let _serial = pty_test_lock();
        let mut ran = 0usize;
        let mut passed = 0usize;

        if command_available("tty") {
            ran += 1;
            let out = run_shell_probe(
                "printf '\\n__TTY_BEGIN__\\n'; tty; printf '\\n__TTY_END__\\n'",
                "__TTY_END__",
            )
            .await;
            // A real PTY makes `tty` report a /dev/ttys… device path,
            // which is the property we want to validate. But the GitHub
            // macOS runner's spawned PTY slave does not always present a
            // device path; there `tty` prints its documented "not a tty"
            // result instead. That is a runner limitation, not a product
            // regression, and it must NOT gate CI (addendum-1 #2 / A5b).
            // So assert the device path ONLY when one is present, accept
            // the "not a tty" headless case as a skip, and still fail on
            // a probe that produced neither (a genuinely broken harness).
            let has_device = out.contains("/dev/");
            let headless = out.to_ascii_lowercase().contains("not a tty");
            assert!(
                has_device || headless,
                "tty probe reported neither a /dev/ device path nor 'not a tty', got {out:?}"
            );
            if has_device {
                passed += 1;
            }
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
            let tmp = tempfile::tempdir().expect("temp workspace");
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
                b"printf '\\n__CWD_HOME_BEGIN__\\n'; pwd; printf '<HOME=%s>\\n' \"$HOME\"; printf '<CHAN_TAB_NAME=%s>\\n' \"$CHAN_TAB_NAME\"; printf '<CHAN_TAB_GROUP=%s>\\n' \"$CHAN_TAB_GROUP\"; printf '<CHAN_WINDOW_ID=%s>\\n' \"$CHAN_WINDOW_ID\"; printf '<CHAN_CONTROL_SOCKET=%s>\\n' \"$CHAN_CONTROL_SOCKET\"; printf '<CHAN_WORKSPACE_NAME=%s>\\n' \"$CHAN_WORKSPACE_NAME\"; printf '<CHAN_WORKSPACE_PATH=%s>\\n' \"$CHAN_WORKSPACE_PATH\"; env | grep -E '^(CHAN|CLAUDE|CODEX|GEMINI)_MCP_' | sort; printf '\\n__CWD_HOME_END__\\n'\r",
            );
            let out = collect_until(&mut terminal.handle, "__CWD_HOME_END__", PROBE_BUDGET).await;
            assert!(
                out.contains(&cwd.display().to_string()),
                "terminal should start at workspace root cwd, got {out:?}"
            );
            assert!(
                !out.contains(&format!("<HOME={}>", cwd.display())),
                "terminal HOME should not be rewritten to workspace root, got {out:?}"
            );
            assert!(
                out.contains("<CHAN_TAB_NAME=build>"),
                "terminal should expose the tab name env var, got {out:?}"
            );
            assert!(
                out.contains("<CHAN_TAB_GROUP=default>"),
                "terminal with no group should expose CHAN_TAB_GROUP=default, got {out:?}"
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
                out.contains(&format!("<CHAN_WORKSPACE_PATH={}>", cwd.display())),
                "terminal should expose the workspace path env var, got {out:?}"
            );
            let ws_name = cwd
                .file_name()
                .expect("temp workspace has a basename")
                .to_string_lossy();
            assert!(
                out.contains(&format!("<CHAN_WORKSPACE_NAME={ws_name}>")),
                "terminal should expose the workspace name env var, got {out:?}"
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
            let tmp = tempfile::tempdir().expect("temp workspace");
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
            let out = collect_until(&mut terminal.handle, "alpha", PROBE_BUDGET).await;
            assert!(
                out.contains("alpha"),
                "less should render piped text, got {out:?}"
            );
            terminal.handle.send_input(b"q");
            terminal.handle.send_input(b"printf '\\n__LESS_END__\\n'\r");
            let out = collect_until(&mut terminal.handle, "__LESS_END__", PROBE_BUDGET).await;
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
        // Serialize against the sibling real-PTY tests so they do not
        // stack real-shell load on each other under the full parallel run.
        let _serial = pty_test_lock();
        let tmp = tempfile::tempdir().expect("temp workspace");
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
        let out = collect_until(&mut terminal.handle, "__MCP_ENV_OFF_END__", PROBE_BUDGET).await;
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
