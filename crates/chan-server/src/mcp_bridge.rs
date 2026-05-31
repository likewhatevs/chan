//! In-process MCP server exposed over a Unix-domain socket.
//!
//! External MCP agents want to launch the chan MCP server as a
//! subprocess so writes round-trip through chan-workspace's gates. The
//! original wiring spawned `chan __mcp <workspace_root>`, which then
//! called `Library::open_workspace` a second time. chan-workspace holds a
//! per-workspace flock for single-writer ownership, so the child failed
//! with `WorkspaceLocked`.
//!
//! The bridge resolves that conflict: chan-server already owns an
//! `Arc<Workspace>` for the workspace it serves, so the MCP service is run
//! in-process. Each external agent connects through `chan __mcp-proxy`
//! to a Unix-domain socket the bridge listens on; the proxy just
//! pipes stdin/stdout through the socket. No second workspace open, no
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

/// Connect stdio to a running chan-server MCP socket. Used by the
/// `chan __mcp-proxy` and `chan-desktop __mcp-proxy` hidden commands.
#[cfg(unix)]
pub async fn run_stdio_proxy(socket: PathBuf) -> std::io::Result<()> {
    use tokio::io::{stdin, stdout};

    let stream = connect_mcp_socket(&socket).await?;
    let (mut read_sock, mut write_sock) = stream.into_split();
    let mut stdin = stdin();
    let mut stdout = stdout();
    let to_socket = tokio::io::copy(&mut stdin, &mut write_sock);
    let from_socket = tokio::io::copy(&mut read_sock, &mut stdout);
    tokio::select! {
        r = to_socket => {
            r?;
        }
        r = from_socket => {
            r?;
        }
    }
    Ok(())
}

#[cfg(unix)]
async fn connect_mcp_socket(socket: &Path) -> std::io::Result<tokio::net::UnixStream> {
    connect_mcp_socket_in(socket, &std::env::temp_dir()).await
}

#[cfg(unix)]
async fn connect_mcp_socket_in(
    socket: &Path,
    fallback_dir: &Path,
) -> std::io::Result<tokio::net::UnixStream> {
    match tokio::net::UnixStream::connect(socket).await {
        Ok(stream) => Ok(stream),
        Err(primary) if should_try_mcp_socket_fallback(&primary) => {
            for candidate in mcp_socket_fallback_candidates_in(fallback_dir, socket) {
                match tokio::net::UnixStream::connect(&candidate).await {
                    Ok(stream) => {
                        tracing::warn!(
                            configured = %socket.display(),
                            fallback = %candidate.display(),
                            "configured MCP socket is stale; using live fallback"
                        );
                        return Ok(stream);
                    }
                    Err(_) => continue,
                }
            }
            Err(primary)
        }
        Err(primary) => Err(primary),
    }
}

#[cfg(unix)]
fn should_try_mcp_socket_fallback(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
    )
}

#[cfg(unix)]
fn mcp_socket_fallback_candidates_in(dir: &Path, preferred: &Path) -> Vec<PathBuf> {
    let read_dir = match std::fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(_) => return Vec::new(),
    };
    let mut candidates: Vec<(std::time::SystemTime, PathBuf)> = read_dir
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path == preferred {
                return None;
            }
            let name = path.file_name()?.to_str()?;
            if !name.starts_with("chan-mcp-") || !name.ends_with(".sock") {
                return None;
            }
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            Some((modified, path))
        })
        .collect();
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    candidates.into_iter().map(|(_, path)| path).collect()
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
/// current workspace Arc.
#[cfg(unix)]
pub fn start<DF>(socket_path: PathBuf, workspace_for: DF) -> std::io::Result<BridgeHandle>
where
    DF: Fn() -> Option<Arc<chan_workspace::Workspace>> + Send + Sync + 'static,
{
    // Stale socket from a previous run that didn't get to clean up
    // (kill -9, panic in Drop): unlink so bind doesn't EADDRINUSE.
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;
    let workspace_for = Arc::new(workspace_for);

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
            let Some(workspace) = workspace_for() else {
                tracing::warn!("mcp bridge session refused: workspace state unavailable");
                continue;
            };
            tokio::spawn(async move {
                let (read, write) = stream.into_split();
                let server = chan_llm::mcp::Server::new(workspace);
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
pub fn start<DF>(_socket_path: PathBuf, _workspace_for: DF) -> std::io::Result<BridgeHandle>
where
    DF: Fn() -> Option<Arc<chan_workspace::Workspace>> + Send + Sync + 'static,
{
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "mcp bridge requires unix-domain sockets",
    ))
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn proxy_connect_falls_back_to_live_socket_when_configured_socket_is_stale() {
        let dir = tempfile::tempdir().unwrap();
        let preferred = dir.path().join("chan-mcp-stale.sock");
        let live = dir
            .path()
            .join(format!("chan-mcp-{}-fallback.sock", std::process::id()));
        let listener = tokio::net::UnixListener::bind(&live).unwrap();
        let accept = tokio::spawn(async move {
            let _ = listener.accept().await.unwrap();
        });

        let stream = connect_mcp_socket_in(&preferred, dir.path()).await.unwrap();
        drop(stream);
        accept.await.unwrap();
    }
}
