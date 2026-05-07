//! chan-tunnel server library.
//!
//! Two halves:
//!
//! * **Tunnel listener.** `tunnel_router()` returns an `axum::Router`
//!   exposing `POST /v1/tunnel`. nginx (`grpc_pass`) forwards h2c
//!   from `tunnel.chan.app`. After the Hello/HelloAck handshake,
//!   the duplex is handed to yamux and the registered drive is
//!   inserted into the shared `Registry`.
//!
//! * **Public listener.** `public_router()` returns an
//!   `axum::Router` that pattern-matches `/u/{user}/{drive}/*rest`
//!   on `drive.chan.app`, looks up the corresponding registered
//!   tunnel, opens a fresh yamux substream, forwards the request,
//!   and pipes the response body back.
//!
//! Skeleton only; the substream proxy and registry plumbing land
//! in a follow-up commit tracked by the chan-tunnel-server task.

#![forbid(unsafe_code)]

use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;

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

    #[error("transport: {0}")]
    Transport(String),
}

/// Result of validating a bearer token. Returned by `Validator`
/// and used to populate the HelloAck plus the (user, drive) entry
/// in the registry. `drive_id` may be `None` if the token is not
/// bound to a single drive (multi-drive tokens are out of scope
/// for v0 but the type stays open for it).
#[derive(Debug, Clone)]
pub struct Validated {
    pub user_id: uuid::Uuid,
    pub username: String,
    pub drive_id: Option<uuid::Uuid>,
    pub drive_name: String,
    pub scopes: Vec<String>,
}

/// Token validation hook. Implemented by `chan-tunneld` against
/// identity-service's `/internal/v1/tokens/validate`; a stub
/// implementation lives in tests.
#[async_trait]
pub trait Validator: Send + Sync + 'static {
    async fn validate(&self, token: &str) -> Result<Validated, ServerError>;
}

/// Shared registry of live tunnels keyed by `(user, drive)`. Public
/// requests look up a tunnel here and open a yamux substream.
/// Implementation lands with the proxy logic.
#[derive(Default)]
pub struct Registry {
    _private: (),
}

impl Registry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
}
