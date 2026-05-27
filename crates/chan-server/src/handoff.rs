//! macOS CLI-to-desktop workspace handoff over a well-known per-user
//! Unix-domain socket.
//!
//! When chan-desktop is running and the user types `chan serve
//! ~/notes` in a terminal, the natural intent is "show me this workspace
//! in the app," not "fail because the desktop already holds the
//! per-workspace flock." This module is the same-user IPC channel that
//! makes that handoff possible.
//!
//! Discovery is a WELL-KNOWN per-user socket path (not the per-pid
//! socket the mcp_bridge / control_socket use): the CLI must find the
//! desktop without knowing its pid. Same-user is enforced by the
//! socket living in a per-user runtime dir with 0600 perms and owned
//! by the user; cross-user attach is simply not discoverable.
//!
//! INVARIANT: exactly one process owns a workspace's writes (the
//! chan-workspace per-workspace flock). In a successful handoff the DESKTOP
//! owns the workspace; the CLI is a launcher that exits WITHOUT opening
//! the workspace. The CLI must therefore consult this module BEFORE it
//! calls `open_workspace`, so it never double-opens.
//!
//! The bearer token never travels over argv/env/logs: in Option B
//! (the ratified design) the desktop spawns its OWN native window
//! against its OWN embedded server, so no token crosses this socket
//! at all. The protocol still carries a version field so a
//! desktop-vs-CLI skew falls back to standalone rather than doing
//! silent cross-version IPC.
//!
//! Reuses the control_socket.rs shape: line-delimited JSON request +
//! response, unlink-stale-before-bind, a Drop guard that unlinks on
//! teardown. The protocol/types compile on every platform; the
//! listener + client are unix-only (handoff is local-desktop only and
//! the desktop ships on unix targets today).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use std::time::Duration;

/// Wire-protocol version. Bump on any incompatible change to the
/// request/response shape. The CLI and desktop compare this in the
/// handshake; a mismatch means NO handoff (fall back to standalone),
/// never a silent best-effort decode of an unknown shape.
pub const PROTOCOL_VERSION: u32 = 1;

/// Human-facing crate version, baked at compile time. Carried in the
/// handshake so the skew message can name concrete versions ("desktop
/// is X, CLI is Y"). Distinct from PROTOCOL_VERSION: two builds can
/// share a protocol while differing in crate version.
pub const CHAN_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Capabilities the desktop advertises. Reserved for forward
/// compatibility: a request the desktop can't satisfy falls back to
/// standalone rather than erroring. Today the only capability is
/// opening a LOCAL workspace window; tunneled-workspace handoff is out of
/// scope (the CLI's `--tunnel-*` path already forces standalone).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Capabilities {
    /// The desktop can open a local registry workspace in a native
    /// window. Always true for a desktop that speaks this protocol;
    /// the field exists so a future desktop can advertise FALSE (e.g.
    /// a headless build) and the CLI falls back cleanly.
    pub open_local_workspace: bool,
}

/// CLI -> desktop request. `tag = "type"` mirrors control_socket so
/// the on-wire shape is `{"type":"open_workspace", ...}`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    /// Ask the desktop to open the given workspace path in a native
    /// window. `protocol` and `cli_version` are the handshake fields;
    /// the desktop checks `protocol` against its own PROTOCOL_VERSION
    /// before acting.
    OpenWorkspace {
        protocol: u32,
        cli_version: String,
        /// The workspace root the CLI was asked to serve. The desktop
        /// canonicalizes + registers it the same way its own
        /// open-local-workspace path does. Sent as a string for stable
        /// JSON across platforms.
        workspace_path: String,
    },
}

/// Desktop -> CLI response. `tag = "status"` mirrors
/// control_socket's `ControlResponse`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Response {
    /// The desktop accepted the request and is opening (or has
    /// raised) the workspace window. The CLI prints a short note and
    /// exits; the desktop owns the workspace lifecycle from here.
    Opened {
        desktop_version: String,
        capabilities: Capabilities,
    },
    /// The desktop speaks a different protocol version. The CLI does
    /// NOT hand off; it prints the skew and falls back to standalone.
    VersionSkew {
        desktop_version: String,
        desktop_protocol: u32,
    },
    /// The desktop could not open the workspace (e.g. a runtime error
    /// mounting it). The CLI logs the reason and falls back to
    /// standalone rather than leaving the user with nothing.
    Error { message: String },
}

/// Resolve the well-known per-user socket path. Prefers
/// `$XDG_RUNTIME_DIR/chan-desktop.sock` (a per-user dir the OS
/// already 0700s on Linux); falls back to `<tmp>/chan-desktop-<uid>.sock`
/// on macOS, which has no XDG_RUNTIME_DIR. The name is kept short for
/// the macOS `sun_path` 104-byte limit. Returns None on non-unix.
pub fn well_known_socket_path() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
            let dir = PathBuf::from(dir);
            if !dir.as_os_str().is_empty() {
                return Some(dir.join("chan-desktop.sock"));
            }
        }
        // macOS / no-XDG fallback. Per-uid filename so two users on
        // one machine don't collide in a shared /tmp; same-user is
        // still enforced by 0600 + ownership.
        let uid = current_uid();
        Some(std::env::temp_dir().join(format!("chan-desktop-{uid}.sock")))
    }
    #[cfg(not(unix))]
    {
        None
    }
}

#[cfg(unix)]
fn current_uid() -> u32 {
    // rustix is already a chan-server dep (terminal_sessions uses its
    // getrlimit); reuse it here rather than reaching for raw libc.
    rustix::process::getuid().as_raw()
}

/// True when a GUI session is present, i.e. it makes sense to hand a
/// workspace to a desktop the user can actually see. On macOS the desktop
/// session is always present for an interactive login; the headless
/// signal we guard against is an SSH session with no display. On
/// Linux we additionally require DISPLAY or WAYLAND_DISPLAY.
///
/// Conservative by design: when unsure we return false so the CLI
/// keeps the load-bearing standalone behavior rather than handing a
/// workspace to a desktop nobody can see.
pub fn gui_session_present() -> bool {
    #[cfg(target_os = "macos")]
    {
        // A remote SSH login sets SSH_CONNECTION / SSH_TTY and has no
        // Aqua session the user can interact with; skip handoff there.
        if std::env::var_os("SSH_CONNECTION").is_some()
            || std::env::var_os("SSH_TTY").is_some()
            || std::env::var_os("SSH_CLIENT").is_some()
        {
            return false;
        }
        true
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if std::env::var_os("SSH_CONNECTION").is_some()
            || std::env::var_os("SSH_TTY").is_some()
            || std::env::var_os("SSH_CLIENT").is_some()
        {
            return false;
        }
        std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// Explicit opt-out for automation: `CHAN_NO_DESKTOP_HANDOFF=1`
/// forces standalone even in a GUI session with a desktop running.
/// Any non-empty, non-"0" value counts as set.
pub fn handoff_opt_out() -> bool {
    match std::env::var("CHAN_NO_DESKTOP_HANDOFF") {
        Ok(v) => !v.is_empty() && v != "0",
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Desktop side: listener on the well-known socket.
// ---------------------------------------------------------------------------

/// Handle owning the handoff listener. Drop = abort the accept loop
/// and unlink the socket file, mirroring control_socket / mcp_bridge.
/// A `kill -9` that skips Drop leaves a stale file; the next bind
/// unlinks it first.
#[cfg(unix)]
pub struct ListenerHandle {
    socket_path: PathBuf,
    accept_loop: Option<tokio::task::JoinHandle<()>>,
}

#[cfg(not(unix))]
pub struct ListenerHandle {
    socket_path: PathBuf,
}

impl ListenerHandle {
    pub fn socket_path(&self) -> &std::path::Path {
        &self.socket_path
    }
}

#[cfg(unix)]
impl Drop for ListenerHandle {
    fn drop(&mut self) {
        if let Some(h) = self.accept_loop.take() {
            h.abort();
        }
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

/// Bind the well-known socket and spawn an accept loop. Each
/// connection carries one `Request`; the desktop responds with a
/// `Response` and closes. `open_workspace` is the desktop callback that
/// spawns/raises the native window for the requested workspace path; it
/// returns `Ok(())` on success or `Err(message)` which the CLI sees
/// as `Response::Error` and falls back to standalone.
///
/// The socket is chmod 0600 immediately after bind so only the
/// owning user can connect (defense in depth on top of the per-user
/// directory). Must be called from within a tokio runtime.
#[cfg(unix)]
pub fn start_listener<F>(socket_path: PathBuf, open_workspace: F) -> std::io::Result<ListenerHandle>
where
    F: Fn(PathBuf) -> Result<(), String> + Send + Sync + 'static,
{
    use std::os::unix::fs::PermissionsExt;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixListener;

    // Unlink any stale socket from a previous run that didn't clean up
    // (kill -9, panic in Drop) so bind doesn't EADDRINUSE.
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;
    // Lock the socket to the owning user. Best-effort: a chmod failure
    // (exotic fs) does not abort the listener, but the per-user
    // directory placement is the primary boundary anyway.
    let _ = std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600));

    let open_workspace = std::sync::Arc::new(open_workspace);
    let accept_loop = tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(pair) => pair,
                Err(e) => {
                    tracing::warn!("handoff accept: {e}");
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
            };
            let open_workspace = open_workspace.clone();
            tokio::spawn(async move {
                let (read, mut write) = stream.into_split();
                let mut reader = BufReader::new(read);
                let mut line = String::new();
                let response = match reader.read_line(&mut line).await {
                    Ok(0) => Response::Error {
                        message: "empty handoff request".into(),
                    },
                    Ok(_) => match serde_json::from_str::<Request>(&line) {
                        Ok(req) => handle_request(req, open_workspace.as_ref()),
                        Err(e) => Response::Error {
                            message: format!("invalid handoff request: {e}"),
                        },
                    },
                    Err(e) => Response::Error {
                        message: format!("read handoff request: {e}"),
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

#[cfg(not(unix))]
pub fn start_listener<F>(
    _socket_path: PathBuf,
    _open_workspace: F,
) -> std::io::Result<ListenerHandle>
where
    F: Fn(PathBuf) -> Result<(), String> + Send + Sync + 'static,
{
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "handoff listener requires unix-domain sockets",
    ))
}

/// Apply the protocol-version gate, then dispatch to the desktop's
/// open-workspace callback. Kept synchronous: the callback itself queues
/// the window spawn onto the desktop's app handle.
#[cfg(unix)]
fn handle_request<F>(req: Request, open_workspace: &F) -> Response
where
    F: Fn(PathBuf) -> Result<(), String>,
{
    match req {
        Request::OpenWorkspace {
            protocol,
            cli_version,
            workspace_path,
        } => {
            if protocol != PROTOCOL_VERSION {
                tracing::info!(
                    cli_version = %cli_version,
                    cli_protocol = protocol,
                    "handoff refused: protocol skew",
                );
                return Response::VersionSkew {
                    desktop_version: CHAN_VERSION.into(),
                    desktop_protocol: PROTOCOL_VERSION,
                };
            }
            // Log the accepted open so the socket -> handler ->
            // window-spawn chain is observable in the desktop log
            // (the CLI prints its own "opened ... in chan-desktop"
            // line; this is the matching desktop-side breadcrumb).
            tracing::info!(
                cli_version = %cli_version,
                workspace_path = %workspace_path,
                "handoff: opening workspace from CLI request",
            );
            match open_workspace(PathBuf::from(workspace_path)) {
                Ok(()) => Response::Opened {
                    desktop_version: CHAN_VERSION.into(),
                    capabilities: Capabilities {
                        open_local_workspace: true,
                    },
                },
                Err(message) => {
                    tracing::warn!(%message, "handoff: open_workspace callback failed");
                    Response::Error { message }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// CLI side: discover + request handoff.
// ---------------------------------------------------------------------------

/// Outcome of a handoff attempt. The CLI maps every non-`HandedOff`
/// variant to "own the server exactly as today" (standalone); the
/// distinct variants exist so the CLI can print the right note.
#[derive(Debug)]
pub enum Outcome {
    /// The desktop opened the workspace window. The CLI exits 0 without
    /// opening the workspace (the desktop owns the flock).
    HandedOff,
    /// No desktop discovered: no socket, connect refused, stale
    /// socket, or any I/O error before a valid response. The
    /// load-bearing default path -> own the server, print URL.
    NoDesktop,
    /// The desktop is a different protocol version. The CLI prints
    /// the skew and falls back to standalone.
    VersionSkew {
        desktop_version: String,
        desktop_protocol: u32,
    },
    /// The desktop answered but refused/failed (e.g. could not mount
    /// the workspace). Falls back to standalone after logging.
    DesktopError { message: String },
}

/// Try to hand `workspace_path` to a running same-user desktop. Connects
/// the well-known socket, sends an `OpenWorkspace` request, and parses
/// the response. Any connect failure / stale socket / read error /
/// malformed reply maps to `Outcome::NoDesktop` so the CLI behaves
/// exactly like today when the desktop is absent.
///
/// A short connect+IO timeout bounds the case where a stale socket
/// file exists but nothing is accepting; the CLI must not hang on a
/// dead desktop.
#[cfg(unix)]
pub async fn try_handoff(workspace_path: &Path) -> Outcome {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;

    let Some(socket_path) = well_known_socket_path() else {
        return Outcome::NoDesktop;
    };
    // No socket file at all is the common no-desktop case; skip the
    // connect attempt (and its log noise) entirely.
    if !socket_path.exists() {
        return Outcome::NoDesktop;
    }

    let connect = UnixStream::connect(&socket_path);
    let stream = match tokio::time::timeout(Duration::from_millis(1500), connect).await {
        Ok(Ok(s)) => s,
        // Refused / stale socket / timeout -> no live desktop.
        Ok(Err(_)) | Err(_) => return Outcome::NoDesktop,
    };

    let req = Request::OpenWorkspace {
        protocol: PROTOCOL_VERSION,
        cli_version: CHAN_VERSION.into(),
        workspace_path: workspace_path.display().to_string(),
    };
    let mut payload = match serde_json::to_vec(&req) {
        Ok(v) => v,
        Err(_) => return Outcome::NoDesktop,
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
        // Write/read error, empty reply, or timeout: treat as no
        // usable desktop and fall back rather than hang or error.
        _ => return Outcome::NoDesktop,
    };

    match serde_json::from_str::<Response>(&line) {
        Ok(Response::Opened { .. }) => Outcome::HandedOff,
        Ok(Response::VersionSkew {
            desktop_version,
            desktop_protocol,
        }) => Outcome::VersionSkew {
            desktop_version,
            desktop_protocol,
        },
        Ok(Response::Error { message }) => Outcome::DesktopError { message },
        // A reply we can't parse means a desktop we can't talk to:
        // fall back rather than guess.
        Err(_) => Outcome::NoDesktop,
    }
}

#[cfg(not(unix))]
pub async fn try_handoff(_workspace_path: &std::path::Path) -> Outcome {
    Outcome::NoDesktop
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips() {
        let req = Request::OpenWorkspace {
            protocol: PROTOCOL_VERSION,
            cli_version: "9.9.9".into(),
            workspace_path: "/tmp/notes".into(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"type\":\"open_workspace\""));
        let back: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(req, back);
    }

    #[test]
    fn response_round_trips() {
        let resp = Response::Opened {
            desktop_version: CHAN_VERSION.into(),
            capabilities: Capabilities {
                open_local_workspace: true,
            },
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"status\":\"opened\""));
        let back: Response = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, back);

        let skew = Response::VersionSkew {
            desktop_version: "0.1.0".into(),
            desktop_protocol: 99,
        };
        let json = serde_json::to_string(&skew).unwrap();
        assert!(json.contains("\"status\":\"version_skew\""));
        assert_eq!(skew, serde_json::from_str::<Response>(&json).unwrap());
    }

    #[test]
    fn opt_out_parsing() {
        // Snapshot + restore so the test is order-independent.
        let prev = std::env::var_os("CHAN_NO_DESKTOP_HANDOFF");
        std::env::remove_var("CHAN_NO_DESKTOP_HANDOFF");
        assert!(!handoff_opt_out());
        std::env::set_var("CHAN_NO_DESKTOP_HANDOFF", "1");
        assert!(handoff_opt_out());
        std::env::set_var("CHAN_NO_DESKTOP_HANDOFF", "0");
        assert!(!handoff_opt_out());
        std::env::set_var("CHAN_NO_DESKTOP_HANDOFF", "");
        assert!(!handoff_opt_out());
        match prev {
            Some(v) => std::env::set_var("CHAN_NO_DESKTOP_HANDOFF", v),
            None => std::env::remove_var("CHAN_NO_DESKTOP_HANDOFF"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn well_known_path_is_some_on_unix() {
        // Don't assert the exact value (env-dependent); just that unix
        // always resolves a path and it ends in the expected filename.
        let p = well_known_socket_path().expect("unix path");
        let s = p.to_string_lossy();
        assert!(s.contains("chan-desktop"), "unexpected path: {s}");
    }

    #[cfg(unix)]
    #[test]
    fn handle_request_rejects_protocol_skew() {
        let req = Request::OpenWorkspace {
            protocol: PROTOCOL_VERSION + 1,
            cli_version: "9.9.9".into(),
            workspace_path: "/tmp/notes".into(),
        };
        // The callback must NOT run on skew: the closure asserts.
        let resp = handle_request(req, &|_p| {
            panic!("open_workspace must not run on protocol skew");
        });
        match resp {
            Response::VersionSkew {
                desktop_protocol, ..
            } => assert_eq!(desktop_protocol, PROTOCOL_VERSION),
            other => panic!("expected VersionSkew, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn handle_request_surfaces_callback_error() {
        let req = Request::OpenWorkspace {
            protocol: PROTOCOL_VERSION,
            cli_version: CHAN_VERSION.into(),
            workspace_path: "/tmp/notes".into(),
        };
        let resp = handle_request(req, &|_p| Err("mount failed".to_string()));
        match resp {
            Response::Error { message } => assert_eq!(message, "mount failed"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    // End-to-end: a listener bound on a temp socket + a client request
    // over try_handoff's wire path (we point the well-known resolver at
    // the temp socket by connecting directly through a sibling helper).
    #[cfg(unix)]
    #[tokio::test]
    async fn listener_round_trip_opened() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("hand.sock");
        let opened = Arc::new(AtomicBool::new(false));
        let opened_cb = opened.clone();
        let _handle = start_listener(sock.clone(), move |path| {
            assert_eq!(path, PathBuf::from("/tmp/notes"));
            opened_cb.store(true, Ordering::SeqCst);
            Ok(())
        })
        .unwrap();

        // The socket should be 0600.
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&sock).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "socket perms should be 0600");

        let resp = request_over(&sock, "/tmp/notes").await;
        assert!(matches!(resp, Response::Opened { .. }));
        assert!(opened.load(Ordering::SeqCst), "callback must have run");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn listener_round_trip_error() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("hand.sock");
        let _handle =
            start_listener(sock.clone(), move |_p| Err("no such workspace".to_string())).unwrap();
        let resp = request_over(&sock, "/tmp/x").await;
        match resp {
            Response::Error { message } => assert_eq!(message, "no such workspace"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    /// Connect directly to `sock` and round-trip one OpenWorkspace. Mirrors
    /// try_handoff's wire framing but targets an explicit socket so the
    /// test doesn't depend on the well-known path.
    #[cfg(unix)]
    async fn request_over(sock: &std::path::Path, workspace: &str) -> Response {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::UnixStream;

        let stream = UnixStream::connect(sock).await.unwrap();
        let req = Request::OpenWorkspace {
            protocol: PROTOCOL_VERSION,
            cli_version: CHAN_VERSION.into(),
            workspace_path: workspace.into(),
        };
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
