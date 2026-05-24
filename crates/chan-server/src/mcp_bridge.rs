//! In-process MCP server exposed over a Unix-domain socket.
//!
//! External MCP agents want to launch the chan MCP server as a
//! subprocess so writes round-trip through chan-drive's gates. The
//! original wiring spawned `chan __mcp <drive_root>`, which then
//! called `Library::open_drive` a second time. chan-drive holds a
//! per-drive flock for single-writer ownership, so the child failed
//! with `DriveLocked`.
//!
//! The bridge resolves that conflict: chan-server already owns an
//! `Arc<Drive>` for the drive it serves, so the MCP service is run
//! in-process. Each external agent connects through `chan __mcp-proxy`
//! to a Unix-domain socket the bridge listens on; the proxy just
//! pipes stdin/stdout through the socket. No second drive open, no
//! flock contention.
//!
//! Lifetime: the bridge spawns at boot inside `build_app`. The
//! returned `BridgeHandle` owns the socket-cleanup `Drop` and the
//! accept-loop join handle; serve()/shutdown drops it explicitly so
//! the socket file is unlinked even when the runtime is torn down
//! abruptly.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rand::RngCore;
#[cfg(unix)]
use tokio::net::UnixListener;
#[cfg(unix)]
use tokio::task::JoinHandle;

/// Pick a unique socket path under the system tmp dir. macOS caps
/// `sun_path` at 104 bytes, so the suffix is short and the directory
/// short; `/tmp/chan-mcp-<pid>-<8 hex>.sock` fits well within that.
pub fn pick_socket_path() -> PathBuf {
    pick_named_socket_path("mcp")
}

pub(crate) fn pick_named_socket_path(name: &str) -> PathBuf {
    let mut bytes = [0u8; 4];
    rand::thread_rng().fill_bytes(&mut bytes);
    let suffix: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    std::env::temp_dir().join(format!("chan-{name}-{}-{suffix}.sock", std::process::id()))
}

/// Bridge handle returned from `start`. Drop = abort the accept loop
/// and unlink the socket file. Held by `AppState` for the lifetime
/// of the chan-server process.
#[cfg(unix)]
pub struct BridgeHandle {
    socket_path: PathBuf,
    accept_loop: Option<JoinHandle<()>>,
}

/// Windows stub: chan-server's MCP bridge relies on Unix-domain
/// sockets, which are not how the chan stack reaches subprocess
/// agents on Windows. The handle still exists so `AppArtifacts` has
/// a stable type across targets; `start` returns `Unsupported` so
/// the caller falls back to `mcp_socket_path = None`.
#[cfg(not(unix))]
pub struct BridgeHandle {
    socket_path: PathBuf,
}

impl BridgeHandle {
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }
}

#[cfg(unix)]
impl Drop for BridgeHandle {
    fn drop(&mut self) {
        if let Some(h) = self.accept_loop.take() {
            h.abort();
        }
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

/// Bind the socket and spawn an accept loop. Each accepted connection
/// gets a fresh `chan_llm::mcp::Server` constructed against the
/// current drive Arc.
#[cfg(unix)]
pub fn start<DF>(socket_path: PathBuf, drive_for: DF) -> std::io::Result<BridgeHandle>
where
    DF: Fn() -> Option<Arc<chan_drive::Drive>> + Send + Sync + 'static,
{
    // Stale socket from a previous run that didn't get to clean up
    // (kill -9, panic in Drop): unlink so bind doesn't EADDRINUSE.
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;
    let drive_for = Arc::new(drive_for);

    let accept_loop = tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(pair) => pair,
                Err(e) => {
                    tracing::warn!("mcp bridge accept: {e}");
                    // Brief pause so a transient error doesn't spin
                    // a tight CPU loop; the listener stays alive.
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    continue;
                }
            };
            let Some(drive) = drive_for() else {
                tracing::warn!("mcp bridge session refused: drive state unavailable");
                continue;
            };
            tokio::spawn(async move {
                let (read, write) = stream.into_split();
                let server = chan_llm::mcp::Server::new(drive);
                if let Err(e) = server.serve_io(read, write).await {
                    tracing::debug!("mcp bridge session: {e}");
                }
            });
        }
    });

    Ok(BridgeHandle {
        socket_path,
        accept_loop: Some(accept_loop),
    })
}

#[cfg(not(unix))]
pub fn start<DF>(_socket_path: PathBuf, _drive_for: DF) -> std::io::Result<BridgeHandle>
where
    DF: Fn() -> Option<Arc<chan_drive::Drive>> + Send + Sync + 'static,
{
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "mcp bridge requires unix-domain sockets",
    ))
}
