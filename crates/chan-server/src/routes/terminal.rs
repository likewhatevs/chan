//! GET /api/terminal/ws - interactive PTY-backed terminal sessions.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::error::err_tunnel_public_locked;
use crate::signal::now_unix_secs;
use crate::state::AppState;

const DEFAULT_COLS: u16 = 80;
const DEFAULT_ROWS: u16 = 24;
const MAX_COLS: u16 = 500;
const MAX_ROWS: u16 = 200;

#[derive(Debug, Deserialize)]
pub struct TerminalQuery {
    cols: Option<u16>,
    rows: Option<u16>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientFrame {
    #[serde(rename = "input")]
    Input { data: String },
    #[serde(rename = "resize")]
    Resize { cols: u16, rows: u16 },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ServerFrame {
    #[serde(rename = "ready")]
    Ready { cols: u16, rows: u16 },
    #[serde(skip)]
    Output { data: Vec<u8> },
    #[serde(rename = "exit")]
    Exit { code: u32 },
    #[serde(rename = "error")]
    Error { message: String },
}

enum PtyCommand {
    Input(String),
    Resize(PtySize),
    Kill,
}

struct PtySession {
    tx: std::sync::mpsc::Sender<PtyCommand>,
    rx: mpsc::UnboundedReceiver<ServerFrame>,
}

impl PtySession {
    fn input(&self, data: String) {
        let _ = self.tx.send(PtyCommand::Input(data));
    }

    fn resize(&self, size: PtySize) {
        let _ = self.tx.send(PtyCommand::Resize(size));
    }

    fn kill(&self) {
        let _ = self.tx.send(PtyCommand::Kill);
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        let _ = self.tx.send(PtyCommand::Kill);
    }
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
    ws.on_upgrade(move |socket| terminal_ws(socket, state, size))
        .into_response()
}

async fn terminal_ws(mut socket: WebSocket, state: Arc<AppState>, size: PtySize) {
    state
        .last_activity
        .store(now_unix_secs(), Ordering::Relaxed);

    let mut session = match spawn_pty_session(state.drive_root.clone(), size) {
        Ok(session) => session,
        Err(e) => {
            let _ = send_frame(
                &mut socket,
                ServerFrame::Error {
                    message: format!("failed to start terminal: {e}"),
                },
            )
            .await;
            return;
        }
    };
    let mut shutdown_rx = state.shutdown_rx.clone();

    let _ = send_frame(
        &mut socket,
        ServerFrame::Ready {
            cols: size.cols,
            rows: size.rows,
        },
    )
    .await;

    loop {
        tokio::select! {
            biased;
            _ = shutdown_rx.changed() => {
                session.kill();
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
                    session.kill();
                    break;
                };
                match msg {
                    Ok(Message::Text(text)) => {
                        match serde_json::from_str::<ClientFrame>(&text) {
                            Ok(ClientFrame::Input { data }) => {
                                session.input(data);
                                state.last_activity.store(now_unix_secs(), Ordering::Relaxed);
                            }
                            Ok(ClientFrame::Resize { cols, rows }) => {
                                session.resize(pty_size(Some(cols), Some(rows)));
                                state.last_activity.store(now_unix_secs(), Ordering::Relaxed);
                            }
                            Err(e) => {
                                let _ = send_frame(
                                    &mut socket,
                                    ServerFrame::Error {
                                        message: format!("invalid terminal frame: {e}"),
                                    },
                                )
                                .await;
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        session.kill();
                        break;
                    }
                    Ok(Message::Ping(_)) | Ok(Message::Pong(_)) | Ok(Message::Binary(_)) => {}
                    Err(_) => {
                        session.kill();
                        break;
                    }
                }
            }
            frame = session.rx.recv() => {
                let Some(frame) = frame else {
                    break;
                };
                let is_exit = matches!(frame, ServerFrame::Exit { .. });
                if send_frame(&mut socket, frame).await.is_err() {
                    session.kill();
                    break;
                }
                state.last_activity.store(now_unix_secs(), Ordering::Relaxed);
                if is_exit {
                    break;
                }
            }
        }
    }
}

async fn send_frame(socket: &mut WebSocket, frame: ServerFrame) -> Result<(), axum::Error> {
    if let ServerFrame::Output { data } = frame {
        return socket.send(Message::Binary(data)).await;
    }
    socket
        .send(Message::Text(serde_json::to_string(&frame).unwrap_or_else(
            |e| format!(r#"{{"type":"error","message":"serialize failed: {e}"}}"#),
        )))
        .await
}

fn pty_size(cols: Option<u16>, rows: Option<u16>) -> PtySize {
    PtySize {
        cols: cols.unwrap_or(DEFAULT_COLS).clamp(1, MAX_COLS),
        rows: rows.unwrap_or(DEFAULT_ROWS).clamp(1, MAX_ROWS),
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn spawn_pty_session(cwd: PathBuf, size: PtySize) -> anyhow::Result<PtySession> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(size)?;
    let shell = default_shell();
    let mut cmd = CommandBuilder::new(&shell);
    cmd.cwd(cwd);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("CLICOLOR", "1");
    cmd.env("CLICOLOR_FORCE", "1");
    cmd.env("FORCE_COLOR", "3");
    cmd.env("CHAN", "1");
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("CI");
    cmd.env_remove("CODEX_CI");
    let mut child = pair.slave.spawn_command(cmd)?;
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader()?;
    let mut writer = pair.master.take_writer()?;
    let mut killer = child.clone_killer();

    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<PtyCommand>();
    let (frame_tx, frame_rx) = mpsc::unbounded_channel::<ServerFrame>();

    {
        let frame_tx = frame_tx.clone();
        std::thread::Builder::new()
            .name("chan-terminal-reader".into())
            .spawn(move || {
                let mut buf = [0u8; 8192];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            if frame_tx
                                .send(ServerFrame::Output {
                                    data: buf[..n].to_vec(),
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = frame_tx.send(ServerFrame::Error {
                                message: format!("terminal read failed: {e}"),
                            });
                            break;
                        }
                    }
                }
            })?;
    }

    {
        let frame_tx = frame_tx.clone();
        std::thread::Builder::new()
            .name("chan-terminal-controller".into())
            .spawn(move || loop {
                while let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        PtyCommand::Input(data) => {
                            if let Err(e) = writer.write_all(data.as_bytes()) {
                                let _ = frame_tx.send(ServerFrame::Error {
                                    message: format!("terminal write failed: {e}"),
                                });
                                let _ = killer.kill();
                                return;
                            }
                            let _ = writer.flush();
                        }
                        PtyCommand::Resize(size) => {
                            if let Err(e) = pair.master.resize(size) {
                                let _ = frame_tx.send(ServerFrame::Error {
                                    message: format!("terminal resize failed: {e}"),
                                });
                            }
                        }
                        PtyCommand::Kill => {
                            let _ = killer.kill();
                            return;
                        }
                    }
                }

                match child.try_wait() {
                    Ok(Some(status)) => {
                        let _ = frame_tx.send(ServerFrame::Exit {
                            code: status.exit_code(),
                        });
                        return;
                    }
                    Ok(None) => std::thread::sleep(Duration::from_millis(25)),
                    Err(e) => {
                        let _ = frame_tx.send(ServerFrame::Error {
                            message: format!("terminal wait failed: {e}"),
                        });
                        return;
                    }
                }
            })?;
    }

    Ok(PtySession {
        tx: cmd_tx,
        rx: frame_rx,
    })
}

fn default_shell() -> String {
    #[cfg(windows)]
    {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
    }
    #[cfg(not(windows))]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::time::{Duration, Instant};

    fn command_available(name: &str) -> bool {
        Command::new("sh")
            .arg("-lc")
            .arg(format!("command -v {name} >/dev/null 2>&1"))
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    async fn collect_until(session: &mut PtySession, needle: &str, timeout: Duration) -> String {
        let deadline = Instant::now() + timeout;
        let mut out = String::new();
        loop {
            if out.contains(needle) || Instant::now() >= deadline {
                return out;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            match tokio::time::timeout(remaining, session.rx.recv()).await {
                Ok(Some(ServerFrame::Output { data })) => {
                    out.push_str(&String::from_utf8_lossy(&data))
                }
                Ok(Some(ServerFrame::Error { message })) => {
                    out.push_str(&format!("\n__ERROR__{message}\n"));
                }
                Ok(Some(_)) => {}
                Ok(None) | Err(_) => return out,
            }
        }
    }

    async fn collect_until_idle(
        session: &mut PtySession,
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
                Ok(Some(ServerFrame::Output { data })) => {
                    out.push_str(&String::from_utf8_lossy(&data));
                    last_output = Instant::now();
                }
                Ok(Some(ServerFrame::Error { message })) => {
                    out.push_str(&format!("\n__ERROR__{message}\n"));
                    last_output = Instant::now();
                }
                Ok(Some(_)) => {}
                Ok(None) => return out,
                Err(_) => {}
            }
        }
    }

    async fn run_shell_probe(command: &str, end: &str) -> String {
        let tmp = tempfile::tempdir().expect("temp drive");
        let mut session =
            spawn_pty_session(tmp.path().to_path_buf(), pty_size(Some(100), Some(31)))
                .expect("spawn pty");
        let _ = collect_until_idle(
            &mut session,
            Duration::from_millis(300),
            Duration::from_millis(100),
        )
        .await;
        session.input("stty -echo 2>/dev/null\r".to_string());
        let _ = collect_until_idle(
            &mut session,
            Duration::from_millis(300),
            Duration::from_millis(100),
        )
        .await;
        session.input(format!("{command}\r"));
        let mut out = collect_until(&mut session, end, Duration::from_secs(5)).await;
        out.push_str(
            &collect_until_idle(
                &mut session,
                Duration::from_millis(300),
                Duration::from_millis(100),
            )
            .await,
        );
        out
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
            let mut session =
                spawn_pty_session(tmp.path().to_path_buf(), pty_size(Some(100), Some(31)))
                    .expect("spawn pty");
            let _ = collect_until_idle(
                &mut session,
                Duration::from_millis(300),
                Duration::from_millis(100),
            )
            .await;
            session.input("stty -echo 2>/dev/null\r".to_string());
            let _ = collect_until_idle(
                &mut session,
                Duration::from_millis(300),
                Duration::from_millis(100),
            )
            .await;
            session.input("printf 'alpha\\nbeta\\n' | less\r".to_string());
            let out = collect_until(&mut session, "alpha", Duration::from_secs(5)).await;
            assert!(
                out.contains("alpha"),
                "less should render piped text, got {out:?}"
            );
            session.input("q".to_string());
            session.input("printf '\\n__LESS_END__\\n'\r".to_string());
            let out = collect_until(&mut session, "__LESS_END__", Duration::from_secs(5)).await;
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
}
