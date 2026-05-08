//! chan-tunnel server library.
//!
//! The eventual entry point is an `axum::Router` exposing
//! `POST /v1/tunnel`; nginx (`grpc_pass`) forwards h2c from
//! `tunnel.chan.app`. After the Hello/HelloAck handshake the
//! duplex is handed to yamux, the registered drive is inserted
//! into the shared `Registry`, and the server side opens new
//! substreams to forward public requests.
//!
//! For the wire test the handshake is exposed as a free function
//! over any tokio duplex.

#![forbid(unsafe_code)]

use async_trait::async_trait;
use chan_tunnel_proto::{read_frame, write_frame, Hello, HelloAck, ProtocolVersion};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};
use yamux::{Config as YamuxConfig, Connection as YamuxConnection, Mode};

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("invalid token")]
    InvalidToken,

    #[error("token does not have tunnel scope")]
    MissingScope,

    #[error("upstream identity service: {0}")]
    Identity(String),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("handshake: {0}")]
    Handshake(String),
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
#[async_trait]
pub trait Validator: Send + Sync + 'static {
    async fn validate(&self, token: &str) -> Result<Validated, ServerError>;
}

/// Public path prefix shape: `/{username}/{drive}`. The fronting
/// proxy splits drive.chan.app traffic between its own SPA / API
/// routes and this tunnel terminator; reserved usernames (api,
/// admin, ...) keep the two from colliding. No trailing slash;
/// rest of the path is the drive-relative request.
fn make_prefix(username: &str, drive: &str) -> String {
    format!("/{username}/{drive}")
}

/// Drive the Hello/HelloAck round-trip over `socket`. Validates
/// the bearer `token` via `validator` and uses the drive name from
/// the client's Hello to build the public path. Returns the yamux
/// server connection ready to open outbound substreams.
pub async fn handshake<S, V>(
    mut socket: S,
    token: &str,
    validator: &V,
) -> Result<(Hello, Validated, YamuxConnection<Compat<S>>), ServerError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    V: Validator + ?Sized,
{
    let hello: Hello = read_frame(&mut socket).await?;
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

    let validated = validator.validate(token).await?;
    if !validated.scopes.iter().any(|s| s == "tunnel") {
        return Err(ServerError::MissingScope);
    }

    let ack = HelloAck {
        protocol: ProtocolVersion::V1,
        prefix: make_prefix(&validated.username, &hello.drive),
        user: validated.username.clone(),
        drive: hello.drive.clone(),
    };
    write_frame(&mut socket, &ack).await?;

    let yamux = YamuxConnection::new(socket.compat(), YamuxConfig::default(), Mode::Server);
    Ok((hello, validated, yamux))
}

mod driver;
mod public;
mod registry;
mod tunnel;

pub use public::public_router;
pub use registry::{OpenError, Registry, TunnelHandle};
pub use tunnel::serve_tunnel_listener;
