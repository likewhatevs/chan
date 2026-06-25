//! CLI-to-devserver workspace registration over a well-known per-user
//! Unix socket.
//!
//! When a devserver is running on a box and the user types `chan open
//! ~/notes` in a terminal there, the natural intent is "add this workspace
//! to the devserver," not "bind a second standalone server that fights the
//! devserver for the workspace flock." This module is the same-user IPC
//! channel that makes that registration possible.
//!
//! Discovery is a WELL-KNOWN per-user endpoint (not the per-pid socket the
//! mcp_bridge / control_socket use): the CLI finds the devserver without
//! knowing its pid. It is a SECOND endpoint alongside the desktop handoff
//! (`chan-desktop.sock`); a box can run a devserver, a desktop, both, or
//! neither. Same-user is enforced by the socket living in a per-user
//! runtime dir with 0600 perms owned by the user, matching
//! [`crate::handoff`].
//!
//! INVARIANT: exactly one process owns a workspace's writes (the
//! per-workspace flock). On a successful registration the DEVSERVER opens
//! and owns the workspace; the CLI is a launcher that exits WITHOUT opening
//! it. The CLI must therefore consult this module BEFORE it calls
//! `open_workspace`, so it never double-opens.
//!
//! The listener and client use Unix-domain sockets on unix and a named pipe on
//! Windows (mirroring [`crate::handoff`]); on any other target the types still
//! compile but discovery resolves to "no devserver" so the CLI keeps its
//! standalone behavior.

use serde::{Deserialize, Serialize};
#[cfg(any(unix, windows))]
use std::path::Path;
use std::path::PathBuf;
#[cfg(any(unix, windows))]
use std::time::Duration;

/// Wire-protocol version of the registration RPC. The CLI and devserver
/// compare it in the handshake; a mismatch means NO registration (the CLI
/// falls back to standalone) rather than a silent decode of an unknown
/// shape. Independent of the management API's `DEVSERVER_API_PROTOCOL` and
/// of the desktop handoff's `PROTOCOL_VERSION`.
pub const PROTOCOL_VERSION: u32 = 1;

/// Human-facing crate version, baked at compile time. Carried in the
/// handshake so a skew message names concrete versions.
pub const CHAN_VERSION: &str = env!("CARGO_PKG_VERSION");

/// CLI to devserver request. `tag = "type"` mirrors [`crate::handoff`], so
/// the on-wire shape is `{"type":"register_workspace", ...}`. The
/// `protocol` + `cli_version` handshake fields gate dispatch before the
/// devserver acts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    /// Ask the devserver to mount the given workspace path and report the
    /// prefix it landed at. Idempotent devserver-side: an already-mounted
    /// root returns its existing prefix.
    RegisterWorkspace {
        protocol: u32,
        cli_version: String,
        /// The workspace root the CLI was asked to serve, sent as a string
        /// for stable JSON across platforms.
        workspace_path: String,
    },
}

impl Request {
    /// The handshake protocol version carried by any request variant.
    pub fn protocol(&self) -> u32 {
        match self {
            Request::RegisterWorkspace { protocol, .. } => *protocol,
        }
    }

    /// The CLI's human version, for skew logging.
    pub fn cli_version(&self) -> &str {
        match self {
            Request::RegisterWorkspace { cli_version, .. } => cli_version,
        }
    }
}

/// Devserver to CLI response. `tag = "type"` mirrors the request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    /// The devserver mounted (or already had) the workspace at `prefix`.
    /// The CLI prints a note and exits; the devserver owns the flock.
    Registered {
        devserver_version: String,
        prefix: String,
    },
    /// The devserver speaks a different protocol version. The CLI does NOT
    /// register; it prints the skew and falls back to standalone.
    VersionSkew {
        devserver_version: String,
        devserver_protocol: u32,
    },
    /// The devserver could not mount the workspace (e.g. a runtime error).
    /// The CLI logs the reason and falls back to standalone.
    Error { message: String },
}

/// Outcome of a registration attempt as the CLI resolves it. Not on the
/// wire: every non-`Registered` variant maps to "own the server exactly as
/// a standalone serve," with a distinct variant so the CLI prints the right
/// note.
#[derive(Debug)]
pub enum Outcome {
    /// The devserver mounted the workspace at `prefix`. The CLI exits 0
    /// without opening it.
    Registered { prefix: String },
    /// No devserver discovered: no socket, connect refused, stale socket,
    /// or any I/O error before a valid response. Own the server standalone.
    NoDevserver,
    /// The devserver is a different protocol version. Fall back to
    /// standalone after printing the skew.
    VersionSkew,
    /// The devserver answered but refused/failed. Fall back to standalone
    /// after logging the message.
    Error(String),
}

/// Resolve the well-known per-user devserver socket path. Prefers
/// `$XDG_RUNTIME_DIR/chan-devserver.sock` (a per-user dir the OS already
/// 0700s on Linux); falls back to `<tmp>/chan-devserver-<uid>.sock` on
/// macOS, which has no `XDG_RUNTIME_DIR`. The name is kept short for the
/// macOS `sun_path` 104-byte limit. Returns `None` off unix.
pub fn well_known_devserver_socket_path() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
            let dir = PathBuf::from(dir);
            if !dir.as_os_str().is_empty() {
                return Some(dir.join("chan-devserver.sock"));
            }
        }
        // macOS / no-XDG fallback. Per-uid filename so two users on one
        // machine do not collide in a shared /tmp; same-user is still
        // enforced by 0600 + ownership.
        let uid = current_uid();
        Some(std::env::temp_dir().join(format!("chan-devserver-{uid}.sock")))
    }
    #[cfg(windows)]
    {
        // Named pipes share one machine-global namespace, so namespace by user
        // to avoid cross-user collision (same-user access is the default pipe
        // ACL). WELL-KNOWN per-user (not per-pid) so the CLI finds the running
        // devserver without its pid, mirroring `handoff::well_known_socket_path`.
        let user = std::env::var("USERNAME").unwrap_or_default();
        let user: String = user
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        let user = if user.is_empty() {
            "default".into()
        } else {
            user
        };
        Some(PathBuf::from(format!(r"\\.\pipe\chan-devserver-{user}")))
    }
    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

#[cfg(unix)]
fn current_uid() -> u32 {
    // rustix is already a chan-server dep; reuse it rather than raw libc.
    rustix::process::getuid().as_raw()
}

/// Explicit opt-out for automation: `CHAN_NO_DEVSERVER_HANDOFF=1` forces
/// standalone even when a devserver is running. Any non-empty, non-"0"
/// value counts as set.
pub fn devserver_handoff_opt_out() -> bool {
    match std::env::var("CHAN_NO_DEVSERVER_HANDOFF") {
        Ok(v) => !v.is_empty() && v != "0",
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Devserver side: listener on the well-known socket.
// ---------------------------------------------------------------------------

/// Handle owning the registration listener. Drop aborts the accept loop and
/// unlinks the socket file, mirroring [`crate::handoff`]. A `kill -9` that
/// skips Drop leaves a stale file; the next bind unlinks it first.
#[cfg(any(unix, windows))]
pub struct ListenerHandle {
    socket_path: PathBuf,
    accept_loop: Option<tokio::task::JoinHandle<()>>,
}

#[cfg(not(any(unix, windows)))]
pub struct ListenerHandle {
    socket_path: PathBuf,
}

impl ListenerHandle {
    pub fn socket_path(&self) -> &std::path::Path {
        &self.socket_path
    }
}

#[cfg(any(unix, windows))]
impl Drop for ListenerHandle {
    fn drop(&mut self) {
        if let Some(h) = self.accept_loop.take() {
            h.abort();
        }
        // A Unix socket leaves a filesystem node to unlink; a Windows named
        // pipe vanishes when the owning process exits, so nothing to remove.
        #[cfg(unix)]
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

/// Bind the well-known socket and spawn an accept loop. Each connection
/// carries one [`Request`]; the devserver's `handler` returns the
/// [`Response`] and the connection closes. The listener applies the
/// protocol-version gate before calling `handler`, so the handler only ever
/// sees protocol-valid requests; a skew becomes [`Response::VersionSkew`]
/// without invoking it.
///
/// The socket is chmod 0600 immediately after bind so only the owning user
/// can connect (defense in depth on top of the per-user directory). Must be
/// called from within a tokio runtime.
#[cfg(unix)]
pub fn start_listener<F, Fut>(socket_path: PathBuf, handler: F) -> std::io::Result<ListenerHandle>
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Response> + Send + 'static,
{
    use std::os::unix::fs::PermissionsExt;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixListener;

    // Unlink any stale socket from a previous run that did not clean up
    // (kill -9, panic in Drop) so bind does not EADDRINUSE.
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;
    // Lock the socket to the owning user. Best-effort: a chmod failure does
    // not abort the listener; the per-user directory is the primary boundary.
    let _ = std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600));

    let handler = std::sync::Arc::new(handler);
    let accept_loop = tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(pair) => pair,
                Err(e) => {
                    tracing::warn!("devserver registration accept: {e}");
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
            };
            let handler = handler.clone();
            tokio::spawn(async move {
                let (read, mut write) = stream.into_split();
                let mut reader = BufReader::new(read);
                let mut line = String::new();
                let response = match reader.read_line(&mut line).await {
                    Ok(0) => Response::Error {
                        message: "empty registration request".into(),
                    },
                    Ok(_) => match serde_json::from_str::<Request>(&line) {
                        Ok(req) => dispatch(req, handler.as_ref()).await,
                        Err(e) => Response::Error {
                            message: format!("invalid registration request: {e}"),
                        },
                    },
                    Err(e) => Response::Error {
                        message: format!("read registration request: {e}"),
                    },
                };
                if let Ok(mut out) = serde_json::to_vec(&response) {
                    out.push(b'\n');
                    let _ = write.write_all(&out).await;
                }
            });
        }
    });

    Ok(ListenerHandle {
        socket_path,
        accept_loop: Some(accept_loop),
    })
}

/// Windows: bind the well-known named pipe and serve registration requests,
/// mirroring [`crate::handoff::start_listener`]'s Windows arm. The first
/// instance owns the name (`first_pipe_instance(true)`); each accept re-arms a
/// fresh instance BEFORE serving so a client arriving during the swap still
/// finds a live server. Same one-line-JSON framing + `dispatch` version gate as
/// the unix arm.
#[cfg(windows)]
pub fn start_listener<F, Fut>(socket_path: PathBuf, handler: F) -> std::io::Result<ListenerHandle>
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Response> + Send + 'static,
{
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::windows::named_pipe::ServerOptions;

    let pipe_name = socket_path.as_os_str().to_owned();
    let mut next = ServerOptions::new()
        .first_pipe_instance(true)
        .create(&pipe_name)?;

    let handler = std::sync::Arc::new(handler);
    let accept_loop = tokio::spawn(async move {
        loop {
            if let Err(e) = next.connect().await {
                tracing::warn!("devserver registration accept: {e}");
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
            // Re-arm BEFORE serving so the next client doesn't race to NotFound.
            let fresh = match ServerOptions::new().create(&pipe_name) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("devserver registration re-arm: {e}");
                    break;
                }
            };
            let connected = std::mem::replace(&mut next, fresh);
            let handler = handler.clone();
            tokio::spawn(async move {
                let (read, mut write) = tokio::io::split(connected);
                let mut reader = BufReader::new(read);
                let mut line = String::new();
                let response = match reader.read_line(&mut line).await {
                    Ok(0) => Response::Error {
                        message: "empty registration request".into(),
                    },
                    Ok(_) => match serde_json::from_str::<Request>(&line) {
                        Ok(req) => dispatch(req, handler.as_ref()).await,
                        Err(e) => Response::Error {
                            message: format!("invalid registration request: {e}"),
                        },
                    },
                    Err(e) => Response::Error {
                        message: format!("read registration request: {e}"),
                    },
                };
                if let Ok(mut out) = serde_json::to_vec(&response) {
                    out.push(b'\n');
                    let _ = write.write_all(&out).await;
                }
            });
        }
    });

    Ok(ListenerHandle {
        socket_path,
        accept_loop: Some(accept_loop),
    })
}

#[cfg(not(any(unix, windows)))]
pub fn start_listener<F, Fut>(_socket_path: PathBuf, _handler: F) -> std::io::Result<ListenerHandle>
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Response> + Send + 'static,
{
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "devserver registration listener requires unix-domain sockets or windows named pipes",
    ))
}

/// Apply the protocol-version gate, then call the devserver's async
/// `handler`. A skew short-circuits to [`Response::VersionSkew`] and the
/// handler never runs, so the devserver never acts on a request it cannot
/// fully understand.
#[cfg(any(unix, windows))]
async fn dispatch<F, Fut>(req: Request, handler: &F) -> Response
where
    F: Fn(Request) -> Fut,
    Fut: std::future::Future<Output = Response>,
{
    if req.protocol() != PROTOCOL_VERSION {
        tracing::info!(
            cli_version = %req.cli_version(),
            cli_protocol = req.protocol(),
            "devserver registration refused: protocol skew",
        );
        return Response::VersionSkew {
            devserver_version: CHAN_VERSION.into(),
            devserver_protocol: PROTOCOL_VERSION,
        };
    }
    handler(req).await
}

// ---------------------------------------------------------------------------
// CLI side: discover + request registration.
// ---------------------------------------------------------------------------

/// Try to register `workspace_path` with a running same-user devserver.
/// Connects the well-known socket, sends a [`Request::RegisterWorkspace`],
/// and parses the response. Any connect failure / stale socket / read error
/// / malformed reply maps to [`Outcome::NoDevserver`], so the CLI behaves
/// exactly like today when no devserver is present.
///
/// A short connect+IO timeout bounds the case where a stale socket file
/// exists but nothing is accepting; the CLI must not hang on a dead
/// devserver.
#[cfg(unix)]
pub async fn try_register_devserver(workspace_path: &Path) -> Outcome {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;

    let Some(socket_path) = well_known_devserver_socket_path() else {
        return Outcome::NoDevserver;
    };
    // No socket file at all is the common no-devserver case; skip the
    // connect attempt (and its log noise) entirely.
    if !socket_path.exists() {
        return Outcome::NoDevserver;
    }

    let connect = UnixStream::connect(&socket_path);
    let stream = match tokio::time::timeout(Duration::from_millis(1500), connect).await {
        Ok(Ok(s)) => s,
        // Refused / stale socket / timeout: no live devserver.
        Ok(Err(_)) | Err(_) => return Outcome::NoDevserver,
    };

    let req = Request::RegisterWorkspace {
        protocol: PROTOCOL_VERSION,
        cli_version: CHAN_VERSION.into(),
        workspace_path: workspace_path.display().to_string(),
    };
    let mut payload = match serde_json::to_vec(&req) {
        Ok(v) => v,
        Err(_) => return Outcome::NoDevserver,
    };
    payload.push(b'\n');

    let (read, mut write) = stream.into_split();
    let io = async {
        write.write_all(&payload).await?;
        write.flush().await?;
        let mut reader = BufReader::new(read);
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        Ok::<String, std::io::Error>(line)
    };
    let line = match tokio::time::timeout(Duration::from_millis(3000), io).await {
        Ok(Ok(line)) if !line.trim().is_empty() => line,
        // Write/read error, empty reply, or timeout: treat as no usable
        // devserver and fall back rather than hang or error.
        _ => return Outcome::NoDevserver,
    };

    match serde_json::from_str::<Response>(&line) {
        Ok(Response::Registered { prefix, .. }) => Outcome::Registered { prefix },
        Ok(Response::VersionSkew { .. }) => Outcome::VersionSkew,
        Ok(Response::Error { message }) => Outcome::Error(message),
        // A reply we cannot parse: fall back rather than guess.
        Err(_) => Outcome::NoDevserver,
    }
}

/// Windows: the same registration round-trip over the well-known named pipe,
/// mirroring [`crate::handoff::try_open_devserver`]'s Windows arm. A missing
/// pipe maps to [`Outcome::NoDevserver`] at once; a momentarily-busy pipe gets
/// a short bounded retry. Any other failure also falls back to standalone.
#[cfg(windows)]
pub async fn try_register_devserver(workspace_path: &Path) -> Outcome {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::windows::named_pipe::ClientOptions;

    const ERROR_PIPE_BUSY: i32 = 231;

    let Some(socket_path) = well_known_devserver_socket_path() else {
        return Outcome::NoDevserver;
    };

    let deadline = std::time::Instant::now() + Duration::from_millis(1500);
    let client = loop {
        match ClientOptions::new().open(&socket_path) {
            Ok(c) => break c,
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) => {
                if std::time::Instant::now() >= deadline {
                    return Outcome::NoDevserver;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            // No pipe (no devserver) or any other open error: standalone.
            Err(_) => return Outcome::NoDevserver,
        }
    };

    let req = Request::RegisterWorkspace {
        protocol: PROTOCOL_VERSION,
        cli_version: CHAN_VERSION.into(),
        workspace_path: workspace_path.display().to_string(),
    };
    let mut payload = match serde_json::to_vec(&req) {
        Ok(v) => v,
        Err(_) => return Outcome::NoDevserver,
    };
    payload.push(b'\n');

    let (read, mut write) = tokio::io::split(client);
    let io = async {
        write.write_all(&payload).await?;
        write.flush().await?;
        let mut reader = BufReader::new(read);
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        Ok::<String, std::io::Error>(line)
    };
    let line = match tokio::time::timeout(Duration::from_millis(3000), io).await {
        Ok(Ok(line)) if !line.trim().is_empty() => line,
        _ => return Outcome::NoDevserver,
    };

    match serde_json::from_str::<Response>(&line) {
        Ok(Response::Registered { prefix, .. }) => Outcome::Registered { prefix },
        Ok(Response::VersionSkew { .. }) => Outcome::VersionSkew,
        Ok(Response::Error { message }) => Outcome::Error(message),
        Err(_) => Outcome::NoDevserver,
    }
}

#[cfg(not(any(unix, windows)))]
pub async fn try_register_devserver(_workspace_path: &std::path::Path) -> Outcome {
    Outcome::NoDevserver
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips() {
        let req = Request::RegisterWorkspace {
            protocol: PROTOCOL_VERSION,
            cli_version: "9.9.9".into(),
            workspace_path: "/tmp/notes".into(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"type\":\"register_workspace\""));
        let back: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(req, back);
        assert_eq!(req.protocol(), PROTOCOL_VERSION);
        assert_eq!(req.cli_version(), "9.9.9");
    }

    #[test]
    fn response_round_trips() {
        let registered = Response::Registered {
            devserver_version: CHAN_VERSION.into(),
            prefix: "/api/notes-1a2b3c".into(),
        };
        let json = serde_json::to_string(&registered).unwrap();
        assert!(json.contains("\"type\":\"registered\""));
        assert_eq!(registered, serde_json::from_str::<Response>(&json).unwrap());

        let skew = Response::VersionSkew {
            devserver_version: "0.1.0".into(),
            devserver_protocol: 99,
        };
        let json = serde_json::to_string(&skew).unwrap();
        assert!(json.contains("\"type\":\"version_skew\""));
        assert_eq!(skew, serde_json::from_str::<Response>(&json).unwrap());
    }

    #[test]
    fn opt_out_parsing() {
        let prev = std::env::var_os("CHAN_NO_DEVSERVER_HANDOFF");
        std::env::remove_var("CHAN_NO_DEVSERVER_HANDOFF");
        assert!(!devserver_handoff_opt_out());
        std::env::set_var("CHAN_NO_DEVSERVER_HANDOFF", "1");
        assert!(devserver_handoff_opt_out());
        std::env::set_var("CHAN_NO_DEVSERVER_HANDOFF", "0");
        assert!(!devserver_handoff_opt_out());
        std::env::set_var("CHAN_NO_DEVSERVER_HANDOFF", "");
        assert!(!devserver_handoff_opt_out());
        match prev {
            Some(v) => std::env::set_var("CHAN_NO_DEVSERVER_HANDOFF", v),
            None => std::env::remove_var("CHAN_NO_DEVSERVER_HANDOFF"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn well_known_path_is_some_on_unix() {
        let p = well_known_devserver_socket_path().expect("unix path");
        let s = p.to_string_lossy();
        assert!(s.contains("chan-devserver"), "unexpected path: {s}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn dispatch_rejects_protocol_skew() {
        let req = Request::RegisterWorkspace {
            protocol: PROTOCOL_VERSION + 1,
            cli_version: "9.9.9".into(),
            workspace_path: "/tmp/notes".into(),
        };
        // The handler returns a distinctive Registered; getting VersionSkew
        // back proves dispatch short-circuited and never ran the handler.
        let handler = |_r: Request| async {
            Response::Registered {
                devserver_version: CHAN_VERSION.into(),
                prefix: "/api/should-not-happen".into(),
            }
        };
        match dispatch(req, &handler).await {
            Response::VersionSkew {
                devserver_protocol, ..
            } => assert_eq!(devserver_protocol, PROTOCOL_VERSION),
            other => panic!("expected VersionSkew, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn listener_round_trip_registered() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("reg.sock");
        let _handle = start_listener(sock.clone(), move |req| async move {
            match req {
                Request::RegisterWorkspace { workspace_path, .. } => {
                    assert_eq!(workspace_path, "/tmp/notes");
                    Response::Registered {
                        devserver_version: CHAN_VERSION.into(),
                        prefix: "/api/notes-1a2b3c".into(),
                    }
                }
            }
        })
        .unwrap();

        // The socket should be 0600.
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&sock).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "socket perms should be 0600");

        let resp = request_over(&sock, "/tmp/notes").await;
        match resp {
            Response::Registered { prefix, .. } => assert_eq!(prefix, "/api/notes-1a2b3c"),
            other => panic!("expected Registered, got {other:?}"),
        }
    }

    /// Connect directly to `sock` and round-trip one RegisterWorkspace.
    /// Mirrors try_register_devserver's wire framing but targets an explicit
    /// socket so the test does not depend on the well-known path.
    #[cfg(unix)]
    async fn request_over(sock: &std::path::Path, workspace: &str) -> Response {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::UnixStream;

        let req = Request::RegisterWorkspace {
            protocol: PROTOCOL_VERSION,
            cli_version: CHAN_VERSION.into(),
            workspace_path: workspace.into(),
        };
        let stream = UnixStream::connect(sock).await.unwrap();
        let mut payload = serde_json::to_vec(&req).unwrap();
        payload.push(b'\n');
        let (read, mut write) = stream.into_split();
        write.write_all(&payload).await.unwrap();
        write.flush().await.unwrap();
        let mut reader = BufReader::new(read);
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        serde_json::from_str(&line).unwrap()
    }
}
