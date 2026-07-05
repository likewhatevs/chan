//! Reverse proxy for `*.devserver.chan.app/{workspace}/...` into the
//! `chan devserver` peer behind the registered tunnel.
//!
//! `{user}` is parsed out of the wildcard `Host` header by
//! `http::dispatch` and handed in. The gate is per-DEVSERVER: there is
//! one devserver per user, and the `{workspace}` path segment is tenant
//! routing only, never a gate key. It is forwarded into the tunnel
//! unchanged and the devserver routes the tenant internally.
//!
//! Auth gate, in this order:
//!
//!   * no live devserver registration for `{user}` -> 404
//!   * `/api/devserver/*` (the local-only management API) -> 404
//!   * request has `?t=<entry-jwt>`:
//!     * verify HS256 + exp + aud (Host) + drv (devserver id) -> mint a
//!       session JWT carrying the entry's `sub`, set host-only
//!       `devserver_gate` and `devserver_csrf` cookies scoped to `Path=/`,
//!       303 to the clean URL
//!     * any failure -> 404
//!   * request has a valid `devserver_gate` cookie (signature + aud + drv)
//!     -> pass through
//!   * anything else (no cookie, expired, wrong aud, wrong devserver)
//!     -> 404
//!
//! The auth assertion is the entry JWT, not "sub matches owner". Identity
//! mints entry tokens only after calling `profile.devserver_access(owner,
//! devserver, caller)`, so a validly-signed entry with the right aud and drv
//! proves the caller is authorized, owner or accepted grantee. The aud
//! claim (= `{owner}.devserver.chan.app`) is what enforces tenant isolation;
//! comparing `sub` against the cached owner would lock out every grantee.
//!
//! 404 is preferred over 401 / 403 on the proxy path so an
//! unauthenticated probe cannot distinguish "devserver does not exist"
//! from "devserver exists but you are not signed in" or "wrong devserver in
//! the cookie." Owners returning after the 24h cookie expires bounce
//! through the id.chan.app dashboard.
//!
//! Two transports, both ride a fresh yamux substream opened on the
//! registered `TunnelHandle`:
//!   * HTTP: hyper h1 client over the substream.
//!   * WebSocket: tungstenite's `client_async` runs the WS handshake
//!     directly on the substream.

use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, FromRequestParts, Request};
use axum::http::{header, request::Parts, HeaderMap, HeaderName, HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use chan_tunnel_proto::gateway_assertion;
use chan_tunnel_server::TunnelHandle;
use futures_util::{SinkExt, StreamExt};
use gateway_common::devserver_gate::{self, TokenType};
use http_body_util::Limited;
use hyper_util::rt::TokioIo;
use rand::RngCore;
use subtle::ConstantTimeEq;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode as TgCloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame as TgCloseFrame;
use tokio_tungstenite::tungstenite::Message as TgMessage;
use tokio_tungstenite::tungstenite::Utf8Bytes as TgUtf8Bytes;
use tokio_util::compat::FuturesAsyncReadCompatExt;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::http::AppState;

/// Wraps a response body with a hard deadline shared with the
/// `proxy_http` send_request timeout. If the upstream goes silent or
/// slow-drips bytes past the deadline, the body errors out (axum
/// closes the client connection). Without this, `request_timeout`
/// only covered headers and an unbounded-time body stream could hold
/// the substream open up to `max_response_bytes`.
///
/// Also owns an `AbortHandle` for the upstream hyper connection task
/// (`conn.with_upgrades()`). The conn task must outlive header
/// receipt so the body can stream, but must not outlive the body
/// (otherwise an aborted client request leaks a yamux substream
/// until the upstream closes on its own). Dropping the body aborts
/// the conn task; finishing the stream first lets the task exit
/// naturally and the abort is a no-op.
///
/// `axum::body::Body` is `Unpin` (boxes internally), and the boxed
/// `tokio::time::Sleep` is pinned at construction time, so the impl
/// avoids pin-projection plumbing.
struct DeadlineBody {
    inner: Body,
    sleep: Pin<Box<tokio::time::Sleep>>,
    /// Abort handle for the upstream conn task. Wrapped in
    /// `tokio_util::task::AbortOnDropHandle`? No: we want the abort
    /// to fire on body drop regardless of whether the stream
    /// completed normally (a completed conn task is a no-op on
    /// abort). Stored directly so the Drop impl can reach it.
    conn: Option<tokio::task::AbortHandle>,
}

impl DeadlineBody {
    fn new(inner: Body, deadline: tokio::time::Instant, conn: tokio::task::AbortHandle) -> Self {
        Self {
            inner,
            sleep: Box::pin(tokio::time::sleep_until(deadline)),
            conn: Some(conn),
        }
    }
}

impl Drop for DeadlineBody {
    fn drop(&mut self) {
        if let Some(c) = self.conn.take() {
            // Aborts an already-finished task as a no-op; aborts an
            // in-flight one so a leaking client doesn't strand the
            // upstream yamux substream.
            c.abort();
        }
    }
}

impl http_body::Body for DeadlineBody {
    type Data = bytes::Bytes;
    type Error = axum::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<std::result::Result<http_body::Frame<Self::Data>, Self::Error>>> {
        if self.sleep.as_mut().poll(cx).is_ready() {
            tracing::warn!("proxy response body exceeded request deadline");
            return Poll::Ready(Some(Err(axum::Error::new(
                "response body exceeded request deadline",
            ))));
        }
        Pin::new(&mut self.inner).poll_frame(cx)
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> http_body::SizeHint {
        self.inner.size_hint()
    }
}

/// Cookie name for the session-shape devserver-gate token. Host-only on
/// `{user}.devserver.chan.app`; `Path=/`; HttpOnly; Secure; SameSite=Lax;
/// 24h lifetime (matches the session JWT exp).
const COOKIE_NAME: &str = "devserver_gate";
const CSRF_COOKIE_NAME: &str = "devserver_csrf";
const CSRF_HEADER_NAME: &str = "x-chan-csrf";

/// Hop-by-hop headers we strip on both legs (RFC 7230 6.1).
/// Match is on the lowercase header name string; `HeaderName` can
/// not live in a const slice due to interior-mutability rules on
/// borrowed temporaries, so we compare via `as_str()`.
const HOP_BY_HOP_NAMES: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

fn is_hop_by_hop(name: &HeaderName) -> bool {
    HOP_BY_HOP_NAMES.contains(&name.as_str())
}

/// Parse the inbound `Connection` header into the list of header
/// names it marks as hop-by-hop. The caller strips each returned name
/// on top of the static HOP_BY_HOP_NAMES list (RFC 7230 6.1 §3).
fn connection_listed_headers(headers: &HeaderMap) -> Vec<HeaderName> {
    let Some(value) = headers
        .get(header::CONNECTION)
        .and_then(|v| v.to_str().ok())
    else {
        return Vec::new();
    };
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        // "close" / "keep-alive" are connection options, not headers.
        .filter(|s| !s.eq_ignore_ascii_case("close") && !s.eq_ignore_ascii_case("keep-alive"))
        .filter_map(|s| HeaderName::from_bytes(s.to_ascii_lowercase().as_bytes()).ok())
        .collect()
}

/// Entry point from `http::dispatch`. `user` came out of the wildcard
/// Host header. The gate is per-DEVSERVER: it resolves the user's single
/// devserver registration and verifies the `devserver_gate` cookie's
/// `drv` against that devserver id. It does NOT peel a path segment.
/// The `{workspace}` path segment is tenant routing only and is
/// forwarded into the tunnel unchanged. axum's extractors are not used
/// at this level because the dispatcher already consumed `Host`.
pub async fn handle(state: AppState, user: String, req: Request) -> Response {
    // One devserver per user: look up by the host's {user} label alone.
    let Some((devserver_id, entry)) = state.registry.get_user_devserver(&user) else {
        return not_found_response(req.headers());
    };

    // The management API is local-only; the proxy carries tenant content
    // only and never proxies `/api/devserver/*` on the public wildcard.
    if is_management_path(req.uri().path()) {
        return not_found_response(req.headers());
    }

    // The audience is the inbound Host. Tokens minted for one
    // subdomain do not validate on another.
    let aud = host_header(req.headers()).unwrap_or_default();

    let is_ws = is_websocket_upgrade(req.headers());

    // The gate always runs: every devserver tunnel is authenticated,
    // there is no un-gated pass-through.
    let caller = match resolve_gate(&state, &req, &devserver_id, &aud) {
        Gate::Pass { sub, role } => GatewayCaller { sub, role },
        Gate::IssueSession { sub, role } => {
            return issue_session_cookie(
                state.cfg.workspace_gate_secret.as_bytes(),
                sub,
                &role,
                &devserver_id,
                &aud,
                req.uri(),
            );
        }
        Gate::Reject => return not_found_response(req.headers()),
    };
    if requires_csrf(req.method()) && !csrf_header_matches_cookie(req.headers()) {
        tracing::warn!(
            aud = %aud,
            devserver_id = %devserver_id,
            method = %req.method(),
            "gateway csrf check failed",
        );
        return (StatusCode::FORBIDDEN, "forbidden").into_response();
    }
    // The registry-cached owner_id is not consulted by the proxy gate
    // (comparing `sub` against the owner would lock out grantees; the
    // signed aud + drv claims are the authorization assertion). It
    // stays in `Entry` as metadata for admin tooling.
    let _ = entry.owner_id;

    // Segment-preserving forward: hand the devserver the full public
    // `/{workspace}/...` path (only the `?t=` entry token is stripped);
    // the devserver is the single tenant-routing authority.
    let upstream_path_and_query = forward_path(req.uri());

    if is_ws {
        let (mut parts, body) = req.into_parts();
        // Pull WebSocketUpgrade out of the parts now that we've
        // gated. axum's extractor checks the upgrade headers we
        // already saw; failures here are malformed-upgrade clients.
        let upgrade = match WebSocketUpgrade::from_request_parts(&mut parts, &()).await {
            Ok(u) => u,
            Err(rej) => {
                tracing::debug!(error = %rej, "ws upgrade extractor refused");
                return rej.into_response();
            }
        };
        let forwarded = forwarded_headers(&parts, &state.cfg.forwarded_proto);
        let assertion = gateway_assertion_value(
            entry.handle.gateway_assertion_key.as_ref(),
            &caller,
            &aud,
            &devserver_id,
        );
        let handle = entry.handle.clone();
        let _ = body; // upgrade swallows the body anyway.
        return upgrade
            .on_upgrade(move |client| async move {
                if let Err(e) = bridge_ws(
                    client,
                    handle,
                    &upstream_path_and_query,
                    &forwarded,
                    assertion,
                )
                .await
                {
                    tracing::warn!(error = ?e, "ws bridge ended with error");
                }
            })
            .into_response();
    }

    let opts = ProxyOpts {
        max_request_bytes: state.cfg.max_request_bytes,
        max_response_bytes: state.cfg.max_response_bytes,
        request_timeout: state.cfg.request_timeout,
        forwarded_proto: state.cfg.forwarded_proto.as_str(),
        assertion: gateway_assertion_value(
            entry.handle.gateway_assertion_key.as_ref(),
            &caller,
            &aud,
            &devserver_id,
        ),
    };
    let res = proxy_http(entry.handle.clone(), req, upstream_path_and_query, opts).await;
    match res {
        Ok(r) => r,
        Err(e) => e.into_response(),
    }
}

/// Per-request configuration slice handed to `proxy_http`. Bundled so
/// new knobs don't churn the call signature.
#[derive(Clone)]
struct ProxyOpts<'a> {
    max_request_bytes: Option<usize>,
    max_response_bytes: Option<usize>,
    request_timeout: Option<std::time::Duration>,
    forwarded_proto: &'a str,
    assertion: Option<HeaderValue>,
}

struct GatewayCaller {
    sub: Uuid,
    role: String,
}

/// Outcome of the auth-gate decision.
enum Gate {
    /// Forward the request unchanged.
    Pass { sub: Uuid, role: String },
    /// Entry token validated; mint a session cookie carrying the
    /// entry's `sub` and 303 to the clean URL (no `?t=` query). `sub`
    /// is the user identified by identity-service at mint time,
    /// owner or accepted grantee, and is propagated into the session
    /// cookie so the upstream attribution chain stays accurate.
    IssueSession { sub: Uuid, role: String },
    /// Anything that should map to 404 on the proxy path: no token,
    /// bad signature, expired, wrong aud, wrong devserver.
    Reject,
}

fn resolve_gate(state: &AppState, req: &Request, devserver_id: &str, aud: &str) -> Gate {
    let secret = state.cfg.workspace_gate_secret.as_bytes();

    // Entry token in `?t=` takes precedence: it's how the dashboard
    // hands a freshly authenticated user off to the wildcard. A valid
    // entry triggers the cookie-mint redirect; a malformed one is
    // rejected outright (no fall-through to the cookie path, so a
    // malicious tenant cannot strip a real session cookie by appending
    // a junk `?t=`).
    //
    // We do not compare `sub` against the registry-cached owner: that
    // would lock out every accepted grantee. The aud + drv claims
    // (signed at mint time by identity, which already checked
    // `devserver_access`) are the authorization assertion. Identity owns
    // the policy; devserver-proxy verifies the assertion. `drv` is the
    // devserver id, matched against the user's live registration.
    if let Some(token) = entry_token_param(req.uri()) {
        return match devserver_gate::decode(secret, &token, TokenType::Entry, aud, devserver_id) {
            Ok(claims) => Gate::IssueSession {
                sub: claims.sub,
                role: claims.role,
            },
            Err(_) => Gate::Reject,
        };
    }

    // No entry token: any one valid session cookie admits. A browser
    // may send several `devserver_gate` cookies under unusual conditions
    // (stale cookie at a different path that got attached to this
    // request); accept the first that verifies under this aud + drv so
    // a stale duplicate doesn't 404 a legitimate session.
    for cookie in cookie_values(req.headers(), COOKIE_NAME) {
        if let Ok(claims) =
            devserver_gate::decode(secret, &cookie, TokenType::Session, aud, devserver_id)
        {
            return Gate::Pass {
                sub: claims.sub,
                role: claims.role,
            };
        }
    }
    Gate::Reject
}

/// True when a request carries a gate credential, an `?t=` entry token
/// or a `devserver_gate` session cookie. The dispatcher uses this to
/// decide what a bare wildcard `/` means: a credential-bearing root is an
/// authenticated open that falls through to the gate and is forwarded to
/// the devserver root (where the launcher SPA is served), while a naked
/// bare-domain hit bounces to the dashboard front door. This does NOT
/// validate the credential. `resolve_gate` does that on the
/// fall-through; it only distinguishes "an open attempt" from "naked".
pub(crate) fn has_gate_credential(uri: &Uri, headers: &HeaderMap) -> bool {
    entry_token_param(uri).is_some() || !cookie_values(headers, COOKIE_NAME).is_empty()
}

/// True when the path targets the devserver's local-only management API
/// (`/api/devserver` or `/api/devserver/...`). The proxy 404s it on the
/// public wildcard so only tenant content reaches the tunnel; the owner
/// manages over the direct connection.
fn is_management_path(path: &str) -> bool {
    path == "/api/devserver" || path.starts_with("/api/devserver/")
}

/// The path forwarded into the tunnel: the full inbound path+query with
/// the `?t=` entry token stripped. The proxy is a segment-PRESERVING
/// forwarder. It does NOT strip the `{workspace}` segment; the devserver
/// mounts each tenant at its public `/{workspace}/` slug and routes
/// internally. Always returns a path that starts with `/`.
fn forward_path(uri: &Uri) -> String {
    let pq = uri.path_and_query().map(|p| p.as_str()).unwrap_or("/");
    strip_entry_token_query(pq)
}

/// Pull `?t=<token>` value out of the URI. Returns None when absent
/// or empty.
fn entry_token_param(uri: &Uri) -> Option<String> {
    let q = uri.query()?;
    for pair in q.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if k == "t" && !v.is_empty() {
            return Some(percent_decode(v));
        }
    }
    None
}

/// Strip the `t=` parameter from a path+query string. Used when
/// building the clean URL we redirect to after the entry token is
/// consumed, and when forwarding upstream so the chan-serve peer
/// never sees the token.
fn strip_entry_token_query(path_and_query: &str) -> String {
    let Some((path, query)) = path_and_query.split_once('?') else {
        return path_and_query.to_string();
    };
    let filtered: Vec<&str> = query
        .split('&')
        .filter(|p| {
            let (k, _) = p.split_once('=').unwrap_or((*p, ""));
            k != "t"
        })
        .collect();
    if filtered.is_empty() {
        path.to_string()
    } else {
        format!("{path}?{}", filtered.join("&"))
    }
}

/// Minimal percent-decode for query-string values. Tokens are
/// base64url + `.` separators, so the only escapes we ever expect
/// are `%3D` (`=`), `%2E` (`.`), and `%2D` (`-`); a real decoder
/// would handle every triplet. We pull in url::form_urlencoded for
/// correctness anyway.
fn percent_decode(s: &str) -> String {
    url::form_urlencoded::parse(format!("v={s}").as_bytes())
        .next()
        .map(|(_, v)| v.into_owned())
        .unwrap_or_else(|| s.to_string())
}

/// Read the `devserver_gate` cookie value from the Cookie header(s).
/// Manual parse: this crate deliberately carries no cookie / session
/// dependency. RFC 6265 cookie-pair: `name=value; name=value; ...`.
/// Returns every match in order so the caller can fall through stale
/// duplicates (e.g. a browser sending an old + a fresh `devserver_gate`
/// under different paths that both got attached to the same request).
/// Quoted values (`name="value"`) get the quotes stripped per RFC.
fn cookie_values(headers: &HeaderMap, cookie_name: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in headers.get_all(header::COOKIE).iter() {
        let Ok(s) = raw.to_str() else { continue };
        for pair in s.split(';') {
            let pair = pair.trim();
            let Some((name, value)) = pair.split_once('=') else {
                continue;
            };
            if name == cookie_name {
                let unquoted = value
                    .strip_prefix('"')
                    .and_then(|v| v.strip_suffix('"'))
                    .unwrap_or(value);
                out.push(unquoted.to_string());
            }
        }
    }
    out
}

/// Read the inbound Host header verbatim (lowercased; HTTP Host
/// values are case-insensitive). Used as the `aud` claim in token
/// verification so a token minted for one subdomain cannot be
/// replayed on another.
fn host_header(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(chan_tunnel_proto::gateway_assertion::canonical_audience)
}

fn requires_csrf(method: &axum::http::Method) -> bool {
    method == axum::http::Method::POST
        || method == axum::http::Method::PUT
        || method == axum::http::Method::PATCH
        || method == axum::http::Method::DELETE
}

fn csrf_header_matches_cookie(headers: &HeaderMap) -> bool {
    let Some(header_value) = headers.get(CSRF_HEADER_NAME).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    cookie_values(headers, CSRF_COOKIE_NAME)
        .into_iter()
        .any(|cookie| timing_safe_eq(cookie.as_bytes(), header_value.as_bytes()))
}

fn timing_safe_eq(a: &[u8], b: &[u8]) -> bool {
    // Length mismatch still returns early, as usual for timing-safe compares.
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}

/// Detect an HTTP/1.1 upgrade-to-WebSocket request. Mirrors what
/// `WebSocketUpgrade::from_request_parts` checks before doing the
/// handshake response. We sniff up front so the auth gate runs the
/// same logic for HTTP and WS without an extra round trip through
/// the extractor.
fn is_websocket_upgrade(headers: &HeaderMap) -> bool {
    let upgrade = headers
        .get(header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("websocket"));
    let conn = headers
        .get(header::CONNECTION)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(',')
                .map(str::trim)
                .any(|t| t.eq_ignore_ascii_case("upgrade"))
        });
    upgrade && conn
}

/// Mint a session JWT for `sub`, set it as a host-only `Path=/`
/// cookie, and 303 to the clean URL (`?t=` stripped). Browsers
/// follow the 303 with the new cookie attached. `sub` comes from the
/// entry token we just verified, owner or accepted grantee, so the
/// session cookie identifies the right user for upstream attribution.
/// `Path=/` is safe because the grant is whole-devserver: every path on
/// this host is content the cookie-holder is authorized to reach, and
/// user-to-user isolation stays on the host-only `aud` claim.
fn issue_session_cookie(
    secret: &[u8],
    sub: Uuid,
    role: &str,
    devserver_id: &str,
    aud: &str,
    uri: &Uri,
) -> Response {
    let session = match devserver_gate::encode_session(secret, sub, role, devserver_id, aud) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = ?e, "failed to mint devserver_gate session token");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    };
    let csrf = random_csrf_token();
    let clean = strip_entry_token_query(uri.path_and_query().map(|p| p.as_str()).unwrap_or("/"));
    // 24h Max-Age matches the JWT exp. Cookies without Max-Age would
    // be session cookies (gone when the browser closes); we want
    // them to outlive a tab close.
    let cookie_value = format!(
        "{COOKIE_NAME}={session}; \
         Path=/; \
         HttpOnly; Secure; SameSite=Lax; Max-Age=86400"
    );
    let csrf_cookie = format!(
        "{CSRF_COOKIE_NAME}={csrf}; \
         Path=/; \
         Secure; SameSite=Lax; Max-Age=86400"
    );
    let mut res = (StatusCode::SEE_OTHER, "").into_response();
    res.headers_mut().insert(
        header::LOCATION,
        HeaderValue::from_str(&clean).unwrap_or(HeaderValue::from_static("/")),
    );
    res.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie_value).unwrap_or(HeaderValue::from_static(COOKIE_NAME)),
    );
    res.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&csrf_cookie).unwrap_or(HeaderValue::from_static(CSRF_COOKIE_NAME)),
    );
    res
}

fn random_csrf_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

async fn proxy_http(
    handle: TunnelHandle,
    req: Request,
    upstream_path_and_query: String,
    opts: ProxyOpts<'_>,
) -> Result<Response> {
    // The full request deadline (headers + body streaming) is anchored
    // at this Instant. send_request is bounded explicitly below; the
    // response body is bounded by wrapping it in DeadlineBody, which
    // shares the same deadline so a slow-drip upstream can't outlast
    // the configured timeout. Bypassed when request_timeout is None.
    let deadline = opts
        .request_timeout
        .map(|d| tokio::time::Instant::now() + d);

    let stream = handle
        .open()
        .await
        .map_err(|e| Error::Upstream(format!("tunnel disconnected: {e}")))?;
    let io = TokioIo::new(stream.compat());

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
        .await
        .map_err(|e| Error::Upstream(format!("upstream h1 handshake: {e}")))?;
    // The conn task must outlive header receipt (so the body can
    // stream back) but must not outlive the body, otherwise an
    // aborted client request leaks a yamux substream. We hand its
    // AbortHandle to DeadlineBody below; on body drop / body
    // completion, abort fires (no-op for an already-finished task).
    let conn_handle = tokio::spawn(async move {
        if let Err(e) = conn.with_upgrades().await {
            tracing::debug!(error = %e, "upstream conn ended");
        }
    });
    let conn_abort = conn_handle.abort_handle();

    let (mut parts, body) = req.into_parts();

    let forwarded = forwarded_headers(&parts, opts.forwarded_proto);
    strip_inbound_headers(&mut parts.headers);
    apply_forwarded(&mut parts.headers, &forwarded);
    apply_gateway_assertion(&mut parts.headers, opts.assertion);

    parts.uri = upstream_path_and_query
        .parse::<Uri>()
        .map_err(|e| Error::Anyhow(anyhow::anyhow!("upstream uri: {e}")))?;

    // Cap inbound request body before it reaches the upstream so a
    // malicious authenticated client cannot stream an unbounded body
    // and tie up the substream + yamux window.
    let body = match opts.max_request_bytes {
        Some(max) => Body::new(Limited::new(body, max)),
        None => Body::new(body),
    };

    let upstream_req = axum::http::Request::from_parts(parts, body);
    let send_fut = sender.send_request(upstream_req);
    let res = match deadline {
        Some(dl) => match tokio::time::timeout_at(dl, send_fut).await {
            Ok(r) => r.map_err(|e| Error::Upstream(format!("send_request: {e}")))?,
            Err(_) => {
                conn_abort.abort();
                tracing::warn!("proxy_http exceeded deadline before response headers");
                return Ok((StatusCode::GATEWAY_TIMEOUT, "upstream timed out").into_response());
            }
        },
        None => send_fut
            .await
            .map_err(|e| Error::Upstream(format!("send_request: {e}")))?,
    };

    let (parts, body) = res.into_parts();
    let mut builder = Response::builder().status(parts.status);
    for (k, v) in strip_response_headers(&parts.headers) {
        builder = builder.header(k, v);
    }
    let bounded: Body = match opts.max_response_bytes {
        Some(max) => Body::new(Limited::new(body, max)),
        None => Body::new(body),
    };
    // Wrap the body when a deadline applies so:
    //  * a slow-drip upstream is bounded end-to-end, and
    //  * dropping the body aborts the conn task so a client that
    //    bails mid-response doesn't leak the yamux substream.
    // When no deadline is configured we let the body stream
    // unwrapped; the conn task exits naturally when the upstream
    // half-closes the substream.
    let response_body = match deadline {
        Some(dl) => Body::new(DeadlineBody::new(bounded, dl, conn_abort)),
        None => {
            // Detach the abort handle so the no-deadline path keeps
            // the conn task alive for the full streaming response
            // exactly as before. `_` drops the unused handle.
            let _ = conn_abort;
            bounded
        }
    };
    builder
        .body(response_body)
        .map_err(|e| Error::Anyhow(anyhow::anyhow!("response: {e}")))
}

/// Drop hop-by-hop headers, Host, Cookie (the devserver_gate cookie has
/// no business at the upstream), Authorization (a user-presented PAT
/// or bearer has no business at the tenant's `chan devserver` either;
/// auth on this leg is the devserver_gate handshake plus the tunnel
/// trust boundary), and existing X-Forwarded-*. Honors the
/// connection-token list per RFC 7230 6.1.
fn strip_inbound_headers(headers: &mut HeaderMap) {
    let connection_listed = connection_listed_headers(headers);
    for h in HOP_BY_HOP_NAMES {
        headers.remove(*h);
    }
    for h in &connection_listed {
        headers.remove(h);
    }
    headers.remove(header::HOST);
    headers.remove(header::COOKIE);
    headers.remove(header::AUTHORIZATION);
    headers.remove(CSRF_HEADER_NAME);
    headers.remove(X_FORWARDED_FOR);
    headers.remove(X_FORWARDED_PROTO);
    headers.remove(X_FORWARDED_HOST);
}

/// Same hop-by-hop filter applied to a response HeaderMap on its way
/// back to the client. Set-Cookie is intentionally NOT stripped: if
/// the upstream tenant content wants to set its own cookies we let
/// it (they will be host-only on the tenant's subdomain, not
/// reachable to the auth surface).
fn strip_response_headers(
    headers: &HeaderMap,
) -> Vec<(axum::http::HeaderName, axum::http::HeaderValue)> {
    let connection_listed = connection_listed_headers(headers);
    headers
        .iter()
        .filter(|(k, _)| !is_hop_by_hop(k))
        .filter(|(k, _)| !connection_listed.iter().any(|h| h == *k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Bidirectional WebSocket pump.
async fn bridge_ws(
    client: WebSocket,
    handle: TunnelHandle,
    path_and_query: &str,
    forwarded: &ForwardedHeaders,
    assertion: Option<HeaderValue>,
) -> anyhow::Result<()> {
    let stream = handle.open().await?;
    let io = stream.compat();

    let upstream_url = format!("ws://chan-tunnel{path_and_query}");
    let mut request = upstream_url
        .as_str()
        .into_client_request()
        .map_err(|e| anyhow::anyhow!("build ws request: {e}"))?;
    apply_forwarded(request.headers_mut(), forwarded);
    apply_gateway_assertion(request.headers_mut(), assertion);

    let (upstream, _resp) = tokio_tungstenite::client_async(request, io)
        .await
        .map_err(|e| anyhow::anyhow!("ws handshake: {e}"))?;

    let (mut up_tx, mut up_rx) = upstream.split();
    let (mut cl_tx, mut cl_rx) = client.split();

    const IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

    let c2u = async {
        loop {
            let msg = match tokio::time::timeout(IDLE_TIMEOUT, cl_rx.next()).await {
                Ok(Some(m)) => m?,
                Ok(None) => break,
                Err(_) => {
                    tracing::debug!("ws client->upstream idle timeout");
                    break;
                }
            };
            let stop = matches!(msg, Message::Close(_));
            up_tx.send(client_to_upstream(msg)).await?;
            if stop {
                break;
            }
        }
        Ok::<(), anyhow::Error>(())
    };
    let u2c = async {
        loop {
            let msg = match tokio::time::timeout(IDLE_TIMEOUT, up_rx.next()).await {
                Ok(Some(m)) => m?,
                Ok(None) => break,
                Err(_) => {
                    tracing::debug!("ws upstream->client idle timeout");
                    break;
                }
            };
            let stop = matches!(msg, TgMessage::Close(_));
            if let Some(translated) = upstream_to_client(msg) {
                cl_tx.send(translated).await?;
            }
            if stop {
                break;
            }
        }
        Ok::<(), anyhow::Error>(())
    };
    tokio::select! {
        r = c2u => r?,
        r = u2c => r?,
    }
    Ok(())
}

// axum and tungstenite each wrap ws text payloads in their own Utf8Bytes
// type with no direct conversion between them; the Bytes round-trip in
// to_tg_utf8 / to_ax_utf8 is zero-copy, and the re-validation cannot fail
// because the source type already guarantees valid UTF-8.
fn to_tg_utf8(s: Utf8Bytes) -> TgUtf8Bytes {
    TgUtf8Bytes::try_from(Bytes::from(s)).expect("axum Utf8Bytes is valid UTF-8")
}

fn client_to_upstream(msg: Message) -> TgMessage {
    match msg {
        Message::Text(s) => TgMessage::Text(to_tg_utf8(s)),
        Message::Binary(b) => TgMessage::Binary(b),
        Message::Ping(b) => TgMessage::Ping(b),
        Message::Pong(b) => TgMessage::Pong(b),
        Message::Close(frame) => TgMessage::Close(frame.map(|f| TgCloseFrame {
            code: TgCloseCode::from(f.code),
            reason: to_tg_utf8(f.reason),
        })),
    }
}

// ---------------------------------------------------------------
// X-Forwarded-* chain
// ---------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub(crate) struct ForwardedHeaders {
    xff: Option<String>,
    proto: String,
    host: Option<String>,
}

const X_FORWARDED_FOR: &str = "x-forwarded-for";
const X_FORWARDED_PROTO: &str = "x-forwarded-proto";
const X_FORWARDED_HOST: &str = "x-forwarded-host";

/// Trust boundary: devserver-proxy is the only thing between the client
/// and the upstream `chan devserver`. Inbound `X-Forwarded-Host` /
/// `X-Forwarded-Proto` are entirely client-controlled (nginx may not
/// scrub them, and the gateway must not assume it does) so we never
/// forward those values; we re-derive `host` from the inbound `Host`
/// header devserver-proxy itself routed on, and `proto` from
/// `cfg.forwarded_proto` (configured to match the terminator that
/// fronts this listener). The inbound XFF chain is preserved because
/// dropping it would break legitimate multi-hop observability for
/// operators; nginx is expected to either strip or normalize it on
/// untrusted ingress.
pub(crate) fn forwarded_headers(parts: &Parts, proto: &str) -> ForwardedHeaders {
    let peer_ip = parts
        .extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| addr.ip().to_string());

    let existing_xff = parts
        .headers
        .get(X_FORWARDED_FOR)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    let xff = match (existing_xff, peer_ip) {
        (Some(chain), Some(p)) => Some(format!("{chain}, {p}")),
        (Some(chain), None) => Some(chain),
        (None, Some(p)) => Some(p),
        (None, None) => None,
    };

    let host = parts
        .headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    ForwardedHeaders {
        xff,
        proto: proto.to_string(),
        host,
    }
}

fn apply_forwarded(headers: &mut HeaderMap, f: &ForwardedHeaders) {
    if let Some(xff) = &f.xff {
        if let Ok(value) = HeaderValue::from_str(xff) {
            headers.insert(X_FORWARDED_FOR, value);
        }
    }
    if let Ok(value) = HeaderValue::from_str(&f.proto) {
        headers.insert(X_FORWARDED_PROTO, value);
    }
    if let Some(host) = &f.host {
        if let Ok(value) = HeaderValue::from_str(host) {
            headers.insert(X_FORWARDED_HOST, value);
        }
    }
}

fn gateway_assertion_value(
    key: Option<&gateway_assertion::AssertionKey>,
    caller: &GatewayCaller,
    aud: &str,
    devserver_id: &str,
) -> Option<HeaderValue> {
    let key = key?;
    let claims = gateway_assertion::claims(caller.sub.to_string(), &caller.role, aud, devserver_id);
    let signed = match gateway_assertion::sign(key, &claims) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = ?e, "failed to sign gateway assertion");
            return None;
        }
    };
    HeaderValue::from_str(&signed).ok()
}

fn apply_gateway_assertion(headers: &mut HeaderMap, assertion: Option<HeaderValue>) {
    headers.remove(gateway_assertion::HEADER_NAME);
    if let Some(value) = assertion {
        headers.insert(gateway_assertion::HEADER_NAME, value);
    }
}

// ---------------------------------------------------------------
// 404 page
// ---------------------------------------------------------------

fn not_found_response(headers: &HeaderMap) -> Response {
    if accepts_html(headers) {
        html_not_found()
    } else {
        Error::NotFound.into_response()
    }
}

fn accepts_html(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("text/html"))
}

fn html_not_found() -> Response {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(NOT_FOUND_HTML))
        .expect("static 404 response is valid")
}

/// Inlined so the binary has no SPA dependency. The mark is loaded
/// via a data: URL so a fresh deploy without nginx routing for
/// `/chan-mark.png` still renders. The watermark uses the same
/// CSS-mask recolor trick as the chan editor.
const NOT_FOUND_HTML: &str = r#"<!doctype html>
<html lang="en">
<meta charset="utf-8">
<title>workspace not found - chan</title>
<meta name="viewport" content="width=device-width,initial-scale=1">
<style>
  :root {
    --bg: #1c1c1e;
    --text: #f5f5f7;
    --text-secondary: #98989d;
  }
  @media (prefers-color-scheme: light) {
    :root {
      --bg: #ffffff;
      --text: #1c1c1e;
      --text-secondary: #6c6c70;
    }
  }
  html, body {
    height: 100%;
    margin: 0;
    background: var(--bg);
    color: var(--text);
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  }
  main {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    text-align: center;
    padding: 2rem;
    box-sizing: border-box;
  }
  h1 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
  }
  p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 14px;
    max-width: 36ch;
  }
</style>
<main>
  <h1>workspace unavailable</h1>
  <p>This workspace is currently unavailable.</p>
</main>
</html>
"#;

// ---------------------------------------------------------------
// WebSocket frame translation
// ---------------------------------------------------------------

// Inverse of to_tg_utf8; same zero-copy Bytes round-trip, same
// infallibility argument.
fn to_ax_utf8(s: TgUtf8Bytes) -> Utf8Bytes {
    Utf8Bytes::try_from(Bytes::from(s)).expect("tungstenite Utf8Bytes is valid UTF-8")
}

fn upstream_to_client(msg: TgMessage) -> Option<Message> {
    Some(match msg {
        TgMessage::Text(s) => Message::Text(to_ax_utf8(s)),
        TgMessage::Binary(b) => Message::Binary(b),
        TgMessage::Ping(b) => Message::Ping(b),
        TgMessage::Pong(b) => Message::Pong(b),
        TgMessage::Close(frame) => Message::Close(frame.map(|f| CloseFrame {
            code: f.code.into(),
            reason: to_ax_utf8(f.reason),
        })),
        TgMessage::Frame(_) => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_management_path_matches_devserver_api() {
        assert!(is_management_path("/api/devserver"));
        assert!(is_management_path("/api/devserver/workspaces"));
        assert!(is_management_path("/api/devserver/x/y"));
        // Tenant content that merely shares a prefix must NOT be 404'd.
        assert!(!is_management_path("/api/devserver-notes"));
        assert!(!is_management_path("/blog/api/devserver"));
        assert!(!is_management_path("/"));
        assert!(!is_management_path("/blog/"));
    }

    #[test]
    fn forward_path_preserves_segment() {
        // Segment-preserving: the {workspace} segment is forwarded
        // unchanged; only the ?t= entry token is stripped.
        let u = |s: &str| s.parse::<Uri>().unwrap();
        assert_eq!(forward_path(&u("/blog/")), "/blog/");
        assert_eq!(forward_path(&u("/blog/assets/x.js")), "/blog/assets/x.js");
        assert_eq!(forward_path(&u("/blog/?a=1")), "/blog/?a=1");
        assert_eq!(forward_path(&u("/blog")), "/blog");
    }

    #[test]
    fn forward_path_drops_t_param() {
        let u = |s: &str| s.parse::<Uri>().unwrap();
        assert_eq!(forward_path(&u("/blog/?t=abc")), "/blog/");
        assert_eq!(forward_path(&u("/blog/?t=abc&keep=1")), "/blog/?keep=1");
        assert_eq!(
            forward_path(&u("/blog/path?a=1&t=secret&b=2")),
            "/blog/path?a=1&b=2"
        );
    }

    #[test]
    fn entry_token_param_extracts() {
        let u = |s: &str| s.parse::<Uri>().unwrap();
        assert_eq!(
            entry_token_param(&u("/x/?t=abc.def.ghi")).as_deref(),
            Some("abc.def.ghi")
        );
        assert_eq!(
            entry_token_param(&u("/x/?a=1&t=tok&b=2")).as_deref(),
            Some("tok")
        );
        assert_eq!(entry_token_param(&u("/x/")), None);
        assert_eq!(entry_token_param(&u("/x/?t=")), None);
    }

    #[test]
    fn cookie_values_extracts_named_cookie() {
        let mut h = HeaderMap::new();
        h.insert(
            header::COOKIE,
            HeaderValue::from_static("foo=bar; devserver_gate=abc.def.ghi; baz=qux"),
        );
        assert_eq!(
            cookie_values(&h, COOKIE_NAME),
            vec!["abc.def.ghi".to_string()]
        );

        // Duplicate cookies: caller is responsible for picking the
        // first that verifies. We return them in header order.
        let mut h = HeaderMap::new();
        h.append(
            header::COOKIE,
            HeaderValue::from_static("devserver_gate=stale.1.x"),
        );
        h.append(
            header::COOKIE,
            HeaderValue::from_static("devserver_gate=fresh.2.y"),
        );
        assert_eq!(
            cookie_values(&h, COOKIE_NAME),
            vec!["stale.1.x", "fresh.2.y"]
        );

        // RFC-style quoted value: quotes stripped.
        let mut h = HeaderMap::new();
        h.insert(
            header::COOKIE,
            HeaderValue::from_static("devserver_gate=\"abc.def.ghi\""),
        );
        assert_eq!(
            cookie_values(&h, COOKIE_NAME),
            vec!["abc.def.ghi".to_string()]
        );

        let mut h = HeaderMap::new();
        h.insert(header::COOKIE, HeaderValue::from_static("foo=bar"));
        assert!(cookie_values(&h, COOKIE_NAME).is_empty());

        assert!(cookie_values(&HeaderMap::new(), COOKIE_NAME).is_empty());
    }

    #[test]
    fn has_gate_credential_detects_token_or_cookie() {
        let u = |s: &str| s.parse::<Uri>().unwrap();
        let empty = HeaderMap::new();
        // Naked root: no token, no cookie -> bounce to dashboard.
        assert!(!has_gate_credential(&u("/"), &empty));
        // `?t=` entry token -> authenticated open.
        assert!(has_gate_credential(&u("/?t=abc.def.ghi"), &empty));
        // An empty `t=` is not a credential.
        assert!(!has_gate_credential(&u("/?t="), &empty));
        // A `devserver_gate` session cookie -> authenticated open.
        let mut h = HeaderMap::new();
        h.insert(
            header::COOKIE,
            HeaderValue::from_static("devserver_gate=abc.def.ghi"),
        );
        assert!(has_gate_credential(&u("/"), &h));
        // An unrelated cookie is not a credential.
        let mut h = HeaderMap::new();
        h.insert(header::COOKIE, HeaderValue::from_static("foo=bar"));
        assert!(!has_gate_credential(&u("/"), &h));
    }

    #[test]
    fn websocket_upgrade_detected() {
        let mut h = HeaderMap::new();
        h.insert(header::UPGRADE, HeaderValue::from_static("websocket"));
        h.insert(header::CONNECTION, HeaderValue::from_static("Upgrade"));
        assert!(is_websocket_upgrade(&h));

        // Connection list with extra options around Upgrade.
        h.insert(
            header::CONNECTION,
            HeaderValue::from_static("keep-alive, Upgrade"),
        );
        assert!(is_websocket_upgrade(&h));

        // Non-WS upgrade.
        h.insert(header::UPGRADE, HeaderValue::from_static("h2c"));
        assert!(!is_websocket_upgrade(&h));

        // No upgrade headers.
        let h = HeaderMap::new();
        assert!(!is_websocket_upgrade(&h));
    }
}
