//! chan-tunnel client library.
//!
//! Used by `chan serve --tunnel-url ... --tunnel-token ...`. Dials
//! the public tunnel endpoint over h2/TLS, completes the
//! Hello/HelloAck handshake, runs yamux over the duplex, and serves
//! every incoming substream with a user-supplied `tower::Service`
//! (typically an `axum::Router`) via hyper.
//!
//! Skeleton only; the dial / serve loop lands in a follow-up commit
//! tracked by the chan-tunnel-client task.

#![forbid(unsafe_code)]

use std::time::Duration;

use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("invalid tunnel url: {0}")]
    InvalidUrl(String),

    #[error("tls: {0}")]
    Tls(String),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("handshake: {0}")]
    Handshake(String),

    #[error("transport closed")]
    TransportClosed,
}

/// Configuration for `Client::run`. The token is intentionally a
/// `String` rather than borrowed: the dial loop may reconnect, and
/// holding a borrow across reconnects forces the caller into
/// awkward lifetimes.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub tunnel_url: Url,
    pub token: String,
    /// Hint sent in the Hello frame; logged server-side and used
    /// only to fail fast when it disagrees with the token's bound
    /// drive.
    pub drive_hint: Option<String>,
    /// Initial reconnect backoff. Doubled up to `max_backoff`.
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            tunnel_url: Url::parse("https://tunnel.chan.app/v1/tunnel")
                .expect("hard-coded url is valid"),
            token: String::new(),
            drive_hint: None,
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(30),
        }
    }
}

/// Result of a successful Hello/HelloAck round-trip. Returned to
/// the caller before the substream-serving loop starts so `chan
/// serve` can wire `--prefix` from the assigned public path.
#[derive(Debug, Clone)]
pub struct Registration {
    pub prefix: String,
    pub user: String,
    pub drive: String,
}
