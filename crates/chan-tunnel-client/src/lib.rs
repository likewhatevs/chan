//! chan-tunnel client library.
//!
//! Used by `chan serve --tunnel-url ... --tunnel-token ...`. The
//! eventual entry point dials the public tunnel endpoint over
//! h2/TLS, runs `handshake` over the resulting bidirectional
//! stream, and serves every yamux substream with a user-supplied
//! `tower::Service` (typically an `axum::Router`) via hyper.
//!
//! For the wire test and for unit testing in isolation, the
//! handshake is exposed as a free function over any tokio duplex.

#![forbid(unsafe_code)]

use std::time::Duration;

use chan_tunnel_proto::{read_frame, write_frame, Hello, HelloAck, ProtocolVersion};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};
use url::Url;
use yamux::{Config as YamuxConfig, Connection as YamuxConnection, Mode};

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

impl From<chan_tunnel_proto::FrameError> for ClientError {
    fn from(e: chan_tunnel_proto::FrameError) -> Self {
        ClientError::Handshake(e.to_string())
    }
}

impl From<chan_tunnel_proto::IoFrameError> for ClientError {
    fn from(e: chan_tunnel_proto::IoFrameError) -> Self {
        match e {
            chan_tunnel_proto::IoFrameError::Io(e) => ClientError::Io(e),
            chan_tunnel_proto::IoFrameError::Frame(e) => ClientError::Handshake(e.to_string()),
        }
    }
}

/// Configuration for the dial loop. The token is intentionally a
/// `String` rather than borrowed: the dial loop may reconnect, and
/// holding a borrow across reconnects forces the caller into
/// awkward lifetimes.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub tunnel_url: Url,
    pub token: String,
    /// Drive name sent in the Hello frame. Combined server-side
    /// with the token's user to form the public path
    /// `/{user}/{drive}/...`. Required.
    pub drive: String,
    /// `chan` version reported in the Hello frame; logs only.
    pub client_version: String,
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
            drive: String::new(),
            client_version: format!("chan-tunnel-client/{}", env!("CARGO_PKG_VERSION")),
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(30),
        }
    }
}

/// What the server told the client during HelloAck. `chan serve`
/// uses `prefix` to wire its router so the user does not pass
/// `--prefix` manually.
#[derive(Debug, Clone)]
pub struct Registration {
    pub prefix: String,
    pub user: String,
    pub drive: String,
}

/// Drive the Hello/HelloAck round-trip over `socket` and return a
/// yamux client connection ready to accept inbound substreams.
///
/// Generic in `S` so the wire test can pass a `tokio::io::duplex`
/// half and the real client can pass an h2-bidi-stream adapter
/// later. The yamux `Connection` returned holds ownership of the
/// socket via a `tokio-util` compat shim; substreams it produces
/// also use futures-io traits.
pub async fn handshake<S>(
    cfg: &ClientConfig,
    mut socket: S,
) -> Result<(Registration, YamuxConnection<Compat<S>>), ClientError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    if !chan_tunnel_proto::is_valid_drive_name(&cfg.drive) {
        return Err(ClientError::Handshake(format!(
            "invalid drive name {:?}; expected lowercase [a-z0-9-], 1-{} chars, no leading/trailing hyphen",
            cfg.drive,
            chan_tunnel_proto::MAX_DRIVE_NAME_LEN,
        )));
    }
    let hello = Hello {
        protocol: ProtocolVersion::V1,
        client_version: cfg.client_version.clone(),
        drive: cfg.drive.clone(),
    };
    write_frame(&mut socket, &hello).await?;

    let ack: HelloAck = read_frame(&mut socket).await?;
    if ack.protocol != ProtocolVersion::V1 {
        return Err(ClientError::Handshake(format!(
            "server returned unsupported protocol {:?}",
            ack.protocol
        )));
    }

    let registration = Registration {
        prefix: ack.prefix,
        user: ack.user,
        drive: ack.drive,
    };
    let yamux = YamuxConnection::new(socket.compat(), YamuxConfig::default(), Mode::Client);
    Ok((registration, yamux))
}
