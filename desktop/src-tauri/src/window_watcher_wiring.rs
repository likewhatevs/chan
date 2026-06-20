//! Binds the surface-agnostic window-watcher core ([`crate::window_watcher`])
//! to the live desktop: the local library's in-process window feed and the
//! Tauri native-window surface.
//!
//! [`spawn_local_window_watcher`] runs one [`watch_loop`] for the embedded
//! local library (`"local"`). The feed snapshots
//! [`EmbeddedServer::assemble_window_records`](crate::embedded::EmbeddedServer::assemble_window_records)
//! and wakes on its aggregate change `Notify`; the surface opens windows
//! through [`serve::open_watched_local_window`] (the shared SPA builder) and
//! closes them by destroying the Tauri window. The watcher is inert until a
//! local window is minted (an empty registry reconciles to nothing); routing
//! the window-creation paths through the registry mint makes it the SOLE driver
//! of local windows, so the reconnect/relaunch duplicate class (L0 Bug A) is
//! unreachable by construction.

use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;

use chan_server::WindowRecord;
use tauri::{AppHandle, Manager};
use tokio::sync::Notify;

use crate::window_watcher::{watch_loop, NativeSurface, WatcherViewState, WindowFeed};
use crate::{serve, AppState};

/// Library id of the embedded local-disk library (Seam-L scheme).
const LOCAL_LIBRARY_ID: &str = "local";

/// The local library's window-set feed, read in-process from the embedded host.
struct LocalWindowFeed {
    state: Arc<AppState>,
    /// The aggregate change signal, captured once at spawn (a stable `Arc` the
    /// host re-hands on every call, so capturing it once is sufficient).
    change: Arc<Notify>,
}

impl WindowFeed for LocalWindowFeed {
    fn snapshot(&self) -> Vec<WindowRecord> {
        self.state
            .embedded()
            .map(|embedded| embedded.assemble_window_records())
            .unwrap_or_default()
    }

    fn change_notify(&self) -> Arc<Notify> {
        self.change.clone()
    }
}

/// The Tauri native-window surface: opens windows via the shared SPA builder,
/// closes them by destroying the OS window, and enumerates the open native
/// windows for a library by their `{library_id}::` label prefix.
struct TauriNativeSurface {
    app: AppHandle,
    addr: SocketAddr,
}

impl NativeSurface for TauriNativeSurface {
    fn open_labels(&self, library_id: &str) -> HashSet<String> {
        let prefix = format!("{library_id}::");
        self.app
            .webview_windows()
            .into_keys()
            .filter(|label| label.starts_with(&prefix))
            .collect()
    }

    fn open(&self, record: &WindowRecord) {
        // `open_watched_local_window` dispatches the actual build to the Tauri
        // main thread internally, so this returns promptly.
        if let Err(e) = serve::open_watched_local_window(&self.app, self.addr, record) {
            tracing::warn!(
                window = %record.window_id,
                error = %e,
                "window watcher: opening a local window failed",
            );
        }
    }

    fn close(&self, label: &str) {
        // Destroying a window must run on the Tauri main thread.
        let app = self.app.clone();
        let dispatch = self.app.clone();
        let label_owned = label.to_string();
        let result = dispatch.run_on_main_thread(move || {
            if let Some(window) = app.get_webview_window(&label_owned) {
                let _ = window.destroy();
            }
        });
        if let Err(e) = result {
            tracing::warn!(window = %label, error = %e, "window watcher: closing a local window failed");
        }
    }
}

/// Spawn the local library's window watcher (one [`watch_loop`] for `"local"`),
/// living for the process lifetime. A no-op when the embedded server is not up.
pub(crate) fn spawn_local_window_watcher(app: AppHandle, state: Arc<AppState>) {
    let Some(embedded) = state.embedded() else {
        tracing::warn!("local window watcher not started: embedded server unavailable");
        return;
    };
    let addr = embedded.addr();
    let change = embedded.library_change_notify();
    let feed = LocalWindowFeed {
        state: Arc::clone(&state),
        change,
    };
    let surface = TauriNativeSurface { app, addr };
    let view = Arc::new(WatcherViewState::default());
    // The local library lives for the whole process, so the watcher is never
    // cancelled — `cancel` is a future that only resolves at process exit
    // (which drops the spawned task).
    tauri::async_runtime::spawn(watch_loop(
        LOCAL_LIBRARY_ID,
        feed,
        surface,
        view,
        std::future::pending::<()>(),
    ));
}
