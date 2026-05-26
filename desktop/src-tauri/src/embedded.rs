//! Embedded local-drive server for chan-desktop.
//!
//! This owns one loopback listener for the desktop process and
//! mounts local drives into chan-server's multi-drive host.

use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::path::Path;
use std::sync::Arc;

use axum::Router;
use tokio::sync::watch;

use crate::serve;

pub struct EmbeddedServer {
    host: Arc<chan_server::DriveHost>,
    addr: SocketAddr,
    shutdown_tx: watch::Sender<bool>,
}

impl EmbeddedServer {
    pub async fn start() -> Result<Self, String> {
        let library = chan_drive::Library::open()
            .map_err(|e| format!("opening chan drive registry for embedded server: {e}"))?;
        let host = Arc::new(chan_server::DriveHost::new(library));
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
        })
    }

    pub async fn open_drive(&self, key: &str) -> Result<String, String> {
        let prefix = prefix_for_key(key);
        let hosted = self
            .host
            .open_registered_drive(Path::new(key), serve_config(self.addr, &prefix))
            .await
            .map_err(|e| map_open_error(key, e))?;
        Ok(hosted.handle.launch_url())
    }

    /// Shared drive registry handle owned by the embedded host.
    /// Every desktop registry mutation and feature toggle routes
    /// through this single `Library` so the in-memory registry the
    /// host opens drives against never goes stale relative to disk.
    pub fn library(&self) -> &chan_drive::Library {
        self.host.library()
    }

    /// Live `Arc<Drive>` for a mounted drive, or `None` when the
    /// path isn't currently mounted. Feature toggles use this to
    /// reach the SAME handle the runtime holds instead of re-opening
    /// (which would hit `DriveAlreadyOpen` against the lifetime
    /// flock).
    pub fn live_drive(&self, root: &Path) -> Option<Arc<chan_drive::Drive>> {
        self.host.live_drive(root)
    }

    pub fn close_prefix(&self, prefix: &str) -> Result<(), String> {
        self.host
            .close_drive(prefix)
            .map_err(|e| format!("closing embedded route {prefix}: {e}"))?;
        Ok(())
    }
}

impl Drop for EmbeddedServer {
    fn drop(&mut self) {
        let _ = self.shutdown_tx.send(true);
    }
}

/// Map an embedded open error to a user-facing string. A drive
/// already held by another chan process (typically a standalone
/// `chan serve <drive>` started before the desktop tried to mount
/// it) surfaces as `DriveLocked`; an in-process handle that hasn't
/// dropped yet surfaces as `DriveAlreadyOpen`. Both reach the SPA
/// verbatim and revert the row's On toggle, so they must read as a
/// clear, non-fatal instruction rather than a raw error chain.
fn map_open_error(key: &str, e: chan_server::Error) -> String {
    use chan_drive::ChanError;
    match e {
        chan_server::Error::Core(ChanError::DriveLocked | ChanError::DriveAlreadyOpen) => {
            "This drive is open in another chan process. Quit it and try again.".to_string()
        }
        other => format!("opening embedded drive {key}: {other}"),
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
    }
}

fn prefix_for_key(key: &str) -> String {
    format!("/{}", serve::drive_window_prefix(key))
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
    fn prefix_for_key_uses_drive_window_prefix() {
        let key = "/tmp/chan notes";
        let prefix = prefix_for_key(key);
        assert!(prefix.starts_with("/drive-"));
        assert_eq!(prefix, format!("/{}", serve::drive_window_prefix(key)));
    }
}
