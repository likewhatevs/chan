//! TLS + h2 dial path for the chan-tunnel client.
//!
//! Resolves the tunnel URL, opens a TCP connection, runs rustls
//! through it (ALPN: h2), runs `h2::client` on top, posts the
//! handshake request, wraps `(SendStream, RecvStream)` in an
//! `H2Duplex`, and finally runs the chan-tunnel handshake.

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
/// attempt and the testable unit.
pub async fn dial(
    cfg: &ClientConfig,
) -> Result<(Registration, YamuxConnection<Compat<H2Duplex>>), ClientError> {
    let host = cfg
        .tunnel_url
        .host_str()
        .ok_or_else(|| ClientError::InvalidUrl("missing host".into()))?
        .to_string();
    let port = cfg.tunnel_url.port_or_known_default().ok_or_else(|| {
        ClientError::InvalidUrl(format!("cannot infer port from {}", cfg.tunnel_url))
    })?;
    if cfg.tunnel_url.scheme() != "https" {
        return Err(ClientError::InvalidUrl(
            "tunnel URL must be https://".into(),
        ));
    }

    let tls_config = build_tls_config()?;
    let connector = TlsConnector::from(Arc::new(tls_config));
    let server_name = ServerName::try_from(host.clone())
        .map_err(|e| ClientError::Tls(format!("invalid SNI {host:?}: {e}")))?;

    let tcp = TcpStream::connect((host.as_str(), port)).await?;
    tcp.set_nodelay(true).ok();
    let tls = connector
        .connect(server_name, tcp)
        .await
        .map_err(|e| ClientError::Tls(format!("rustls handshake: {e}")))?;

    let (mut send_req, conn) = h2::client::handshake(tls)
        .await
        .map_err(|e| ClientError::Handshake(format!("h2 handshake: {e}")))?;
    // Drive h2 frames in the background. Returns when the
    // connection ends; we ignore the result here because the
    // duplex's reads/writes will surface the error through
    // `H2Duplex` as an io error.
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            tracing::debug!(error = %e, "h2 client conn ended");
        }
    });

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
        other => return Err(ClientError::Handshake(format!("unexpected status {other}"))),
    }

    let recv = resp.into_body();
    let duplex = H2Duplex::new(send, recv);

    handshake(cfg, duplex).await
}

fn build_tls_config() -> Result<RustlsClientConfig, ClientError> {
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
