//! TLS + h2 dial path for the chan-tunnel client.
//!
//! Resolves the tunnel URL, opens a TCP connection, runs rustls
//! through it (ALPN: h2), runs `h2::client` on top, posts the
//! handshake request, wraps `(SendStream, RecvStream)` in an
//! `H2Duplex`, and finally runs the chan-tunnel handshake.
//!
//! When the URL scheme is `http://` we skip TLS and run h2 in
//! cleartext (h2c) directly over the TCP socket. The server side
//! (chan-tunnel-server) is already h2c-only in production: nginx
//! terminates TLS at drive.chan.app and `grpc_pass`-es `/v1/tunnel`
//! as h2c into drive-proxy's tunnel listener. The h2c branch exists
//! so a local stack can dial the drive-proxy h2c port without
//! standing up a TLS terminator.

use std::sync::Arc;

use chan_tunnel_proto::H2Duplex;
use http::{Method, StatusCode};
use rustls::{ClientConfig as RustlsClientConfig, RootCertStore};
use rustls_pki_types::ServerName;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_util::compat::Compat;
use yamux::Connection as YamuxConnection;

use crate::{handshake, ClientConfig, ClientError, Registration};

/// Open one tunnel against the configured URL: TLS + h2 + tunnel
/// handshake. Returns the registration the server assigned plus
/// the yamux connection ready to be passed to `serve_substreams`.
///
/// Reconnect logic lives in `run`; this function is the single
/// attempt and the testable unit. Builds a fresh TLS config every
/// call, which on macOS walks the keychain. For long-running hosts
/// that reconnect repeatedly, prefer `dial_with_tls` and reuse the
/// `Arc<RustlsClientConfig>`.
pub async fn dial(
    cfg: &ClientConfig,
) -> Result<(Registration, YamuxConnection<Compat<H2Duplex>>), ClientError> {
    let tls = if cfg.tunnel_url.scheme() == "https" {
        Some(Arc::new(build_tls_config()?))
    } else {
        None
    };
    dial_with_tls(cfg, tls.as_ref()).await
}

/// Same as `dial` but reuses a pre-built TLS config. The reconnect
/// loop in `run()` calls this so every retry doesn't re-walk the
/// system trust store; the macOS keychain in particular costs tens
/// of milliseconds per load.
pub async fn dial_with_tls(
    cfg: &ClientConfig,
    tls: Option<&Arc<RustlsClientConfig>>,
) -> Result<(Registration, YamuxConnection<Compat<H2Duplex>>), ClientError> {
    let host = cfg
        .tunnel_url
        .host_str()
        .ok_or_else(|| ClientError::InvalidUrl("missing host".into()))?
        .to_string();
    let port = cfg.tunnel_url.port_or_known_default().ok_or_else(|| {
        ClientError::InvalidUrl(format!("cannot infer port from {}", cfg.tunnel_url))
    })?;
    let scheme = cfg.tunnel_url.scheme();
    if scheme != "https" && scheme != "http" {
        return Err(ClientError::InvalidUrl(
            "tunnel URL scheme must be https:// or http://".into(),
        ));
    }

    let tcp = TcpStream::connect((host.as_str(), port)).await?;
    tcp.set_nodelay(true).ok();

    // Drive h2 frames in the background; the connection future has
    // a different type per branch (rustls TlsStream vs raw TcpStream),
    // so spawn inside each arm and only return the SendRequest.
    let mut send_req = if scheme == "https" {
        let tls_config = tls
            .ok_or_else(|| ClientError::Tls("https:// dial called without a TLS config".into()))?
            .clone();
        let connector = TlsConnector::from(tls_config);
        let server_name = ServerName::try_from(host.clone())
            .map_err(|e| ClientError::Tls(format!("invalid SNI {host:?}: {e}")))?;
        let tls = connector
            .connect(server_name, tcp)
            .await
            .map_err(|e| ClientError::Tls(format!("rustls handshake: {e}")))?;
        let (s, conn) = h2::client::handshake(tls)
            .await
            .map_err(|e| ClientError::Handshake(format!("h2 handshake: {e}")))?;
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                tracing::debug!(error = %e, "h2 client conn ended");
            }
        });
        s
    } else {
        let (s, conn) = h2::client::handshake(tcp)
            .await
            .map_err(|e| ClientError::Handshake(format!("h2c handshake: {e}")))?;
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                tracing::debug!(error = %e, "h2 client conn ended");
            }
        });
        s
    };

    let req = http::Request::builder()
        .method(Method::POST)
        .uri(cfg.tunnel_url.as_str())
        .header(http::header::AUTHORIZATION, format!("Bearer {}", cfg.token))
        .body(())
        .map_err(|e| ClientError::Handshake(format!("build request: {e}")))?;
    let (resp_fut, send) = send_req
        .send_request(req, false)
        .map_err(|e| ClientError::Handshake(format!("send_request: {e}")))?;
    let resp = resp_fut
        .await
        .map_err(|e| ClientError::Handshake(format!("response: {e}")))?;

    match resp.status() {
        StatusCode::OK => {}
        StatusCode::UNAUTHORIZED => {
            return Err(ClientError::Handshake("unauthorized (bad token)".into()))
        }
        StatusCode::FORBIDDEN => {
            return Err(ClientError::Handshake(
                "forbidden (token missing tunnel scope)".into(),
            ));
        }
        other => return Err(ClientError::Handshake(format!("unexpected status {other}"))),
    }

    let recv = resp.into_body();
    let duplex = H2Duplex::new(send, recv);

    handshake(cfg, duplex).await
}

pub fn build_tls_config() -> Result<RustlsClientConfig, ClientError> {
    // rustls 0.23 expects the default crypto provider to be
    // installed once per process. Re-install attempts are no-ops
    // and we ignore the result so multiple `dial` calls in a
    // single process don't fight over it.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let mut roots = RootCertStore::empty();
    let native = rustls_native_certs::load_native_certs();
    if !native.errors.is_empty() {
        tracing::warn!(
            count = native.errors.len(),
            "rustls-native-certs reported per-cert errors; continuing with the rest",
        );
    }
    for cert in native.certs {
        let _ = roots.add(cert);
    }
    if roots.is_empty() {
        return Err(ClientError::Tls(
            "no native CA certificates available; cannot verify tunnel server".into(),
        ));
    }

    let mut config = RustlsClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    config.alpn_protocols = vec![b"h2".to_vec()];
    Ok(config)
}
