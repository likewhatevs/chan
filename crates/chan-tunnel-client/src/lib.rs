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

mod dial;

pub use dial::{build_tls_config, dial, dial_with_tls};

use std::pin::Pin;
use std::time::Duration;

use chan_tunnel_proto::{read_frame, write_frame, Hello, HelloAck, ProtocolVersion};
use futures::AsyncRead as FutAsyncRead;
use futures::AsyncWrite as FutAsyncWrite;
use hyper_util::rt::TokioIo;
use hyper_util::service::TowerToHyperService;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
use tokio_util::compat::{Compat, FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};
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
    /// Expose the drive to anonymous visitors. When false (the
    /// default), only the drive owner's signed-in id.chan.app
    /// session can reach `drive.chan.app/{user}/{drive}`. When
    /// true, the drive-proxy auth gate skips the OAuth bounce.
    pub public: bool,
    /// Initial reconnect backoff. Doubled up to `max_backoff`.
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    /// Wall-clock cap on a single dial attempt: TCP connect, TLS,
    /// h2 handshake, response, Hello/HelloAck. Without this, an
    /// unreachable host or a black-holed network can hang each
    /// attempt for the OS-level TCP timeout (minutes), defeating
    /// the retry backoff. 30s covers the trans-pacific case with
    /// margin; bump for satellite links.
    pub dial_timeout: Duration,
    /// Optional channel for `run` to publish lifecycle events on.
    /// Useful when the caller wants to surface "connected", "lost
    /// connection", "retrying in Xs" to its own UI. Backpressure:
    /// `run` uses `try_send`, so a slow consumer drops events
    /// rather than blocking the tunnel.
    pub events: Option<mpsc::Sender<TunnelEvent>>,
    /// Optional outbound HTTP proxy. When set, the client opens a
    /// TCP connection to the proxy and runs an HTTP/1.1 CONNECT to
    /// the tunnel host:port; TLS (if any) and h2 then run inside
    /// the resulting tunnel. Supports basic auth via the URL's
    /// userinfo (`http://user:pass@proxy.example:3128`). Schemes:
    /// `http://` only (plain CONNECT). HTTPS-to-proxy and SOCKS
    /// are out of scope; route those through a local stunnel /
    /// SOCKS-to-HTTP shim if needed.
    ///
    /// Env vars (`HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`) are NOT
    /// honoured automatically: the embedded callers (Swift /
    /// Kotlin / CLI) get a deterministic surface this way.
    pub proxy: Option<Url>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            tunnel_url: Url::parse("https://drive.chan.app/v1/tunnel")
                .expect("hard-coded url is valid"),
            token: String::new(),
            drive: String::new(),
            client_version: format!("chan-tunnel-client/{}", env!("CARGO_PKG_VERSION")),
            public: false,
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(30),
            dial_timeout: Duration::from_secs(30),
            events: None,
            proxy: None,
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

/// Lifecycle events emitted by `run`. Callers subscribe via
/// `ClientConfig::events`. Cloning these is cheap; they're meant
/// to be tee'd to logs and a UI.
#[derive(Debug, Clone)]
pub enum TunnelEvent {
    /// A successful registration. Carries the server-assigned
    /// public prefix.
    Connected(Registration),
    /// The currently-registered tunnel ended (clean close from the
    /// server, or substream-loop error). `run` will sleep for
    /// `retry_in` then dial again.
    Disconnected { retry_in: Duration },
    /// Dial failed before registration (TLS error, h2 error, 401,
    /// network unreachable, etc.). `run` will sleep for `retry_in`
    /// then try again. `error` is best-effort human-readable.
    DialFailed { error: String, retry_in: Duration },
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
        public: cfg.public,
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

/// Serve every inbound yamux substream with `router` until the
/// connection closes. Each substream is one HTTP/1.1 request from
/// the public side; we run hyper's h1 server over it with the
/// user-supplied axum router as the service.
///
/// `with_upgrades()` is enabled so the substream stays alive after
/// a WebSocket 101 response; the bytes ride the existing yamux
/// substream until either end closes.
pub async fn serve_substreams<S>(
    mut conn: YamuxConnection<S>,
    router: axum::Router,
) -> Result<(), ClientError>
where
    S: FutAsyncRead + FutAsyncWrite + Unpin + Send + 'static,
{
    loop {
        let next = futures::future::poll_fn(|cx| Pin::new(&mut conn).poll_next_inbound(cx)).await;
        match next {
            Some(Ok(stream)) => {
                let router = router.clone();
                tokio::spawn(async move {
                    serve_one_substream(stream, router).await;
                });
            }
            Some(Err(_)) | None => return Ok(()),
        }
    }
}

/// Run the tunnel client until cancelled: dial, register, serve
/// substreams, reconnect on disconnect with exponential backoff.
///
/// Designed for `chan serve` to call as a long-lived future;
/// dropping it cancels everything cleanly. Returns only on
/// configuration errors that retrying cannot recover from
/// (invalid URL, invalid drive name, missing token).
pub async fn run(cfg: ClientConfig, router: axum::Router) -> Result<(), ClientError> {
    if cfg.token.is_empty() {
        return Err(ClientError::Handshake(
            "ClientConfig.token is empty; nothing to authenticate with".into(),
        ));
    }
    if !chan_tunnel_proto::is_valid_drive_name(&cfg.drive) {
        return Err(ClientError::Handshake(format!(
            "invalid drive name {:?}",
            cfg.drive
        )));
    }
    match cfg.tunnel_url.scheme() {
        "https" | "http" => {}
        other => {
            return Err(ClientError::InvalidUrl(format!(
                "tunnel URL scheme must be https:// or http://, got {other}://"
            )));
        }
    }
    // h2c (http://) is fine for a local dev stack but ships the
    // bearer token in cleartext; warn loudly when someone points it
    // at a non-loopback host. Loopback detection is best-effort:
    // hostname "localhost" and the standard 127.x / ::1 literals
    // count; everything else gets the warning. We don't refuse
    // outright because there are legitimate cases (private VPN,
    // Tailscale, in-cluster service).
    if cfg.tunnel_url.scheme() == "http" {
        let host = cfg.tunnel_url.host_str().unwrap_or("");
        let is_loopback =
            host == "localhost" || host.starts_with("127.") || host == "::1" || host == "[::1]";
        if !is_loopback {
            tracing::warn!(
                host = %host,
                "tunnel URL is http://; bearer token will be sent in cleartext. \
                 Use https:// for non-loopback hosts.",
            );
        }
    }

    // Build the TLS config once; rustls-native-certs walks the
    // OS trust store on every call (slow on macOS keychain) and
    // the reconnect loop would otherwise re-pay that on every
    // attempt. Lazy: only build for https:// URLs.
    let tls = if cfg.tunnel_url.scheme() == "https" {
        Some(std::sync::Arc::new(build_tls_config()?))
    } else {
        None
    };

    let mut backoff = cfg.initial_backoff;
    loop {
        // Cap a single dial attempt so an unreachable host doesn't
        // hang for minutes (OS TCP timeout) and starve the retry
        // backoff. Per-leg timeouts inside `dial` would be more
        // precise but a single global timeout is the simpler knob
        // and surfaces as one config field.
        let attempt =
            tokio::time::timeout(cfg.dial_timeout, dial_with_tls(&cfg, tls.as_ref())).await;
        let attempt = match attempt {
            Ok(r) => r,
            Err(_) => Err(ClientError::Handshake(format!(
                "dial timed out after {:?}",
                cfg.dial_timeout
            ))),
        };
        match attempt {
            Ok((registration, yconn)) => {
                tracing::info!(
                    user = %registration.user,
                    drive = %registration.drive,
                    prefix = %registration.prefix,
                    "tunnel connected",
                );
                emit(&cfg.events, TunnelEvent::Connected(registration.clone()));
                backoff = cfg.initial_backoff;
                if let Err(e) = serve_substreams(yconn, router.clone()).await {
                    tracing::warn!(error = %e, "tunnel substream loop ended");
                } else {
                    tracing::info!("tunnel disconnected");
                }
                emit(&cfg.events, TunnelEvent::Disconnected { retry_in: backoff });
            }
            Err(e) => {
                tracing::warn!(error = %e, ?backoff, "tunnel dial failed; retrying");
                emit(
                    &cfg.events,
                    TunnelEvent::DialFailed {
                        error: e.to_string(),
                        retry_in: backoff,
                    },
                );
            }
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(cfg.max_backoff);
    }
}

/// Best-effort send. Drops the event if the receiver is gone or
/// full so a slow consumer can't stall the dial loop.
fn emit(tx: &Option<mpsc::Sender<TunnelEvent>>, ev: TunnelEvent) {
    if let Some(tx) = tx {
        let _ = tx.try_send(ev);
    }
}

async fn serve_one_substream(stream: yamux::Stream, router: axum::Router) {
    let io = TokioIo::new(stream.compat());
    // The router takes Request<axum::body::Body>; hyper hands us
    // Request<hyper::body::Incoming>. Wrap the incoming body into
    // axum's so we can call the router. axum 0.7's serve helper
    // does the same internally.
    let service = tower::service_fn(move |req: http::Request<hyper::body::Incoming>| {
        let router = router.clone();
        async move {
            let (parts, body) = req.into_parts();
            let req = http::Request::from_parts(parts, axum::body::Body::new(body));
            Ok::<_, std::convert::Infallible>(
                tower::ServiceExt::oneshot(router, req)
                    .await
                    .into_response(),
            )
        }
    });
    let service = TowerToHyperService::new(service);
    if let Err(e) = hyper::server::conn::http1::Builder::new()
        .serve_connection(io, service)
        .with_upgrades()
        .await
    {
        tracing::debug!(error = %e, "substream serve_connection ended");
    }
}

use axum::response::IntoResponse;
