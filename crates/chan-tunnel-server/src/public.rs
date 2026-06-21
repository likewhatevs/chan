//! Public router: serves `/{user}/{workspace}/*rest` on
//! `workspace.chan.app`.
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
/// permissive: the consumer is the workspace-proxy host process, which
/// already runs behind nginx; tighter caps are operator-driven.
#[derive(Debug, Clone)]
pub struct PublicConfig {
    /// Max bytes in a forwarded request body. Defaults to
    /// `DEFAULT_REQUEST_BODY_CAP`; oversized requests get a 413.
    pub request_body_cap: usize,
    /// Trust the `X-Forwarded-For` value on incoming requests as
    /// the existing proxy chain. When `true`, the public router
    /// appends its own `ConnectInfo` IP to the value it received.
    /// When `false` (default), incoming `X-Forwarded-For` is
    /// discarded and the value sent downstream is just the
    /// `ConnectInfo` IP.
    ///
    /// Enable only when the immediate upstream (e.g. nginx) is
    /// configured to *overwrite* the header with the real client
    /// address (e.g. `proxy_set_header X-Forwarded-For $remote_addr`).
    /// If the upstream uses `$proxy_add_x_forwarded_for` or passes
    /// the public header through, the chain is attacker-controlled
    /// and trusting it lets a public client spoof its source IP.
    pub trust_forwarded_for: bool,
    /// If non-empty, the `Host` header on each public request must
    /// end with one of these suffixes (case-insensitive); otherwise
    /// the router replies 421 Misdirected Request. Use this as a
    /// second wall when the public listener might be exposed past
    /// the fronting proxy (e.g. accidental direct hit, host
    /// misconfiguration). Empty (default) disables the check and
    /// trusts the upstream routing layer.
    ///
    /// Match is by ASCII suffix: `[".workspace.chan.app"]` matches
    /// `alice.workspace.chan.app` but not `workspace.chan.app` itself; add
    /// the bare hostname too if that should be allowed.
    pub allowed_host_suffixes: Vec<String>,
    /// Wall-clock cap on the upstream (chan-serve) handshake +
    /// response-headers phase: opens the substream, runs hyper's
    /// h1 handshake, writes the rewritten request, and waits for
    /// the response headers. A 504 Gateway Timeout is returned
    /// when this elapses. Body streaming after headers is *not*
    /// bounded by this knob; a long download is allowed to take as
    /// long as the public client and chan-serve are happy to move
    /// bytes. Default 30s; raise for slow LLM endpoints.
    pub upstream_request_timeout: Duration,
    /// Max bytes in the response body streamed back to the public
    /// client. Defaults to `DEFAULT_RESPONSE_BODY_CAP`. Counterpart
    /// to `request_body_cap`: a misbehaving (or compromised)
    /// chan-serve emitting an unbounded body otherwise gets to use
    /// the tunnel server's egress bandwidth freely. The body
    /// streams through until the cap is reached, at which point the
    /// underlying body errors mid-stream; the public client sees a
    /// truncated read.
    pub response_body_cap: usize,
    /// Per-visitor rate limit in requests per second. `0` disables
    /// the layer entirely (default). Pairs with
    /// `rate_limit_burst`: requests above the bucket size return
    /// 429 Too Many Requests. Key is the peer IP from `ConnectInfo`;
    /// behind a trusted proxy (nginx + overwriting X-Forwarded-For)
    /// the visible peer is always the proxy itself so the limiter
    /// keys on a single tenant, which is wrong. In that deployment
    /// shape, the gateway should rate-limit upstream (nginx
    /// `limit_req_zone $binary_remote_addr`) instead.
    pub rate_limit_per_second: u64,
    /// Burst above `rate_limit_per_second` a visitor can spend in a
    /// short window without being throttled. Default 32; ignored
    /// when `rate_limit_per_second == 0`.
    pub rate_limit_burst: u32,
}

impl Default for PublicConfig {
    fn default() -> Self {
        Self {
            request_body_cap: DEFAULT_REQUEST_BODY_CAP,
            trust_forwarded_for: false,
            allowed_host_suffixes: Vec::new(),
            upstream_request_timeout: DEFAULT_UPSTREAM_REQUEST_TIMEOUT,
            response_body_cap: DEFAULT_RESPONSE_BODY_CAP,
            rate_limit_per_second: 0,
            rate_limit_burst: 32,
        }
    }
}

/// Default for `PublicConfig::upstream_request_timeout`. Covers
/// the slowest realistic chan-serve response-headers latency
/// (cold-start handlers, brief disk contention) with margin.
pub const DEFAULT_UPSTREAM_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Idle cap on a hijacked upgrade (typically WebSocket). If neither
/// side moves bytes for this long, the bridge is torn down. Keeps a
/// public client that 101'd and went silent from pinning yamux
/// substream + chan-serve resources forever. 5 minutes is generous
/// for editor-style sessions; clients that disagree can reconnect.
const UPGRADE_IDLE_TIMEOUT: Duration = Duration::from_secs(300);

/// Default cap on a forwarded request body. 10 MiB covers normal
/// editor saves and small attachments; raise via `PublicConfig` for
/// workspaces that handle larger media uploads. Uncapped would let the
/// public side stream gigabytes through to chan-serve, paid for in
/// the tunnel server's egress and chan-serve's memory.
pub const DEFAULT_REQUEST_BODY_CAP: usize = 10 * 1024 * 1024;

/// Default cap on a response body streamed back to a public client.
/// 100 MiB covers most media downloads (full-resolution images,
/// short audio / video clips) without blocking large reads outright;
/// workspaces that serve big files (e.g. dataset snapshots) bump it
/// explicitly. Uncapped would let a compromised chan-serve burn the
/// tunnel server's egress bandwidth on a single request.
pub const DEFAULT_RESPONSE_BODY_CAP: usize = 100 * 1024 * 1024;

#[derive(Clone)]
struct PublicState {
    registry: Arc<Registry>,
    trust_forwarded_for: bool,
    /// Lowercased host suffix allowlist. Empty disables the check.
    allowed_host_suffixes: Arc<[String]>,
    upstream_request_timeout: Duration,
    response_body_cap: u64,
}

/// Build the public router with default knobs.
pub fn public_router(registry: Arc<Registry>) -> Router {
    public_router_with(registry, PublicConfig::default())
}

/// Build the public router with explicit knobs. Use this when the
/// host wants a non-default body cap (media-heavy workspaces) or to
/// chain in additional middleware.
pub fn public_router_with(registry: Arc<Registry>, cfg: PublicConfig) -> Router {
    let allowed_host_suffixes: Arc<[String]> = cfg
        .allowed_host_suffixes
        .iter()
        .map(|s| s.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .into();
    let state = PublicState {
        registry,
        trust_forwarded_for: cfg.trust_forwarded_for,
        allowed_host_suffixes,
        upstream_request_timeout: cfg.upstream_request_timeout,
        response_body_cap: cfg.response_body_cap as u64,
    };
    let mut router = Router::new()
        .route("/:user/:workspace", any(handle_root))
        .route("/:user/:workspace/", any(handle_root))
        .route("/:user/:workspace/*rest", any(handle_rest))
        .layer(RequestBodyLimitLayer::new(cfg.request_body_cap))
        .with_state(state);
    if cfg.rate_limit_per_second > 0 {
        // Per-IP token bucket. tower-governor defaults to
        // SmartIpKeyExtractor which honours X-Forwarded-For /
        // X-Real-IP; we don't want that, since we already strip
        // X-Forwarded-For at the request layer (see
        // `build_forwarded`). PeerIpKeyExtractor keys on the
        // direct ConnectInfo only, matching the trust model: the
        // operator is responsible for fronting nginx limit_req
        // when the visible peer is always a proxy.
        let conf = tower_governor::governor::GovernorConfigBuilder::default()
            .per_second(cfg.rate_limit_per_second)
            .burst_size(cfg.rate_limit_burst)
            .key_extractor(tower_governor::key_extractor::PeerIpKeyExtractor)
            .finish()
            .expect("governor config: per_second > 0 and burst > 0");
        router = router.layer(tower_governor::GovernorLayer {
            config: std::sync::Arc::new(conf),
        });
    }
    router
}

/// Returns true if `host` (the request's `Host` header, with any
/// `:port` stripped) ends with one of the configured suffixes. An
/// empty allowlist disables the check (returns true).
fn host_allowed(host: Option<&str>, allowed: &[String]) -> bool {
    if allowed.is_empty() {
        return true;
    }
    let Some(host) = host else {
        return false;
    };
    // Drop any port; matching is on hostname only.
    let host = host.split(':').next().unwrap_or("").to_ascii_lowercase();
    if host.is_empty() {
        return false;
    }
    allowed.iter().any(|suffix| host.ends_with(suffix))
}

async fn handle_root(
    Path((user, workspace)): Path<(String, String)>,
    State(state): State<PublicState>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    request: Request<Body>,
) -> Response<Body> {
    proxy(state, user, workspace, String::new(), connect_info, request).await
}

async fn handle_rest(
    Path((user, workspace, rest)): Path<(String, String, String)>,
    State(state): State<PublicState>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    request: Request<Body>,
) -> Response<Body> {
    proxy(state, user, workspace, rest, connect_info, request).await
}

async fn proxy(
    state: PublicState,
    user: String,
    workspace: String,
    rest: String,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    mut request: Request<Body>,
) -> Response<Body> {
    if !state.allowed_host_suffixes.is_empty() {
        let host = request
            .headers()
            .get(header::HOST)
            .and_then(|v| v.to_str().ok());
        if !host_allowed(host, &state.allowed_host_suffixes) {
            // 421 Misdirected Request signals "this server can't
            // produce a response for the combination of scheme and
            // authority in the target URI." Standard fit for a
            // host-routing mismatch.
            return error(
                StatusCode::MISDIRECTED_REQUEST,
                "host not allowed by router policy",
            );
        }
    }

    let handle = match state.registry.get(&user, &workspace) {
        Some(h) => h,
        None => return error(StatusCode::BAD_GATEWAY, "tunnel not connected"),
    };

    // Shared deadline across substream open, h1 handshake, and
    // send_request. The body stream after response headers is
    // intentionally not bound here: long downloads / uploads ride
    // the substream as long as both ends are still moving bytes.
    let deadline = tokio::time::Instant::now() + state.upstream_request_timeout;

    let substream = match tokio::time::timeout_at(deadline, handle.open()).await {
        Ok(Ok(s)) => s,
        Ok(Err(_)) => return error(StatusCode::BAD_GATEWAY, "tunnel disconnected"),
        Err(_) => {
            tracing::warn!(
                timeout = ?state.upstream_request_timeout,
                "tunnel substream open timed out",
            );
            return error(StatusCode::GATEWAY_TIMEOUT, "upstream open timed out");
        }
    };

    // futures-io -> tokio AsyncRead/Write -> hyper rt::Read/Write.
    let io = TokioIo::new(substream.compat());

    // `with_upgrades()` keeps the connection task alive past a 101
    // so the upgraded byte stream stays attached to the substream.
    let (mut sender, conn) =
        match tokio::time::timeout_at(deadline, hyper::client::conn::http1::handshake(io)).await {
            Ok(Ok(pair)) => pair,
            Ok(Err(e)) => {
                tracing::warn!(error = %e, "substream h1 handshake failed");
                return error(StatusCode::BAD_GATEWAY, "substream h1 handshake failed");
            }
            Err(_) => {
                tracing::warn!("substream h1 handshake timed out");
                return error(StatusCode::GATEWAY_TIMEOUT, "upstream handshake timed out");
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
    let forwarded =
        match build_forwarded(rest, request, peer_ip.as_deref(), state.trust_forwarded_for) {
            Ok(req) => req,
            Err(e) => {
                tracing::warn!(error = %e, "failed to build forwarded request");
                return error(StatusCode::BAD_GATEWAY, "request rewrite failed");
            }
        };

    let mut resp = match tokio::time::timeout_at(deadline, sender.send_request(forwarded)).await {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "forwarded request failed");
            return error(StatusCode::BAD_GATEWAY, "upstream request failed");
        }
        Err(_) => {
            tracing::warn!(
                timeout = ?state.upstream_request_timeout,
                "upstream response headers timed out",
            );
            return error(StatusCode::GATEWAY_TIMEOUT, "upstream response timed out");
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

    let (mut parts, body) = resp.into_parts();
    // Cap the response stream. `Limited` aborts mid-body once the
    // cap is hit; the public client sees a truncated read. The cap
    // is the policy hook for "a misbehaving chan-serve should not
    // get to burn unbounded egress on a single request".
    //
    // Strip Content-Length: a wrapped body that truncates can no
    // longer honour the upstream's declared length, and hyper
    // refuses to serialise a body whose length disagrees with the
    // header. The response goes out chunked instead, which is fine
    // for HTTP/1.1 public clients.
    parts.headers.remove(header::CONTENT_LENGTH);
    let limited = http_body_util::Limited::new(body, state.response_body_cap as usize);
    let body = Body::new(limited.map_err(axum::Error::new));
    Response::from_parts(parts, body)
}

/// Build the request that goes into the tunnel: same method,
/// headers, and body as the public request, with the path rewritten
/// to drop the `/{user}/{workspace}` prefix and the URI scheme/authority
/// dropped (h1 over a substream doesn't use them).
///
/// Header policy (defense-in-depth against header spoofing from the
/// public side; the immediate upstream is assumed to be a trusted
/// proxy that already normalised these but we do not rely on it):
///
/// - `Forwarded`, `X-Forwarded-Proto`, `X-Forwarded-Host`,
///   `X-Real-IP` are stripped unconditionally; the public router
///   sets these fresh from its own view of the request.
/// - `Proxy-Authorization`, `Proxy-Authenticate` are stripped (they
///   are hop-by-hop credentials that have no business reaching
///   chan-serve).
/// - `Authorization`, `Cookie`, and `Set-Cookie` request headers
///   are stripped. Public-router authentication, when present, is
///   handled by the fronting workspace-proxy layer; public visitors must
///   not be able to inject bearer tokens or cookie state into the
///   local chan-serve process.
/// - `X-Forwarded-For`: if `trust_forwarded_for` is `false`
///   (default), the incoming value is discarded and the resulting
///   value is just the `ConnectInfo` peer IP. If `true`, the
///   ConnectInfo IP is appended to the incoming chain. Trusting it
///   is only safe when the immediate upstream proxy is configured
///   to *overwrite* X-Forwarded-For (e.g. nginx
///   `proxy_set_header X-Forwarded-For $remote_addr`); otherwise a
///   public client can spoof its source IP.
/// - `X-Forwarded-Proto` is set to `"https"` (production assumption:
///   the gateway terminates TLS). Hosts running cleartext stacks
///   can rewrite in their own middleware.
/// - `X-Forwarded-Host` is set from the request's `Host` header.
fn build_forwarded(
    rest: String,
    request: Request<Body>,
    peer_ip: Option<&str>,
    trust_forwarded_for: bool,
) -> Result<Request<Body>, http::Error> {
    let (mut parts, body) = request.into_parts();

    let original_host = parts
        .headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Capture the incoming XFF *before* sanitisation, in case we
    // are configured to trust the upstream-normalised chain.
    let incoming_xff = if trust_forwarded_for {
        parts
            .headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    } else {
        None
    };

    // Strip every public-side-controlled forwarded / proxy header
    // before re-injecting our own. `remove` is a no-op when absent.
    for name in [
        "forwarded",
        "x-forwarded-for",
        "x-forwarded-proto",
        "x-forwarded-host",
        "x-real-ip",
        "proxy-authorization",
        "proxy-authenticate",
        "authorization",
        "cookie",
        "set-cookie",
    ] {
        parts.headers.remove(name);
    }

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

    if let Some(ip) = peer_ip {
        let value = match incoming_xff {
            Some(existing) => format!("{existing}, {ip}"),
            None => ip.to_string(),
        };
        if let Ok(hv) = HeaderValue::from_str(&value) {
            parts
                .headers
                .insert(HeaderName::from_static("x-forwarded-for"), hv);
        }
    } else if let Some(existing) = incoming_xff {
        if let Ok(hv) = HeaderValue::from_str(&existing) {
            parts
                .headers
                .insert(HeaderName::from_static("x-forwarded-for"), hv);
        }
    }
    parts.headers.insert(
        HeaderName::from_static("x-forwarded-proto"),
        HeaderValue::from_static("https"),
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    fn req_with(headers: &[(&str, &str)], host: Option<&str>) -> Request<Body> {
        let mut b = Request::builder().method("GET").uri("/notes/foo");
        if let Some(h) = host {
            b = b.header(header::HOST, h);
        }
        for (k, v) in headers {
            b = b.header(*k, *v);
        }
        b.body(Body::empty()).unwrap()
    }

    fn hv<'a>(req: &'a Request<Body>, name: &str) -> Option<&'a str> {
        req.headers().get(name).and_then(|v| v.to_str().ok())
    }

    #[test]
    fn untrusted_xff_is_replaced_with_connect_ip() {
        let req = req_with(
            &[("x-forwarded-for", "1.2.3.4")],
            Some("alice.workspace.chan.app"),
        );
        let out = build_forwarded("foo".into(), req, Some("10.0.0.1"), false).unwrap();
        assert_eq!(hv(&out, "x-forwarded-for"), Some("10.0.0.1"));
    }

    #[test]
    fn trusted_xff_is_appended_to_existing_chain() {
        let req = req_with(
            &[("x-forwarded-for", "1.2.3.4")],
            Some("alice.workspace.chan.app"),
        );
        let out = build_forwarded("foo".into(), req, Some("10.0.0.1"), true).unwrap();
        assert_eq!(hv(&out, "x-forwarded-for"), Some("1.2.3.4, 10.0.0.1"));
    }

    #[test]
    fn forwarded_and_real_ip_are_stripped() {
        let req = req_with(
            &[
                ("forwarded", "for=1.2.3.4;proto=http"),
                ("x-real-ip", "1.2.3.4"),
            ],
            Some("alice.workspace.chan.app"),
        );
        let out = build_forwarded("foo".into(), req, Some("10.0.0.1"), false).unwrap();
        assert!(out.headers().get("forwarded").is_none());
        assert!(out.headers().get("x-real-ip").is_none());
    }

    #[test]
    fn proxy_auth_headers_are_stripped() {
        let req = req_with(
            &[
                ("proxy-authorization", "Basic AAA"),
                ("proxy-authenticate", "Basic"),
            ],
            Some("alice.workspace.chan.app"),
        );
        let out = build_forwarded("foo".into(), req, Some("10.0.0.1"), true).unwrap();
        assert!(out.headers().get("proxy-authorization").is_none());
        assert!(out.headers().get("proxy-authenticate").is_none());
    }

    #[test]
    fn public_credentials_are_stripped() {
        let req = req_with(
            &[
                ("authorization", "Bearer public-supplied"),
                ("cookie", "sid=attacker"),
                ("set-cookie", "sid=attacker"),
            ],
            Some("alice.workspace.chan.app"),
        );
        let out = build_forwarded("foo".into(), req, Some("10.0.0.1"), false).unwrap();
        assert!(out.headers().get("authorization").is_none());
        assert!(out.headers().get("cookie").is_none());
        assert!(out.headers().get("set-cookie").is_none());
    }

    #[test]
    fn forwarded_proto_is_always_https() {
        let req = req_with(
            &[("x-forwarded-proto", "http")],
            Some("alice.workspace.chan.app"),
        );
        let out = build_forwarded("foo".into(), req, Some("10.0.0.1"), false).unwrap();
        assert_eq!(hv(&out, "x-forwarded-proto"), Some("https"));
    }

    #[test]
    fn forwarded_host_comes_from_host_header() {
        let req = req_with(
            &[("x-forwarded-host", "evil.example")],
            Some("alice.workspace.chan.app"),
        );
        let out = build_forwarded("foo".into(), req, Some("10.0.0.1"), false).unwrap();
        assert_eq!(
            hv(&out, "x-forwarded-host"),
            Some("alice.workspace.chan.app")
        );
    }

    #[test]
    fn rest_is_used_as_path() {
        let req = req_with(&[], Some("alice.workspace.chan.app"));
        let out = build_forwarded("inner/path".into(), req, Some("10.0.0.1"), false).unwrap();
        assert_eq!(out.uri().path(), "/inner/path");
    }

    #[test]
    fn host_allowed_empty_allowlist_lets_everything_through() {
        assert!(host_allowed(Some("anything.example"), &[]));
        assert!(host_allowed(None, &[]));
    }

    #[test]
    fn host_allowed_matches_suffix_case_insensitive() {
        let allow = vec![".workspace.chan.app".to_string()];
        assert!(host_allowed(Some("alice.workspace.chan.app"), &allow));
        assert!(host_allowed(Some("ALICE.Workspace.Chan.App"), &allow));
        assert!(host_allowed(Some("alice.workspace.chan.app:8443"), &allow));
        assert!(!host_allowed(Some("evil.example"), &allow));
        assert!(!host_allowed(Some("workspace.chan.app"), &allow)); // bare apex not in suffix
        assert!(!host_allowed(None, &allow));
        assert!(!host_allowed(Some(""), &allow));
    }

    #[test]
    fn host_allowed_multiple_suffixes() {
        let allow = vec![
            ".workspace.chan.app".to_string(),
            "workspace.chan.app".to_string(),
        ];
        assert!(host_allowed(Some("alice.workspace.chan.app"), &allow));
        assert!(host_allowed(Some("workspace.chan.app"), &allow));
    }

    #[test]
    fn empty_rest_becomes_root() {
        let req = req_with(&[], Some("alice.workspace.chan.app"));
        let out = build_forwarded(String::new(), req, Some("10.0.0.1"), false).unwrap();
        assert_eq!(out.uri().path(), "/");
    }

    #[tokio::test]
    async fn proxy_times_out_when_substream_open_waits_forever() {
        let registry = Registry::new();
        let (_handle, _open_rx, _shutdown_rx) = registry
            .register_with_cap(Arc::from("alice"), Arc::from("notes"), None, 0)
            .unwrap();
        let state = PublicState {
            registry,
            trust_forwarded_for: false,
            allowed_host_suffixes: Vec::new().into(),
            upstream_request_timeout: Duration::from_millis(25),
            response_body_cap: DEFAULT_RESPONSE_BODY_CAP as u64,
        };
        let request = Request::builder()
            .method("GET")
            .uri("/alice/notes/stalled")
            .body(Body::empty())
            .unwrap();

        let response = tokio::time::timeout(
            Duration::from_secs(1),
            proxy(
                state,
                "alice".into(),
                "notes".into(),
                "stalled".into(),
                None,
                request,
            ),
        )
        .await
        .expect("proxy should return on upstream_request_timeout");

        assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);
    }
}
