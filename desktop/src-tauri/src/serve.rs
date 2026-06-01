//! Local-workspace runtime and workspace-window helpers.
//!
//! chan-desktop opens local workspaces through the embedded chan-server
//! `WorkspaceHost`. Each running workspace is tracked in `AppState.serves`
//! with its route prefix and token-bearing URL. chan-desktop links
//! `chan-workspace` and `chan-server` directly; there is no `chan`
//! binary at runtime. Registry mutations and feature toggles run
//! in-process against the embedded host's shared `Library`, and
//! local serving never spawns `chan serve`.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Per-process monotonic counter appended to every workspace-window
/// label so the user can open more than one window for the same
/// workspace (local or tunneled). Tauri requires unique window labels
/// per process; the prefix encodes the workspace identity and the seq
/// disambiguates instances.
static WINDOW_SEQ: AtomicU64 = AtomicU64::new(0);

fn next_window_seq() -> u64 {
    WINDOW_SEQ.fetch_add(1, Ordering::Relaxed)
}

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::config::{self, WindowConfig};
use crate::AppState;

/// Tauri event emitted when any local runtime starts or stops. The
/// frontend reacts by re-fetching the workspace list.
pub const SERVES_CHANGED: &str = "serves-changed";

const MAX_WINDOWS_PER_WORKSPACE: usize = 10;

/// Live state for one running serve. Held in `AppState.serves`
/// keyed by canonical workspace path.
pub struct ServeHandle {
    prefix: String,
    pub url: Option<String>,
}

impl ServeHandle {
    fn embedded(prefix: String, url: String) -> Self {
        Self {
            prefix,
            url: Some(url),
        }
    }
}

/// Open a local workspace through the embedded chan-server host.
pub async fn start(app: AppHandle, state: Arc<AppState>, key: String) -> Result<(), String> {
    if state.serves.lock().unwrap().contains_key(&key) {
        return Ok(());
    }
    let Some(embedded) = state.embedded.get() else {
        return Err("embedded local server is unavailable".to_string());
    };
    let url = embedded.open_workspace(&key).await?;
    let prefix = url_prefix_from_local_url(&url)?;
    {
        let mut serves = state.serves.lock().unwrap();
        if serves.contains_key(&key) {
            drop(serves);
            if let Err(e) = embedded.close_prefix(&prefix) {
                tracing::warn!(key = %key, error = %e, "closing duplicate embedded workspace failed");
            }
            return Ok(());
        }
        serves.insert(key.clone(), ServeHandle::embedded(prefix, url.clone()));
    }
    let _ = app.emit(SERVES_CHANGED, ());
    if let Err(e) = spawn_local_workspace_window(&app, &key, &url) {
        if let Some(handle) = state.serves.lock().unwrap().remove(&key) {
            stop_handle(None, &state, &key, handle);
        }
        let _ = app.emit(SERVES_CHANGED, ());
        return Err(e);
    }
    Ok(())
}

fn url_prefix_from_local_url(url: &str) -> Result<String, String> {
    let parsed = url
        .parse::<url::Url>()
        .map_err(|e| format!("parsing embedded workspace URL: {e}"))?;
    let path = parsed.path().trim_end_matches('/');
    let path = path.strip_suffix("/index.html").unwrap_or(path);
    if path.is_empty() {
        Ok(String::new())
    } else {
        Ok(path.to_string())
    }
}

/// Stop a running serve. No-op if the workspace isn't running. Removes
/// the live entry before waiting so an immediate stop -> start can
/// mount a fresh runtime instead of observing stale map state.
pub fn stop(app: Option<&AppHandle>, state: &AppState, key: &str) {
    let handle = state.serves.lock().unwrap().remove(key);
    if let Some(h) = handle {
        stop_handle(app, state, key, h);
    }
}

/// Stop every running serve. Called from the Tauri Exit hook so
/// embedded workspace state shuts down before the desktop exits.
pub fn stop_all(state: &AppState) {
    let handles: Vec<(String, ServeHandle)> = state.serves.lock().unwrap().drain().collect();
    for (key, h) in handles {
        stop_handle(None, state, &key, h);
    }
}

fn stop_handle(app: Option<&AppHandle>, state: &AppState, key: &str, handle: ServeHandle) {
    if let Some(embedded) = state.embedded.get() {
        if let Err(e) = embedded.close_prefix(&handle.prefix) {
            tracing::warn!(key = %key, error = %e, "closing embedded workspace failed");
        }
    }
    if let Some(app) = app {
        close_local_workspace_windows(app, key);
        let _ = app.emit(SERVES_CHANGED, ());
    }
}

/// Stable Tauri window-label prefix for a local workspace. Used to
/// recognise every window that belongs to the workspace when the user
/// has opened more than one (close-all on serve exit, capability
/// matching). Tauri labels must match `[a-zA-Z0-9_-]+`, and workspace
/// keys are filesystem paths, so we hash the key.
pub fn workspace_window_prefix(key: &str) -> String {
    let mut h = DefaultHasher::new();
    key.hash(&mut h);
    format!("workspace-{:016x}", h.finish())
}

/// Fresh, unique window label for a new local-workspace webview.
/// Every call yields a distinct label so multi-window works; the
/// prefix is still identifiable for cleanup. Format:
/// `workspace-<hash>-<seq>` where `seq` is a per-process atomic.
pub fn new_workspace_window_label(key: &str) -> String {
    format!("{}-{}", workspace_window_prefix(key), next_window_seq())
}

/// Window title for a local-workspace webview: the workspace path verbatim.
/// `fullstack-b-14` swapped the earlier "chan workspace: <basename>"
/// shape after @@Alex flagged that the path is the more useful
/// signal in the OS window switcher than the prefix + basename.
fn workspace_title(key: &str) -> String {
    key.to_string()
}

/// Stable window-label prefix for a tunneled workspace, namespaced
/// separately from `workspace-*` so a local workspace path and a tunneled
/// workspace slug don't collide.
pub fn tunnel_window_prefix(tenant_label: &str, workspace: &str) -> String {
    let mut h = DefaultHasher::new();
    tenant_label.hash(&mut h);
    workspace.hash(&mut h);
    format!("tunnel-{:016x}", h.finish())
}

/// Fresh, unique window label for a tunneled workspace webview. Same
/// shape as `new_workspace_window_label`.
pub fn new_tunnel_window_label(tenant_label: &str, workspace: &str) -> String {
    format!(
        "{}-{}",
        tunnel_window_prefix(tenant_label, workspace),
        next_window_seq()
    )
}

/// Stable window-label prefix for an outbound URL attachment.
pub fn outbound_window_prefix(id: &str) -> String {
    let mut h = DefaultHasher::new();
    id.hash(&mut h);
    format!("outbound-{:016x}", h.finish())
}

/// Fresh, unique window label for an outbound URL webview.
pub fn new_outbound_window_label(id: &str) -> String {
    format!("{}-{}", outbound_window_prefix(id), next_window_seq())
}

/// True when a Tauri label belongs to a per-workspace webview.
pub fn is_workspace_webview_label(label: &str) -> bool {
    label.starts_with("workspace-")
        || label.starts_with("tunnel-")
        || label.starts_with("outbound-")
}

/// Spawn a new local-workspace webview window pointing at `url`. Each
/// call opens an independent window; multiple windows per workspace are
/// supported. Pops the most-recent WindowConfig for this workspace (if
/// any) so the new window reuses the previous `?w=<label>` and URL
/// hash, restoring panes / tabs (via `session.json`) and overlay
/// state across the close/reopen cycle. A user-initiated close
/// pushes the closing window's state back to the stack so the next
/// open repeats the restore. The Tauri close handler does NOT stop
/// the underlying local runtime; the On toggle (plus
/// `close_local_workspace_windows` on runtime teardown) remains the single
/// authority on workspace lifecycle.
pub fn spawn_local_workspace_window(app: &AppHandle, key: &str, url: &str) -> Result<(), String> {
    ensure_window_capacity(app, &workspace_window_prefix(key))?;
    let config_key = config::local_window_key(key);
    let restore = pop_compatible_config(app, &config_key, &workspace_window_prefix(key));
    let label = match restore.as_ref() {
        Some(c) => c.window_label.clone(),
        None => new_workspace_window_label(key),
    };
    let url_hash = restore
        .as_ref()
        .map(|c| c.url_hash.clone())
        .unwrap_or_default();
    let zoom_level = restore.as_ref().map(|c| c.zoom_level).unwrap_or(1.0);
    let title = workspace_title(key);
    build_workspace_window(app, &label, &title, url, &url_hash, config_key, zoom_level)
}

/// Spawn a new tunneled-workspace webview window. Same multi-window
/// semantics and config-stack restore as the local variant.
pub fn spawn_tunneled_workspace_window(
    app: &AppHandle,
    tenant_label: &str,
    workspace: &str,
    url: &str,
) -> Result<(), String> {
    ensure_window_capacity(app, &tunnel_window_prefix(tenant_label, workspace))?;
    let config_key = config::tunnel_window_key(tenant_label, workspace);
    let prefix = tunnel_window_prefix(tenant_label, workspace);
    let restore = pop_compatible_config(app, &config_key, &prefix);
    let label = match restore.as_ref() {
        Some(c) => c.window_label.clone(),
        None => new_tunnel_window_label(tenant_label, workspace),
    };
    let url_hash = restore
        .as_ref()
        .map(|c| c.url_hash.clone())
        .unwrap_or_default();
    let zoom_level = restore.as_ref().map(|c| c.zoom_level).unwrap_or(1.0);
    // `fullstack-b-14`: matches the local-workspace title shape; the
    // tunneled workspace has no local filesystem path, so we use the
    // closest analog ("<tenant>·<workspace>") with no prefix.
    let title = format!("{tenant_label} \u{00b7} {workspace}");
    build_workspace_window(app, &label, &title, url, &url_hash, config_key, zoom_level)
}

/// Spawn a new outbound URL webview window. The desktop does not own
/// the remote process; this only creates another webview pointed at
/// the persisted URL.
pub fn spawn_outbound_workspace_window(
    app: &AppHandle,
    id: &str,
    title: &str,
    url: &str,
) -> Result<(), String> {
    ensure_window_capacity(app, &outbound_window_prefix(id))?;
    let config_key = config::outbound_window_key(id);
    let prefix = outbound_window_prefix(id);
    let restore = pop_compatible_config(app, &config_key, &prefix);
    let label = match restore.as_ref() {
        Some(c) => c.window_label.clone(),
        None => new_outbound_window_label(id),
    };
    let url_hash = restore
        .as_ref()
        .map(|c| c.url_hash.clone())
        .unwrap_or_default();
    let zoom_level = restore.as_ref().map(|c| c.zoom_level).unwrap_or(1.0);
    build_workspace_window(app, &label, title, url, &url_hash, config_key, zoom_level)
}

/// Pop the top-of-stack window config for `config_key` only if the
/// stored label is safe to reuse. The label must still match the
/// workspace's current hash prefix (defends against the workspace key
/// changing canonicalisation under us) and must not already be
/// live in this process (Tauri requires unique labels per
/// process). When the popped entry fails either check, it gets
/// dropped on the floor; we don't keep cycling through stale
/// stack entries trying to find a usable one, since the next
/// close will push a fresh entry anyway.
fn pop_compatible_config(
    app: &AppHandle,
    config_key: &str,
    expected_prefix: &str,
) -> Option<WindowConfig> {
    let state = app.state::<Arc<AppState>>();
    let entry = state.pop_window_config(config_key)?;
    if !entry.window_label.starts_with(expected_prefix) {
        tracing::debug!(
            label = %entry.window_label,
            prefix = %expected_prefix,
            "discarding window config with stale prefix",
        );
        return None;
    }
    if app.get_webview_window(&entry.window_label).is_some() {
        tracing::debug!(
            label = %entry.window_label,
            "discarding window config; label still live",
        );
        return None;
    }
    Some(entry)
}

/// Build and show a chan-style workspace webview window on the main
/// thread. Internal: call `spawn_local_workspace_window` /
/// `spawn_tunneled_workspace_window` / `spawn_outbound_workspace_window`
/// from outside. Centralising the
/// key-bridge JS, the size defaults, the zoom-hotkey polyfill, and
/// the drag-drop handler off in one place means workspace UX changes
/// don't fork between the local and tunneled paths.
///
/// `url_hash_seed` carries any popped URL hash from the
/// window-config stack: applied verbatim to the URL fragment so
/// overlay state (file browser path, search query, graph scope)
/// restores alongside the panes/tabs that come back from
/// `session.json`. Empty when there's nothing to restore.
///
/// `config_key` is the WindowConfig identity key (`local_window_key`
/// or `tunnel_window_key`). Stamped onto the close handler so a
/// user-initiated close pushes the window's final URL hash back
/// into the LRU stack.
fn build_workspace_window(
    app: &AppHandle,
    window_label: &str,
    title: &str,
    url: &str,
    url_hash_seed: &str,
    config_key: String,
    zoom_seed: f64,
) -> Result<(), String> {
    let Ok(mut parsed) = url.parse::<tauri::Url>() else {
        return Err(format!("bad chan URL for {window_label}: {url}"));
    };
    parsed.query_pairs_mut().append_pair("w", window_label);
    if !url_hash_seed.is_empty() {
        parsed.set_fragment(Some(url_hash_seed));
    }
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
            // `fullstack-b-19`: explicit `zoom_in` / `zoom_out` /
            // `zoom_reset` IPC commands fired from KEY_BRIDGE_JS
            // are the primary path; this Tauri-level polyfill stays
            // on as a mousewheel + pinch fallback (the chord
            // overlap is harmless because KEY_BRIDGE_JS's capture-
            // phase listener calls preventDefault before the
            // polyfill's bubble-phase listener sees the keydown).
            // Requires `core:webview:allow-set-webview-zoom` on
            // workspace-* / tunnel-* / outbound-* windows per
            // capabilities/workspace.json.
            .zoom_hotkeys_enabled(true)
            // Hand HTML5 drag-and-drop to the page. Tauri's OS-level
            // drag handler swallows dragover events otherwise, so
            // chan's pane-to-pane tab moves never see the highlight /
            // drop the receiving pane expects.
            .disable_drag_drop_handler()
            .build()
        {
            Ok(window) => {
                // `fullstack-b-19`: restore the persisted zoom level from
                // the popped WindowConfig (if any). 1.0 is the chan-
                // desktop default; skip the IPC round-trip when there's
                // nothing to apply. Best-effort: a Tauri set_zoom error
                // here just leaves the new window at default zoom; the
                // user can re-press Cmd++/Cmd+- to recover.
                if (zoom_seed - 1.0).abs() > f64::EPSILON {
                    if let Err(e) = window.set_zoom(zoom_seed) {
                        tracing::warn!(
                            label = %label_owned,
                            error = %e,
                            "restoring window zoom level failed",
                        );
                    } else {
                        let state = app_owned.state::<Arc<AppState>>();
                        state
                            .live_window_zooms
                            .lock()
                            .unwrap()
                            .insert(label_owned.clone(), zoom_seed);
                    }
                }
                let app_for_close = app_owned.clone();
                let label_for_close = label_owned.clone();
                let key_for_close = config_key.clone();
                window.on_window_event(move |event| {
                    if matches!(event, WindowEvent::CloseRequested { .. }) {
                        capture_window_config_on_close(
                            &app_for_close,
                            &label_for_close,
                            &key_for_close,
                        );
                    }
                });
            }
            Err(e) => {
                tracing::warn!(label = %label_owned, error = %e, "opening workspace window failed")
            }
        }
    });
    res.map_err(|e| format!("scheduling workspace window for {window_label}: {e}"))
}

/// Snapshot the closing window's URL hash and push the resulting
/// WindowConfig onto the LRU stack. Best-effort: a webview that's
/// already torn down reports no URL and we skip the push. The
/// hash is read from `WebviewWindow::url()` because the webview
/// SPA writes the latest state to `location.hash` via
/// `persistStateToHash`, and Tauri's URL reflection picks that up
/// on platforms with the WKWebView / WebView2 backends.
///
/// `fullstack-b-19`: also drains the live zoom level for this
/// window into `WindowConfig.zoom_level` so the next open of the
/// same workspace restores the zoom.
fn capture_window_config_on_close(app: &AppHandle, window_label: &str, config_key: &str) {
    let Some(window) = app.get_webview_window(window_label) else {
        return;
    };
    // Reading the URL hash is best-effort and must never crash the app on a
    // window close. Two nil-URL failure modes trip a panic deep in the runtime
    // (a nil/empty webview URL fails tauri-runtime-wry's `.parse().expect()` /
    // wry's `.URL().unwrap()`); that panic runs on the event-loop thread and
    // takes the WHOLE app down. The chan-side `match` below cannot catch it
    // because the panic is upstream of the returned `Result`.
    //   - Outbound windows point at a remote we do not own; when that remote is
    //     down the WKWebView never finishes navigating and reports a nil URL.
    //     The hash is chan-SPA restore state, meaningless for an outbound
    //     remote, so skip the read entirely (no url() call, no panic).
    //   - A local/tunnel window whose backend died before close can hit the
    //     same nil-URL panic, so guard that read with catch_unwind (the release
    //     profile unwinds, so this is catchable) and degrade to an empty hash.
    let url_hash = if window_label.starts_with("outbound-") {
        String::new()
    } else {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| window.url())) {
            Ok(Ok(u)) => u.fragment().unwrap_or("").to_string(),
            Ok(Err(e)) => {
                tracing::debug!(
                    label = %window_label,
                    error = %e,
                    "could not read url for closing window; pushing empty hash",
                );
                String::new()
            }
            Err(_) => {
                tracing::warn!(
                    label = %window_label,
                    "reading url for a closing window panicked (dead webview); pushing empty hash",
                );
                String::new()
            }
        }
    };
    let state = app.state::<Arc<AppState>>();
    let zoom_level = state
        .live_window_zooms
        .lock()
        .unwrap()
        .remove(window_label)
        .unwrap_or(1.0);
    state.push_window_config(WindowConfig {
        key: config_key.to_string(),
        window_label: window_label.to_string(),
        url_hash,
        zoom_level,
        saved_at: 0,
    });
}

fn ensure_window_capacity(app: &AppHandle, prefix: &str) -> Result<(), String> {
    let count = app
        .webview_windows()
        .keys()
        .filter(|label| label.starts_with(prefix))
        .count();
    if count >= MAX_WINDOWS_PER_WORKSPACE {
        return Err(format!(
            "Workspace already has {MAX_WINDOWS_PER_WORKSPACE} open windows; close one before opening another."
        ));
    }
    Ok(())
}

/// Destroy every webview window opened for this local workspace when
/// the local runtime is closed. Walks `webview_windows()` and
/// matches by prefix because the user may have opened several
/// windows for the same workspace.
pub fn close_local_workspace_windows(app: &AppHandle, key: &str) {
    close_windows_with_prefix(app, &workspace_window_prefix(key))
}

/// Destroy every webview window opened for this tunneled workspace.
/// Used by the tunnel supervisor when a (label, workspace) pair drops
/// out of the registry; the remote has gone away, so the per-tenant
/// listener no longer routes for it and any open window now points
/// at nothing useful.
pub fn close_tunneled_workspace_windows(app: &AppHandle, tenant_label: &str, workspace: &str) {
    close_windows_with_prefix(app, &tunnel_window_prefix(tenant_label, workspace))
}

/// Destroy every webview window opened for this outbound URL
/// attachment. Used when the user forgets the attachment row.
pub fn close_outbound_workspace_windows(app: &AppHandle, id: &str) {
    close_windows_with_prefix(app, &outbound_window_prefix(id))
}

/// Destroy every tunneled-workspace webview window in the process,
/// regardless of which (label, workspace) it belongs to. Used by the
/// tunnel module on `stop_listening`: the tunnel listener and
/// every per-tenant listener are about to be cancelled, so the
/// open windows would all error on their next request anyway.
pub fn close_all_tunneled_workspace_windows(app: &AppHandle) {
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

/// Native keyboard shortcuts for workspace webviews. Translates chords
/// into the host-agnostic `chan:command` window event that chan's
/// App.svelte listens for. Runs before any page script, in capture
/// phase with stopImmediatePropagation, so this script is the sole
/// authority on every chord it claims, so chan's onWindowKey doesn't
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
  // `fullstack-b-17`: Cmd+R reloads the webview, Cmd+Opt+I opens
  // DevTools. Both bypass the SPA event bus and invoke their
  // Tauri IPC commands directly so a frozen Svelte runtime or a
  // broken chord registry can't lock the dev affordances away.
  function invokeIpc(e, cmd) {
    e.preventDefault();
    e.stopImmediatePropagation();
    const tauri = window.__TAURI__;
    if (tauri && tauri.core && typeof tauri.core.invoke === 'function') {
      tauri.core.invoke(cmd).catch((err) => {
        console.error('[chan] IPC ' + cmd + ' failed:', err);
      });
    }
  }
  // `fullstack-42` pruned every native chord whose action is now
  // covered by Pane Mode (Cmd+K). Dropped: Cmd+P, Cmd+N, Cmd+`,
  // Cmd+[/Cmd+], Cmd+Shift+M, Cmd+Shift+F. Kept: Cmd+W (close
  // tab; pairs with Ctrl+D from fullstack-41), Cmd+F/G (find on page),
  // Cmd+1..9 (jump to tab), Cmd+Shift+T (reopen closed),
  // Cmd+Shift+[/] (tab nav), Cmd+Shift+G (find prev).
  // `fullstack-b-2`: Cmd+T comes back as a direct chord for
  // "new terminal in active pane".
  // `fullstack-a-32`: Cmd+O / Cmd+P / Cmd+Shift+M added as direct
  // chords for File Browser / Team Work / Graph (with the
  // matching `app.files.toggle` / `app.terminal.teamWork` /
  // `app.graph.toggle` commands routed through the context-aware
  // helpers in App.svelte). Universal Hybrid NAV `t/o/p/v` covers
  // the web/Win/Linux fallback path.
  function onKey(e) {
    const meta = e.metaKey || e.ctrlKey;
    if (!meta) return;
    const shift = e.shiftKey;
    const alt = e.altKey;
    const code = e.code;
    if (alt) {
      // Cmd+Opt+I (macOS) / Ctrl+Alt+I (Linux/Windows) → DevTools.
      // No other meta+alt chord today; bail out for everything else
      // so we don't shadow the webview's defaults.
      if (!shift && code === 'KeyI') {
        invokeIpc(e, 'open_devtools');
      }
      return;
    }
    // `fullstack-b-19`: zoom chords route regardless of shift so
    // Cmd+= (US) and Cmd+Shift+= (= Cmd++) both fire zoom_in.
    // NumpadAdd / NumpadSubtract similarly. Cmd+0 / Cmd+Numpad0
    // reset to 100 %.
    switch (code) {
      case 'Equal':
      case 'NumpadAdd':
        invokeIpc(e, 'zoom_in');
        return;
      case 'Minus':
      case 'NumpadSubtract':
        invokeIpc(e, 'zoom_out');
        return;
      case 'Digit0':
      case 'Numpad0':
        invokeIpc(e, 'zoom_reset');
        return;
    }
    if (!shift) {
      switch (code) {
        // Reload. macOS binds Cmd+R (metaKey); Linux/Windows moves to
        // Ctrl+Shift+R (shift branch below) so plain Ctrl+R reaches a
        // focused terminal's shell reverse-search. Gating on metaKey
        // here leaves Linux/macOS plain Ctrl+R untouched (no
        // preventDefault -> falls through to xterm), mirroring the
        // Cmd+W idiom below.
        case 'KeyR': if (e.metaKey) invokeIpc(e, 'reload_window'); return;
        case 'KeyT': fire(e, 'app.terminal.toggle'); return;
        case 'KeyO': fire(e, 'app.files.toggle');    return;
        case 'KeyP': fire(e, 'app.terminal.teamWork'); return;
        // `phase-12 lane-e` (addendum-2 Q6/Q7): Cmd+W closes the tab
        // on macOS. On Linux the platform mod is Ctrl and Ctrl+W is
        // readline delete-word inside a focused terminal, so DON'T
        // claim it - let it reach xterm. Linux closes tabs with Ctrl+D
        // (context-aware via the SPA's onCtrlDCapture, which leaves a
        // focused terminal to its EOF). Gating on metaKey (Cmd) leaves
        // Linux Ctrl+W untouched (no preventDefault -> reaches xterm).
        case 'KeyW': if (e.metaKey) fire(e, 'app.tab.close'); return;
        case 'KeyS': fire(e, 'app.search.toggle');    return;
        case 'KeyF': fire(e, 'app.find.open');        return;
        case 'KeyG': fire(e, 'app.find.next');        return;
        // `phase-13 r2` (B-slice 2): Cmd+I no longer opens Dashboard.
        // @@Alex freed it for the editor's italic chord (bound in
        // Wysiwyg.svelte's CM6 keymap); Dashboard is reachable via
        // Hybrid Nav `Cmd+. i` + the Dashboard hamburger. With no
        // `KeyI` case here, Cmd+I falls through to the focused webview
        // (the editor toggles italic; otherwise inert). Cmd+Opt+I
        // (DevTools, alt branch above) and Cmd+Shift+I (broadcast,
        // shift branch below) are unaffected.
        case 'BracketLeft':  fire(e, 'app.pane.prev'); return;
        case 'BracketRight': fire(e, 'app.pane.next'); return;
        // `phase-12 lane-e` (addendum-2): Cmd+/ split right. Split
        // bottom is Cmd+Shift+/ (shift branch below) - moved off Cmd+\
        // in `desktop-fixes` because 1Password's system-wide Cmd+\
        // hotkey is dispatched by macOS before the key reaches this
        // webview, so chan never received it. Web reaches splits via
        // Hybrid Nav `/` and `?`.
        case 'Slash':        fire(e, 'app.pane.splitRight'); return;
      }
      const m = code.match(/^Digit([1-9])$/);
      if (m) {
        fire(e, 'app.tab.jump', { index: Number(m[1]) - 1 });
        return;
      }
    } else {
      switch (code) {
        // Reload on Linux/Windows: Ctrl+Shift+R. Gate on !metaKey so
        // macOS Cmd+Shift+R does NOT reload (macOS reloads on Cmd+R in
        // the !shift branch above); the !metaKey form fires only for the
        // Ctrl+Shift+R that Linux/Windows users press.
        case 'KeyR': if (!e.metaKey) invokeIpc(e, 'reload_window'); return;
        case 'KeyG':         fire(e, 'app.find.prev');     return;
        case 'KeyT':         fire(e, 'app.tab.reopenClosed'); return;
        case 'KeyM':         fire(e, 'app.graph.toggle');  return;
        // `lane-c addendum-3`: Cmd+Shift+I toggles broadcast-input
        // select-all/deselect-all for the active terminal (mirrors
        // iTerm). macOS ONLY: gate on metaKey so Linux Ctrl+Shift+I
        // stays the webview DevTools chord (web has no binding at all -
        // cmd+shift+i is the browser DevTools there too).
        case 'KeyI': if (e.metaKey) fire(e, 'app.terminal.broadcastToggle'); return;
        case 'BracketLeft':  fire(e, 'app.tab.prev');      return;
        case 'BracketRight': fire(e, 'app.tab.next');      return;
        // `desktop-fixes`: Cmd+Shift+/ (= Cmd+?) splits the active pane
        // bottom, pairing with Cmd+/ split-right above. Replaces the old
        // Cmd+\ binding that 1Password's global hotkey shadowed.
        case 'Slash':        fire(e, 'app.pane.splitDown');  return;
      }
    }
  }
  window.addEventListener('keydown', onKey, true);
})();
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoke_handler_registers_reload_window_and_open_devtools() {
        // `fullstack-b-17`: the IPC commands `reload_window` and
        // `open_devtools` MUST be in the `tauri::generate_handler!`
        // list so the SPA's tab context-menu (via -a-36) and the
        // accelerator path can reach them. The generate_handler!
        // macro does not catch a missing handler at compile time,
        // so we pin it here against the source file. Tests live in
        // serve.rs because main.rs has no test module today; using
        // `include_str!` keeps the pin source-of-truth-correct.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(MAIN_RS.contains("reload_window,"));
        assert!(MAIN_RS.contains("open_devtools,"));
        assert!(MAIN_RS.contains("fn reload_window(window: tauri::WebviewWindow)"));
        assert!(MAIN_RS.contains("fn open_devtools(window: tauri::WebviewWindow)"));
    }

    #[test]
    fn key_bridge_wires_zoom_chords_to_ipc() {
        // `fullstack-b-19`: Cmd+= / Cmd+- / Cmd+0 (and their
        // Numpad variants) route directly to the chan-desktop
        // zoom IPC commands. Routed BEFORE the shift branch so
        // Cmd+Shift+= (= Cmd++) also zooms in. Capture-phase
        // listener stops the keydown so Tauri's `zoom_hotkeys_enabled`
        // polyfill (still on as a mousewheel + pinch fallback)
        // doesn't double-fire.
        assert!(KEY_BRIDGE_JS.contains("invokeIpc(e, 'zoom_in')"));
        assert!(KEY_BRIDGE_JS.contains("invokeIpc(e, 'zoom_out')"));
        assert!(KEY_BRIDGE_JS.contains("invokeIpc(e, 'zoom_reset')"));
        assert!(KEY_BRIDGE_JS.contains("case 'Equal':"));
        assert!(KEY_BRIDGE_JS.contains("case 'Minus':"));
        assert!(KEY_BRIDGE_JS.contains("case 'Digit0':"));
        assert!(KEY_BRIDGE_JS.contains("case 'NumpadAdd':"));
        assert!(KEY_BRIDGE_JS.contains("case 'NumpadSubtract':"));
        assert!(KEY_BRIDGE_JS.contains("case 'Numpad0':"));
    }

    #[test]
    fn key_bridge_wires_cmd_shift_i_to_broadcast_toggle() {
        // `lane-c addendum-3`: Cmd+Shift+I toggles broadcast-input
        // select-all/deselect-all for the active terminal. macOS ONLY:
        // gated on metaKey so Linux Ctrl+Shift+I stays the DevTools chord
        // (and web has no binding - cmd+shift+i is DevTools there too).
        assert!(KEY_BRIDGE_JS.contains("if (e.metaKey) fire(e, 'app.terminal.broadcastToggle')"));
    }

    #[test]
    fn invoke_handler_registers_zoom_commands() {
        // `fullstack-b-19`: zoom_in / zoom_out / zoom_reset must be
        // in `tauri::generate_handler!` so KEY_BRIDGE_JS's IPC
        // invocations reach a registered command. generate_handler!
        // doesn't catch missing entries at compile time; pin here.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(MAIN_RS.contains("zoom_in,"));
        assert!(MAIN_RS.contains("zoom_out,"));
        assert!(MAIN_RS.contains("zoom_reset,"));
    }

    #[test]
    fn invoke_handler_registers_workspace_features_ipcs() {
        // `fullstack-b-28a`: the launcher's expand panel calls
        // `get_workspace_features` on first open + `set_workspace_features`
        // on every checkbox flip. Pin both sides so a future rename
        // gets caught.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(MAIN_RS.contains("get_workspace_features,"));
        assert!(MAIN_RS.contains("set_workspace_features,"));
        assert!(MAIN_RS.contains("fn get_workspace_features("));
        assert!(MAIN_RS.contains("fn set_workspace_features("));
    }

    #[test]
    fn invoke_handler_registers_outbound_attach_ipcs() {
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(MAIN_RS.contains("add_outbound_workspace,"));
        assert!(MAIN_RS.contains("open_outbound_workspace,"));
        assert!(MAIN_RS.contains("remove_outbound_workspace,"));
        assert!(MAIN_RS.contains("fn add_outbound_workspace("));
        assert!(MAIN_RS.contains("fn open_outbound_workspace("));
        assert!(MAIN_RS.contains("fn remove_outbound_workspace("));
    }

    #[test]
    fn invoke_handler_registers_default_workspace_ipcs() {
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(MAIN_RS.contains("default_workspace_status,"));
        assert!(MAIN_RS.contains("choose_default_workspace,"));
        assert!(MAIN_RS.contains("create_default_workspace,"));
        assert!(MAIN_RS.contains("factory_reset_default_workspace,"));
        assert!(MAIN_RS.contains("fn default_workspace_status("));
        assert!(MAIN_RS.contains("fn choose_default_workspace("));
        assert!(MAIN_RS.contains("fn create_default_workspace("));
        assert!(MAIN_RS.contains("fn factory_reset_default_workspace("));
    }

    #[test]
    fn pick_and_add_shows_preflight_dialog_before_add_workspace() {
        // `fullstack-b-28b` slice iii: pickAndAdd MUST gate the
        // add_workspace invocation behind the pre-flight modal so the
        // user always sees the round-2-plan explanatory copy +
        // the feature toggles BEFORE chan-workspace's BOOT runs.
        const MAIN_JS: &str = include_str!("../../src/main.js");
        assert!(
            MAIN_JS.contains("showPreflightDialog("),
            "main.js must call showPreflightDialog from pickAndAdd",
        );
        assert!(
            MAIN_JS.contains("features: choice.features"),
            "main.js must thread the pre-flight choice through to add_workspace",
        );
    }

    #[test]
    fn preflight_dialog_carries_round2_plan_explanatory_copy() {
        // `fullstack-b-28b` slice iii: the round-2-plan flagged
        // the explanatory copy as "load-bearing"; @@Alex wants
        // users to understand the baseline before they choose
        // what to layer on". Pin the load-bearing phrases so a
        // future refactor can't silently drop them.
        const MAIN_JS: &str = include_str!("../../src/main.js");
        assert!(
            MAIN_JS.contains("BM25 keyword search is"),
            "preflight modal must explain the BM25 baseline",
        );
        assert!(
            MAIN_JS.contains("can't be disabled"),
            "preflight modal must explain that the baseline is mandatory",
        );
        assert!(
            MAIN_JS.contains("dense-vector embeddings"),
            "preflight modal must describe semantic search via dense embeddings",
        );
        assert!(
            MAIN_JS.contains("tokei"),
            "preflight modal must name tokei as the language-detection engine",
        );
        assert!(
            MAIN_JS.contains("COCOMO"),
            "preflight modal must name COCOMO as the estimate model",
        );
    }

    #[test]
    fn invoke_handler_registers_compute_workspace_preflight() {
        // `fullstack-b-28b` slice iv: the pre-flight modal calls
        // `compute_workspace_preflight` after mount to populate the
        // report rows. Mirrors the other IPC registration pins
        // so a rename catches deliberately.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(MAIN_RS.contains("compute_workspace_preflight,"));
        assert!(MAIN_RS.contains("fn compute_workspace_preflight("));
    }

    #[test]
    fn preflight_modal_renders_report_rows_after_b28b_iv() {
        // `fullstack-b-28b` slice iv: the modal kicks off
        // `compute_workspace_preflight` after mount and renders the
        // returned facts via `renderPreflightReport`. Pin both
        // the invoke + the renderer + the load-bearing report
        // labels so a future refactor can't silently revert to
        // the slice-iii "toggles only" shape.
        const MAIN_JS: &str = include_str!("../../src/main.js");
        assert!(
            MAIN_JS.contains("invoke('compute_workspace_preflight'"),
            "main.js must invoke compute_workspace_preflight from showPreflightDialog",
        );
        assert!(
            MAIN_JS.contains("renderPreflightReport(reportEl, report)"),
            "main.js must render the resolved report into the dialog",
        );
        for label in [
            "'Files'",
            "'Markdown'",
            "'Size'",
            "'Media'",
            "'Source control'",
        ] {
            assert!(
                MAIN_JS.contains(label),
                "preflight modal must surface {label} report row",
            );
        }
    }

    #[test]
    fn registry_and_feature_commands_run_in_process_not_via_chan_cli() {
        // The in-process registry refactor dropped the `chan`
        // binary entirely: `add_workspace`, `remove_workspace`, and the
        // feature commands route through the embedded host's shared
        // `Library` / live `Arc<Workspace>` rather than spawning chan.
        // Pin the in-process call shape so a future change can't
        // silently reintroduce a subprocess dependency, and assert
        // the deleted subprocess argument shapes are gone.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(
            MAIN_RS.contains("embedded.library()"),
            "registry commands must route through the embedded shared Library",
        );
        assert!(
            MAIN_RS.contains("register_workspace") && MAIN_RS.contains("unregister_workspace"),
            "add_workspace/remove_workspace must use Library register/unregister in-process",
        );
        assert!(
            MAIN_RS.contains("set_semantic_enabled") && MAIN_RS.contains("set_reports_enabled"),
            "feature toggles must call chan-workspace set_* in-process",
        );
        assert!(
            !MAIN_RS.contains("read_features_via_chan_index_status"),
            "the `chan index status --json` read path must be gone",
        );
        assert!(
            !MAIN_RS.contains("\"--semantic-search\"") && !MAIN_RS.contains("\"enable-semantic\""),
            "no chan CLI feature-flag arguments may remain",
        );
    }

    #[test]
    fn bin_status_machinery_is_gone() {
        // The bundled-binary preflight + gating was deleted with the
        // subprocess paths. Pin the absence so a future change can't
        // quietly re-add a `chan` binary dependency or its gating.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(
            !MAIN_RS.contains("chan_bin_status"),
            "chan_bin_status command + registration must be gone",
        );
        assert!(
            !MAIN_RS.contains("fn require_bin"),
            "require_bin gating helper must be gone",
        );
        assert!(
            !MAIN_RS.contains("struct BinStatus"),
            "BinStatus struct must be gone",
        );
        // serve.rs must no longer carry the binary resolver. Build
        // the needle at runtime so this assertion's own source text
        // doesn't satisfy the `contains` check it performs.
        let serve_rs = include_str!("serve.rs");
        let resolver_sig = format!("fn resolve{}binary", "_chan_");
        assert!(
            !serve_rs.contains(&resolver_sig),
            "binary resolution helpers must be gone from serve.rs",
        );
    }

    #[test]
    fn launcher_calls_workspace_features_ipcs() {
        // `fullstack-b-28a`: the SPA-side launcher MUST invoke
        // both IPCs so the expand panel reflects + persists
        // toggle state. Pin the invoke names alongside the
        // Rust registration above.
        const MAIN_JS: &str = include_str!("../../src/main.js");
        assert!(
            MAIN_JS.contains("invoke('get_workspace_features'"),
            "main.js must invoke get_workspace_features on panel open"
        );
        assert!(
            MAIN_JS.contains("invoke('set_workspace_features'"),
            "main.js must invoke set_workspace_features on checkbox change"
        );
    }

    #[test]
    fn launcher_prompts_for_existing_user_default_workspace() {
        const MAIN_JS: &str = include_str!("../../src/main.js");
        assert!(
            MAIN_JS.contains("invoke('default_workspace_status'"),
            "launcher must query default-workspace migration status",
        );
        assert!(
            MAIN_JS.contains("showDefaultWorkspaceDialog"),
            "launcher must prompt when a default workspace choice is needed",
        );
        assert!(
            MAIN_JS.contains("invoke('choose_default_workspace'"),
            "launcher must let users choose an existing default workspace",
        );
        assert!(
            MAIN_JS.contains("invoke('create_default_workspace'"),
            "launcher must let users create Documents/Chan as default",
        );
        assert!(
            MAIN_JS.contains("showMissingDefaultWorkspaceDialog"),
            "launcher must confirm before factory-resetting missing default workspace metadata",
        );
        assert!(
            MAIN_JS.contains("invoke('factory_reset_default_workspace'"),
            "launcher must route confirmed missing-default reset to Rust",
        );
    }

    #[test]
    fn launcher_features_panel_carries_round2_plan_toggles() {
        // `fullstack-b-28a`: the panel HTML ships both feature
        // labels + the brief copy. Pin the label strings so a
        // future renaming requires deliberate coordination
        // (Settings copy in `-a-76` mirrors these labels).
        const MAIN_JS: &str = include_str!("../../src/main.js");
        assert!(
            MAIN_JS.contains("Semantic search"),
            "features panel must label the BGE toggle as 'Semantic search'"
        );
        assert!(
            MAIN_JS.contains("Reports"),
            "features panel must label the chan-report toggle as 'Reports'"
        );
        assert!(
            MAIN_JS.contains("data-feat=\"bge\""),
            "features panel must bind the BGE checkbox to the bge field"
        );
        assert!(
            MAIN_JS.contains("data-feat=\"reports\""),
            "features panel must bind the reports checkbox to the reports field"
        );
    }

    #[test]
    fn new_window_accelerator_uses_cmd_shift_n() {
        // `fullstack-b-27`: the "New Window" menu item moves from
        // `CmdOrCtrl+N` to `CmdOrCtrl+Shift+N` to free Cmd+N for
        // the SPA's New Draft handler (`fullstack-a-66`). Pin the
        // chord so a future menu edit can't silently revert to
        // plain Cmd+N and re-clash with the SPA chord.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(
            MAIN_RS.contains(".accelerator(\"CmdOrCtrl+Shift+N\")"),
            "main.rs must bind the New Window menu item to CmdOrCtrl+Shift+N"
        );
        assert!(
            !MAIN_RS.contains(".accelerator(\"CmdOrCtrl+N\")"),
            "main.rs must NOT bind any menu item to plain CmdOrCtrl+N (reserved for SPA New Draft)"
        );
    }

    #[test]
    fn key_bridge_wires_reload_and_devtools_ipc() {
        // `fullstack-b-17`: Cmd+R fires the `reload_window` IPC and
        // Cmd+Opt+I fires `open_devtools`, bypassing the SPA event
        // bus so a frozen Svelte runtime can't lock the dev
        // affordances away. The accelerator path goes through
        // `invokeIpc(...)` (not the `chan:command` `fire(...)`
        // bridge), so the contract pin checks both the IPC command
        // names and the case-label they're bound from.
        assert!(KEY_BRIDGE_JS.contains("invokeIpc(e, 'reload_window')"));
        assert!(KEY_BRIDGE_JS.contains("invokeIpc(e, 'open_devtools')"));
        // Reload is per-OS: Cmd+R (metaKey, no-shift branch) on macOS and
        // Ctrl+Shift+R (!metaKey, shift branch) on Linux/Windows, so plain
        // Ctrl+R is never claimed and reaches the terminal's reverse-search.
        assert!(KEY_BRIDGE_JS.contains("case 'KeyR': if (e.metaKey) invokeIpc(e, 'reload_window')"));
        assert!(
            KEY_BRIDGE_JS.contains("case 'KeyR': if (!e.metaKey) invokeIpc(e, 'reload_window')")
        );
        assert!(KEY_BRIDGE_JS.contains("code === 'KeyI'"));
    }

    #[test]
    fn embedded_url_prefix_parser_strips_query_and_trailing_slash() {
        let prefix = url_prefix_from_local_url("http://127.0.0.1:1234/workspace-abcd/?t=token")
            .expect("prefix");
        assert_eq!(prefix, "/workspace-abcd");
    }

    #[test]
    fn embedded_url_prefix_parser_strips_index_html() {
        let prefix =
            url_prefix_from_local_url("http://127.0.0.1:1234/workspace-abcd/index.html?t=token")
                .expect("prefix");
        assert_eq!(prefix, "/workspace-abcd");
    }

    #[test]
    fn key_bridge_invokes_tauri_ipc_via_core_invoke() {
        // The `invokeIpc` helper grabs `window.__TAURI__.core.invoke`
        // (Tauri 2's invoke surface; was `window.__TAURI__.invoke`
        // in Tauri 1). Pin so a future bridge rewrite doesn't
        // silently regress to the v1 shape. The new shape returns
        // undefined from a webview without the v2 IPC surface
        // attached, which silently swallows the Cmd+R / Cmd+Opt+I
        // accelerators.
        assert!(KEY_BRIDGE_JS.contains("window.__TAURI__"));
        assert!(KEY_BRIDGE_JS.contains("tauri.core.invoke"));
    }

    #[test]
    fn key_bridge_drops_chords_covered_by_pane_mode() {
        // `fullstack-42` pruned every native chord that now has a
        // Pane Mode equivalent. `fullstack-b-2` brought
        // `app.terminal.toggle` back (Cmd+T). `fullstack-a-32`
        // brings back `app.files.toggle` (Cmd+O), `app.graph.toggle`
        // (Cmd+Shift+M), and `app.terminal.teamWork` (Cmd+P) as
        // direct chords with context-aware semantics. `phase-12
        // lane-e` (addendum-2) brings `app.search.toggle` back as a
        // direct chord too (Cmd+S = workspace-wide search), so it moves to
        // the keeps-list below. The remaining absences catch
        // accidental reverts of chords that should still go through
        // Pane Mode only.
        assert!(!KEY_BRIDGE_JS.contains("app.file.new"));
        assert!(!KEY_BRIDGE_JS.contains("Backquote"));
    }

    #[test]
    fn key_bridge_keeps_independent_chords() {
        // Tab close + reopen + Find on page + tab nav + tab jump
        // are NOT duplicated by Pane Mode and must stay reachable
        // through the native bridge. Cmd+T / Cmd+O / Cmd+P /
        // Cmd+Shift+M are the `fullstack-a-32` context-aware
        // spawn chord family.
        assert!(KEY_BRIDGE_JS.contains("app.terminal.toggle"));
        assert!(KEY_BRIDGE_JS.contains("app.files.toggle"));
        assert!(KEY_BRIDGE_JS.contains("app.terminal.teamWork"));
        assert!(KEY_BRIDGE_JS.contains("app.pane.prev"));
        assert!(KEY_BRIDGE_JS.contains("app.pane.next"));
        assert!(KEY_BRIDGE_JS.contains("app.graph.toggle"));
        assert!(KEY_BRIDGE_JS.contains("app.tab.close"));
        assert!(KEY_BRIDGE_JS.contains("app.tab.reopenClosed"));
        assert!(KEY_BRIDGE_JS.contains("app.find.open"));
        assert!(KEY_BRIDGE_JS.contains("app.tab.jump"));
        assert!(KEY_BRIDGE_JS.contains("app.tab.next"));
        assert!(KEY_BRIDGE_JS.contains("app.tab.prev"));
        // `phase-12 lane-e` (addendum-2): Cmd+S search + Cmd+/ (right)
        // / Cmd+Shift+/ (bottom) splits route through the native bridge
        // too.
        assert!(KEY_BRIDGE_JS.contains("app.search.toggle"));
        assert!(KEY_BRIDGE_JS.contains("app.pane.splitRight"));
        assert!(KEY_BRIDGE_JS.contains("app.pane.splitDown"));
        // `phase-13 r2` (B-slice 2): Cmd+I was freed for the editor's
        // italic chord, so the native bridge no longer maps it to
        // Dashboard. Pin the removal so a regression that re-adds the
        // case is caught (Dashboard is Hybrid-Nav-only now).
        assert!(!KEY_BRIDGE_JS.contains("app.dashboard.open"));
    }

    #[test]
    fn workspace_title_is_the_path_verbatim() {
        // `fullstack-b-14`: titles are the workspace path so the OS
        // window switcher surfaces the disambiguating signal.
        // Earlier shape "chan workspace: <basename>" lost the path
        // detail and collided when two workspaces shared a basename.
        assert_eq!(
            workspace_title("/Users/alex/dev/github.com/fiorix/chan"),
            "/Users/alex/dev/github.com/fiorix/chan",
        );
        // Trailing slash, edge case, etc. are passed through; we
        // don't sanitize; the caller's path is the source of truth.
        assert_eq!(workspace_title("/tmp/scratch/"), "/tmp/scratch/");
        assert_eq!(workspace_title(""), "");
    }

    // `fullstack-b-7`: workspace and tunnel webviews host the SPA, which
    // routes external http(s) link clicks through tauri-plugin-opener
    // via the `plugin:opener|open_url` IPC. Without these permissions
    // the IPC denies, the SPA falls back to the clipboard-copy notify
    // branch, and "click external link" looks like a no-op to the
    // user (the bug Alex reported on 2026-05-20). Pin the capability
    // shape here so a future capability-file edit can't silently drop
    // the permissions without the test catching it.
    const WORKSPACE_CAPABILITY_JSON: &str = include_str!("../capabilities/workspace.json");
    const DEFAULT_CAPABILITY_JSON: &str = include_str!("../capabilities/default.json");
    const APP_PERMISSIONS_TOML: &str = include_str!("../permissions/app.toml");

    fn capability_permissions(raw: &str) -> Vec<String> {
        let v: serde_json::Value = serde_json::from_str(raw).expect("capability JSON parses");
        v["permissions"]
            .as_array()
            .expect("permissions is an array")
            .iter()
            .map(|p| p.as_str().expect("permission is a string").to_string())
            .collect()
    }

    fn capability_windows(raw: &str) -> Vec<String> {
        let v: serde_json::Value = serde_json::from_str(raw).expect("capability JSON parses");
        v["windows"]
            .as_array()
            .expect("windows is an array")
            .iter()
            .map(|w| w.as_str().expect("window glob is a string").to_string())
            .collect()
    }

    fn capability_remote_urls(raw: &str) -> Vec<String> {
        let v: serde_json::Value = serde_json::from_str(raw).expect("capability JSON parses");
        v["remote"]["urls"]
            .as_array()
            .expect("remote urls is an array")
            .iter()
            .map(|u| {
                u.as_str()
                    .expect("remote URL pattern is a string")
                    .to_string()
            })
            .collect()
    }

    fn app_permission_set(id: &str) -> Vec<String> {
        let v: toml::Value = toml::from_str(APP_PERMISSIONS_TOML).expect("app permissions parse");
        v["set"]
            .as_array()
            .expect("permission sets is an array")
            .iter()
            .find(|set| set["identifier"].as_str() == Some(id))
            .unwrap_or_else(|| panic!("missing app permission set {id}"))["permissions"]
            .as_array()
            .expect("permission set entries are an array")
            .iter()
            .map(|p| p.as_str().expect("permission id is a string").to_string())
            .collect()
    }

    #[test]
    fn workspace_capability_grants_opener_to_workspace_tunnel_and_outbound_windows() {
        let windows = capability_windows(WORKSPACE_CAPABILITY_JSON);
        assert!(
            windows.iter().any(|w| w == "workspace-*"),
            "workspace capability must target workspace-* windows: {windows:?}",
        );
        assert!(
            windows.iter().any(|w| w == "tunnel-*"),
            "workspace capability must target tunnel-* windows: {windows:?}",
        );
        assert!(
            windows.iter().any(|w| w == "outbound-*"),
            "workspace capability must target outbound-* windows: {windows:?}",
        );
        let perms = capability_permissions(WORKSPACE_CAPABILITY_JSON);
        assert!(
            perms.iter().any(|p| p == "workspace-window"),
            "workspace capability must include workspace-window app commands: {perms:?}",
        );
        assert!(
            perms.iter().any(|p| p == "opener:allow-open-url"),
            "workspace capability must include opener:allow-open-url: {perms:?}",
        );
    }

    #[test]
    fn workspace_capability_covers_loopback_server_urls() {
        // Workspace windows load chan-server through loopback HTTP
        // origins. Without a remote URL match, Tauri omits the IPC
        // bridge and workspace-window app commands such as reload_window
        // or the zoom chords never reach Rust.
        let remote_urls = capability_remote_urls(WORKSPACE_CAPABILITY_JSON);
        assert!(
            remote_urls.iter().any(|u| u == "http://127.0.0.1:*"),
            "workspace capability must include 127.0.0.1 loopback: {remote_urls:?}",
        );
        assert!(
            remote_urls.iter().any(|u| u == "http://localhost:*"),
            "workspace capability must include localhost loopback: {remote_urls:?}",
        );
    }

    #[test]
    fn app_acl_allows_workspace_window_commands() {
        let workspace_set = app_permission_set("workspace-window");
        for expected in [
            "allow-reload-window",
            "allow-open-devtools",
            "allow-save-file-to-downloads",
            "allow-zoom-in",
            "allow-zoom-out",
            "allow-zoom-reset",
        ] {
            assert!(
                workspace_set.iter().any(|p| p == expected),
                "workspace-window app permission set must include {expected}: {workspace_set:?}",
            );
        }
    }

    #[test]
    fn default_capability_covers_extra_launcher_windows() {
        // `fullstack-83` lets Cmd+N spawn `main-N` launcher windows.
        // They must inherit the same capability as `main`, or
        // external link handling and other plugin IPCs break for the
        // user the moment they open a second launcher.
        let windows = capability_windows(DEFAULT_CAPABILITY_JSON);
        assert!(
            windows.iter().any(|w| w == "main"),
            "default capability must still target main: {windows:?}",
        );
        assert!(
            windows.iter().any(|w| w == "main-*"),
            "default capability must target additional main-N launchers: {windows:?}",
        );
        let perms = capability_permissions(DEFAULT_CAPABILITY_JSON);
        assert!(
            perms.iter().any(|p| p == "main-window"),
            "default capability must include main-window app commands: {perms:?}",
        );
        assert!(
            perms.iter().any(|p| p == "opener:allow-open-url"),
            "default capability must include opener:allow-open-url: {perms:?}",
        );
    }
}
