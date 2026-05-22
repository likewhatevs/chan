#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod config;
mod registry;
mod serve;
mod tunnel;
mod watcher;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::menu::{Menu, MenuItemBuilder, MenuItemKind, PredefinedMenuItem, WINDOW_SUBMENU_ID};
use tauri::{Emitter, Manager, RunEvent, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tokio::process::Command;

use config::{Config, ConfigStore, DriveFeatures, WindowConfig};
use serve::ServeHandle;
use tunnel::TunnelState;

const CHAN_BUSY_CHANGED: &str = "chan-busy";
const SYSTEM_NOTICE: &str = "system-notice";

/// Process-wide state. Shared via `Arc` because the serve supervisor
/// hands clones to per-drive reader threads.
pub struct AppState {
    store: Mutex<ConfigStore>,
    /// Live `chan serve` children keyed by canonical drive path.
    /// Holds the captured URL once chan prints it.
    serves: Mutex<HashMap<String, ServeHandle>>,
    /// Embedded chan-tunnel-server. Owns the tunnel listener on
    /// 127.0.0.1:7777, the shared registry, and the per-tenant
    /// loopback listeners that proxy into registered remote
    /// `chan serve` instances.
    tunnel: Arc<TunnelState>,
    /// Result of the boot-time check that the bundled `chan` binary
    /// is present and the desktop is running from a real install
    /// location. Frozen for the life of the process. When `!ok`, the
    /// frontend disables every action that would try to spawn chan,
    /// and the mutating IPC commands short-circuit with `reason`.
    bin_status: BinStatus,
    /// `fullstack-b-19`: per-live-window zoom level. Tracks the
    /// current zoom for every open webview keyed by window label so
    /// `zoom_in` / `zoom_out` / `zoom_reset` can compute the next
    /// level without spawning a JS eval round-trip to read the
    /// current. Drained into `WindowConfig.zoom_level` by the close
    /// handler so the LRU restore from `-b-1` picks the level up on
    /// the next open. Missing entry reads as 1.0 (the chan-desktop
    /// default).
    pub live_window_zooms: Mutex<HashMap<String, f64>>,
}

/// Defense-in-depth sidecar reap: `RunEvent::Exit` is the primary
/// teardown path, but a panic unwinding through `tauri::App` can
/// bypass it entirely. Dropping the last `Arc<AppState>` (which
/// includes panic unwind on the runtime, since chan-desktop builds
/// with the default `unwind` panic strategy) signals every running
/// chan serve via `serve::stop_all`. Idempotent: stop_all drains
/// the serves map, so a normal-exit run followed by Drop is a
/// no-op on the second pass.
impl Drop for AppState {
    fn drop(&mut self) {
        serve::stop_all(self);
    }
}

/// Frontend-visible verdict from the boot-time `chan` preflight.
/// `kind` discriminates the error so the UI can choose copy:
///   * `"ok"`          — binary found, environment is fine.
///   * `"translocated"` — macOS App Translocation detected; the app
///     is running from a randomized read-only path because it was
///     launched from outside `/Applications`. `chan serve` would
///     fail silently. User must move the bundle.
///   * `"missing"`     — bundled sidecar not next to chan-desktop.
///     Corrupt install; should never happen in a packaged build.
#[derive(Debug, Clone, Serialize)]
pub struct BinStatus {
    pub ok: bool,
    pub kind: &'static str,
    pub reason: String,
}

impl BinStatus {
    fn ok_status() -> Self {
        Self {
            ok: true,
            kind: "ok",
            reason: String::new(),
        }
    }
}

impl AppState {
    /// Set the URL on a running serve handle. Returns `true` on a
    /// real change so the caller can decide whether to emit an
    /// event. Caller must NOT hold `serves` lock.
    pub fn set_serve_url(&self, key: &str, url: &str) -> bool {
        let mut serves = self.serves.lock().unwrap();
        let Some(h) = serves.get_mut(key) else {
            return false;
        };
        if h.url.as_deref() == Some(url) {
            return false;
        }
        h.url = Some(url.to_string());
        true
    }

    /// Last port this drive's `chan serve` bound to, if any. Used
    /// by the supervisor to prefer the same port across restarts so
    /// open browser tabs don't permanently dead-end on reconnect.
    pub fn drive_port(&self, key: &str) -> Option<u16> {
        self.store
            .lock()
            .unwrap()
            .get()
            .ok()?
            .sidecar
            .get(key)
            .and_then(|s| s.last_port)
    }

    /// Persist the port chosen for this drive's serve.
    pub fn set_drive_port(&self, key: &str, port: u16) -> std::io::Result<()> {
        let mut store = self.store.lock().unwrap();
        let mut cfg = store.get()?;
        cfg.sidecar.entry(key.to_string()).or_default().last_port = Some(port);
        store.save(&cfg)
    }

    /// Push a closing window's layout onto the LRU stack. Best
    /// effort: any I/O error is logged and dropped so a flaky
    /// config disk doesn't leak through the WindowEvent handler.
    pub fn push_window_config(&self, entry: WindowConfig) {
        let mut store = self.store.lock().unwrap();
        let mut cfg = match store.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "loading config to push window state failed");
                return;
            }
        };
        config::push_window_config(&mut cfg, entry);
        if let Err(e) = store.save(&cfg) {
            tracing::warn!(error = %e, "persisting window config stack failed");
        }
    }

    /// Pop the most-recent WindowConfig matching `key`, removing
    /// it from the stack on disk. Returns `None` when no entry
    /// exists or the config file can't be read. Same best-effort
    /// posture as `push_window_config`.
    pub fn pop_window_config(&self, key: &str) -> Option<WindowConfig> {
        let mut store = self.store.lock().unwrap();
        let mut cfg = match store.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "loading config to pop window state failed");
                return None;
            }
        };
        let popped = config::pop_window_config(&mut cfg, key)?;
        if let Err(e) = store.save(&cfg) {
            tracing::warn!(error = %e, "persisting window config stack failed");
        }
        Some(popped)
    }
}

/// Merged drive view returned to the frontend. Two flavours share
/// the wire shape so the existing renderer can iterate one list:
///
/// * `kind = "local"`: a chan-registry entry, backed by a
///   `chan serve` child the desktop spawned. Includes the canonical
///   filesystem path, registry-derived name, and live URL.
/// * `kind = "tunneled"`: a remote `chan serve` that dialed into
///   the embedded tunnel server. No path; `name` is `"{label} ·
///   {drive}"`; `url` points at the per-tenant loopback listener.
///
/// Fields specific to tunneled rows are optional so the JSON shape
/// is a strict superset of the local row; the renderer reads `kind`
/// once and chooses which optionals to surface.
#[derive(Debug, Clone, Serialize)]
struct Drive {
    kind: &'static str,
    path: String,
    name: String,
    on: bool,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    drive: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    public: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    peer_addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    connected_at: Option<String>,
}

#[tauri::command]
fn list_drives(state: State<Arc<AppState>>) -> Result<Vec<Drive>, String> {
    let serves = state.serves.lock().unwrap();
    let entries = registry::read().map_err(err)?;

    // `on` is derived from a live serve handle, never persisted.
    // That way a desktop restart comes up with everything off
    // (matching reality: nothing is actually running yet) and
    // there is no chance of a stale on=true sticking around after
    // chan died unexpectedly.
    let mut merged: Vec<Drive> = entries
        .into_iter()
        .map(|e| {
            let key = canonical_key(&e.path);
            let display_path = key.clone();
            let name = e
                .name
                .or_else(|| basename(&e.path))
                .unwrap_or_else(|| display_path.clone());
            let handle = serves.get(&key);
            let on = handle.is_some();
            let url = handle.and_then(|h| h.url.clone()).unwrap_or_default();
            Drive {
                kind: "local",
                path: display_path,
                name,
                on,
                url,
                label: None,
                drive: None,
                public: None,
                peer_addr: None,
                connected_at: None,
            }
        })
        .collect();

    // Tunneled rows: one per registered (label, drive) in the
    // embedded chan-tunnel-server. URL is populated by the
    // supervisor as soon as the per-tenant listener binds; an
    // empty URL means "just registered, the listener will follow
    // on the next 500ms tick".
    for t in state.tunnel.snapshot() {
        merged.push(Drive {
            kind: "tunneled",
            path: String::new(),
            name: format!("{} \u{00b7} {}", t.label, t.drive),
            on: true,
            url: t.url,
            label: Some(t.label),
            drive: Some(t.drive),
            public: Some(t.public),
            peer_addr: t.peer_addr,
            connected_at: Some(t.connected_at),
        });
    }

    Ok(merged)
}

#[tauri::command]
async fn add_drive(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    path: String,
) -> Result<(), String> {
    require_bin(&state.bin_status)?;
    let path = canonical_key(Path::new(&path));
    let bin = serve::resolve_chan_binary()?;
    emit_chan_busy(&app, true, "add", &path);
    let out = Command::new(&bin)
        .args(["add", &path])
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|e| format!("running `chan add`: {e}"));
    emit_chan_busy(&app, false, "add", &path);
    let out = out?;
    if !out.status.success() {
        return Err(format!(
            "`chan add` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    // Auto-start: opening a drive from the desktop is the user's
    // way of saying "make this drive usable now". Spinning up the
    // serve immediately is what they expect; otherwise the freshly
    // added row sits there with On=off and Launch disabled, which
    // looks broken.
    serve::start(app, Arc::clone(&state), path, &bin)?;
    Ok(())
}

#[tauri::command]
async fn remove_drive(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    path: String,
) -> Result<(), String> {
    require_bin(&state.bin_status)?;
    let key = canonical_key(Path::new(&path));
    serve::stop(&state, &key);

    emit_chan_busy(&app, true, "remove", &key);
    let out = Command::new(serve::resolve_chan_binary()?)
        .args(["remove", &key])
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|e| format!("running `chan remove`: {e}"));
    emit_chan_busy(&app, false, "remove", &key);
    let out = out?;
    if !out.status.success() {
        return Err(format!(
            "`chan remove` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    let mut store = state.store.lock().unwrap();
    let mut cfg = store.get().map_err(err)?;
    cfg.sidecar.remove(&key);
    store.save(&cfg).map_err(err)?;
    Ok(())
}

#[tauri::command]
fn set_drive_on(
    app: tauri::AppHandle,
    state: State<Arc<AppState>>,
    path: String,
    on: bool,
) -> Result<(), String> {
    let key = canonical_key(Path::new(&path));
    if on {
        require_bin(&state.bin_status)?;
        serve::start(app, Arc::clone(&state), key, &serve::resolve_chan_binary()?)?;
    } else {
        serve::stop(&state, &key);
    }
    Ok(())
}

/// `fullstack-b-22`: result returned by [`reclaim_drive_lock`].
/// Frontend reads the fields to decide whether to show a success
/// toast, a "killed but retry failed" warning, or a "no orphan
/// found, you may need to `pkill chan`" surface.
#[derive(Debug, Clone, Serialize)]
struct ReclaimResult {
    /// Pids that were signaled (SIGTERM, escalated to SIGKILL).
    /// Empty when no orphan was found that matches the drive key.
    killed_pids: Vec<u32>,
    /// True when the retry `serve::start` after the kill succeeded
    /// (chan serve handed off to the supervisor; the SPA will pick
    /// up the URL via the usual `serves-changed` event).
    retry_succeeded: bool,
    /// Pre-formatted line for the frontend to render in a toast /
    /// status message. Already contains drive key context.
    message: String,
}

/// `fullstack-b-22`: minimum-viable lock-takeover for orphan
/// `chan serve` sidecars. When chan-desktop is killed ungracefully
/// (SIGKILL, panic that bypasses the unwind, OS reboot mid-run),
/// the bundled chan children get reparented to PID 1 and continue
/// holding the per-drive flock. A fresh chan-desktop launch sees
/// `serve-failed` with `"drive is locked by another process"` in
/// the stderr tail; the SPA then prompts the user and routes here
/// to reclaim.
///
/// Skips elaborate detection heuristics per the task body's
/// minimum-viable framing: any process whose argv contains `chan`,
/// ` serve `, and the drive key is treated as a takeover
/// candidate. The Reclaim button is the user's opt-in.
#[tauri::command]
fn reclaim_drive_lock(
    app: tauri::AppHandle,
    state: State<Arc<AppState>>,
    path: String,
) -> Result<ReclaimResult, String> {
    let key = canonical_key(Path::new(&path));
    let candidates = serve::find_orphan_chan_serve_candidates(&key)?;
    if candidates.is_empty() {
        return Ok(ReclaimResult {
            killed_pids: vec![],
            retry_succeeded: false,
            message: format!(
                "No orphan `chan serve` process matched {key}. The drive lock may be \
                 held by an unrelated process; manual `pkill chan` may be needed."
            ),
        });
    }
    let pids: Vec<u32> = candidates.iter().map(|c| c.pid).collect();
    for pid in &pids {
        serve::kill_orphan_with_grace(*pid);
    }
    require_bin(&state.bin_status)?;
    match serve::start(
        app,
        Arc::clone(&state),
        key.clone(),
        &serve::resolve_chan_binary()?,
    ) {
        Ok(()) => Ok(ReclaimResult {
            killed_pids: pids,
            retry_succeeded: true,
            message: format!("Reclaimed {key} from orphan sidecar."),
        }),
        Err(e) => Ok(ReclaimResult {
            killed_pids: pids,
            retry_succeeded: false,
            message: format!("Killed orphan sidecar(s) for {key} but the retry start failed: {e}"),
        }),
    }
}

/// `fullstack-b-25`: surface the candidate set to the SPA so the
/// reclaim dialog can render PID + command line per row. The user
/// only confirms the kill after seeing what would be SIGTERM'd.
/// Race window between this call and the `reclaim_drive_lock`
/// follow-up is acceptable: a stale-but-recent candidate list is
/// fine; the reclaim path re-enumerates internally before the
/// kill.
#[tauri::command]
fn find_drive_lock_candidates(path: String) -> Result<Vec<serve::OrphanCandidate>, String> {
    let key = canonical_key(Path::new(&path));
    serve::find_orphan_chan_serve_candidates(&key)
}

#[tauri::command]
fn get_config(state: State<Arc<AppState>>) -> Result<Config, String> {
    state.store.lock().unwrap().get().map_err(err)
}

/// `fullstack-b-28a`: read the persisted feature toggles for a
/// drive. Returns the default `{bge: false, reports: false}` for
/// any drive that has no sidecar entry yet — the launcher's
/// expand panel calls this on render so first-time drives show
/// up with both toggles off as the round-2-plan specifies.
///
/// Stub: persistence lives in chan-desktop's sidecar config
/// until `systacean-27` ships the chan-drive config API; `-b-28b`
/// will swap the body without changing the IPC contract.
#[tauri::command]
fn get_drive_features(
    state: State<Arc<AppState>>,
    path: String,
) -> Result<DriveFeatures, String> {
    let key = canonical_key(Path::new(&path));
    let cfg = state.store.lock().unwrap().get().map_err(err)?;
    Ok(cfg.sidecar.get(&key).map(|s| s.features).unwrap_or_default())
}

/// `fullstack-b-28a`: write the feature toggle pair for a drive.
/// Both fields are written together so a partial flip doesn't
/// leave a half-state on disk; the SPA always sends the current
/// full state on every change.
///
/// Stub: persistence lives in chan-desktop's sidecar config
/// until `systacean-27` ships the chan-drive config API; `-b-28b`
/// will swap the body to call the chan-drive `Drive::set_feature_*`
/// helpers without changing the IPC contract.
#[tauri::command]
fn set_drive_features(
    state: State<Arc<AppState>>,
    path: String,
    features: DriveFeatures,
) -> Result<(), String> {
    let key = canonical_key(Path::new(&path));
    let mut store = state.store.lock().unwrap();
    let mut cfg = store.get().map_err(err)?;
    cfg.sidecar.entry(key).or_default().features = features;
    store.save(&cfg).map_err(err)
}

#[derive(Debug, Clone, Serialize)]
struct TunnelStatus {
    /// True while the tunnel listener is bound.
    listening: bool,
    /// Actual bound port (only populated while `listening`).
    port: Option<u16>,
    /// User's preferred port from the sidecar config. `0` means
    /// "let the OS assign one". UI uses this to populate the port
    /// input field.
    preferred_port: u16,
    /// Either the user's saved label or a freshly-suggested one if
    /// they've never typed anything. Suggestions avoid colliding
    /// with labels currently registered in the running tunnel:
    /// "tunnel" → "tunnel-1" → ... up to 999.
    preferred_label: String,
    /// User's saved drive name or a default ("notes"). No
    /// collision check — drive uniqueness is scoped per label, and
    /// the desktop doesn't track which labels are remotely
    /// preferred.
    preferred_drive: String,
    /// Pre-formatted `ssh -R` reverse-forward snippet. `None` when
    /// the tunnel isn't listening (no port to reference yet).
    ssh_snippet: Option<String>,
    /// Pre-formatted `chan serve` command with the bound port,
    /// canonical TUNNEL_PATH, and the user's chosen label/drive
    /// already substituted. Copy-paste ready.
    chan_serve_snippet: Option<String>,
}

/// Build the `ssh -R` and `chan serve` snippets that the listen
/// panel renders verbatim. Pre-formatting them here means JS does
/// zero templating — and the canonical URL path (with
/// `TUNNEL_PATH`) lives in exactly one place in the codebase.
fn build_snippets(port: u16, label: &str, drive: &str) -> (String, String) {
    let ssh = format!("ssh -R {port}:localhost:{port} user@remote");
    // `--no-browser` keeps chan serve from launching the remote's
    // default browser at startup (it has nothing to point at — the
    // visitor URL belongs to chan-desktop, which is what auto-opens
    // the drive webview on this side instead). `PATH` goes last so
    // the user only needs to edit one trailing argument.
    let chan = format!(
        "chan serve --tunnel-url=http://127.0.0.1:{port}{path} \
         --tunnel-token={label} --tunnel-drive={drive} --no-browser PATH",
        path = chan_tunnel_proto::TUNNEL_PATH,
    );
    (ssh, chan)
}

/// Pick a label suggestion: if the user has one saved, use it
/// verbatim. Otherwise try "tunnel"; if a remote is already
/// registered under that label, walk "tunnel-1", "tunnel-2", ...
/// until we find a free one. Falls back to `tunnel` at the end of
/// the range (uniqueness is best-effort; the registry's
/// last-writer-wins eviction is the real arbiter).
fn suggest_label(saved: &str, state: &AppState) -> String {
    if !saved.is_empty() {
        return saved.to_string();
    }
    let in_use: std::collections::HashSet<String> = state
        .tunnel
        .snapshot()
        .into_iter()
        .map(|d| d.label)
        .collect();
    let base = "tunnel";
    if !in_use.contains(base) {
        return base.to_string();
    }
    for i in 1..1000 {
        let candidate = format!("{base}-{i}");
        if !in_use.contains(&candidate) {
            return candidate;
        }
    }
    base.to_string()
}

fn suggest_drive(saved: &str) -> String {
    if saved.is_empty() {
        "notes".to_string()
    } else {
        saved.to_string()
    }
}

#[tauri::command]
fn tunnel_status(state: State<Arc<AppState>>) -> Result<TunnelStatus, String> {
    let cfg = state.store.lock().unwrap().get().map_err(err)?.tunnel;
    let preferred_label = suggest_label(&cfg.preferred_label, &state);
    let preferred_drive = suggest_drive(&cfg.preferred_drive);
    let port = state.tunnel.tunnel_port();
    let listening = state.tunnel.is_listening();
    let (ssh_snippet, chan_serve_snippet) = match (listening, port) {
        (true, Some(p)) => {
            let (s, c) = build_snippets(p, &preferred_label, &preferred_drive);
            (Some(s), Some(c))
        }
        _ => (None, None),
    };
    Ok(TunnelStatus {
        listening,
        port,
        preferred_port: cfg.preferred_port,
        preferred_label,
        preferred_drive,
        ssh_snippet,
        chan_serve_snippet,
    })
}

/// Start the tunnel listener with the user's chosen port, label,
/// and drive. Validates `label` / `drive` against the protocol's
/// charset rules so the rendered snippet matches what the wire
/// will actually accept. Persists all three for the next session.
#[tauri::command]
async fn tunnel_start(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    preferred_port: u16,
    label: String,
    drive: String,
) -> Result<u16, String> {
    require_bin(&state.bin_status)?;
    let label = label.trim().to_string();
    let drive = drive.trim().to_string();
    if !chan_tunnel_proto::is_valid_username(&label) {
        return Err(format!(
            "invalid label {label:?}: ASCII alphanumerics plus '-' / '_', \
             first char alphanumeric, ≤64 chars",
        ));
    }
    if !chan_tunnel_proto::is_valid_drive_name(&drive) {
        return Err(format!(
            "invalid drive name {drive:?}: lowercase ASCII alphanumerics plus '-', \
             first and last char alphanumeric, ≤32 chars",
        ));
    }
    {
        let mut store = state.store.lock().unwrap();
        let mut cfg = store.get().map_err(err)?;
        cfg.tunnel.preferred_port = preferred_port;
        cfg.tunnel.preferred_label = label;
        cfg.tunnel.preferred_drive = drive;
        store.save(&cfg).map_err(err)?;
    }
    let tunnel = Arc::clone(&state.tunnel);
    tunnel::start_listening(app, tunnel, preferred_port).await
}

#[tauri::command]
fn tunnel_stop(app: tauri::AppHandle, state: State<Arc<AppState>>) {
    tunnel::stop_listening(&app, &state.tunnel);
}

/// Open an additional in-app Tauri webview for a running local
/// drive. The first window is auto-opened by the serve supervisor
/// when chan prints its URL; subsequent clicks on Launch reach
/// here and add new windows alongside it. Errors if the drive is
/// not currently running (no URL captured yet).
#[tauri::command]
fn open_local_drive(
    app: tauri::AppHandle,
    state: State<Arc<AppState>>,
    path: String,
) -> Result<(), String> {
    let key = canonical_key(Path::new(&path));
    let url = state
        .serves
        .lock()
        .unwrap()
        .get(&key)
        .and_then(|h| h.url.clone())
        .ok_or_else(|| format!("drive {key} is not running"))?;
    serve::spawn_local_drive_window(&app, &key, &url)?;
    Ok(())
}

/// Open an additional in-app Tauri webview for a tunneled drive.
/// Each call yields a NEW window — the first one is opened by the
/// supervisor on registration, and the Launch button calls this
/// for subsequent windows. Errors if the per-tenant listener
/// hasn't bound yet (URL not formed).
#[tauri::command]
fn open_tunneled_drive(
    app: tauri::AppHandle,
    state: State<Arc<AppState>>,
    label: String,
    drive: String,
) -> Result<(), String> {
    let url = state
        .tunnel
        .snapshot()
        .into_iter()
        .find(|d| d.label == label && d.drive == drive)
        .map(|d| d.url)
        .ok_or_else(|| format!("no tunneled drive {label}/{drive}"))?;
    if url.is_empty() {
        return Err(format!(
            "tunneled drive {label}/{drive} has no URL yet; per-tenant listener still binding",
        ));
    }
    serve::spawn_tunneled_drive_window(&app, &label, &drive, &url)?;
    Ok(())
}

/// User's home directory as a plain string, for the Drives window
/// to abbreviate paths to `~/...`. Returns an empty string when the
/// platform can't resolve it.
#[tauri::command]
fn home_dir() -> String {
    dirs::home_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default()
}

/// Open the given folder in the OS file manager. macOS: Finder,
/// Linux: default file manager, Windows: Explorer. Used by the
/// Drives window's path cell so users can jump to the drive folder
/// from the row. Trusts the caller to pass a path the user just saw
/// in the list — paths come from `list_drives`, which sources from
/// the chan registry; no shell interpolation, args are passed as
/// argv to the OS open command.
#[tauri::command]
fn reveal_in_finder(path: String) -> Result<(), String> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };
    let status = std::process::Command::new(opener)
        .arg(&path)
        .status()
        .map_err(|e| format!("opening {path}: {e}"))?;
    if !status.success() {
        return Err(format!("opening {path}: {opener} exited with {status}"));
    }
    Ok(())
}

fn show_window(app: &tauri::AppHandle, label: &str) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(label) {
        w.show().map_err(err)?;
        w.set_focus().map_err(err)?;
    }
    Ok(())
}

/// Reload the calling webview window. Drives the SPA's tab
/// context-menu "Reload" entry (via `fullstack-a-36`) AND the
/// `Cmd+R` accelerator wired in `KEY_BRIDGE_JS`. The accelerator
/// path bypasses the SPA event bus and invokes this command
/// directly so a SPA-side fault (frozen Svelte runtime, JS error
/// in the chord handler) doesn't lock the dev affordance away.
#[tauri::command]
fn reload_window(window: tauri::WebviewWindow) -> Result<(), String> {
    // Tauri 2's `WebviewWindow::eval` runs JS inside the webview;
    // we use it instead of the missing-in-2 `reload()` method.
    window
        .eval("window.location.reload()")
        .map_err(|e| format!("reloading window: {e}"))
}

/// Open the DevTools inspector on the calling webview. Mirrors
/// the SPA's "Open Inspector" context-menu entry from `-a-36`
/// AND the `Cmd+Opt+I` accelerator in `KEY_BRIDGE_JS`. Requires
/// the `devtools` Cargo feature on the `tauri` crate (enabled in
/// `desktop/src-tauri/Cargo.toml`) so release builds carry the
/// inspector affordance, not just debug builds. Tauri 2 removed
/// the `app.devTools` JSON config key in favour of this
/// compile-time flag.
#[tauri::command]
fn open_devtools(window: tauri::WebviewWindow) {
    window.open_devtools();
}

/// `fullstack-b-19`: browser-style zoom controls. Step size is
/// 10 % per Cmd++/Cmd+- press; the clamp range matches Tauri's own
/// `zoom_hotkeys_enabled` polyfill semantics (0.25-5.0).
const ZOOM_STEP: f64 = 0.10;
const ZOOM_MIN: f64 = 0.25;
const ZOOM_MAX: f64 = 5.0;

/// Read the current zoom level for `label` from process state,
/// defaulting to 1.0 (chan-desktop's initial zoom). Pure read; the
/// IPC handlers compute the next level locally and write back.
fn current_zoom(state: &AppState, label: &str) -> f64 {
    state
        .live_window_zooms
        .lock()
        .unwrap()
        .get(label)
        .copied()
        .unwrap_or(1.0)
}

fn apply_zoom(window: &tauri::WebviewWindow, state: &AppState, next: f64) -> Result<(), String> {
    let clamped = next.clamp(ZOOM_MIN, ZOOM_MAX);
    window
        .set_zoom(clamped)
        .map_err(|e| format!("setting webview zoom on {}: {e}", window.label()))?;
    state
        .live_window_zooms
        .lock()
        .unwrap()
        .insert(window.label().to_string(), clamped);
    Ok(())
}

/// Zoom the calling webview one step up (Cmd++ / Ctrl++).
#[tauri::command]
fn zoom_in(window: tauri::WebviewWindow, state: State<Arc<AppState>>) -> Result<(), String> {
    let current = current_zoom(&state, window.label());
    apply_zoom(&window, &state, current + ZOOM_STEP)
}

/// Zoom the calling webview one step down (Cmd+- / Ctrl+-).
#[tauri::command]
fn zoom_out(window: tauri::WebviewWindow, state: State<Arc<AppState>>) -> Result<(), String> {
    let current = current_zoom(&state, window.label());
    apply_zoom(&window, &state, current - ZOOM_STEP)
}

/// Reset the calling webview to 100 % (Cmd+0 / Ctrl+0).
#[tauri::command]
fn zoom_reset(window: tauri::WebviewWindow, state: State<Arc<AppState>>) -> Result<(), String> {
    apply_zoom(&window, &state, 1.0)
}

/// Canonical-path key used for sidecar lookups, serve identity, and
/// the displayed path. `canonicalize` falls back to the input on
/// error so we still produce a stable key for not-yet-existing or
/// asleep paths.
fn canonical_key(p: &Path) -> String {
    p.canonicalize()
        .unwrap_or_else(|_| PathBuf::from(p))
        .display()
        .to_string()
}

fn basename(p: &Path) -> Option<String> {
    p.file_name().map(|s| s.to_string_lossy().into_owned())
}

/// Detect macOS App Translocation. When Gatekeeper sees an unsigned
/// or quarantined app launched from outside `/Applications` (e.g.
/// double-clicked inside a mounted .dmg), it runs the bundle from a
/// randomized read-only path under
/// `/private/var/folders/.../AppTranslocation/<UUID>/d/...`. The
/// bundled `chan` sidecar is found at that path, but the runtime
/// environment is hostile enough that `chan serve` exits without
/// printing its ready banner, producing the silent-toggle-flip bug.
/// We treat this as "binary unusable" and refuse to spawn anything.
#[cfg(target_os = "macos")]
fn is_app_translocated() -> bool {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().contains("/AppTranslocation/"))
        .unwrap_or(false)
}

/// Boot-time preflight. Runs once before `AppState` is built and the
/// result is stored verbatim. Order matters: translocation is
/// checked first because the bundled sidecar is found at the
/// translocated path but the broader runtime environment is hostile
/// regardless of which `chan` we'd pick; a PATH-installed `chan`
/// doesn't rescue a translocated install. After that we let
/// `resolve_chan_binary` pick PATH or bundled per the locked
/// Round-2 decision 3 (PATH-first w/ bundled fallback) and validate
/// the resolved path's existence + exact version match.
fn compute_bin_status() -> BinStatus {
    #[cfg(target_os = "macos")]
    {
        if is_app_translocated() {
            return BinStatus {
                ok: false,
                kind: "translocated",
                reason: "Chan is running from a disk image. macOS App \
                         Translocation puts the app in a randomized \
                         read-only path that breaks the drive service. \
                         Drag Chan.app to your Applications folder, then \
                         reopen it from there."
                    .to_string(),
            };
        }
    }
    let bin = match serve::resolve_chan_binary() {
        Ok(p) => p,
        Err(e) => {
            return BinStatus {
                ok: false,
                kind: "missing",
                reason: e,
            };
        }
    };
    if !bin.exists() {
        return BinStatus {
            ok: false,
            kind: "missing",
            reason: format!("chan sidecar not found at {}", bin.display()),
        };
    }
    match serve::probe_chan_version(&bin) {
        Ok(()) => BinStatus::ok_status(),
        Err(e) => BinStatus {
            ok: false,
            kind: "version-mismatch",
            reason: e,
        },
    }
}

#[tauri::command]
fn chan_bin_status(state: State<Arc<AppState>>) -> BinStatus {
    state.bin_status.clone()
}

/// Short-circuit guard for any IPC command that would spawn chan.
/// Frontend disables the corresponding controls, but a determined
/// caller (or a stale event handler) could still reach the command;
/// returning the human-readable reason here keeps the UX consistent
/// with the persistent banner shown by the renderer.
fn require_bin(s: &BinStatus) -> Result<(), String> {
    if s.ok {
        Ok(())
    } else {
        Err(s.reason.clone())
    }
}

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn emit_chan_busy(app: &tauri::AppHandle, busy: bool, op: &str, path: &str) {
    let _ = app.emit(
        CHAN_BUSY_CHANGED,
        serde_json::json!({ "busy": busy, "op": op, "path": path }),
    );
}

fn emit_system_notice(app: &tauri::AppHandle, level: &str, message: impl Into<String>) {
    let _ = app.emit(
        SYSTEM_NOTICE,
        serde_json::json!({ "level": level, "message": message.into() }),
    );
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("CHAN_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn,chan_desktop=info")),
        )
        .with_writer(std::io::stderr)
        .init();
}

fn main() {
    init_tracing();
    let store = ConfigStore::new().expect("failed to init config store");
    let bin_status = compute_bin_status();
    let state = Arc::new(AppState {
        store: Mutex::new(store),
        serves: Mutex::new(HashMap::new()),
        tunnel: TunnelState::new(),
        bin_status,
        live_window_zooms: Mutex::new(HashMap::new()),
    });
    let state_for_exit = Arc::clone(&state);
    let state_for_setup = Arc::clone(&state);

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(state)
        .setup(move |app| {
            install_app_menu(app.handle())?;

            // Deep-link callbacks from the system browser
            // (`chan://auth/callback#...`). Cold-start URLs and
            // runtime URLs both flow through the same handler so the
            // sign-in completes whether the user clicked "Open with
            // chan-desktop" before or after the app was running.
            use tauri_plugin_deep_link::DeepLinkExt;
            let app_for_links = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    auth::handle_callback(&app_for_links, url.as_str());
                }
            });
            if let Ok(Some(urls)) = app.deep_link().get_current() {
                for url in urls {
                    auth::handle_callback(app.handle(), url.as_str());
                }
            }

            // Closing the main window via the red traffic light or
            // Cmd+W should hide it, not destroy it: hidden serve
            // children can still keep the process alive, and
            // reopening via Dock click or the Window > Drives menu
            // item should be instant. Without this, a closed main
            // window cannot be brought back without quitting and
            // relaunching.
            if let Some(main) = app.get_webview_window("main") {
                let main_for_event = main.clone();
                main.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = main_for_event.hide();
                    }
                });
                let _ = main.show();
                let _ = main.set_focus();
            }

            // Registry watcher. Leaked: we want it alive for the
            // process lifetime and the inner Watcher type is
            // unnameable through `manage`.
            match watcher::spawn(app.handle().clone(), &registry::path()) {
                Ok(d) => {
                    Box::leak(Box::new(d));
                }
                Err(e) => {
                    tracing::warn!(error = %e, "registry watcher disabled");
                    emit_system_notice(
                        app.handle(),
                        "warning",
                        "Auto-refresh disabled; close and reopen the window after running chan add.",
                    );
                }
            }

            // Tunnel listener is OFF until the user explicitly
            // clicks "Attach" in the Drives window. We just
            // construct the empty TunnelState during boot; binding
            // 127.0.0.1 happens on the IPC `tunnel_start` call.
            let _ = state_for_setup.tunnel.clone();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_drives,
            add_drive,
            remove_drive,
            set_drive_on,
            reclaim_drive_lock,
            find_drive_lock_candidates,
            get_drive_features,
            set_drive_features,
            get_config,
            home_dir,
            reveal_in_finder,
            reload_window,
            open_devtools,
            zoom_in,
            zoom_out,
            zoom_reset,
            tunnel_status,
            tunnel_start,
            tunnel_stop,
            open_local_drive,
            open_tunneled_drive,
            chan_bin_status,
            auth::auth_status,
            auth::open_signin,
            auth::signout,
        ])
        .build(tauri::generate_context!())
        .expect("error building tauri application");

    app.run(move |_app, event| {
        match event {
            RunEvent::Exit => {
                // Best-effort: SIGKILL every running chan child so
                // they don't outlive the desktop. The OS reclaims
                // the ports within seconds.
                serve::stop_all(&state_for_exit);
                // Cancel the tunnel listener (if active) and every
                // per-tenant listener. Tasks exit when their cancel
                // token fires; the process is on its way out, so we
                // don't await them.
                tunnel::shutdown(&state_for_exit.tunnel);
            }
            // macOS: Dock click or `open -a` while the process is
            // still alive. If no windows are visible (main has been
            // hidden / closed and the user has no drive windows
            // open), bring the main window back.
            #[cfg(target_os = "macos")]
            RunEvent::Reopen {
                has_visible_windows: false,
                ..
            } => {
                let _ = show_window(_app, "main");
            }
            _ => {}
        }
    });
}

/// Inject window-navigation items into the default Tauri menu.
/// Tauri's `Menu::default` produces the standard macOS menubar
/// (app / File / Edit / View / Window / Help) but its Window
/// submenu only has Minimize / Zoom / Close — a closed main
/// window has no menu path back. We prepend Drives, Settings,
/// and Logs items to that submenu so each app window is
/// reachable by name.
///
/// Settings has Cmd+, but no chan-desktop-owned UI behind it:
/// chan owns the Settings concept per-drive. The handler dispatches
/// `app.settings.toggle` into the focused drive webview, where
/// chan's `runCommand` opens its settings overlay. Cmd+, with the
/// Drives window focused is a no-op.
fn install_app_menu(app: &tauri::AppHandle) -> tauri::Result<()> {
    let menu = Menu::default(app)?;

    // Drives keeps no accelerator: Cmd+1..9 is reserved for
    // jump-to-tab in drive windows (handled by the per-drive key
    // bridge script in serve.rs). The menu entry still surfaces the
    // window by name.
    let drive_manager = MenuItemBuilder::with_id("win-main", "Drives").build(app)?;
    // `fullstack-83`: Cmd+N spawns a fresh launcher window. The
    // existing "main" window stays untouched (singleton label);
    // additional launchers land on `main-<N>` so each carries its
    // own state independently. Convention for future chan-desktop
    // shortcuts: declare a MenuItemBuilder here with the
    // `CmdOrCtrl+<key>` accelerator, prepend into the Window
    // submenu below, and add a matching `on_menu_event` branch.
    // `fullstack-b-27`: moved from `CmdOrCtrl+N` to
    // `CmdOrCtrl+Shift+N` so the SPA's New Draft handler (per
    // `fullstack-a-66`) can claim plain Cmd+N without the menu
    // accelerator intercepting first. Menu label stays
    // "New Window"; only the chord moves.
    let new_window = MenuItemBuilder::with_id("app-new-window", "New Window")
        .accelerator("CmdOrCtrl+Shift+N")
        .build(app)?;
    let settings = MenuItemBuilder::with_id("chan-settings", "Settings…")
        .accelerator("CmdOrCtrl+,")
        .build(app)?;

    if let Some(window_submenu) = menu
        .get(WINDOW_SUBMENU_ID)
        .and_then(|k| k.as_submenu().cloned())
    {
        let sep = PredefinedMenuItem::separator(app)?;
        window_submenu.prepend_items(&[&drive_manager, &new_window, &settings, &sep])?;
        // Strip the default "Close Window" item so Cmd+W reaches the
        // drive webview's key bridge (which dispatches `app.tab.close`
        // to chan). The trade-off: non-drive windows (main, console)
        // lose their Cmd+W shortcut — closing them is still possible
        // via the red traffic light. Match by text since muda assigns
        // predefined items an opaque generated id.
        if let Ok(items) = window_submenu.items() {
            for item in items {
                if let MenuItemKind::Predefined(p) = &item {
                    if let Ok(text) = p.text() {
                        if text.to_lowercase().contains("close") {
                            let _ = window_submenu.remove(&item);
                        }
                    }
                }
            }
        }
    }

    app.set_menu(menu)?;
    app.on_menu_event(|app, event| match event.id().as_ref() {
        "win-main" => {
            let _ = show_window(app, "main");
        }
        "app-new-window" => {
            if let Err(e) = open_new_launcher_window(app) {
                tracing::warn!(error = %e, "open new launcher window failed");
            }
        }
        "chan-settings" => {
            dispatch_to_focused_drive(app, "app.settings.toggle");
        }
        _ => {}
    });
    Ok(())
}

/// `fullstack-83`: spawn a fresh launcher (drive-picker) window via
/// `WebviewWindowBuilder`. The label is picked from the next free
/// `main-N` slot so each launcher carries its own per-window state
/// (mirrors the `drive-N` / `tunnel-N` convention). New windows use
/// the same `index.html` entry as the singleton `main`, so the
/// SPA's `boot()` path runs and the user lands on the drive
/// picker — never inheriting any existing launcher's runtime
/// state.
fn open_new_launcher_window(app: &tauri::AppHandle) -> Result<(), String> {
    let label = next_launcher_label(app);
    if app.get_webview_window(&label).is_some() {
        // Defensive: the slot picker scans existing windows so a
        // collision shouldn't happen. If it ever does, surface a
        // clear error rather than panicking on `build`.
        return Err(format!("launcher label {label} already exists"));
    }
    WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
        .title("Chan Desktop")
        .inner_size(960.0, 600.0)
        .min_inner_size(720.0, 400.0)
        .resizable(true)
        .build()
        .map_err(|e| format!("building launcher window {label}: {e}"))?;
    Ok(())
}

/// Pick the next free `main-N` label. Launchers spawn from the
/// File → New Window menu item; the singleton `main` from
/// tauri.conf.json keeps its bare label so existing
/// `show_window(app, "main")` callers and the `Drives` menu
/// entry keep working.
fn next_launcher_label(app: &tauri::AppHandle) -> String {
    let existing: std::collections::HashSet<String> = app
        .webview_windows()
        .into_keys()
        .filter(|l| l == "main" || l.starts_with("main-"))
        .collect();
    for n in 2u32..u32::MAX {
        let candidate = format!("main-{n}");
        if !existing.contains(&candidate) {
            return candidate;
        }
    }
    // Practically unreachable; falls back to a UUID-ish suffix so
    // the menu action still does *something* if a hostile loop
    // saturates the integer range.
    format!(
        "main-{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    )
}

/// Eval a `chan:command` dispatch on the currently-focused drive
/// webview. Used by menu items that should defer to chan's per-drive
/// behavior (Settings). No-op when the focused window isn't a drive,
/// matching the "each window owns its own settings" model.
fn dispatch_to_focused_drive(app: &tauri::AppHandle, command: &str) {
    let Some(w) = app
        .webview_windows()
        .into_values()
        .find(|w| w.label().starts_with("drive-") && w.is_focused().unwrap_or(false))
    else {
        return;
    };
    let js = format!(
        "window.dispatchEvent(new CustomEvent('chan:command', {{detail: {{name: {}}}}}));",
        serde_json::to_string(command).unwrap_or_else(|_| "\"\"".into())
    );
    let _ = w.eval(&js);
}
