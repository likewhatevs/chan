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
//! This is the request/response path. WebSocket upgrades land in
//! a follow-up commit and use `hyper::upgrade::on` on both ends
//! plus `tokio::io::copy_bidirectional`.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{Request, Response, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::any;
use axum::Router;
use http_body_util::BodyExt;
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
    request: Request<Body>,
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

    let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
        Ok(pair) => pair,
        Err(e) => {
            tracing::warn!(error = %e, "h1 handshake over substream failed");
            return error(StatusCode::BAD_GATEWAY, "tunnel handshake failed");
        }
    };
    // Drive the substream connection in the background. Returns
    // when either end of the substream closes.
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            tracing::debug!(error = %e, "tunnel substream conn ended");
        }
    });

    let forwarded = match build_forwarded(rest, request) {
        Ok(req) => req,
        Err(e) => {
            tracing::warn!(error = %e, "failed to build forwarded request");
            return error(StatusCode::BAD_GATEWAY, "request rewrite failed");
        }
    };

    match sender.send_request(forwarded).await {
        Ok(resp) => {
            let (parts, body) = resp.into_parts();
            // Re-wrap the body in axum's `Body`; hyper 1's body
            // already implements `http_body::Body` over `Bytes`,
            // so the conversion is a generic boxed body.
            let body = Body::new(body.map_err(axum::Error::new));
            Response::from_parts(parts, body)
        }
        Err(e) => {
            tracing::warn!(error = %e, "forwarded request failed");
            error(StatusCode::BAD_GATEWAY, "upstream request failed")
        }
    }
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
