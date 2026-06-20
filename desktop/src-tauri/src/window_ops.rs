//! Consumer for the `cs window <op>` bridge.
//!
//! The embedded chan-server hands each `cs window new|open|rm|hide|title`
//! request to the desktop over an mpsc channel (see
//! `chan_server::DesktopBridge`); this module owns the receiver and turns
//! each [`DesktopWindowOp`] into the matching Tauri window action, then
//! completes the op's `oneshot` so the blocked `cs` command unblocks.
//!
//! Tauri window operations (build / show / destroy / set_title / dialogs)
//! must run on the main thread, so the handlers hop there via
//! [`on_main`]; the async terminal-open path mirrors the existing
//! `spawn_terminal_window` flow (async-runtime task, build dispatches to
//! main internally). Each op runs in its own task so a blocking `rm`
//! confirmation dialog can't stall unrelated ops.

use std::path::Path;
use std::sync::Arc;

use chan_server::{DesktopWindowOp, NewWindowKind};
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;

use crate::{serve, AppState};

/// Drain the window-ops channel until it closes (process exit drops the
/// host that owns the sender). Spawned once from Tauri `.setup()`.
pub async fn run(app: AppHandle, state: Arc<AppState>, mut rx: mpsc::Receiver<DesktopWindowOp>) {
    while let Some(op) = rx.recv().await {
        let app = app.clone();
        let state = Arc::clone(&state);
        tauri::async_runtime::spawn(async move {
            handle(app, state, op).await;
        });
    }
}

async fn handle(app: AppHandle, state: Arc<AppState>, op: DesktopWindowOp) {
    match op {
        DesktopWindowOp::New { kind, reply } => {
            let result = match kind {
                NewWindowKind::Terminal => {
                    serve::spawn_local_terminal_window(app.clone(), Arc::clone(&state)).await
                }
                NewWindowKind::Workspace { key } => new_workspace_window(&state, &key).await,
            };
            let _ = reply.send(result);
        }
        DesktopWindowOp::Open { id, reply } => {
            let app2 = app.clone();
            let state2 = Arc::clone(&state);
            let result = on_main(&app, move || {
                serve::open_window_by_label(&app2, &state2, &id)
            })
            .await
            .and_then(|inner| inner);
            let _ = reply.send(result);
        }
        DesktopWindowOp::Hide { id, reply } => {
            let app2 = app.clone();
            let result = on_main(&app, move || hide_window(&app2, &id))
                .await
                .and_then(|inner| inner);
            let _ = reply.send(result);
        }
        DesktopWindowOp::Close { id, force, reply } => {
            close_window(app, state, id, force, reply).await;
        }
    }
}

/// `cs window new` (workspace tenant): open another window of the
/// workspace rooted at `key` by minting a window into the library registry
/// (the watcher opens it), the same way `open_local_workspace` does. Errors
/// when that workspace isn't currently running locally. Returns the new
/// window's composite native label.
async fn new_workspace_window(state: &Arc<AppState>, key: &str) -> Result<String, String> {
    let canon = crate::canonical_key(Path::new(key));
    if !state.serves.lock().unwrap().contains_key(&canon) {
        return Err(format!("workspace {key} is not running"));
    }
    let record = state
        .embedded()
        .ok_or_else(|| "embedded local server is unavailable".to_string())?
        .mint_window(chan_server::WindowKind::Workspace, Some(canon))?;
    Ok(crate::window_watcher::native_label(&record))
}

/// `cs window hide`: route through the OS close-button path so the
/// existing bury handler (which knows this window's restore key) runs.
/// `close()` requests a close — the handler hides SPA windows — unlike
/// `destroy()`, which `rm` uses to truly remove.
fn hide_window(app: &AppHandle, id: &str) -> Result<(), String> {
    match app.get_webview_window(id) {
        Some(w) => w.close().map_err(|e| format!("hiding {id}: {e}")),
        None => Err(format!("no window {id}")),
    }
}

/// `cs window rm`: truly destroy the window. When it still has live
/// terminal shells and `force` is unset, prompt first and only destroy on
/// confirm — the reply (and thus the blocked CLI) waits for the dialog.
/// Reply: `Ok(true)` destroyed, `Ok(false)` no live window (the server
/// then deletes any saved layout), `Err("cancelled")` declined.
async fn close_window(
    app: AppHandle,
    state: Arc<AppState>,
    id: String,
    force: bool,
    reply: tokio::sync::oneshot::Sender<Result<bool, String>>,
) {
    // Probe liveness / title / live-shells on the main thread.
    let probe = {
        let app2 = app.clone();
        let state2 = Arc::clone(&state);
        let id2 = id.clone();
        on_main(&app, move || {
            app2.get_webview_window(&id2).map(|w| {
                let title = w.title().unwrap_or_else(|_| id2.clone());
                let shells = serve::window_has_live_shells(&state2, &id2);
                (title, shells)
            })
        })
        .await
    };
    let live = match probe {
        Ok(live) => live,
        Err(e) => {
            let _ = reply.send(Err(e));
            return;
        }
    };
    let Some((title, shells)) = live else {
        // No live window: nothing to destroy. The server handles deleting
        // a saved layout for this id.
        let _ = reply.send(Ok(false));
        return;
    };

    if shells && !force {
        // Confirm before killing live terminals; the dialog callback
        // completes the reply (so the CLI blocks until the user answers).
        let app2 = app.clone();
        let id2 = id.clone();
        let scheduled = on_main(&app, move || {
            use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
            app2.clone()
                .dialog()
                .message(format!(
                    "\"{title}\" has running terminals. Removing it will end them."
                ))
                .title("Remove window?")
                .buttons(MessageDialogButtons::OkCancelCustom(
                    "Remove".into(),
                    "Cancel".into(),
                ))
                .show(move |confirmed| {
                    if confirmed {
                        let destroyed = app2
                            .get_webview_window(&id2)
                            .map(|w| w.destroy().is_ok())
                            .unwrap_or(false);
                        let _ = reply.send(Ok(destroyed));
                    } else {
                        let _ = reply.send(Err("cancelled — window not removed".to_string()));
                    }
                });
        })
        .await;
        if let Err(e) = scheduled {
            // The reply was moved into the dialog callback; if scheduling
            // the dialog itself failed, nothing will complete it — but the
            // sender dropped, so the server maps that to an error.
            tracing::warn!(window = %id, error = %e, "scheduling rm confirmation dialog failed");
        }
        return;
    }

    // No shells, or forced: destroy now.
    let id2 = id.clone();
    let app2 = app.clone();
    let result = on_main(&app, move || match app2.get_webview_window(&id2) {
        Some(w) => w
            .destroy()
            .map(|()| true)
            .map_err(|e| format!("destroying {id2}: {e}")),
        None => Ok(false),
    })
    .await
    .and_then(|inner| inner);
    let _ = reply.send(result);
}

/// Run `f` on the Tauri main thread and await its result. Tauri window
/// ops must run there; this bridges the async consumer task to it.
async fn on_main<T: Send + 'static>(
    app: &AppHandle,
    f: impl FnOnce() -> T + Send + 'static,
) -> Result<T, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(f());
    })
    .map_err(|e| format!("scheduling on main thread: {e}"))?;
    rx.await
        .map_err(|_| "main-thread task dropped before replying".to_string())
}
