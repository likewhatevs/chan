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

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{Request, Response, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::any;
use axum::Router;
use http_body_util::BodyExt;
use hyper::upgrade::OnUpgrade;
use hyper_util::rt::TokioIo;
use tokio_util::compat::FuturesAsyncReadCompatExt;

use crate::registry::Registry;

#[derive(Clone)]
struct PublicState {
    registry: Arc<Registry>,
}

pub fn public_router(registry: Arc<Registry>) -> Router {
    let state = PublicState { registry };
    Router::new()
        .route("/:user/:drive", any(handle_root))
        .route("/:user/:drive/", any(handle_root))
        .route("/:user/:drive/*rest", any(handle_rest))
        .with_state(state)
}

async fn handle_root(
    Path((user, drive)): Path<(String, String)>,
    State(state): State<PublicState>,
    request: Request<Body>,
) -> Response<Body> {
    proxy(state, user, drive, String::new(), request).await
}

async fn handle_rest(
    Path((user, drive, rest)): Path<(String, String, String)>,
    State(state): State<PublicState>,
    request: Request<Body>,
) -> Response<Body> {
    proxy(state, user, drive, rest, request).await
}

async fn proxy(
    state: PublicState,
    user: String,
    drive: String,
    rest: String,
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
            tracing::warn!(error = %e, "h1 handshake over substream failed");
            return error(StatusCode::BAD_GATEWAY, "tunnel handshake failed");
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

    let forwarded = match build_forwarded(rest, request) {
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
            let mut public_io = TokioIo::new(public_io);
            let mut tunnel_io = TokioIo::new(tunnel_io);
            if let Err(e) = tokio::io::copy_bidirectional(&mut public_io, &mut tunnel_io).await {
                tracing::debug!(error = %e, "upgraded copy ended");
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
fn build_forwarded(rest: String, request: Request<Body>) -> Result<Request<Body>, http::Error> {
    let (mut parts, body) = request.into_parts();

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

    Ok(Request::from_parts(parts, body))
}

fn error(status: StatusCode, msg: &'static str) -> Response<Body> {
    (status, msg).into_response()
}
