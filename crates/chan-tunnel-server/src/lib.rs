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

use std::sync::Arc;

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

    #[error("token bound to a different drive than the client claims")]
    DriveMismatch,

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
/// and used to populate the HelloAck plus the (user, drive) entry
/// in the registry.
#[derive(Debug, Clone)]
pub struct Validated {
    pub user_id: uuid::Uuid,
    pub username: String,
    pub drive_id: Option<uuid::Uuid>,
    pub drive_name: String,
    pub scopes: Vec<String>,
}

/// Token validation hook. Implemented by `chan-tunneld` against
/// identity-service's `/internal/v1/tokens/validate`; tests use a
/// stub.
#[async_trait]
pub trait Validator: Send + Sync + 'static {
    async fn validate(&self, token: &str) -> Result<Validated, ServerError>;
}

/// Public path prefix shape: `/u/{username}/{drive}`. Stable so
/// chan-gateway's drive-proxy can route to the same scheme without
/// having to know per-tunnel state. No trailing slash; rest of the
/// path is the drive-relative request.
fn make_prefix(username: &str, drive: &str) -> String {
    format!("/u/{username}/{drive}")
}

/// Drive the Hello/HelloAck round-trip over `socket`. Validates
/// the bearer `token` via `validator`, asserts any client-side
/// drive hint matches the token's binding, and returns the yamux
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

    let validated = validator.validate(token).await?;
    if !validated.scopes.iter().any(|s| s == "tunnel") {
        return Err(ServerError::MissingScope);
    }
    if let Some(hint) = &hello.drive_hint {
        if hint != &validated.drive_name {
            return Err(ServerError::DriveMismatch);
        }
    }

    let ack = HelloAck {
        protocol: ProtocolVersion::V1,
        prefix: make_prefix(&validated.username, &validated.drive_name),
        user: validated.username.clone(),
        drive: validated.drive_name.clone(),
    };
    write_frame(&mut socket, &ack).await?;

    let yamux = YamuxConnection::new(socket.compat(), YamuxConfig::default(), Mode::Server);
    Ok((hello, validated, yamux))
}

/// Shared registry of live tunnels keyed by `(user, drive)`.
/// Public requests look up a tunnel here and open a yamux
/// substream. Implementation lands with the proxy logic.
#[derive(Default)]
pub struct Registry {
    _private: (),
}

impl Registry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
}
