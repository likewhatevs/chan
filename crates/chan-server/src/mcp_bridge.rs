//! In-process MCP server exposed over a Unix-domain socket.
//!
//! chan-llm's gemini_cli / claude_cli backends both want to launch
//! the chan MCP server as a subprocess so the agent's writes round-
//! trip through chan-drive's gates. The original wiring spawned
//! `chan __mcp <drive_root>`, which then called `Library::open_drive`
//! a second time. chan-drive holds a per-drive flock for single-
//! writer ownership, so the child failed with `DriveLocked` and the
//! agent silently fell back to its native (un-sandboxed) tools.
//!
//! The bridge resolves that conflict: chan-server already owns an
//! `Arc<Drive>` for the drive it serves, so the MCP service is run
//! in-process. Each agent session connects through `chan __mcp-proxy`
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
use tokio::net::UnixListener;
use tokio::task::JoinHandle;

/// Pick a unique socket path under the system tmp dir. macOS caps
/// `sun_path` at 104 bytes, so the suffix is short and the directory
/// short; `/tmp/chan-mcp-<pid>-<8 hex>.sock` fits well within that.
pub fn pick_socket_path() -> PathBuf {
    let mut bytes = [0u8; 4];
    rand::thread_rng().fill_bytes(&mut bytes);
    let suffix: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    std::env::temp_dir().join(format!("chan-mcp-{}-{}.sock", std::process::id(), suffix))
}

/// Bridge handle returned from `start`. Drop = abort the accept loop
/// and unlink the socket file. Held by `AppState` for the lifetime
/// of the chan-server process.
pub struct BridgeHandle {
    socket_path: PathBuf,
    accept_loop: Option<JoinHandle<()>>,
}

impl BridgeHandle {
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }
}

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
/// current drive Arc and the live `auto_apply_writes` setting (read
/// at connect time so the user can toggle it mid-session and the
/// next agent turn picks up the change).
pub fn start<DF, AF>(
    socket_path: PathBuf,
    drive_for: DF,
    auto_apply_for: AF,
) -> std::io::Result<BridgeHandle>
where
    DF: Fn() -> Arc<chan_drive::Drive> + Send + Sync + 'static,
    AF: Fn() -> bool + Send + Sync + 'static,
{
    // Stale socket from a previous run that didn't get to clean up
    // (kill -9, panic in Drop): unlink so bind doesn't EADDRINUSE.
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;
    let drive_for = Arc::new(drive_for);
    let auto_apply_for = Arc::new(auto_apply_for);

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
            let drive = drive_for();
            let auto_apply = auto_apply_for();
            tokio::spawn(async move {
                let (read, write) = stream.into_split();
                let server = chan_llm::mcp::Server::new(drive, auto_apply);
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
