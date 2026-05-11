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

use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::thread;

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

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
        // chan-desktop owns the window: the webview loads the
        // token-bearing URL once and the SPA caches the token in
        // sessionStorage, so the across-restart breakage that
        // motivated --no-token is moot here. Keeping the token
        // shuts out localhost-fingerprinting from web pages and
        // other local processes that can reach 127.0.0.1.
        "--no-browser",
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

    let window_label = window_label_for(&key);
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
    let window_label2 = window_label.clone();
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
                open_drive_window(&app2, &key2, &window_label2, line.trim());
            }
        }

        // Reader hit EOF: chan exited (intentional kill or crash).
        // Reap and remove from the live map. `list_drives` derives
        // the row's On state from this map, so removal alone is
        // enough to bring the toggle back to off on the next render.
        {
            let mut serves = state2.serves.lock().unwrap();
            if let Some(mut h) = serves.remove(&key2) {
                let _ = h.child.wait();
            }
        }
        // Tear down the drive window if it's still around (e.g. the
        // serve died on its own, or the user toggled Off). Closing
        // the window in response to the user clicking its X already
        // happens via the window's own CloseRequested handler.
        close_drive_window(&app2, &window_label2);
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

/// Stable Tauri window label for a drive. Tauri labels must match
/// `[a-zA-Z0-9_-]+`, and drive keys are filesystem paths, so we hash
/// the key and prefix it. Same key always maps to the same label.
fn window_label_for(key: &str) -> String {
    let mut h = DefaultHasher::new();
    key.hash(&mut h);
    format!("drive-{:016x}", h.finish())
}

/// Best-effort window title: drive folder basename, fall back to the
/// full key. Only used to label the webview window we open for a
/// running serve.
fn drive_title(key: &str) -> String {
    let base = Path::new(key)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| key.to_string());
    format!("chan: {base}")
}

/// Open the drive's webview window on the main thread. The window
/// loads chan's local URL directly. Closing the window stops the
/// serve, so this is the inverse of `set_drive_on(false)`.
fn open_drive_window(app: &AppHandle, key: &str, label: &str, url: &str) {
    let Ok(parsed) = url.parse::<tauri::Url>() else {
        eprintln!("chan-desktop: bad chan URL for {key}: {url}");
        return;
    };
    let app_owned = app.clone();
    let key_owned = key.to_string();
    let label_owned = label.to_string();
    let title = drive_title(key);
    let res = app.run_on_main_thread(move || {
        // Tear down any leftover window with the same label (e.g.
        // a quick Off-then-On cycle where the previous window's
        // cleanup hasn't fully landed yet). Labels must be unique
        // and `build` panics on collision.
        if let Some(old) = app_owned.get_webview_window(&label_owned) {
            let _ = old.destroy();
        }
        let win = match WebviewWindowBuilder::new(
            &app_owned,
            &label_owned,
            WebviewUrl::External(parsed),
        )
        .title(title)
        .inner_size(1200.0, 800.0)
        .min_inner_size(640.0, 400.0)
        .resizable(true)
        .build()
        {
            Ok(w) => w,
            Err(e) => {
                eprintln!("chan-desktop: opening drive window for {key_owned}: {e}");
                return;
            }
        };
        let app_for_event = app_owned.clone();
        let key_for_event = key_owned.clone();
        win.on_window_event(move |event| {
            if matches!(event, WindowEvent::CloseRequested { .. }) {
                let state = app_for_event.state::<Arc<AppState>>();
                stop(&state, &key_for_event);
            }
        });
    });
    if let Err(e) = res {
        eprintln!("chan-desktop: scheduling drive window for {key}: {e}");
    }
}

/// Destroy the drive's webview window if it still exists. Used by
/// the reader thread when the serve has gone away on its own (so the
/// stale window doesn't linger pointing at a dead port). Safe to
/// call when no such window exists.
fn close_drive_window(app: &AppHandle, label: &str) {
    let app_owned = app.clone();
    let label_owned = label.to_string();
    let _ = app.run_on_main_thread(move || {
        if let Some(w) = app_owned.get_webview_window(&label_owned) {
            let _ = w.destroy();
        }
    });
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
