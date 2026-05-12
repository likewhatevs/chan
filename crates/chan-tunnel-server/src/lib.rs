//! chan-tunnel server library.
//!
//! The eventual entry point is an `axum::Router` exposing
//! `POST /v1/tunnel`; nginx (`grpc_pass`) forwards h2c from
//! `drive.chan.app/v1/tunnel` to drive-proxy's tunnel listener.
//! After the Hello/HelloAck handshake the duplex is handed to
//! yamux, the registered drive is inserted into the shared
//! `Registry`, and the server side opens new substreams to forward
//! public requests.
//!
//! For the wire test the handshake is exposed as a free function
//! over any tokio duplex.

#![forbid(unsafe_code)]

use std::time::Duration;

use async_trait::async_trait;
use chan_tunnel_proto::{read_frame, write_frame, Hello, HelloAck, ProtocolVersion};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};
use yamux::{Config as YamuxConfig, Connection as YamuxConnection, Mode};

/// Hard cap on how long the server waits for a client's Hello after
/// sending 200. A peer that connects, gets the OK, then never sends
/// the framed Hello (slow loris) is bounded by this. 15s is plenty
/// for the trans-pacific case; tighter would risk false positives
/// on slow mobile uplinks.
const HELLO_READ_TIMEOUT: Duration = Duration::from_secs(15);

/// Cap on the HTTP/2 connection-level handshake (SETTINGS exchange).
/// A peer that opens TCP and never speaks h2 stays bounded by this.
pub(crate) const H2_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Cap on the wait for the peer's first stream (the POST /v1/tunnel)
/// after the h2 handshake completes. A peer that finishes SETTINGS
/// and then idles is bounded here.
pub(crate) const FIRST_STREAM_TIMEOUT: Duration = Duration::from_secs(10);

/// Cap on the validator round-trip. Independent of any timeout the
/// `Validator` impl might enforce internally so a hung identity
/// service cannot pin a tunnel handshake task indefinitely.
pub(crate) const VALIDATE_TIMEOUT: Duration = Duration::from_secs(10);

/// Soft cap on how many tunnel connections can be in the
/// authenticate + handshake phase simultaneously. Above this, new
/// TCP accepts are rejected immediately (closing the socket) so the
/// listener cannot be exhausted with half-open / slow-loris peers
/// that have not yet reached the per-stage timeouts above. 1024 is
/// plenty for normal client churn; a real outage will recover within
/// one timeout cycle.
pub(crate) const MAX_INFLIGHT_HANDSHAKES: usize = 1024;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("invalid token")]
    InvalidToken,

    #[error("token does not have tunnel scope")]
    MissingScope,

    /// Upstream identity service failure. The wrapped string is
    /// logged at the listener (`tracing::warn!`) and may end up in
    /// operator-visible journals, so `Validator` implementations
    /// MUST NOT include the bearer token, any prefix of it, any URL
    /// that carries it as a query parameter, or any header value
    /// that echoes it. Treat the payload as user-visible.
    #[error("upstream identity service: {0}")]
    Identity(String),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("handshake: {0}")]
    Handshake(String),

    #[error("user {user} reached max concurrent drives ({max})")]
    TooManyDrives { user: String, max: usize },
}

impl From<chan_tunnel_proto::FrameError> for ServerError {
    fn from(e: chan_tunnel_proto::FrameError) -> Self {
        ServerError::Handshake(e.to_string())
    }
}

impl From<chan_tunnel_proto::IoFrameError> for ServerError {
    fn from(e: chan_tunnel_proto::IoFrameError) -> Self {
        match e {
            chan_tunnel_proto::IoFrameError::Io(e) => ServerError::Io(e),
            chan_tunnel_proto::IoFrameError::Frame(e) => ServerError::Handshake(e.to_string()),
        }
    }
}

/// Result of validating a bearer token. Returned by `Validator`
/// and used to populate the HelloAck plus the registry key.
/// Tokens are user-scoped: one validated token can register any
/// number of `(username, drive)` tunnels, each from a separate
/// `chan serve` instance.
#[derive(Debug, Clone)]
pub struct Validated {
    pub user_id: uuid::Uuid,
    pub username: String,
    pub scopes: Vec<String>,
}

/// Token validation hook. Implemented by the consumer (e.g. an
/// identity-service client); tests use a stub.
///
/// **Token-handling contract.** The `token` argument is the bearer
/// secret. Implementations MUST NOT:
///
/// - log it (including at debug / trace levels),
/// - return it (or any prefix of it) inside `ServerError::Identity`,
///   `ServerError::Handshake`, or any other error variant,
/// - return any URL that carries it as a query parameter or path
///   segment,
/// - persist it beyond the call duration.
///
/// The chan-tunnel listener logs `ServerError` values via
/// `tracing::warn!`, so anything echoed back will land in operator
/// journals. The crate itself never logs the token; the seam is
/// the only place this guarantee can be broken.
#[async_trait]
pub trait Validator: Send + Sync + 'static {
    async fn validate(&self, token: &str) -> Result<Validated, ServerError>;
}

/// Public path prefix shape: `/{drive}`. The fronting proxy now
/// uses wildcard subdomains (`{user}.drive.chan.app`), so the
/// username lives in the host header, not in the path. The path
/// prefix is exactly the drive segment so chan-server's
/// `<meta name="chan-prefix">` makes the SPA's relative URLs
/// resolve correctly under `{user}.drive.chan.app/{drive}/...`. No
/// trailing slash; rest of the path is the drive-relative request.
fn make_prefix(_username: &str, drive: &str) -> String {
    format!("/{drive}")
}

/// Drive the Hello/HelloAck round-trip over `socket`. Validates
/// the bearer `token` via `validator` and uses the drive name from
/// the client's Hello to build the public path. Returns the yamux
/// server connection ready to open outbound substreams.
///
/// `pre_ack` runs after the token is validated and before the
/// HelloAck is written. Returning an error from it aborts the
/// handshake without registering anything; the caller uses it for
/// post-validate policy checks (per-user drive limits, etc.).
///
/// Order of operations: validator runs first, *then* the Hello is
/// read and the drive name validated. The tunnel listener
/// (`handle_tunnel_conn`) needs that order to send 401 on bad
/// tokens before committing to the body, and consistency keeps the
/// two paths from diverging.
pub async fn handshake<S, V, F>(
    socket: S,
    token: &str,
    validator: &V,
    pre_ack: F,
) -> Result<(Hello, Validated, YamuxConnection<Compat<S>>), ServerError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    V: Validator + ?Sized,
    F: FnOnce(&Hello, &Validated) -> Result<(), ServerError>,
{
    let validated = validator.validate(token).await?;
    if !validated.scopes.iter().any(|s| s == "tunnel") {
        return Err(ServerError::MissingScope);
    }
    handshake_validated(socket, validated, pre_ack).await
}

/// Like `handshake` but takes an already-validated identity. Used
/// by the tunnel listener to validate the token *before* sending
/// the 200 response so a 401 can come back when validation fails;
/// once we've replied 200, this finishes the wire dance (Hello in,
/// drive-name check, pre_ack, HelloAck out, yamux wrap).
pub async fn handshake_validated<S, F>(
    mut socket: S,
    validated: Validated,
    pre_ack: F,
) -> Result<(Hello, Validated, YamuxConnection<Compat<S>>), ServerError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    F: FnOnce(&Hello, &Validated) -> Result<(), ServerError>,
{
    // Defense-in-depth: the validator has already authenticated the
    // token, but the username it returns flows into the public URL
    // path /{user}/{drive}. If the upstream identity service ever
    // emits a username with `/`, `..`, whitespace, or other
    // path-affecting bytes, the public router would mis-route or
    // leak the prefix. Refuse here so the rest of the pipeline can
    // assume the username is URL-safe.
    if !chan_tunnel_proto::is_valid_username(&validated.username) {
        return Err(ServerError::Handshake(format!(
            "validator returned an unsafe username for the public path: {:?}",
            validated.username
        )));
    }
    let hello: Hello = match tokio::time::timeout(HELLO_READ_TIMEOUT, read_frame(&mut socket)).await
    {
        Ok(r) => r?,
        Err(_) => {
            return Err(ServerError::Handshake(format!(
                "timed out waiting for Hello after {HELLO_READ_TIMEOUT:?}"
            )));
        }
    };
    if hello.protocol != ProtocolVersion::V1 {
        return Err(ServerError::Handshake(format!(
            "client requested unsupported protocol {:?}",
            hello.protocol
        )));
    }
    if !chan_tunnel_proto::is_valid_drive_name(&hello.drive) {
        return Err(ServerError::Handshake(format!(
            "invalid drive name {:?}",
            hello.drive
        )));
    }

    pre_ack(&hello, &validated)?;

    let ack = HelloAck {
        protocol: ProtocolVersion::V1,
        prefix: make_prefix(&validated.username, &hello.drive),
        user: validated.username.clone(),
        drive: hello.drive.clone(),
    };
    write_frame(&mut socket, &ack).await?;

    let yamux = YamuxConnection::new(socket.compat(), tunnel_yamux_config(), Mode::Server);
    Ok((hello, validated, yamux))
}

/// Yamux config with tighter caps than the upstream default. The
/// upstream `Config::default` allows 8192 concurrent substreams per
/// connection; that's a single tunnel's per-process budget, and a
/// public visitor that opens many slow requests can fill it. 256
/// is plenty for normal browser-shaped concurrency (a handful of
/// pipelined requests + a WebSocket or two) and bounds the worst
/// case to a manageable memory footprint.
fn tunnel_yamux_config() -> YamuxConfig {
    let mut cfg = YamuxConfig::default();
    cfg.set_max_num_streams(256);
    cfg
}

mod driver;
mod public;
mod registry;
mod tunnel;

pub use public::{public_router, public_router_with, PublicConfig, DEFAULT_REQUEST_BODY_CAP};
pub use registry::{DriveInfo, OpenError, Registry, TunnelHandle, TunnelInfo};
pub use tunnel::serve_tunnel_listener;
