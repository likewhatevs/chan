//! Per-drive `chan serve` supervisor.
//!
//! For each drive the user toggles On, we spawn `chan serve <path>
//! --host 127.0.0.1 --port N` as a child process, pipe its stderr,
//! and tail it line by line on a dedicated thread. We need the
//! tail thread for two unrelated reasons:
//!
//! 1. chan prints the bound URL on stderr ("chan is ready:" then a
//!    line with the URL). We capture that and stash it in
//!    `AppState.serves` so `list_drives` can hand it to the UI and
//!    the row's Launch button comes alive.
//! 2. When dev mode is on, every line is also forwarded to the
//!    frontend as a `chan-log` event so the console window can
//!    display it.
//!
//! Stop is currently `Child::kill` (SIGKILL on Unix). chan never
//! gets a chance to flush or unbind cleanly; the OS reclaims the
//! port within seconds. Upgrading to SIGTERM with a grace period
//! is a follow-up; see design.md section 3.4.

use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::thread;

use tauri::{AppHandle, Emitter};

use crate::AppState;

/// Tauri event emitted when any serve's state changes (started,
/// URL discovered, exited). The frontend reacts by re-fetching the
/// drive list.
pub const SERVES_CHANGED: &str = "serves-changed";

/// Tauri event emitted for every captured stderr line. Payload:
/// `{ path: String, line: String }`. The console window subscribes
/// when dev mode is on.
pub const CHAN_LOG: &str = "chan-log";

#[derive(serde::Serialize, Clone)]
struct LogLine<'a> {
    path: &'a str,
    line: &'a str,
}

/// Live state for one running serve. Held in `AppState.serves`
/// keyed by canonical drive path.
pub struct ServeHandle {
    pub child: Child,
    pub url: Option<String>,
}

/// Spawn `chan serve` for a drive. On success the child is inserted
/// into `state.serves` under `key`; the URL is filled in
/// asynchronously by the stderr-tailing thread once chan prints it.
///
/// `verbose` controls the `-vv` flag; pass `cfg.dev_mode` from the
/// caller. Verbosity is fixed for the lifetime of one serve: the
/// caller has to stop and restart the drive to pick up a dev-mode
/// toggle. We accept that wart rather than adding hot-reload of a
/// child process flag.
pub fn start(
    app: AppHandle,
    state: Arc<AppState>,
    key: String,
    chan_bin: &str,
    verbose: bool,
) -> Result<(), String> {
    if state.serves.lock().unwrap().contains_key(&key) {
        return Ok(());
    }
    let preferred = state.drive_port(&key);
    let port = pick_port_preferring(preferred).map_err(|e| format!("allocating port: {e}"))?;
    if let Err(e) = state.set_drive_port(&key, port) {
        eprintln!("chan-desktop: persisting port for {key}: {e}");
    }

    let mut cmd = Command::new(chan_bin);
    if verbose {
        cmd.arg("-vv");
    }
    cmd.args([
        "serve",
        &key,
        "--host",
        "127.0.0.1",
        "--port",
        &port.to_string(),
        // Loopback only and we already trust everything on the
        // local machine, so the rotating bearer token only buys us
        // surviving-tab breakage on every restart. Drop it for any
        // serve the desktop spawns. Terminal-initiated serves are
        // not under our control and keep their default behaviour.
        "--no-token",
    ])
    .stdout(Stdio::null())
    .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("spawning `chan serve`: {e}"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "no stderr handle".to_string())?;

    state
        .serves
        .lock()
        .unwrap()
        .insert(key.clone(), ServeHandle { child, url: None });
    let _ = app.emit(SERVES_CHANGED, ());

    // Reader thread. Owns the stderr pipe; on EOF the child has
    // exited (or has been killed), so we reap and clean up state.
    let app2 = app.clone();
    let state2 = state.clone();
    let key2 = key.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        let mut saw_ready_banner = false;
        for line in reader.lines() {
            let Ok(line) = line else { break };

            if verbose {
                let _ = app2.emit(
                    CHAN_LOG,
                    LogLine {
                        path: &key2,
                        line: &line,
                    },
                );
            }

            // chan's banner: "chan is ready:" then a line with the
            // URL. The URL line is the first non-empty line after
            // the banner. We're forgiving about exact match because
            // the banner format is owned by chan, not us.
            if !saw_ready_banner {
                if line.contains("chan is ready") {
                    saw_ready_banner = true;
                }
            } else if !line.trim().is_empty() && state2.set_serve_url(&key2, line.trim()) {
                let _ = app2.emit(SERVES_CHANGED, ());
                saw_ready_banner = false; // only capture the first URL
            }
        }

        // Reader hit EOF: chan exited. Reap and drop.
        let mut serves = state2.serves.lock().unwrap();
        if let Some(mut h) = serves.remove(&key2) {
            let _ = h.child.wait();
        }
        drop(serves);
        let _ = state2.set_drive_off(&key2);
        let _ = app2.emit(SERVES_CHANGED, ());
    });

    Ok(())
}

/// Stop a running serve. No-op if the drive isn't running. Returns
/// when the kill signal has been delivered; the reader thread will
/// finish state cleanup once stderr closes.
pub fn stop(state: &AppState, key: &str) {
    let mut serves = state.serves.lock().unwrap();
    if let Some(h) = serves.get_mut(key) {
        let _ = h.child.kill();
    }
}

/// Stop every running serve. Called from the Tauri Exit hook so
/// chan children don't outlive the desktop process.
pub fn stop_all(state: &AppState) {
    let mut serves = state.serves.lock().unwrap();
    for (_, h) in serves.iter_mut() {
        let _ = h.child.kill();
    }
}

/// Bind 127.0.0.1:0 to let the OS hand us a free port, then close
/// the listener and return the number. Classic TOCTOU: another
/// process could grab the port between close and `chan serve`'s
/// bind. Acceptable for a desktop app launching its own children.
fn pick_port() -> std::io::Result<u16> {
    let l = TcpListener::bind("127.0.0.1:0")?;
    Ok(l.local_addr()?.port())
}

/// Try to bind a previously-used port for this drive so a
/// stop-then-start cycle leaves any open browser tabs on a URL that
/// is still routable. Falls back to a fresh OS-assigned port when
/// the preferred port is taken or when there is no preference yet.
fn pick_port_preferring(preferred: Option<u16>) -> std::io::Result<u16> {
    if let Some(p) = preferred {
        if TcpListener::bind(("127.0.0.1", p)).is_ok() {
            return Ok(p);
        }
    }
    pick_port()
}
