//! Public router: serves `/{user}/{drive}/*rest` on
//! `drive.chan.app`.
//!
//! For each request, looks up the corresponding `TunnelHandle`,
//! opens a fresh yamux substream, runs an HTTP/1.1 client over it
//! against the registered `chan serve`, and pipes the response
//! back. h1 (not h2) over yamux because the substream itself
//! already provides the per-request multiplexing layer; stacking
//! h2 inside would be mux-on-mux.
//!
//! WebSocket and other Upgrade flows are handled inline: when the
//! substream returns 101, the public-side and tunnel-side
//! `OnUpgrade` futures are awaited in a detached task and bridged
//! with `tokio::io::copy_bidirectional`. Bytes ride the same
//! yamux substream until either end closes.

use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::{ConnectInfo, Path, State};
use axum::http::{header, HeaderName, HeaderValue, Request, Response, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::any;
use axum::Router;
use http_body_util::BodyExt;
use hyper::upgrade::OnUpgrade;
use hyper_util::rt::TokioIo;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tower_http::limit::RequestBodyLimitLayer;

use crate::registry::Registry;

/// Public-side router knobs the host can override. Keep this
/// permissive: the consumer is the drive-proxy host process, which
/// already runs behind nginx; tighter caps are operator-driven.
#[derive(Debug, Clone)]
pub struct PublicConfig {
    /// Max bytes in a forwarded request body. Defaults to
    /// `DEFAULT_REQUEST_BODY_CAP`; oversized requests get a 413.
    pub request_body_cap: usize,
}

impl Default for PublicConfig {
    fn default() -> Self {
        Self {
            request_body_cap: DEFAULT_REQUEST_BODY_CAP,
        }
    }
}

/// Idle cap on a hijacked upgrade (typically WebSocket). If neither
/// side moves bytes for this long, the bridge is torn down. Keeps a
/// public client that 101'd and went silent from pinning yamux
/// substream + chan-serve resources forever. 5 minutes is generous
/// for editor-style sessions; clients that disagree can reconnect.
const UPGRADE_IDLE_TIMEOUT: Duration = Duration::from_secs(300);

/// Default cap on a forwarded request body. 10 MiB covers normal
/// editor saves and small attachments; raise via `PublicConfig` for
/// drives that handle larger media uploads. Uncapped would let the
/// public side stream gigabytes through to chan-serve, paid for in
/// the tunnel server's egress and chan-serve's memory.
pub const DEFAULT_REQUEST_BODY_CAP: usize = 10 * 1024 * 1024;

#[derive(Clone)]
struct PublicState {
    registry: Arc<Registry>,
}

/// Build the public router with default knobs.
pub fn public_router(registry: Arc<Registry>) -> Router {
    public_router_with(registry, PublicConfig::default())
}

/// Build the public router with explicit knobs. Use this when the
/// host wants a non-default body cap (media-heavy drives) or to
/// chain in additional middleware.
pub fn public_router_with(registry: Arc<Registry>, cfg: PublicConfig) -> Router {
    let state = PublicState { registry };
    Router::new()
        .route("/:user/:drive", any(handle_root))
        .route("/:user/:drive/", any(handle_root))
        .route("/:user/:drive/*rest", any(handle_rest))
        .layer(RequestBodyLimitLayer::new(cfg.request_body_cap))
        .with_state(state)
}

async fn handle_root(
    Path((user, drive)): Path<(String, String)>,
    State(state): State<PublicState>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    request: Request<Body>,
) -> Response<Body> {
    proxy(state, user, drive, String::new(), connect_info, request).await
}

async fn handle_rest(
    Path((user, drive, rest)): Path<(String, String, String)>,
    State(state): State<PublicState>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    request: Request<Body>,
) -> Response<Body> {
    proxy(state, user, drive, rest, connect_info, request).await
}

async fn proxy(
    state: PublicState,
    user: String,
    drive: String,
    rest: String,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    mut request: Request<Body>,
) -> Response<Body> {
    let handle = match state.registry.get(&user, &drive) {
        Some(h) => h,
        None => return error(StatusCode::BAD_GATEWAY, "tunnel not connected"),
    };

    let substream = match handle.open().await {
        Ok(s) => s,
        Err(_) => return error(StatusCode::BAD_GATEWAY, "tunnel disconnected"),
    };

    // futures-io -> tokio AsyncRead/Write -> hyper rt::Read/Write.
    let io = TokioIo::new(substream.compat());

    // `with_upgrades()` keeps the connection task alive past a 101
    // so the upgraded byte stream stays attached to the substream.
    let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
        Ok(pair) => pair,
        Err(e) => {
            tracing::warn!(error = %e, "substream h1 handshake failed");
            return error(StatusCode::BAD_GATEWAY, "substream h1 handshake failed");
        }
    };
    tokio::spawn(async move {
        if let Err(e) = conn.with_upgrades().await {
            tracing::debug!(error = %e, "tunnel substream conn ended");
        }
    });

    // Pull the public-side OnUpgrade future *before* the request is
    // consumed. axum/hyper stash it in the request extensions; the
    // body is left untouched so we can still forward it.
    let public_upgrade: OnUpgrade = hyper::upgrade::on(&mut request);

    let peer_ip = connect_info.map(|ConnectInfo(a)| a.ip().to_string());
    let forwarded = match build_forwarded(rest, request, peer_ip.as_deref()) {
        Ok(req) => req,
        Err(e) => {
            tracing::warn!(error = %e, "failed to build forwarded request");
            return error(StatusCode::BAD_GATEWAY, "request rewrite failed");
        }
    };

    let mut resp = match sender.send_request(forwarded).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "forwarded request failed");
            return error(StatusCode::BAD_GATEWAY, "upstream request failed");
        }
    };

    // 101 Switching Protocols: hijack both ends and ferry bytes.
    // hyper fires both `OnUpgrade` futures *after* its own state
    // machine sends/receives the 101, which is why this needs to
    // run in a spawned task; the futures here can't resolve before
    // we return the response to axum.
    if resp.status() == StatusCode::SWITCHING_PROTOCOLS {
        let tunnel_upgrade: OnUpgrade = hyper::upgrade::on(&mut resp);
        tokio::spawn(async move {
            let public_io = match public_upgrade.await {
                Ok(io) => io,
                Err(e) => {
                    tracing::warn!(error = %e, "public upgrade failed");
                    return;
                }
            };
            let tunnel_io = match tunnel_upgrade.await {
                Ok(io) => io,
                Err(e) => {
                    tracing::warn!(error = %e, "tunnel upgrade failed");
                    return;
                }
            };
            let public_io = TokioIo::new(public_io);
            let tunnel_io = TokioIo::new(tunnel_io);
            // Activity counter shared between both wrapped halves;
            // each successful read/write bumps it so the watchdog
            // below can tell "idle" from "still flowing". Without
            // this, a public client that 101'd and went silent
            // would pin a yamux substream until either end happened
            // to close.
            let activity = Arc::new(AtomicU64::new(0));
            let mut public_io = Activity::new(public_io, activity.clone());
            let mut tunnel_io = Activity::new(tunnel_io, activity.clone());
            let activity_for_watchdog = activity.clone();
            let copy = tokio::io::copy_bidirectional(&mut public_io, &mut tunnel_io);
            let watchdog = async move {
                let mut last_seen = activity_for_watchdog.load(Ordering::Relaxed);
                let tick = (UPGRADE_IDLE_TIMEOUT / 4).max(Duration::from_secs(15));
                loop {
                    tokio::time::sleep(tick).await;
                    let now = activity_for_watchdog.load(Ordering::Relaxed);
                    if now == last_seen {
                        return;
                    }
                    last_seen = now;
                }
            };
            tokio::select! {
                res = copy => {
                    if let Err(e) = res {
                        tracing::debug!(error = %e, "upgraded copy ended");
                    }
                }
                _ = watchdog => {
                    tracing::info!(
                        idle = ?UPGRADE_IDLE_TIMEOUT,
                        "upgraded copy reaped after idle timeout",
                    );
                }
            }
        });
    }

    let (parts, body) = resp.into_parts();
    let body = Body::new(body.map_err(axum::Error::new));
    Response::from_parts(parts, body)
}

/// Build the request that goes into the tunnel: same method,
/// headers, and body as the public request, with the path rewritten
/// to drop the `/{user}/{drive}` prefix and the URI scheme/authority
/// dropped (h1 over a substream doesn't use them).
///
/// Also injects standard `X-Forwarded-*` headers so chan-serve can
/// see the real public client IP, the original scheme, and the host
/// the user typed. Without these, every request looks like it came
/// from the loopback substream over plain HTTP.
fn build_forwarded(
    rest: String,
    request: Request<Body>,
    peer_ip: Option<&str>,
) -> Result<Request<Body>, http::Error> {
    let (mut parts, body) = request.into_parts();

    let original_host = parts
        .headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    // Best-effort: behind nginx grpc_pass, the public scheme isn't
    // visible directly. Honor an existing X-Forwarded-Proto from the
    // upstream proxy if present, else default to "https" since the
    // tunnel is always served over TLS in production. Hosts that
    // run cleartext local stacks can rewrite this in middleware.
    let original_proto = parts
        .headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "https".to_string());

    let path = if rest.is_empty() {
        "/".to_string()
    } else {
        format!("/{rest}")
    };
    let path_and_query = match parts.uri.query() {
        Some(q) => format!("{path}?{q}"),
        None => path,
    };
    parts.uri = Uri::builder().path_and_query(path_and_query).build()?;

    // Append (don't replace) X-Forwarded-For so chained proxies
    // accumulate the chain rather than clobber it.
    if let Some(ip) = peer_ip {
        let value = match parts
            .headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
        {
            Some(existing) => format!("{existing}, {ip}"),
            None => ip.to_string(),
        };
        if let Ok(hv) = HeaderValue::from_str(&value) {
            parts
                .headers
                .insert(HeaderName::from_static("x-forwarded-for"), hv);
        }
    }
    if let Ok(hv) = HeaderValue::from_str(&original_proto) {
        parts
            .headers
            .insert(HeaderName::from_static("x-forwarded-proto"), hv);
    }
    if let Some(host) = original_host {
        if let Ok(hv) = HeaderValue::from_str(&host) {
            parts
                .headers
                .insert(HeaderName::from_static("x-forwarded-host"), hv);
        }
    }

    Ok(Request::from_parts(parts, body))
}

fn error(status: StatusCode, msg: &'static str) -> Response<Body> {
    (status, msg).into_response()
}

/// AsyncRead + AsyncWrite passthrough that bumps a shared counter
/// on every byte that actually flowed. The counter is wall-clock
/// monotonic milliseconds via `Instant`'s elapsed-since-process-start;
/// the watchdog only reads it for change-detection so absolute
/// values don't matter.
///
/// Using `Instant::elapsed()` rather than `SystemTime::now()` keeps
/// us safe from wall-clock jumps (NTP slew, suspend/resume) which
/// would otherwise look like activity.
struct Activity<S> {
    inner: S,
    counter: Arc<AtomicU64>,
    base: Instant,
}

impl<S> Activity<S> {
    fn new(inner: S, counter: Arc<AtomicU64>) -> Self {
        Self {
            inner,
            counter,
            base: Instant::now(),
        }
    }

    fn bump(&self) {
        let ms = self.base.elapsed().as_millis() as u64;
        // Relaxed is fine: we only need to *change* the value; a
        // brief race where two writers stamp the same value does
        // no harm. Ordering across threads isn't required.
        self.counter.store(ms, Ordering::Relaxed);
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for Activity<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let pre = buf.filled().len();
        let res = Pin::new(&mut self.inner).poll_read(cx, buf);
        if matches!(res, Poll::Ready(Ok(()))) && buf.filled().len() > pre {
            self.bump();
        }
        res
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for Activity<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let res = Pin::new(&mut self.inner).poll_write(cx, buf);
        if let Poll::Ready(Ok(n)) = &res {
            if *n > 0 {
                self.bump();
            }
        }
        res
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}
