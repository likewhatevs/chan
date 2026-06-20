//! Embedded local-workspace server for chan-desktop.
//!
//! This owns one loopback listener for the desktop process and
//! mounts local workspaces into chan-server's multi-workspace host.

use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::Router;
use chan_server::{DesktopBridge, DesktopWindowOp, SharedWindowTitles, WindowRecord, WindowTitles};
use tokio::sync::{mpsc, watch, Notify};

use crate::serve;

/// Bound on the window-ops channel: interactive `cs window` calls are
/// low-rate, so 32 is far above any real concurrency while still capping a
/// runaway caller.
const WINDOW_OPS_CHANNEL_CAPACITY: usize = 32;

pub struct EmbeddedServer {
    host: Arc<chan_server::WorkspaceHost>,
    addr: SocketAddr,
    shutdown_tx: watch::Sender<bool>,
    /// Cached launch URL of the single shared `/terminal` tenant that backs
    /// ALL standalone terminal windows, so their PTYs live in one registry
    /// (cross-window terminal moves work) under one global Terminal-N
    /// namespace. `None` until the first terminal window opens it; reused
    /// thereafter. The async lock serializes concurrent first-opens so two
    /// windows can't double-mount the prefix.
    terminal_url: tokio::sync::Mutex<Option<String>>,
    /// Receiver end of the `cs window <op>` bridge, parked here until Tauri
    /// `.setup()` (where the `AppHandle` exists) takes it and spawns the
    /// consumer task. `None` once taken, so a double-take can't spawn two
    /// consumers. The sender lives inside the host's [`DesktopBridge`].
    pending_window_ops: tokio::sync::Mutex<Option<mpsc::Receiver<DesktopWindowOp>>>,
}

impl EmbeddedServer {
    pub async fn start() -> Result<Self, String> {
        let library = chan_workspace::Library::open()
            .map_err(|e| format!("opening chan workspace registry for embedded server: {e}"))?;
        // Install the desktop bridge: a window-ops channel (the consumer
        // is spawned in Tauri `.setup()` once the AppHandle exists) plus a
        // shared title map every tenant reads and the desktop writes as it
        // builds/destroys webviews.
        let (window_ops_tx, window_ops_rx) = mpsc::channel(WINDOW_OPS_CHANNEL_CAPACITY);
        let bridge = DesktopBridge {
            window_ops: Some(window_ops_tx),
            window_titles: Arc::new(WindowTitles::new()),
        };
        let host = Arc::new(chan_server::WorkspaceHost::with_desktop_bridge(
            library,
            bridge,
            chan_server::route_builder(),
        ));
        // Register the host's self-handle so its per-tenant control sockets can
        // reach it for teardown — otherwise the desktop's tenants report
        // `UnserveMode::Unsupported` and `chan unserve` fails. Parity with the
        // devserver path's `host.install_self()`.
        host.install_self();
        // Install the local library's window registry so the window feed has
        // data (~/.chan/windows.json, library id "local").
        chan_server::install_local_window_registry(&host);
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .map_err(|e| format!("binding embedded chan server: {e}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|e| format!("setting embedded listener nonblocking: {e}"))?;
        let addr = listener
            .local_addr()
            .map_err(|e| format!("reading embedded listener addr: {e}"))?;
        let listener = tokio::net::TcpListener::from_std(listener)
            .map_err(|e| format!("adopting embedded listener: {e}"))?;
        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        let app = host.clone().router();
        tauri::async_runtime::spawn(async move {
            let result = serve_router(listener, app, async move {
                let _ = shutdown_rx.changed().await;
            })
            .await;
            if let Err(e) = result {
                tracing::warn!(error = %e, "embedded chan server stopped");
            }
        });
        Ok(Self {
            host,
            addr,
            shutdown_tx,
            terminal_url: tokio::sync::Mutex::new(None),
            pending_window_ops: tokio::sync::Mutex::new(Some(window_ops_rx)),
        })
    }

    /// The shared window-title map the desktop writes (on window build /
    /// rename / destroy) and the server reads for `cs window list`.
    pub fn window_titles(&self) -> SharedWindowTitles {
        self.host.desktop_bridge().window_titles.clone()
    }

    /// Take the `cs window <op>` receiver exactly once (in Tauri
    /// `.setup()`). Returns `None` on a second call so a re-entrant setup
    /// can't spawn two consumer tasks.
    pub fn take_window_ops_rx(&self) -> Option<mpsc::Receiver<DesktopWindowOp>> {
        self.pending_window_ops
            .try_lock()
            .ok()
            .and_then(|mut slot| slot.take())
    }

    /// True when the workspace mounted for `key` still has at least one
    /// live PTY session bound to `window_label`. The `cs window rm`
    /// confirmation uses this (alongside the terminal-tenant variant) to
    /// decide whether to prompt before killing a window's shells.
    pub fn workspace_window_has_live_shells(&self, key: &str, window_label: &str) -> bool {
        self.host
            .tenant_has_window_sessions(&prefix_for_key(key), window_label)
    }

    pub async fn open_workspace(&self, key: &str) -> Result<String, String> {
        use chan_workspace::ChanError;
        // A workspace just turned OFF can keep its flock for a beat: a
        // background indexer / in-flight request still holding an
        // `Arc<Workspace>` releases it shortly after the runtime is dropped.
        // A quick OFF -> ON would otherwise spuriously hit
        // `WorkspaceAlreadyOpen` (our own releasing handle) or
        // `WorkspaceLocked`. Retry briefly so the toggle settles instead of
        // erroring; a genuine other-process lock still surfaces after the
        // short budget. Mirrors `unregister_with_retry` on the close side.
        const MAX_ATTEMPTS: usize = 8;
        const BACKOFF: std::time::Duration = std::time::Duration::from_millis(150);
        let prefix = prefix_for_key(key);
        for attempt in 1..=MAX_ATTEMPTS {
            match self
                .host
                .open_registered_workspace(Path::new(key), serve_config(self.addr, &prefix))
                .await
            {
                Ok(hosted) => return Ok(hosted.handle.launch_url()),
                Err(
                    e @ chan_server::Error::Core(
                        ChanError::WorkspaceLocked | ChanError::WorkspaceAlreadyOpen,
                    ),
                ) => {
                    if attempt == MAX_ATTEMPTS {
                        return Err(map_open_error(key, e));
                    }
                    tokio::time::sleep(BACKOFF).await;
                }
                Err(other) => return Err(map_open_error(key, other)),
            }
        }
        unreachable!("retry loop returns on the final attempt")
    }

    /// Shared workspace registry handle owned by the embedded host.
    /// Every desktop registry mutation and feature toggle routes
    /// through this single `Library` so the in-memory registry the
    /// host opens workspaces against never goes stale relative to disk.
    pub fn library(&self) -> &chan_workspace::Library {
        self.host.library()
    }

    pub fn close_prefix(&self, prefix: &str) -> Result<(), String> {
        self.host
            .close_workspace(prefix)
            .map_err(|e| format!("closing embedded route {prefix}: {e}"))?;
        Ok(())
    }

    /// Return the tokened launch URL of the single shared `/terminal` tenant
    /// (`http://<addr>/terminal/index.html?t=<token>`), mounting it on first
    /// use. ALL standalone terminal windows load this one URL (each with its
    /// own `?w=<label>` appended by the caller), so their PTYs share a single
    /// registry: cross-window terminal moves work and a global Terminal-N
    /// sequence is possible. The tenant lives for the process lifetime; there
    /// is no per-window teardown (orphaned PTYs idle-prune). The async lock is
    /// held across the mount so two simultaneous first-opens can't both try to
    /// mount `/terminal`.
    pub async fn open_terminal(&self) -> Result<String, String> {
        const PREFIX: &str = "/terminal";
        let mut cached = self.terminal_url.lock().await;
        if let Some(url) = cached.as_ref() {
            return Ok(url.clone());
        }
        // Persist each standalone-terminal window's pane layout on disk (keyed
        // by `?w=<window_id>`) so it restores across a desktop relaunch — with
        // fresh shells, since the PTYs don't survive. Best-effort: if the dir
        // can't be made the tenant falls back to its in-memory layout store.
        let session_dir = local_terminal_session_dir();
        let hosted = self
            .host
            .open_terminal_session(serve_config(self.addr, PREFIX), session_dir)
            .await
            .map_err(|e| format!("opening the shared embedded terminal tenant: {e}"))?;
        let url = hosted.handle.launch_url();
        *cached = Some(url.clone());
        Ok(url)
    }

    /// True when the shared `/terminal` tenant still has at least one
    /// live PTY session bound to `window_label` (sessions carry the
    /// SPA's `?w=` window id, which IS the Tauri label for desktop
    /// windows). The close handler uses this to decide bury-vs-close
    /// for a standalone terminal window: shells running -> hide the
    /// window and keep them; none -> let the window really close.
    /// Sync (read lock + roster snapshot), safe on the event-loop
    /// thread. `false` when the tenant was never mounted.
    pub fn terminal_window_has_live_shells(&self, window_label: &str) -> bool {
        self.host
            .tenant_has_window_sessions("/terminal", window_label)
    }

    /// Mount a fresh terminal tenant whose PTY runs `command` (a single
    /// shell command line, through the login shell so an interactive
    /// script gets a real PTY) and return its tokened launch URL. Each
    /// call mounts its own tenant under a unique prefix, so a control
    /// terminal running one devserver's connect script stays separate
    /// from the shared standalone-terminal tenant and from other control
    /// terminals.
    pub async fn open_terminal_with_command(
        &self,
        command: String,
    ) -> Result<(String, String), String> {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let prefix = format!("/control-{}", SEQ.fetch_add(1, Ordering::Relaxed));
        let hosted = self
            .host
            .open_terminal_session_with_command(
                serve_config(self.addr, &prefix),
                Some(command),
                None,
            )
            .await
            .map_err(|e| format!("opening a command terminal tenant: {e}"))?;
        Ok((hosted.handle.launch_url(), prefix))
    }

    /// Raw output (replay-ring scrollback) of the control-terminal tenant
    /// mounted at `prefix`, decoded lossily. Lets the connect flow scrape a
    /// token the connect script printed; empty when no such tenant exists.
    pub fn read_control_terminal_output(&self, prefix: &str) -> String {
        String::from_utf8_lossy(&self.host.terminal_tenant_scrollback(prefix)).into_owned()
    }

    /// Exit status of the control-terminal tenant's PTY (the connect script),
    /// or `None` while it is still running. The connect flow polls this beside
    /// the scrollback scrape: `Some(code)` means the script exited (a failed
    /// connect) so the scrape can fail fast instead of waiting out its full
    /// budget. The status is the tenant's, independent of the control window,
    /// so it still reports after the window is closed.
    pub fn control_terminal_exit(&self, prefix: &str) -> Option<u32> {
        self.host.terminal_tenant_last_exit(prefix)
    }

    /// Close the control-terminal tenant mounted at `prefix`, reaping its PTY
    /// (the devserver connect script) synchronously. Called on disconnect and
    /// forget: destroying the control-terminal window alone leaves the connect
    /// script RUNNING on the host because the tenant outlives the window. Must
    /// NOT route through `close_prefix`/`close_workspace`, whose terminal-PTY
    /// reap rides a fragile prune-task drop race; `close_terminal_tenant` Kills
    /// the children directly. A no-op when nothing is mounted there (idempotent
    /// across a repeated teardown).
    pub fn close_control_terminal(&self, prefix: &str) -> Result<(), String> {
        self.host
            .close_terminal_tenant(prefix)
            .map(|_| ())
            .map_err(|e| format!("closing control terminal {prefix}: {e}"))
    }

    /// The loopback address the embedded server listens on. The window
    /// watcher assembles a window's tenant URL (`http://{addr}{prefix}…`)
    /// from this plus the record's prefix/token.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// The library's authoritative window set (Seam W), each persisted
    /// registry row joined with its serving tenant's live `prefix`/`token`/
    /// `connected`. The local window watcher reconciles native windows to
    /// this. Empty until a window is minted.
    pub fn assemble_window_records(&self) -> Vec<WindowRecord> {
        self.host.assemble_window_records()
    }

    /// The aggregate window-set change signal (registry mint/discard +
    /// tenant on/off + presence) the watcher's feed awaits. NOT the raw
    /// registry change signal — that misses tenant transitions.
    pub fn library_change_notify(&self) -> Arc<Notify> {
        self.host.library_change_notify()
    }

    /// Mint a window into the local library registry and return its assembled
    /// record. The minted record fires the aggregate change signal, so the
    /// window watcher's feed surfaces it and opens its native window — the
    /// registry is the sole window-creation authority (a minted window can
    /// never be double-opened). A workspace window resolves its live tenant
    /// (the workspace must be running) for a prefix/token to attach to.
    pub fn mint_window(
        &self,
        kind: chan_server::WindowKind,
        workspace_path: Option<String>,
    ) -> Result<WindowRecord, String> {
        self.host
            .mint_window(kind, workspace_path)
            .map_err(|e| format!("minting a window: {e}"))
    }

    /// Discard a window: remove its registry row and reap its terminal
    /// sessions, then fire the aggregate change signal so the watcher reconciles
    /// the native window closed (the L5 discard op — `^W`/`^D`/empty-pane). The
    /// record is gone, so the watcher never reopens it. Returns whether a row
    /// existed.
    pub fn discard_window(&self, window_id: &str) -> Result<bool, String> {
        self.host
            .discard_window(window_id)
            .map_err(|e| format!("discarding window {window_id}: {e}"))
    }
}

impl Drop for EmbeddedServer {
    fn drop(&mut self) {
        let _ = self.shutdown_tx.send(true);
    }
}

/// Map an embedded open error to a user-facing string. A workspace
/// already held by another chan process (typically a standalone
/// `chan serve <workspace>` started before the desktop tried to mount
/// it) surfaces as `WorkspaceLocked`; an in-process handle that hasn't
/// dropped yet surfaces as `WorkspaceAlreadyOpen`. Both reach the SPA
/// verbatim and revert the row's On toggle, so they must read as a
/// clear, non-fatal instruction rather than a raw error chain.
fn map_open_error(key: &str, e: chan_server::Error) -> String {
    use chan_workspace::ChanError;
    match e {
        chan_server::Error::Core(ChanError::WorkspaceLocked | ChanError::WorkspaceAlreadyOpen) => {
            "This workspace is open in another chan process. Quit it and try again.".to_string()
        }
        other => format!("opening embedded workspace {key}: {other}"),
    }
}

/// On-disk dir for the standalone `/terminal` tenant's per-window layout blobs
/// (`~/.chan/terminal-sessions`, created on first use). `None` if the home dir
/// can't be resolved — the tenant then keeps layout in-memory (it just won't
/// persist across relaunch).
fn local_terminal_session_dir() -> Option<std::path::PathBuf> {
    let dir = dirs::home_dir()?.join(".chan").join("terminal-sessions");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn serve_config(addr: SocketAddr, prefix: &str) -> chan_server::ServeConfig {
    chan_server::ServeConfig {
        addr,
        no_token: false,
        prefix: prefix.to_string(),
        idle_timeout: None,
        open_browser: false,
        search_aggression: None,
        settings_disabled: false,
        tunnel_public: false,
        // The embedded desktop server has no controlling terminal for the
        // serve-progress stream, so it stays quiet like open_browser.
        verbose: false,
    }
}

fn prefix_for_key(key: &str) -> String {
    format!("/{}", serve::workspace_window_prefix(key))
}

async fn serve_router(
    listener: tokio::net::TcpListener,
    app: Router,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> Result<(), std::io::Error> {
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(|e| std::io::Error::other(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_for_key_uses_workspace_window_prefix() {
        let key = "/tmp/chan notes";
        let prefix = prefix_for_key(key);
        assert!(prefix.starts_with("/workspace-"));
        assert_eq!(prefix, format!("/{}", serve::workspace_window_prefix(key)));
    }
}
