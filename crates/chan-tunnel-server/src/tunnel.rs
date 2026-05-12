//! Tunnel listener: accepts h2c POSTs from `chan serve` clients
//! and registers them in the shared `Registry`.
//!
//! nginx terminates TLS for `drive.chan.app` and `grpc_pass`es
//! `/v1/tunnel` as cleartext h2 (h2c) to this listener; everything
//! else on the apex hits the axum HTTP listener. We run `h2::server`
//! directly on the TCP socket; using axum/hyper here would force us
//! to glue the bidirectional body back together with mpsc senders.
//! Raw h2 lets us hand the `(SendStream, RecvStream)` straight to
//! `H2Duplex`.
//!
//! One tunnel = one h2 connection = one accepted stream. Anything
//! else (additional streams, wrong method, wrong path, missing
//! Authorization) gets a final-frame error response and the rest
//! of the connection is treated as a keepalive driver until the
//! peer closes.
use std::net::SocketAddr;
use std::sync::Arc;

use chan_tunnel_proto::{H2Duplex, TUNNEL_PATH};
use http::{header, Method, Response, StatusCode};
use tokio::net::{TcpListener, TcpStream};

use crate::driver::drive_tunnel;
use crate::registry::Registry;
use crate::{handshake_validated, ServerError, Validator};

/// Accept loop for a TCP listener bound to a tunnel-only port.
/// Returns only when the listener errors; per-connection failures
/// are logged and never bubble up.
///
/// `max_drives_per_user` caps the number of distinct drives a
/// single user may have registered concurrently. `0` disables the
/// limit. A reconnect of a drive the user already has registered is
/// always allowed; the registry's last-writer-wins policy evicts
/// the stale entry before the count is checked again.
pub async fn serve_tunnel_listener(
    listener: TcpListener,
    validator: Arc<dyn Validator>,
    registry: Arc<Registry>,
    max_drives_per_user: usize,
) -> std::io::Result<()> {
    loop {
        let (tcp, peer) = listener.accept().await?;
        let validator = validator.clone();
        let registry = registry.clone();
        tokio::spawn(async move {
            if let Err(e) =
                handle_tunnel_conn(tcp, peer, validator, registry, max_drives_per_user).await
            {
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
    peer: SocketAddr,
    validator: Arc<dyn Validator>,
    registry: Arc<Registry>,
    max_drives_per_user: usize,
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

    // Spawn the h2 frame driver BEFORE we await on the validator.
    // The h2 connection only makes progress while somebody is
    // polling it; the validate call is potentially a network round
    // trip to the identity service, and without an active driver
    // the connection would stall (no PINGs, no frame parsing).
    // The drainer also rejects any stream beyond the first one
    // (clients should only ever open the tunnel POST).
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

    // Validate the token BEFORE sending 200. This lets us return
    // 401/403 for bad tokens or missing scope, which the dial path
    // (chan-tunnel-client::dial) special-cases. Sending 200 first
    // and then closing the stream collapses every auth failure into
    // a generic "handshake error" on the client, which made auth
    // problems indistinguishable from transport ones.
    let validated = match validator.validate(&token).await {
        Ok(v) => v,
        Err(e) => {
            let status = match &e {
                ServerError::InvalidToken => StatusCode::UNAUTHORIZED,
                ServerError::Identity(_) => StatusCode::BAD_GATEWAY,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            let resp = Response::builder()
                .status(status)
                .body(())
                .expect("constant response");
            let _ = respond.send_response(resp, true);
            return Err(e);
        }
    };
    if !validated.scopes.iter().any(|s| s == "tunnel") {
        let resp = Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(())
            .expect("constant response");
        let _ = respond.send_response(resp, true);
        return Err(ServerError::MissingScope);
    }

    let resp = Response::builder()
        .status(StatusCode::OK)
        .body(())
        .expect("constant response");
    let send = respond
        .send_response(resp, false)
        .map_err(|e| ServerError::Handshake(format!("send_response: {e}")))?;

    let duplex = H2Duplex::new(send, recv_body);
    let registry_for_check = registry.clone();
    let (hello, validated, yconn) = handshake_validated(duplex, validated, |hello, validated| {
        if max_drives_per_user == 0 {
            return Ok(());
        }
        let drives = registry_for_check.list_drives_for(&validated.username);
        let already_present = drives
            .iter()
            .any(|d| d.drive.as_ref() == hello.drive.as_str());
        if !already_present && drives.len() >= max_drives_per_user {
            return Err(ServerError::TooManyDrives {
                user: validated.username.clone(),
                max: max_drives_per_user,
            });
        }
        Ok(())
    })
    .await?;

    let user: Arc<str> = Arc::from(validated.username.as_str());
    let drive: Arc<str> = Arc::from(hello.drive.as_str());
    let public = hello.public;
    let (handle, open_rx, shutdown_rx) =
        registry.register(user.clone(), drive.clone(), public, Some(peer));
    tracing::info!(%user, %drive, public, "tunnel registered");

    drive_tunnel(yconn, open_rx, shutdown_rx, registry.clone(), handle).await;
    tracing::info!(%user, %drive, "tunnel driver exited");
    Ok(())
}

/// Pull a Bearer token out of an Authorization header. Per RFC 6750
/// the scheme name is case-insensitive ("Bearer", "bearer", "BEARER"
/// all valid); some clients in the wild only emit lowercase, so a
/// strict prefix match would 401 them. Token value is trimmed and
/// rejected if empty.
fn extract_bearer<B>(request: &http::Request<B>) -> Option<String> {
    let raw = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())?;
    let (scheme, token) = raw.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("Bearer") {
        return None;
    }
    let token = token.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::extract_bearer;
    use http::header::AUTHORIZATION;

    fn req_with_auth(value: &str) -> http::Request<()> {
        http::Request::builder()
            .header(AUTHORIZATION, value)
            .body(())
            .unwrap()
    }

    #[test]
    fn extract_bearer_canonical() {
        assert_eq!(
            extract_bearer(&req_with_auth("Bearer abc")).as_deref(),
            Some("abc")
        );
    }

    #[test]
    fn extract_bearer_case_insensitive() {
        for scheme in ["bearer", "BEARER", "BeArEr"] {
            assert_eq!(
                extract_bearer(&req_with_auth(&format!("{scheme} tok"))).as_deref(),
                Some("tok"),
                "scheme {scheme}",
            );
        }
    }

    #[test]
    fn extract_bearer_rejects_other_schemes() {
        assert!(extract_bearer(&req_with_auth("Basic dXNlcjpwYXNz")).is_none());
        assert!(extract_bearer(&req_with_auth("Token abc")).is_none());
    }

    #[test]
    fn extract_bearer_empty_or_whitespace_token_rejected() {
        assert!(extract_bearer(&req_with_auth("Bearer ")).is_none());
        assert!(extract_bearer(&req_with_auth("Bearer    ")).is_none());
    }

    #[test]
    fn extract_bearer_trims_token() {
        assert_eq!(
            extract_bearer(&req_with_auth("Bearer   spaced  ")).as_deref(),
            Some("spaced")
        );
    }
}
