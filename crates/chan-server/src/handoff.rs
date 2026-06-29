//! CLI-to-desktop workspace handoff over a well-known per-user endpoint:
//! a Unix-domain socket on unix, a named pipe on Windows.
//!
//! When chan-desktop is running and the user types `chan open
//! ~/notes` in a terminal, the natural intent is "show me this workspace
//! in the app," not "fail because the desktop already holds the
//! per-workspace flock." This module is the same-user IPC channel that
//! makes that handoff possible.
//!
//! Discovery is a WELL-KNOWN per-user endpoint (not the per-pid socket
//! the mcp_bridge / control_socket use): the CLI must find the desktop
//! without knowing its pid. On unix, same-user is enforced by the socket
//! living in a per-user runtime dir with 0600 perms and owned by the
//! user; on Windows, by the per-user pipe name plus the default named-pipe
//! ACL (same-user access), matching control_socket.
//!
//! INVARIANT: exactly one process owns a workspace's writes (the
//! chan-workspace per-workspace flock). In a successful handoff the DESKTOP
//! owns the workspace; the CLI is a launcher that exits WITHOUT opening
//! the workspace. The CLI must therefore consult this module BEFORE it
//! calls `open_workspace`, so it never double-opens.
//!
//! The bearer token never travels over argv/env/logs: by design
//! the desktop spawns its OWN native window
//! against its OWN embedded server, so no token crosses this socket
//! at all. The protocol still carries a version field so a
//! desktop-vs-CLI skew falls back to standalone rather than doing
//! silent cross-version IPC.
//!
//! Reuses the control_socket.rs shape: line-delimited JSON request +
//! response and a Drop guard for teardown (unix unlinks the socket file;
//! a Windows named pipe vanishes with the process). The protocol/types
//! compile on every platform; the listener + client are implemented on
//! unix (UDS) and Windows (named pipe), the desktop's two targets.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
#[cfg(any(unix, windows))]
use std::path::Path;
#[cfg(any(unix, windows))]
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
/// the on-wire shape is `{"type":"open_workspace", ...}`. Every variant
/// carries the `protocol` + `cli_version` handshake fields; the listener
/// checks `protocol` against its own PROTOCOL_VERSION before dispatching.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    /// Ask the desktop to open the given workspace path in a native
    /// window.
    OpenWorkspace {
        protocol: u32,
        cli_version: String,
        /// The workspace root the CLI was asked to serve. The desktop
        /// canonicalizes + registers it the same way its own
        /// open-local-workspace path does. Sent as a string for stable
        /// JSON across platforms.
        workspace_path: String,
    },
    /// Ask the running desktop to drive its `tauri-plugin-updater`. With
    /// `check_only` the desktop reports whether an update is available and
    /// does not install; otherwise it kicks off check -> download -> install
    /// in the background (fire-and-return: the CLI gets `UpgradeStarted` at
    /// once, the desktop owns the progress + self-relaunch). This is how a
    /// `chan upgrade` from the desktop-dispatched `chan` binary updates the
    /// app instead of replacing a CLI tarball.
    Upgrade {
        protocol: u32,
        cli_version: String,
        check_only: bool,
    },
    /// Ask the desktop to register a devserver by URL — the `chan open {url}`
    /// handoff. The desktop writes the `{url, name, script}` entry into the same
    /// config its launcher devserver registry reads; the launcher's Connect
    /// button drives the dial (the CLI does not auto-connect). The CLI sends
    /// this from its `open` command; the desktop's
    /// listener handles it. The bearer token is NOT carried here (same as the workspace
    /// handoff — the desktop owns credentials), so a tokened devserver is set up
    /// from the launcher dialog, not the CLI handoff.
    OpenDevserver {
        protocol: u32,
        cli_version: String,
        /// The full devserver URL, scheme included.
        url: String,
        /// Optional user label for the launcher section header.
        name: Option<String>,
        /// Optional connect script run before the dial.
        script: Option<String>,
    },
    /// Ask the running desktop to tear down the workspace it serves at
    /// `workspace_path` -- the `chan close` / `chan workspace rm` handoff. The
    /// desktop stops serving the tenant and, with `remove`, unregisters it from
    /// its own library + overlay. Like the open handoff, this goes through the
    /// well-known socket because the per-pid control socket reaches the embedded
    /// host but CANNOT update the desktop's runtime map: a control-socket close
    /// leaves the launcher showing the workspace stale-on and a restart
    /// resurrects it.
    CloseWorkspace {
        protocol: u32,
        cli_version: String,
        /// The workspace root to stop serving. The desktop canonicalizes +
        /// matches it the same way its own close path does. Sent as a string
        /// for stable JSON across platforms.
        workspace_path: String,
        /// `chan workspace rm` / `chan close --remove`: also unregister the
        /// workspace from the desktop's library + overlay, not just stop
        /// serving it.
        remove: bool,
    },
}

impl Request {
    /// The handshake protocol version carried by any request variant.
    pub fn protocol(&self) -> u32 {
        match self {
            Request::OpenWorkspace { protocol, .. }
            | Request::Upgrade { protocol, .. }
            | Request::OpenDevserver { protocol, .. }
            | Request::CloseWorkspace { protocol, .. } => *protocol,
        }
    }

    /// The CLI's human version, for skew logging.
    pub fn cli_version(&self) -> &str {
        match self {
            Request::OpenWorkspace { cli_version, .. }
            | Request::Upgrade { cli_version, .. }
            | Request::OpenDevserver { cli_version, .. }
            | Request::CloseWorkspace { cli_version, .. } => cli_version,
        }
    }
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
    /// The desktop accepted an `Upgrade { check_only: false }` and kicked
    /// off the install in the background. The CLI prints a note and exits;
    /// the desktop owns the download/install/relaunch.
    UpgradeStarted { desktop_version: String },
    /// The desktop answered an `Upgrade { check_only: true }`: `available`
    /// is the announced version when an update exists, or `None` when the
    /// desktop is already current.
    UpgradeChecked {
        desktop_version: String,
        available: Option<String>,
    },
    /// The desktop registered (and is connecting) the devserver from an
    /// `OpenDevserver`. The CLI prints a note and exits; the desktop owns the
    /// devserver lifecycle from here.
    DevserverRegistered { desktop_version: String },
    /// The desktop tore down the workspace from a `CloseWorkspace` request
    /// (stopped serving it, and unregistered it when `remove`). The CLI prints
    /// a note and exits. A skew / error / absent desktop instead falls back to
    /// the per-pid control-socket teardown.
    Closed { desktop_version: String },
    /// The desktop refused a `CloseWorkspace` request because unmount/remove
    /// would kill live terminal sessions.
    CloseRefused {
        error: String,
        active_terminals: usize,
    },
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
    #[cfg(windows)]
    {
        // Named pipes share one machine-global namespace, so namespace by
        // user to avoid cross-user collision (same-user access is the
        // default pipe ACL, matching control_socket). The name is WELL-KNOWN
        // (per-user, not per-pid) so the CLI finds the desktop without its
        // pid. Sanitize the username to the chars a pipe name allows.
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
        Some(PathBuf::from(format!(r"\\.\pipe\chan-desktop-{user}")))
    }
    #[cfg(not(any(unix, windows)))]
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
    #[cfg(windows)]
    {
        // Windows is a GUI OS; an interactive (or RDP) session always has a
        // desktop the user can see. The hand-off still only proceeds if a
        // chan-desktop is actually running (try_handoff connects the pipe).
        true
    }
    #[cfg(not(any(unix, windows)))]
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

/// Force the CLI-to-desktop handoff ON for a `Standalone`-personality binary:
/// `CHAN_DESKTOP_HANDOFF=1`. The desktop-installed Windows shim sets this so the
/// bundled console `chan.exe` hands a `chan open` off to the running desktop
/// exactly like the macOS/Linux desktop shim does via `Personality::Desktop`
/// (on those platforms the shim re-execs the desktop binary, so the personality
/// already carries the intent; on Windows the shim re-execs a separate
/// `chan.exe`, which is always `Standalone`, so it needs this explicit signal).
/// `CHAN_NO_DESKTOP_HANDOFF` still wins — the opt-out is checked at the handoff
/// itself. Any non-empty, non-"0" value counts as set.
pub fn handoff_forced() -> bool {
    match std::env::var("CHAN_DESKTOP_HANDOFF") {
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
/// carries one `Request`; the desktop's `handler` returns the `Response` and
/// the connection closes. The listener applies the protocol-version gate
/// before calling `handler`, so the handler only ever sees protocol-valid
/// requests; a skew becomes `Response::VersionSkew` without invoking it.
///
/// `handler` is `async` because an `Upgrade` request drives
/// `tauri-plugin-updater`'s network check. `OpenWorkspace` work stays
/// effectively synchronous (the desktop queues the window spawn onto its app
/// handle and returns immediately).
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

    // Unlink any stale socket from a previous run that didn't clean up
    // (kill -9, panic in Drop) so bind doesn't EADDRINUSE.
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;
    // Lock the socket to the owning user. Best-effort: a chmod failure
    // (exotic fs) does not abort the listener, but the per-user
    // directory placement is the primary boundary anyway.
    let _ = std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600));

    let handler = std::sync::Arc::new(handler);
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
            let handler = handler.clone();
            tokio::spawn(async move {
                let (read, mut write) = stream.into_split();
                let mut reader = BufReader::new(read);
                let mut line = String::new();
                let response = match reader.read_line(&mut line).await {
                    Ok(0) => Response::Error {
                        message: "empty handoff request".into(),
                    },
                    Ok(_) => match serde_json::from_str::<Request>(&line) {
                        Ok(req) => dispatch(req, handler.as_ref()).await,
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

/// Windows: the same handoff protocol over a named pipe (the cross-platform
/// transport chan already uses for the `cs` control socket). Mirrors the
/// control_socket accept loop: hold one idle pipe instance and, on each client
/// connect, swap in a FRESH instance BEFORE handing the connected one to a task,
/// so a client arriving during the swap still finds a live server instance.
/// Same per-connection framing (read one JSON line -> `dispatch` -> write one
/// JSON line) as the unix arm; `dispatch` applies the protocol-version gate.
#[cfg(windows)]
pub fn start_listener<F, Fut>(socket_path: PathBuf, handler: F) -> std::io::Result<ListenerHandle>
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Response> + Send + 'static,
{
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::windows::named_pipe::ServerOptions;

    let pipe_name = socket_path.as_os_str().to_owned();
    // `first_pipe_instance(true)` fails if another process already owns this
    // name — the singleton / squatter guard, mirroring how the unix bind owns
    // its filesystem socket path.
    let mut next = ServerOptions::new()
        .first_pipe_instance(true)
        .create(&pipe_name)?;

    let handler = std::sync::Arc::new(handler);
    let accept_loop = tokio::spawn(async move {
        loop {
            if let Err(e) = next.connect().await {
                tracing::warn!("handoff accept: {e}");
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
            // Re-arm BEFORE serving so the next client doesn't race to NotFound.
            let fresh = match ServerOptions::new().create(&pipe_name) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("handoff re-arm: {e}");
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
                        message: "empty handoff request".into(),
                    },
                    Ok(_) => match serde_json::from_str::<Request>(&line) {
                        Ok(req) => dispatch(req, handler.as_ref()).await,
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

#[cfg(not(any(unix, windows)))]
pub fn start_listener<F, Fut>(_socket_path: PathBuf, _handler: F) -> std::io::Result<ListenerHandle>
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Response> + Send + 'static,
{
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "handoff listener requires unix-domain sockets or windows named pipes",
    ))
}

/// Apply the protocol-version gate, then call the desktop's async `handler`.
/// A skew short-circuits to `VersionSkew` and the handler never runs (so the
/// desktop never acts on a request it can't fully understand).
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
            "handoff refused: protocol skew",
        );
        return Response::VersionSkew {
            desktop_version: CHAN_VERSION.into(),
            desktop_protocol: PROTOCOL_VERSION,
        };
    }
    handler(req).await
}

// ---------------------------------------------------------------------------
// CLI side: discover + request handoff.
// ---------------------------------------------------------------------------

/// Outcome of a handoff attempt. The CLI maps every non-`HandedOff`
/// variant to "own the server exactly as today" (standalone); the
/// distinct variants exist so the CLI can print the right note.
#[derive(Debug)]
pub enum Outcome {
    /// The desktop accepted the request: it opened the workspace window
    /// (`try_handoff`) or registered the devserver (`try_open_devserver`).
    /// The CLI exits 0 without opening anything itself (the desktop owns the
    /// flock / the devserver lifecycle).
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
    /// The desktop refused close/remove because live terminal sessions would be
    /// killed.
    CloseRefused { active_terminals: usize },
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
        // A reply we can't parse, or an upgrade / devserver reply to an
        // open-workspace request (a desktop we can't talk to sanely): fall back
        // rather than guess.
        Ok(Response::UpgradeStarted { .. })
        | Ok(Response::UpgradeChecked { .. })
        | Ok(Response::DevserverRegistered { .. })
        | Ok(Response::Closed { .. })
        | Ok(Response::CloseRefused { .. })
        | Err(_) => Outcome::NoDesktop,
    }
}

/// Windows: connect the well-known named pipe and round-trip one
/// `OpenWorkspace`. A missing pipe (no desktop) maps to `NoDesktop` at once —
/// it must NOT hang the common no-desktop `chan open`; a momentarily-busy pipe
/// (the desktop mid accept-swap) gets a short bounded retry. Same request /
/// timed response / parse as the unix arm.
#[cfg(windows)]
pub async fn try_handoff(workspace_path: &Path) -> Outcome {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::windows::named_pipe::ClientOptions;

    // Win32 ERROR_PIPE_BUSY: all instances are busy; retry briefly.
    const ERROR_PIPE_BUSY: i32 = 231;

    let Some(socket_path) = well_known_socket_path() else {
        return Outcome::NoDesktop;
    };

    let deadline = std::time::Instant::now() + Duration::from_millis(1500);
    let client = loop {
        match ClientOptions::new().open(&socket_path) {
            Ok(c) => break c,
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) => {
                if std::time::Instant::now() >= deadline {
                    return Outcome::NoDesktop;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            // NotFound (no desktop listening) / refused / other: fall back.
            Err(_) => return Outcome::NoDesktop,
        }
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
        Ok(Response::UpgradeStarted { .. })
        | Ok(Response::UpgradeChecked { .. })
        | Ok(Response::DevserverRegistered { .. })
        | Ok(Response::Closed { .. })
        | Ok(Response::CloseRefused { .. })
        | Err(_) => Outcome::NoDesktop,
    }
}

#[cfg(not(any(unix, windows)))]
pub async fn try_handoff(_workspace_path: &std::path::Path) -> Outcome {
    Outcome::NoDesktop
}

/// Try to make a running same-user desktop tear down the workspace it serves at
/// `workspace_path` -- the `chan close` / `chan workspace rm` handoff. Connects
/// the well-known socket, sends a `CloseWorkspace` request, and maps the reply
/// via [`map_close_response`]. Any connect failure / stale socket / read error /
/// malformed reply maps to `Outcome::NoDesktop` so the CLI falls back to the
/// per-pid control-socket teardown exactly like today when the desktop is
/// absent. Mirrors `try_handoff`'s framing + timeouts.
#[cfg(unix)]
pub async fn try_close_workspace(workspace_path: &Path, remove: bool) -> Outcome {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;

    let Some(socket_path) = well_known_socket_path() else {
        return Outcome::NoDesktop;
    };
    if !socket_path.exists() {
        return Outcome::NoDesktop;
    }

    let connect = UnixStream::connect(&socket_path);
    let stream = match tokio::time::timeout(Duration::from_millis(1500), connect).await {
        Ok(Ok(s)) => s,
        Ok(Err(_)) | Err(_) => return Outcome::NoDesktop,
    };

    let req = Request::CloseWorkspace {
        protocol: PROTOCOL_VERSION,
        cli_version: CHAN_VERSION.into(),
        workspace_path: workspace_path.display().to_string(),
        remove,
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
        _ => return Outcome::NoDesktop,
    };

    map_close_response(&line)
}

/// Windows: the same `CloseWorkspace` round-trip over the well-known named pipe,
/// mirroring `try_handoff`'s Windows arm (a missing pipe maps to `NoDesktop` at
/// once; a momentarily-busy pipe gets a short bounded retry).
#[cfg(windows)]
pub async fn try_close_workspace(workspace_path: &Path, remove: bool) -> Outcome {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::windows::named_pipe::ClientOptions;

    const ERROR_PIPE_BUSY: i32 = 231;

    let Some(socket_path) = well_known_socket_path() else {
        return Outcome::NoDesktop;
    };

    let deadline = std::time::Instant::now() + Duration::from_millis(1500);
    let client = loop {
        match ClientOptions::new().open(&socket_path) {
            Ok(c) => break c,
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) => {
                if std::time::Instant::now() >= deadline {
                    return Outcome::NoDesktop;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            Err(_) => return Outcome::NoDesktop,
        }
    };

    let req = Request::CloseWorkspace {
        protocol: PROTOCOL_VERSION,
        cli_version: CHAN_VERSION.into(),
        workspace_path: workspace_path.display().to_string(),
        remove,
    };
    let mut payload = match serde_json::to_vec(&req) {
        Ok(v) => v,
        Err(_) => return Outcome::NoDesktop,
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
        _ => return Outcome::NoDesktop,
    };

    map_close_response(&line)
}

#[cfg(not(any(unix, windows)))]
pub async fn try_close_workspace(_workspace_path: &std::path::Path, _remove: bool) -> Outcome {
    Outcome::NoDesktop
}

/// Map a `CloseWorkspace` reply line to an [`Outcome`]. Shared by the unix +
/// windows arms: only `Closed` is a success; a skew falls back to `VersionSkew`,
/// an `Error` to `DesktopError`, and any other reply / unparseable line to
/// `NoDesktop` so the caller drops to the control-socket teardown rather than
/// guessing.
#[cfg(any(unix, windows))]
fn map_close_response(line: &str) -> Outcome {
    match serde_json::from_str::<Response>(line) {
        Ok(Response::Closed { .. }) => Outcome::HandedOff,
        Ok(Response::CloseRefused {
            active_terminals, ..
        }) => Outcome::CloseRefused { active_terminals },
        Ok(Response::VersionSkew {
            desktop_version,
            desktop_protocol,
        }) => Outcome::VersionSkew {
            desktop_version,
            desktop_protocol,
        },
        Ok(Response::Error { message }) => Outcome::DesktopError { message },
        Ok(Response::Opened { .. })
        | Ok(Response::UpgradeStarted { .. })
        | Ok(Response::UpgradeChecked { .. })
        | Ok(Response::DevserverRegistered { .. })
        | Err(_) => Outcome::NoDesktop,
    }
}

/// Try to register a devserver by `url` with a running same-user
/// desktop — the `chan open {url}` path. Connects the well-known socket, sends
/// an `OpenDevserver` request, and maps the reply: `DevserverRegistered` ->
/// `HandedOff`; a protocol skew -> `VersionSkew`; an `Error` -> `DesktopError`;
/// any other reply / connect failure / stale socket / malformed line ->
/// `NoDesktop` (the caller surfaces "no desktop to register the devserver
/// into"). Mirrors `try_handoff`'s framing + timeouts. `name` / `script` ride
/// along as the optional launcher label + connect script; the bearer token
/// never travels this socket (the desktop owns credentials).
#[cfg(unix)]
pub async fn try_open_devserver(url: &str, name: Option<&str>, script: Option<&str>) -> Outcome {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;

    let Some(socket_path) = well_known_socket_path() else {
        return Outcome::NoDesktop;
    };
    if !socket_path.exists() {
        return Outcome::NoDesktop;
    }

    let connect = UnixStream::connect(&socket_path);
    let stream = match tokio::time::timeout(Duration::from_millis(1500), connect).await {
        Ok(Ok(s)) => s,
        Ok(Err(_)) | Err(_) => return Outcome::NoDesktop,
    };

    let req = Request::OpenDevserver {
        protocol: PROTOCOL_VERSION,
        cli_version: CHAN_VERSION.into(),
        url: url.to_string(),
        name: name.map(str::to_string),
        script: script.map(str::to_string),
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
        _ => return Outcome::NoDesktop,
    };

    map_devserver_response(&line)
}

/// Windows: the same `OpenDevserver` round-trip over the well-known named pipe,
/// mirroring `try_handoff`'s Windows arm (a missing pipe maps to `NoDesktop` at
/// once; a momentarily-busy pipe gets a short bounded retry).
#[cfg(windows)]
pub async fn try_open_devserver(url: &str, name: Option<&str>, script: Option<&str>) -> Outcome {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::windows::named_pipe::ClientOptions;

    const ERROR_PIPE_BUSY: i32 = 231;

    let Some(socket_path) = well_known_socket_path() else {
        return Outcome::NoDesktop;
    };

    let deadline = std::time::Instant::now() + Duration::from_millis(1500);
    let client = loop {
        match ClientOptions::new().open(&socket_path) {
            Ok(c) => break c,
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) => {
                if std::time::Instant::now() >= deadline {
                    return Outcome::NoDesktop;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            Err(_) => return Outcome::NoDesktop,
        }
    };

    let req = Request::OpenDevserver {
        protocol: PROTOCOL_VERSION,
        cli_version: CHAN_VERSION.into(),
        url: url.to_string(),
        name: name.map(str::to_string),
        script: script.map(str::to_string),
    };
    let mut payload = match serde_json::to_vec(&req) {
        Ok(v) => v,
        Err(_) => return Outcome::NoDesktop,
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
        _ => return Outcome::NoDesktop,
    };

    map_devserver_response(&line)
}

#[cfg(not(any(unix, windows)))]
pub async fn try_open_devserver(_url: &str, _name: Option<&str>, _script: Option<&str>) -> Outcome {
    Outcome::NoDesktop
}

/// Map an `OpenDevserver` reply line to an [`Outcome`]. Shared by the unix +
/// windows arms so the response handling stays in one place: only
/// `DevserverRegistered` is a success; a workspace/upgrade reply to a
/// devserver request (or an unparseable line) is a desktop we can't talk to
/// sanely, so it falls back to `NoDesktop` rather than guessing.
#[cfg(any(unix, windows))]
fn map_devserver_response(line: &str) -> Outcome {
    match serde_json::from_str::<Response>(line) {
        Ok(Response::DevserverRegistered { .. }) => Outcome::HandedOff,
        Ok(Response::VersionSkew {
            desktop_version,
            desktop_protocol,
        }) => Outcome::VersionSkew {
            desktop_version,
            desktop_protocol,
        },
        Ok(Response::Error { message }) => Outcome::DesktopError { message },
        Ok(Response::Opened { .. })
        | Ok(Response::UpgradeStarted { .. })
        | Ok(Response::UpgradeChecked { .. })
        | Ok(Response::Closed { .. })
        | Ok(Response::CloseRefused { .. })
        | Err(_) => Outcome::NoDesktop,
    }
}

/// Outcome of an upgrade-trigger attempt against the well-known socket.
/// Mirrors [`Outcome`] for the `Upgrade` request: every non-`Started` /
/// non-`Checked` variant means the CLI couldn't drive a running desktop.
#[derive(Debug)]
pub enum UpgradeOutcome {
    /// The desktop kicked off the install in the background (`check_only` was
    /// false). The CLI prints a note and exits.
    Started { desktop_version: String },
    /// The desktop reported an availability check (`check_only` was true).
    /// `available` is `Some(version)` when an update exists, else `None`.
    Checked {
        desktop_version: String,
        available: Option<String>,
    },
    /// No desktop discovered: no socket, connect refused, stale socket, read
    /// error, or malformed reply. The caller may launch one and retry.
    NoDesktop,
    /// The desktop speaks a different protocol version.
    VersionSkew {
        desktop_version: String,
        desktop_protocol: u32,
    },
    /// The desktop answered but the updater failed (e.g. unavailable / check
    /// error).
    DesktopError { message: String },
}

/// Ask a running same-user desktop to drive its updater. With `check_only`
/// the desktop reports availability without installing; otherwise it starts
/// the install in the background and returns at once (fire-and-return). Any
/// connect failure / stale socket / read error / malformed reply maps to
/// `UpgradeOutcome::NoDesktop` so the caller can launch a desktop and retry.
#[cfg(unix)]
pub async fn try_upgrade(check_only: bool) -> UpgradeOutcome {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;

    let Some(socket_path) = well_known_socket_path() else {
        return UpgradeOutcome::NoDesktop;
    };
    if !socket_path.exists() {
        return UpgradeOutcome::NoDesktop;
    }

    let connect = UnixStream::connect(&socket_path);
    let stream = match tokio::time::timeout(Duration::from_millis(1500), connect).await {
        Ok(Ok(s)) => s,
        Ok(Err(_)) | Err(_) => return UpgradeOutcome::NoDesktop,
    };

    let req = Request::Upgrade {
        protocol: PROTOCOL_VERSION,
        cli_version: CHAN_VERSION.into(),
        check_only,
    };
    let mut payload = match serde_json::to_vec(&req) {
        Ok(v) => v,
        Err(_) => return UpgradeOutcome::NoDesktop,
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
    // A `check_only` round-trip hits the network on the desktop side
    // (updater.check), so allow a longer read window than the open-workspace
    // path; the install kickoff (check_only=false) still returns promptly.
    let line = match tokio::time::timeout(Duration::from_secs(15), io).await {
        Ok(Ok(line)) if !line.trim().is_empty() => line,
        _ => return UpgradeOutcome::NoDesktop,
    };

    match serde_json::from_str::<Response>(&line) {
        Ok(Response::UpgradeStarted { desktop_version }) => {
            UpgradeOutcome::Started { desktop_version }
        }
        Ok(Response::UpgradeChecked {
            desktop_version,
            available,
        }) => UpgradeOutcome::Checked {
            desktop_version,
            available,
        },
        Ok(Response::VersionSkew {
            desktop_version,
            desktop_protocol,
        }) => UpgradeOutcome::VersionSkew {
            desktop_version,
            desktop_protocol,
        },
        Ok(Response::Error { message }) => UpgradeOutcome::DesktopError { message },
        // An open-workspace / devserver reply to an upgrade request, or an
        // unparseable line: a desktop we can't talk to sanely.
        Ok(Response::Opened { .. })
        | Ok(Response::DevserverRegistered { .. })
        | Ok(Response::Closed { .. })
        | Ok(Response::CloseRefused { .. })
        | Err(_) => UpgradeOutcome::NoDesktop,
    }
}

#[cfg(not(unix))]
pub async fn try_upgrade(_check_only: bool) -> UpgradeOutcome {
    UpgradeOutcome::NoDesktop
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

        let upgrade = Request::Upgrade {
            protocol: PROTOCOL_VERSION,
            cli_version: "9.9.9".into(),
            check_only: true,
        };
        let json = serde_json::to_string(&upgrade).unwrap();
        assert!(json.contains("\"type\":\"upgrade\""));
        assert_eq!(upgrade, serde_json::from_str::<Request>(&json).unwrap());
        assert_eq!(upgrade.protocol(), PROTOCOL_VERSION);
        assert_eq!(upgrade.cli_version(), "9.9.9");

        let devserver = Request::OpenDevserver {
            protocol: PROTOCOL_VERSION,
            cli_version: "9.9.9".into(),
            url: "https://box.example.com:8787".into(),
            name: Some("box".into()),
            script: None,
        };
        let json = serde_json::to_string(&devserver).unwrap();
        assert!(json.contains("\"type\":\"open_devserver\""));
        assert!(json.contains("\"url\":\"https://box.example.com:8787\""));
        assert_eq!(devserver, serde_json::from_str::<Request>(&json).unwrap());

        let close = Request::CloseWorkspace {
            protocol: PROTOCOL_VERSION,
            cli_version: "9.9.9".into(),
            workspace_path: "/tmp/notes".into(),
            remove: true,
        };
        let json = serde_json::to_string(&close).unwrap();
        assert!(json.contains("\"type\":\"close_workspace\""));
        assert!(json.contains("\"remove\":true"));
        assert_eq!(close, serde_json::from_str::<Request>(&json).unwrap());
        assert_eq!(close.protocol(), PROTOCOL_VERSION);
        assert_eq!(close.cli_version(), "9.9.9");
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

        let started = Response::UpgradeStarted {
            desktop_version: CHAN_VERSION.into(),
        };
        let json = serde_json::to_string(&started).unwrap();
        assert!(json.contains("\"status\":\"upgrade_started\""));
        assert_eq!(started, serde_json::from_str::<Response>(&json).unwrap());

        let checked = Response::UpgradeChecked {
            desktop_version: CHAN_VERSION.into(),
            available: Some("9.9.9".into()),
        };
        let json = serde_json::to_string(&checked).unwrap();
        assert!(json.contains("\"status\":\"upgrade_checked\""));
        assert_eq!(checked, serde_json::from_str::<Response>(&json).unwrap());

        let registered = Response::DevserverRegistered {
            desktop_version: CHAN_VERSION.into(),
        };
        let json = serde_json::to_string(&registered).unwrap();
        assert!(json.contains("\"status\":\"devserver_registered\""));
        assert_eq!(registered, serde_json::from_str::<Response>(&json).unwrap());

        let closed = Response::Closed {
            desktop_version: CHAN_VERSION.into(),
        };
        let json = serde_json::to_string(&closed).unwrap();
        assert!(json.contains("\"status\":\"closed\""));
        assert_eq!(closed, serde_json::from_str::<Response>(&json).unwrap());

        let refused = Response::CloseRefused {
            error: "live_terminals".into(),
            active_terminals: 2,
        };
        let json = serde_json::to_string(&refused).unwrap();
        assert_eq!(refused, serde_json::from_str::<Response>(&json).unwrap());
        assert!(json.contains("\"status\":\"close_refused\""));
        assert!(json.contains("\"error\":\"live_terminals\""));
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn map_close_response_maps_variants() {
        // `Closed` is the only success; everything else falls back so the CLI
        // drops to the per-pid control-socket teardown rather than guessing.
        let closed = format!("{{\"status\":\"closed\",\"desktop_version\":\"{CHAN_VERSION}\"}}");
        assert!(matches!(map_close_response(&closed), Outcome::HandedOff));

        let refused =
            "{\"status\":\"close_refused\",\"error\":\"live_terminals\",\"active_terminals\":2}";
        assert!(matches!(
            map_close_response(refused),
            Outcome::CloseRefused {
                active_terminals: 2
            }
        ));

        let skew =
            "{\"status\":\"version_skew\",\"desktop_version\":\"0.1.0\",\"desktop_protocol\":99}";
        assert!(matches!(
            map_close_response(skew),
            Outcome::VersionSkew { .. }
        ));

        let err = "{\"status\":\"error\",\"message\":\"nope\"}";
        assert!(matches!(
            map_close_response(err),
            Outcome::DesktopError { .. }
        ));

        // An open-workspace reply to a close request (a desktop we can't talk to
        // sanely) and unparseable lines both fall back.
        let opened = format!(
            "{{\"status\":\"opened\",\"desktop_version\":\"{CHAN_VERSION}\",\
             \"capabilities\":{{\"open_local_workspace\":true}}}}"
        );
        assert!(matches!(map_close_response(&opened), Outcome::NoDesktop));
        assert!(matches!(map_close_response("not json"), Outcome::NoDesktop));
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

    #[test]
    fn forced_parsing() {
        // Pins the `CHAN_DESKTOP_HANDOFF` name — the desktop shim sets it to opt
        // the bundled console chan.exe into the handoff; a rename here would
        // silently break that cross-crate contract. Snapshot + restore for
        // order-independence.
        let prev = std::env::var_os("CHAN_DESKTOP_HANDOFF");
        std::env::remove_var("CHAN_DESKTOP_HANDOFF");
        assert!(!handoff_forced());
        std::env::set_var("CHAN_DESKTOP_HANDOFF", "1");
        assert!(handoff_forced());
        std::env::set_var("CHAN_DESKTOP_HANDOFF", "0");
        assert!(!handoff_forced());
        std::env::set_var("CHAN_DESKTOP_HANDOFF", "");
        assert!(!handoff_forced());
        match prev {
            Some(v) => std::env::set_var("CHAN_DESKTOP_HANDOFF", v),
            None => std::env::remove_var("CHAN_DESKTOP_HANDOFF"),
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

    #[cfg(windows)]
    #[test]
    fn well_known_path_is_named_pipe_on_windows() {
        // Windows resolves a per-user named pipe in the `\\.\pipe\` namespace.
        let p = well_known_socket_path().expect("windows path");
        let s = p.to_string_lossy();
        assert!(
            s.starts_with(r"\\.\pipe\chan-desktop-"),
            "unexpected path: {s}"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn dispatch_rejects_protocol_skew() {
        let req = Request::OpenWorkspace {
            protocol: PROTOCOL_VERSION + 1,
            cli_version: "9.9.9".into(),
            workspace_path: "/tmp/notes".into(),
        };
        // The handler returns a distinctive Opened; getting VersionSkew back
        // proves dispatch short-circuited and never ran the handler.
        let handler = |_r: Request| async {
            Response::Opened {
                desktop_version: CHAN_VERSION.into(),
                capabilities: Capabilities {
                    open_local_workspace: true,
                },
            }
        };
        match dispatch(req, &handler).await {
            Response::VersionSkew {
                desktop_protocol, ..
            } => assert_eq!(desktop_protocol, PROTOCOL_VERSION),
            other => panic!("expected VersionSkew, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn dispatch_runs_handler_on_match() {
        let req = Request::OpenWorkspace {
            protocol: PROTOCOL_VERSION,
            cli_version: CHAN_VERSION.into(),
            workspace_path: "/tmp/notes".into(),
        };
        let handler = |_r: Request| async {
            Response::Error {
                message: "mount failed".to_string(),
            }
        };
        match dispatch(req, &handler).await {
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
        let _handle = start_listener(sock.clone(), move |req| {
            let opened_cb = opened_cb.clone();
            async move {
                match req {
                    Request::OpenWorkspace { workspace_path, .. } => {
                        assert_eq!(workspace_path, "/tmp/notes");
                        opened_cb.store(true, Ordering::SeqCst);
                        Response::Opened {
                            desktop_version: CHAN_VERSION.into(),
                            capabilities: Capabilities {
                                open_local_workspace: true,
                            },
                        }
                    }
                    Request::Upgrade { .. } => Response::Error {
                        message: "unexpected upgrade".into(),
                    },
                    Request::OpenDevserver { .. } => Response::Error {
                        message: "unexpected open_devserver".into(),
                    },
                    Request::CloseWorkspace { .. } => Response::Error {
                        message: "unexpected close".into(),
                    },
                }
            }
        })
        .unwrap();

        // The socket should be 0600.
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&sock).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "socket perms should be 0600");

        let resp = request_over(&sock, "/tmp/notes").await;
        assert!(matches!(resp, Response::Opened { .. }));
        assert!(opened.load(Ordering::SeqCst), "handler must have run");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn listener_round_trip_error() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("hand.sock");
        let _handle = start_listener(sock.clone(), move |_req| async move {
            Response::Error {
                message: "no such workspace".to_string(),
            }
        })
        .unwrap();
        let resp = request_over(&sock, "/tmp/x").await;
        match resp {
            Response::Error { message } => assert_eq!(message, "no such workspace"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn listener_round_trip_upgrade_checked() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("hand.sock");
        let _handle = start_listener(sock.clone(), move |req| async move {
            match req {
                Request::Upgrade { check_only, .. } => {
                    assert!(check_only, "test sends check_only=true");
                    Response::UpgradeChecked {
                        desktop_version: CHAN_VERSION.into(),
                        available: Some("9.9.9".into()),
                    }
                }
                Request::OpenWorkspace { .. } => Response::Error {
                    message: "unexpected open".into(),
                },
                Request::OpenDevserver { .. } => Response::Error {
                    message: "unexpected open_devserver".into(),
                },
                Request::CloseWorkspace { .. } => Response::Error {
                    message: "unexpected close".into(),
                },
            }
        })
        .unwrap();
        match upgrade_over(&sock, true).await {
            Response::UpgradeChecked { available, .. } => {
                assert_eq!(available, Some("9.9.9".to_string()))
            }
            other => panic!("expected UpgradeChecked, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn listener_round_trip_devserver_registered() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("hand.sock");
        let _handle = start_listener(sock.clone(), move |req| async move {
            match req {
                Request::OpenDevserver {
                    url, name, script, ..
                } => {
                    assert_eq!(url, "https://box.example.com:8787");
                    assert_eq!(name.as_deref(), Some("box"));
                    assert_eq!(script.as_deref(), Some("echo connect"));
                    Response::DevserverRegistered {
                        desktop_version: CHAN_VERSION.into(),
                    }
                }
                Request::OpenWorkspace { .. } => Response::Error {
                    message: "unexpected open".into(),
                },
                Request::Upgrade { .. } => Response::Error {
                    message: "unexpected upgrade".into(),
                },
                Request::CloseWorkspace { .. } => Response::Error {
                    message: "unexpected close".into(),
                },
            }
        })
        .unwrap();
        match devserver_over(
            &sock,
            "https://box.example.com:8787",
            Some("box"),
            Some("echo connect"),
        )
        .await
        {
            Response::DevserverRegistered { .. } => {}
            other => panic!("expected DevserverRegistered, got {other:?}"),
        }
    }

    /// Connect directly to `sock` and round-trip one OpenWorkspace. Mirrors
    /// try_handoff's wire framing but targets an explicit socket so the
    /// test doesn't depend on the well-known path.
    #[cfg(unix)]
    async fn request_over(sock: &std::path::Path, workspace: &str) -> Response {
        let req = Request::OpenWorkspace {
            protocol: PROTOCOL_VERSION,
            cli_version: CHAN_VERSION.into(),
            workspace_path: workspace.into(),
        };
        round_trip(sock, &req).await
    }

    /// Round-trip one `Upgrade` request, mirroring try_upgrade's framing.
    #[cfg(unix)]
    async fn upgrade_over(sock: &std::path::Path, check_only: bool) -> Response {
        let req = Request::Upgrade {
            protocol: PROTOCOL_VERSION,
            cli_version: CHAN_VERSION.into(),
            check_only,
        };
        round_trip(sock, &req).await
    }

    /// Round-trip one `OpenDevserver` request, mirroring try_open_devserver's
    /// framing.
    #[cfg(unix)]
    async fn devserver_over(
        sock: &std::path::Path,
        url: &str,
        name: Option<&str>,
        script: Option<&str>,
    ) -> Response {
        let req = Request::OpenDevserver {
            protocol: PROTOCOL_VERSION,
            cli_version: CHAN_VERSION.into(),
            url: url.into(),
            name: name.map(str::to_string),
            script: script.map(str::to_string),
        };
        round_trip(sock, &req).await
    }

    #[cfg(unix)]
    async fn round_trip(sock: &std::path::Path, req: &Request) -> Response {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::UnixStream;

        let stream = UnixStream::connect(sock).await.unwrap();
        let mut payload = serde_json::to_vec(req).unwrap();
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
