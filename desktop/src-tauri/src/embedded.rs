//! Embedded local-workspace server for chan-desktop.
//!
//! This owns one loopback listener for the desktop process and
//! mounts local workspaces into chan-server's multi-workspace host.

use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::path::Path;
use std::sync::Arc;

use axum::Router;
use tokio::sync::watch;

use crate::serve;

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
}

impl EmbeddedServer {
    pub async fn start() -> Result<Self, String> {
        let library = chan_workspace::Library::open()
            .map_err(|e| format!("opening chan workspace registry for embedded server: {e}"))?;
        let host = Arc::new(chan_server::WorkspaceHost::new(library));
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
        })
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
        let hosted = self
            .host
            .open_terminal_session(serve_config(self.addr, PREFIX))
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
        // serve-progress stream (B10), so it stays quiet like open_browser.
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
