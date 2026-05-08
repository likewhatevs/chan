//! Tunnel listener: accepts h2c POSTs from `chan serve` clients
//! and registers them in the shared `Registry`.
//!
//! nginx terminates TLS for `tunnel.chan.app` and `grpc_pass`es
//! cleartext h2 (h2c) to this listener. We run `h2::server`
//! directly on the TCP socket; using axum/hyper here would force
//! us to glue the bidirectional body back together with mpsc
//! senders. Raw h2 lets us hand the `(SendStream, RecvStream)`
//! straight to `H2Duplex`.
//!
//! One tunnel = one h2 connection = one accepted stream. Anything
//! else (additional streams, wrong method, wrong path, missing
//! Authorization) gets a final-frame error response and the rest
//! of the connection is treated as a keepalive driver until the
//! peer closes.
use std::sync::Arc;

use chan_tunnel_proto::{H2Duplex, TUNNEL_PATH};
use http::{header, Method, Response, StatusCode};
use tokio::net::{TcpListener, TcpStream};

use crate::driver::drive_tunnel;
use crate::registry::Registry;
use crate::{handshake, ServerError, Validator};

/// Accept loop for a TCP listener bound to a tunnel-only port.
/// Returns only when the listener errors; per-connection failures
/// are logged and never bubble up.
pub async fn serve_tunnel_listener(
    listener: TcpListener,
    validator: Arc<dyn Validator>,
    registry: Arc<Registry>,
) -> std::io::Result<()> {
    loop {
        let (tcp, peer) = listener.accept().await?;
        let validator = validator.clone();
        let registry = registry.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_tunnel_conn(tcp, validator, registry).await {
                tracing::warn!(%peer, error = %e, "tunnel connection ended with error");
            } else {
                tracing::debug!(%peer, "tunnel connection closed");
            }
        });
    }
}

/// Drive a single client's h2 connection through accept,
/// validate, handshake, register, and tunnel-driver lifecycle.
async fn handle_tunnel_conn(
    tcp: TcpStream,
    validator: Arc<dyn Validator>,
    registry: Arc<Registry>,
) -> Result<(), ServerError> {
    let _ = tcp.set_nodelay(true);
    let mut conn = h2::server::handshake(tcp)
        .await
        .map_err(|e| ServerError::Handshake(format!("h2 handshake: {e}")))?;

    let (request, mut respond) = match conn.accept().await {
        Some(Ok(rs)) => rs,
        Some(Err(e)) => return Err(ServerError::Handshake(format!("h2 accept: {e}"))),
        None => return Ok(()),
    };

    if request.method() != Method::POST || request.uri().path() != TUNNEL_PATH {
        let resp = Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(())
            .expect("constant response");
        let _ = respond.send_response(resp, true);
        // Drain any further streams so the peer's GOAWAY arrives
        // cleanly; we don't expect any.
        while conn.accept().await.is_some() {}
        return Ok(());
    }

    let token = match extract_bearer(&request) {
        Some(t) => t,
        None => {
            let resp = Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(())
                .expect("constant response");
            let _ = respond.send_response(resp, true);
            while conn.accept().await.is_some() {}
            return Ok(());
        }
    };

    let (_parts, recv_body) = request.into_parts();
    let resp = Response::builder()
        .status(StatusCode::OK)
        .body(())
        .expect("constant response");
    let send = respond
        .send_response(resp, false)
        .map_err(|e| ServerError::Handshake(format!("send_response: {e}")))?;

    // Spawn the h2 frame driver before touching the duplex. The
    // duplex's reads and writes only make progress while somebody
    // is polling the connection; `accept()` does that, and rejects
    // any future streams (clients should only ever open the one
    // tunnel POST).
    tokio::spawn(async move {
        while let Some(rs) = conn.accept().await {
            if let Ok((_req, mut respond)) = rs {
                let resp = Response::builder()
                    .status(StatusCode::CONFLICT)
                    .body(())
                    .expect("constant response");
                let _ = respond.send_response(resp, true);
            }
        }
    });

    let duplex = H2Duplex::new(send, recv_body);
    let (hello, validated, yconn) = handshake(duplex, &token, validator.as_ref()).await?;

    let user: Arc<str> = Arc::from(validated.username.as_str());
    let drive: Arc<str> = Arc::from(hello.drive.as_str());
    let (handle, open_rx, shutdown_rx) = registry.register(user.clone(), drive.clone());
    tracing::info!(%user, %drive, "tunnel registered");

    drive_tunnel(yconn, open_rx, shutdown_rx, registry.clone(), handle).await;
    tracing::info!(%user, %drive, "tunnel driver exited");
    Ok(())
}

fn extract_bearer<B>(request: &http::Request<B>) -> Option<String> {
    request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
