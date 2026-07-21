use std::net::SocketAddr;

use anyhow::Context;
use url::Url;

pub const PROTECTED_OVERLAY: &str = "protected-overlay";

/// Enforce the repository-wide internal transport rule. HTTPS is always
/// acceptable. Cleartext HTTP is acceptable only to a parsed loopback IP
/// literal, or when the operator explicitly declares an authenticated,
/// encrypted overlay. DNS names such as `localhost` are not peer proof.
pub fn require_protected_http_url(name: &str, url: &Url) -> anyhow::Result<()> {
    let mode = std::env::var("CHAN_GATEWAY_INTERNAL_TRANSPORT").ok();
    require_protected_http_url_with_mode(name, url, mode.as_deref())
}

pub fn require_protected_http_url_with_mode(
    name: &str,
    url: &Url,
    mode: Option<&str>,
) -> anyhow::Result<()> {
    if !matches!(url.scheme(), "http" | "https") {
        anyhow::bail!("{name} must use http or https");
    }
    if url.scheme() == "https" {
        return Ok(());
    }
    let host = url.host().context("internal URL must have a host")?;
    let loopback_ip = match host {
        url::Host::Ipv4(address) => address.is_loopback(),
        url::Host::Ipv6(address) => address.is_loopback(),
        url::Host::Domain(_) => false,
    };
    if loopback_ip || mode == Some(PROTECTED_OVERLAY) {
        return Ok(());
    }
    anyhow::bail!(
        "{name} uses non-loopback cleartext; require TLS or set \
         CHAN_GATEWAY_INTERNAL_TRANSPORT=protected-overlay only when the deployment supplies \
         authenticated encryption"
    )
}

/// A cleartext internal listener may bind loopback without an overlay. Any
/// wider bind requires the same explicit protected-overlay declaration used
/// for outbound internal URLs.
pub fn require_protected_listener(name: &str, bind: SocketAddr) -> anyhow::Result<()> {
    let mode = std::env::var("CHAN_GATEWAY_INTERNAL_TRANSPORT").ok();
    require_protected_listener_with_mode(name, bind, mode.as_deref())
}

pub fn require_protected_listener_with_mode(
    name: &str,
    bind: SocketAddr,
    mode: Option<&str>,
) -> anyhow::Result<()> {
    if bind.ip().is_loopback() || mode == Some(PROTECTED_OVERLAY) {
        return Ok(());
    }
    anyhow::bail!(
        "{name} is a non-loopback cleartext listener; set \
         CHAN_GATEWAY_INTERNAL_TRANSPORT=protected-overlay only on an authenticated, encrypted overlay"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleartext_requires_ip_loopback_or_exact_overlay_mode() {
        for allowed in ["http://127.0.0.1:7000", "http://[::1]:7000"] {
            assert!(require_protected_http_url_with_mode(
                "TEST_URL",
                &allowed.parse().unwrap(),
                None,
            )
            .is_ok());
        }
        for rejected in [
            "http://localhost:7000",
            "http://127.0.0.1.example:7000",
            "http://10.0.0.5:7000",
        ] {
            assert!(require_protected_http_url_with_mode(
                "TEST_URL",
                &rejected.parse().unwrap(),
                None,
            )
            .is_err());
        }
        assert!(require_protected_http_url_with_mode(
            "TEST_URL",
            &"http://10.0.0.5:7000".parse().unwrap(),
            Some(PROTECTED_OVERLAY),
        )
        .is_ok());
        assert!(require_protected_http_url_with_mode(
            "TEST_URL",
            &"http://10.0.0.5:7000".parse().unwrap(),
            Some("protected_overlay"),
        )
        .is_err());
        assert!(require_protected_http_url_with_mode(
            "TEST_URL",
            &"https://internal.example:7000".parse().unwrap(),
            None,
        )
        .is_ok());
    }

    #[test]
    fn listener_requires_loopback_or_exact_overlay_mode() {
        assert!(require_protected_listener_with_mode(
            "BIND",
            "127.0.0.1:7001".parse().unwrap(),
            None,
        )
        .is_ok());
        assert!(require_protected_listener_with_mode(
            "BIND",
            "0.0.0.0:7001".parse().unwrap(),
            None,
        )
        .is_err());
        assert!(require_protected_listener_with_mode(
            "BIND",
            "10.0.0.5:7001".parse().unwrap(),
            Some(PROTECTED_OVERLAY),
        )
        .is_ok());
    }
}
