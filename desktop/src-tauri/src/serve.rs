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

use std::collections::VecDeque;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;

/// Per-process monotonic counter appended to every drive-window
/// label so the user can open more than one window for the same
/// drive (local or tunneled). Tauri requires unique window labels
/// per process; the prefix encodes the drive identity and the seq
/// disambiguates instances.
static WINDOW_SEQ: AtomicU64 = AtomicU64::new(0);

fn next_window_seq() -> u64 {
    WINDOW_SEQ.fetch_add(1, Ordering::Relaxed)
}

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::AppState;

/// Tauri event emitted when any serve's state changes (started,
/// URL discovered, exited). The frontend reacts by re-fetching the
/// drive list.
pub const SERVES_CHANGED: &str = "serves-changed";

/// Tauri event emitted when a `chan serve` exits before printing
/// the URL banner, i.e. failed to start. Payload is
/// `ServeFailedPayload`. The frontend uses this to pop a modal
/// dialog with the captured stderr so the user can tell why instead
/// of just seeing the On toggle flip back to off.
pub const SERVE_FAILED: &str = "serve-failed";

/// Tauri event emitted when a `chan serve` exits abnormally after
/// it had already printed a URL and was therefore visible to the
/// user. Payload is `ServeFailedPayload`; the frontend shows it as
/// a soft inline notice rather than a startup-failure modal.
pub const SERVE_CRASHED: &str = "serve-crashed";

/// Cap on stderr lines retained for the serve-failed payload. Chan's
/// startup output is short; 50 lines is enough to capture the
/// failure context without unbounded memory growth if the child
/// crashes mid-stream.
const STDERR_TAIL_MAX: usize = 50;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(15);
const STOP_GRACE: Duration = Duration::from_secs(5);
const MAX_WINDOWS_PER_DRIVE: usize = 10;

#[derive(Debug, Clone, Serialize)]
pub struct ServeFailedPayload {
    pub key: String,
    /// Process exit code if the child terminated normally. `None` on
    /// platforms or paths where we couldn't reap it.
    pub exit_code: Option<i32>,
    /// Unix signal number if the child was killed by a signal.
    /// Always `None` on non-Unix.
    pub exit_signal: Option<i32>,
    /// Last `STDERR_TAIL_MAX` stderr lines, oldest first. Empty when
    /// the child died before writing anything.
    pub stderr_tail: Vec<String>,
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
pub fn start(
    app: AppHandle,
    state: Arc<AppState>,
    key: String,
    chan_bin: &Path,
) -> Result<(), String> {
    if state.serves.lock().unwrap().contains_key(&key) {
        return Ok(());
    }
    let preferred = state.drive_port(&key);
    let port = pick_port_preferring(preferred).map_err(|e| format!("allocating port: {e}"))?;
    if let Err(e) = state.set_drive_port(&key, port) {
        tracing::warn!(key = %key, error = %e, "persisting serve port failed");
    }

    let mut cmd = Command::new(chan_bin);
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

    state
        .serves
        .lock()
        .unwrap()
        .insert(key.clone(), ServeHandle { child, url: None });
    let _ = app.emit(SERVES_CHANGED, ());

    // Reader thread. Owns the stderr pipe; on EOF the child has
    // exited (or has been killed), so we reap and clean up state.
    // We also keep a rolling tail of stderr lines so a startup
    // failure (EOF before the URL banner) can be surfaced to the
    // user with context instead of just flipping the toggle off.
    let app2 = app.clone();
    let state2 = state.clone();
    let key2 = key.clone();
    let startup_complete = Arc::new(AtomicBool::new(false));
    let startup_complete_for_reader = startup_complete.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        let mut saw_ready_banner = false;
        let mut saw_url = false;
        let mut tail: VecDeque<String> = VecDeque::with_capacity(STDERR_TAIL_MAX + 1);
        for line in reader.lines() {
            let Ok(line) = line else { break };

            if tail.len() == STDERR_TAIL_MAX {
                tail.pop_front();
            }
            tail.push_back(line.clone());

            // chan's banner: "chan is ready:" then a line with the
            // URL. The URL line is the first non-empty line after
            // the banner. We're forgiving about exact match because
            // the banner format is owned by chan, not us.
            if !saw_ready_banner {
                if line.contains("chan is ready") {
                    saw_ready_banner = true;
                }
            } else if !line.trim().is_empty() && state2.set_serve_url(&key2, line.trim()) {
                saw_url = true;
                startup_complete_for_reader.store(true, Ordering::Release);
                let _ = app2.emit(SERVES_CHANGED, ());
                saw_ready_banner = false; // only capture the first URL
                let _ = spawn_local_drive_window(&app2, &key2, line.trim());
            }
        }

        // Reader hit EOF: chan exited (intentional kill or crash).
        // Reap and remove from the live map. `list_drives` derives
        // the row's On state from this map, so removal alone is
        // enough to bring the toggle back to off on the next render.
        let handle = state2.serves.lock().unwrap().remove(&key2);
        let was_tracked = handle.is_some();
        let exit_status = handle.and_then(|mut h| h.child.wait().ok());

        // Startup failure: EOF before we ever captured the URL. The
        // toggle would silently revert to off otherwise; emit a
        // structured event so the renderer can show the captured
        // stderr in a modal. A mid-flight crash gets a softer inline
        // event because the drive had been working previously.
        let (exit_code, exit_signal) = exit_info(exit_status.as_ref());
        if !was_tracked {
            // `stop` / `stop_all` removes the handle before
            // terminating the child, so this EOF is intentional.
        } else if !saw_url {
            let _ = app2.emit(
                SERVE_FAILED,
                ServeFailedPayload {
                    key: key2.clone(),
                    exit_code,
                    exit_signal,
                    stderr_tail: tail.into_iter().collect(),
                },
            );
        } else if !normal_termination(exit_code, exit_signal) {
            let _ = app2.emit(
                SERVE_CRASHED,
                ServeFailedPayload {
                    key: key2.clone(),
                    exit_code,
                    exit_signal,
                    stderr_tail: tail.into_iter().collect(),
                },
            );
        }

        // Tear down every drive window we opened for this key.
        // Window CloseRequested is a no-op now (multi-window means
        // closing one window must NOT stop the serve), so this is
        // the single point where on-exit cleanup happens.
        close_local_drive_windows(&app2, &key2);
        let _ = app2.emit(SERVES_CHANGED, ());
    });

    let state_for_watchdog = state.clone();
    let key_for_watchdog = key;
    thread::spawn(move || {
        thread::sleep(STARTUP_TIMEOUT);
        if startup_complete.load(Ordering::Acquire) {
            return;
        }
        let mut serves = state_for_watchdog.serves.lock().unwrap();
        if let Some(handle) = serves.get_mut(&key_for_watchdog) {
            if handle.url.is_none() {
                tracing::warn!(key = %key_for_watchdog, "chan serve startup timed out");
                let _ = handle.child.kill();
            }
        }
    });

    Ok(())
}

/// Stop a running serve. No-op if the drive isn't running. Removes
/// the live entry before waiting so an immediate stop -> start can
/// spawn a fresh child instead of observing stale map state.
pub fn stop(state: &AppState, key: &str) {
    let handle = state.serves.lock().unwrap().remove(key);
    if let Some(h) = handle {
        stop_child(h.child, Instant::now() + STOP_GRACE);
    }
}

/// Stop every running serve. Called from the Tauri Exit hook so
/// chan children don't outlive the desktop process.
pub fn stop_all(state: &AppState) {
    let handles: Vec<ServeHandle> = state
        .serves
        .lock()
        .unwrap()
        .drain()
        .map(|(_, h)| h)
        .collect();
    let deadline = Instant::now() + STOP_GRACE;
    for h in handles {
        stop_child(h.child, deadline);
    }
}

fn stop_child(mut child: Child, deadline: Instant) {
    #[cfg(unix)]
    {
        let pid = nix::unistd::Pid::from_raw(child.id() as i32);
        let _ = nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGTERM);
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill();
    }

    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            Ok(None) => break,
            Err(e) => {
                tracing::warn!(pid = child.id(), error = %e, "waiting for chan serve failed");
                break;
            }
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn exit_info(status: Option<&ExitStatus>) -> (Option<i32>, Option<i32>) {
    let Some(status) = status else {
        return (None, None);
    };
    let code = status.code();
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        status.signal()
    };
    #[cfg(not(unix))]
    let signal: Option<i32> = None;
    (code, signal)
}

fn normal_termination(exit_code: Option<i32>, exit_signal: Option<i32>) -> bool {
    if exit_code == Some(0) {
        return true;
    }
    #[cfg(unix)]
    {
        matches!(
            exit_signal,
            Some(x) if x == nix::libc::SIGTERM || x == nix::libc::SIGINT
        )
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// Stable Tauri window-label prefix for a local drive. Used to
/// recognise every window that belongs to the drive when the user
/// has opened more than one (close-all on serve exit, capability
/// matching). Tauri labels must match `[a-zA-Z0-9_-]+`, and drive
/// keys are filesystem paths, so we hash the key.
pub fn drive_window_prefix(key: &str) -> String {
    let mut h = DefaultHasher::new();
    key.hash(&mut h);
    format!("drive-{:016x}", h.finish())
}

/// Fresh, unique window label for a new local-drive webview.
/// Every call yields a distinct label so multi-window works; the
/// prefix is still identifiable for cleanup. Format:
/// `drive-<hash>-<seq>` where `seq` is a per-process atomic.
pub fn new_drive_window_label(key: &str) -> String {
    format!("{}-{}", drive_window_prefix(key), next_window_seq())
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
    format!("chan drive: {base}")
}

/// Stable window-label prefix for a tunneled drive, namespaced
/// separately from `drive-*` so a local drive and a tunneled drive
/// with the same canonical name don't collide.
pub fn tunnel_window_prefix(tenant_label: &str, drive: &str) -> String {
    let mut h = DefaultHasher::new();
    tenant_label.hash(&mut h);
    drive.hash(&mut h);
    format!("tunnel-{:016x}", h.finish())
}

/// Fresh, unique window label for a tunneled drive webview. Same
/// shape as `new_drive_window_label`.
pub fn new_tunnel_window_label(tenant_label: &str, drive: &str) -> String {
    format!(
        "{}-{}",
        tunnel_window_prefix(tenant_label, drive),
        next_window_seq()
    )
}

/// Spawn a new local-drive webview window pointing at `url`. Each
/// call opens an independent window; multiple windows per drive are
/// supported. The Tauri close handler is a no-op — closing the
/// window does NOT stop the underlying `chan serve`. The On toggle
/// (and `close_local_drive_windows` from the reader thread on EOF)
/// is the single authority on drive lifecycle.
pub fn spawn_local_drive_window(app: &AppHandle, key: &str, url: &str) -> Result<(), String> {
    ensure_window_capacity(app, &drive_window_prefix(key))?;
    let label = new_drive_window_label(key);
    let title = drive_title(key);
    build_drive_window(app, &label, &title, url)
}

/// Spawn a new tunneled-drive webview window. Same multi-window
/// semantics as the local variant; same no-op close handler.
pub fn spawn_tunneled_drive_window(
    app: &AppHandle,
    tenant_label: &str,
    drive: &str,
    url: &str,
) -> Result<(), String> {
    ensure_window_capacity(app, &tunnel_window_prefix(tenant_label, drive))?;
    let label = new_tunnel_window_label(tenant_label, drive);
    let title = format!("chan drive: {tenant_label} \u{00b7} {drive}");
    build_drive_window(app, &label, &title, url)
}

/// Build and show a chan-style drive webview window on the main
/// thread. Internal — call `spawn_local_drive_window` /
/// `spawn_tunneled_drive_window` from outside. Centralising the
/// key-bridge JS, the size defaults, the zoom-hotkey polyfill, and
/// the drag-drop handler off in one place means drive UX changes
/// don't fork between the local and tunneled paths.
fn build_drive_window(
    app: &AppHandle,
    window_label: &str,
    title: &str,
    url: &str,
) -> Result<(), String> {
    let Ok(mut parsed) = url.parse::<tauri::Url>() else {
        return Err(format!("bad chan URL for {window_label}: {url}"));
    };
    parsed.query_pairs_mut().append_pair("w", window_label);
    let app_owned = app.clone();
    let label_owned = window_label.to_string();
    let title_owned = title.to_string();
    let res = app.run_on_main_thread(move || {
        // Defensive: window labels are unique-per-instance now, so
        // a collision shouldn't happen. If it ever does (e.g. some
        // future code reusing a stable label), destroy the stale
        // window so `build` doesn't panic.
        if let Some(old) = app_owned.get_webview_window(&label_owned) {
            let _ = old.destroy();
        }
        match WebviewWindowBuilder::new(&app_owned, &label_owned, WebviewUrl::External(parsed))
            .title(title_owned)
            .inner_size(1200.0, 800.0)
            .min_inner_size(640.0, 400.0)
            .resizable(true)
            .initialization_script(KEY_BRIDGE_JS)
            // Tauri polyfill: Cmd/Ctrl + [+ = -] and mousewheel zoom,
            // 20% per step, 20%-1000%. Requires the
            // `core:webview:allow-set-webview-zoom` permission on
            // drive-* / tunnel-* windows in capabilities/drive.json.
            .zoom_hotkeys_enabled(true)
            // Hand HTML5 drag-and-drop to the page. Tauri's OS-level
            // drag handler swallows dragover events otherwise, so
            // chan's pane-to-pane tab moves never see the highlight /
            // drop the receiving pane expects.
            .disable_drag_drop_handler()
            .build()
        {
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(label = %label_owned, error = %e, "opening drive window failed")
            }
        }
    });
    res.map_err(|e| format!("scheduling drive window for {window_label}: {e}"))
}

fn ensure_window_capacity(app: &AppHandle, prefix: &str) -> Result<(), String> {
    let count = app
        .webview_windows()
        .keys()
        .filter(|label| label.starts_with(prefix))
        .count();
    if count >= MAX_WINDOWS_PER_DRIVE {
        return Err(format!(
            "Drive already has {MAX_WINDOWS_PER_DRIVE} open windows; close one before opening another."
        ));
    }
    Ok(())
}

/// Destroy every webview window opened for this local drive. Used
/// by the reader thread when the serve has gone away (intentional
/// kill or crash) so stale windows don't linger pointing at a dead
/// port. Walks `webview_windows()` and matches by prefix because
/// the user may have opened several windows for the same drive.
pub fn close_local_drive_windows(app: &AppHandle, key: &str) {
    close_windows_with_prefix(app, &drive_window_prefix(key))
}

/// Destroy every webview window opened for this tunneled drive.
/// Used by the tunnel supervisor when a (label, drive) pair drops
/// out of the registry — the remote has gone away, so the per-tenant
/// listener no longer routes for it and any open window now points
/// at nothing useful.
pub fn close_tunneled_drive_windows(app: &AppHandle, tenant_label: &str, drive: &str) {
    close_windows_with_prefix(app, &tunnel_window_prefix(tenant_label, drive))
}

/// Destroy every tunneled-drive webview window in the process,
/// regardless of which (label, drive) it belongs to. Used by the
/// tunnel module on `stop_listening`: the tunnel listener and
/// every per-tenant listener are about to be cancelled, so the
/// open windows would all error on their next request anyway.
pub fn close_all_tunneled_drive_windows(app: &AppHandle) {
    close_windows_with_prefix(app, "tunnel-")
}

fn close_windows_with_prefix(app: &AppHandle, prefix: &str) {
    let app_owned = app.clone();
    let prefix_owned = prefix.to_string();
    let _ = app.run_on_main_thread(move || {
        // Snapshot first; destroying inside the iterator would
        // mutate the underlying map mid-walk.
        let labels: Vec<String> = app_owned
            .webview_windows()
            .keys()
            .filter(|l| l.starts_with(&prefix_owned))
            .cloned()
            .collect();
        for l in labels {
            if let Some(w) = app_owned.get_webview_window(&l) {
                let _ = w.destroy();
            }
        }
    });
}

/// Native keyboard shortcuts for drive webviews. Translates chords
/// into the host-agnostic `chan:command` window event that chan's
/// App.svelte listens for. Runs before any page script, in capture
/// phase with stopImmediatePropagation, so this script is the sole
/// authority on every chord it claims — chan's onWindowKey doesn't
/// fire for these even if its keymap drifts.
///
/// Layout mirrors VS Code; chords that browsers reserve at OS level
/// (Cmd+W, Cmd+N, Cmd+Shift+[/], Cmd+1..9) are bound here because
/// the native webview doesn't have those reservations. chan's web
/// fallbacks (Alt+Shift, Ctrl+Alt) keep working independently.
const KEY_BRIDGE_JS: &str = r#"
(() => {
  function fire(e, name, detail) {
    e.preventDefault();
    e.stopImmediatePropagation();
    window.dispatchEvent(new CustomEvent('chan:command',
      { detail: Object.assign({ name: name }, detail || {}) }));
  }
  function onKey(e) {
    const meta = e.metaKey || e.ctrlKey;
    if (!meta || e.altKey) return;
    const shift = e.shiftKey;
    const code = e.code;
    if (!shift) {
      switch (code) {
        case 'KeyP': fire(e, 'app.files.toggle');     return;
        case 'KeyN': fire(e, 'app.file.new');         return;
        case 'KeyW': fire(e, 'app.tab.close');        return;
        case 'KeyF': fire(e, 'app.find.open');        return;
        case 'KeyG': fire(e, 'app.find.next');        return;
        case 'Backquote': fire(e, 'app.terminal.toggle'); return;
        case 'BracketLeft': fire(e, 'app.pane.prev'); return;
        case 'BracketRight': fire(e, 'app.pane.next'); return;
      }
      const m = code.match(/^Digit([1-9])$/);
      if (m) {
        fire(e, 'app.tab.jump', { index: Number(m[1]) - 1 });
        return;
      }
    } else {
      switch (code) {
        case 'KeyF':         fire(e, 'app.search.toggle'); return;
        case 'KeyG':         fire(e, 'app.find.prev');     return;
        case 'KeyI':         fire(e, 'app.terminal.broadcast.toggle'); return;
        case 'KeyM':         fire(e, 'app.graph.toggle');  return;
        case 'BracketLeft':  fire(e, 'app.tab.prev');      return;
        case 'BracketRight': fire(e, 'app.tab.next');      return;
      }
    }
  }
  window.addEventListener('keydown', onKey, true);
})();
"#;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_termination_accepts_zero_exit() {
        assert!(normal_termination(Some(0), None));
        assert!(!normal_termination(Some(70), None));
    }

    #[cfg(unix)]
    #[test]
    fn normal_termination_accepts_sigterm_and_sigint() {
        assert!(normal_termination(None, Some(nix::libc::SIGTERM)));
        assert!(normal_termination(None, Some(nix::libc::SIGINT)));
        assert!(!normal_termination(None, Some(nix::libc::SIGKILL)));
    }

    #[cfg(unix)]
    #[test]
    fn stop_child_reaps_process() {
        let child = Command::new("sh")
            .args(["-c", "sleep 30"])
            .spawn()
            .expect("spawn sleep");
        let pid = child.id() as i32;
        stop_child(child, Instant::now() + Duration::from_secs(1));
        let still_alive = nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None).is_ok();
        assert!(!still_alive, "child process should be gone");
    }
}
