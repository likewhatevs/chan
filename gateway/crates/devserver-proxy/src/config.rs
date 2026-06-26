use std::net::SocketAddr;

use anyhow::Context;
use url::Url;

/// Runtime config sourced from environment variables.
#[derive(Clone)]
pub struct Config {
    /// Public listener (devserver.chan.app apex + *.devserver.chan.app
    /// wildcard). Behind nginx + TLS.
    pub bind_addr: SocketAddr,
    /// Tunnel listener (apex `/v1/tunnel`). h2c behind nginx
    /// `grpc_pass`; `chan devserver` instances dial
    /// `https://devserver.chan.app/v1/tunnel` over h2/TLS, terminated at
    /// nginx and forwarded here cleartext.
    pub tunnel_bind_addr: SocketAddr,
    /// Apex hostname (e.g. `devserver.chan.app`). Used to distinguish
    /// the admin/healthz surface from the wildcard reverse-proxy
    /// surface in the Host-keyed router.
    pub apex_host: String,
    /// Wildcard suffix including the leading dot (e.g.
    /// `.devserver.chan.app`). The proxy router parses `{user}` out of
    /// every Host that ends with this suffix.
    pub wildcard_suffix: String,
    /// Base URL of identity-service. devserver-proxy POSTs to
    /// `{identity_url}/internal/v1/tokens/validate` to validate the
    /// PAT every `chan devserver` presents in its tunnel handshake.
    pub identity_url: Url,
    /// Bearer devserver-proxy presents on identity-service's
    /// `/internal/v1/tokens/validate`. Sourced from
    /// `IDENTITY_INTERNAL_TOKEN`; required, no fallback.
    pub identity_auth_token: String,
    /// Absolute URL the wildcard root (`{user}.devserver.chan.app/`)
    /// 302s to. The dashboard lives at id.chan.app/workspaces in prod;
    /// dev sets this to `http://id.localtest.me:17000/workspaces`. If
    /// unset, devserver-proxy derives a sensible default by swapping
    /// the `devserver.` prefix on `apex_host` for `id.` and assuming
    /// `https`.
    pub dashboard_url: String,
    /// HMAC-SHA256 secret used to verify entry tokens from identity
    /// and mint session tokens for the `devserver_gate` cookie. Same
    /// value also configured on identity-service. The env var keeps the
    /// generic `WORKSPACE_GATE_SECRET` name (a cross-service shared
    /// secret), so the field tracks the var rather than the cookie.
    /// Required.
    pub workspace_gate_secret: String,
    /// Maximum number of distinct workspaces a single user can have
    /// registered concurrently. `0` disables the cap. Reconnects of
    /// an already-registered workspace are always allowed (last-writer-
    /// wins eviction in the tunnel registry handles that).
    pub max_workspaces_per_user: usize,
    /// Bearer for the `/admin/v1/*` tree. `None` makes every admin
    /// route 401, which is the safe default if the env var is
    /// missing on a fresh deploy.
    pub admin_token: Option<String>,
    /// Cap on response bytes streamed back from an upstream `chan
    /// devserver` per request. `None` (env unset or `0`) disables the
    /// cap.
    pub max_response_bytes: Option<usize>,
    /// Cap on inbound request body bytes forwarded to the upstream.
    /// `None` disables.
    pub max_request_bytes: Option<usize>,
    /// Hard cap on total time a single proxied HTTP request may
    /// consume. `None` disables; default 60s. WebSocket requests use
    /// per-half idle timeouts instead.
    pub request_timeout: Option<std::time::Duration>,
    /// Value to set on the outbound `X-Forwarded-Proto` header before
    /// forwarding to the upstream `chan devserver`. devserver-proxy itself
    /// does not see TLS (nginx terminates), so we cannot derive this
    /// from the inbound connection; the inbound `X-Forwarded-Proto`
    /// is client-controlled and must not be trusted. Defaults to
    /// `https`; override with `FORWARDED_PROTO=http` for local dev.
    pub forwarded_proto: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let bind_addr: SocketAddr = std::env::var("BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:7002".to_string())
            .parse()
            .context("BIND_ADDR must be host:port")?;

        let tunnel_bind_addr: SocketAddr = std::env::var("TUNNEL_BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:7100".to_string())
            .parse()
            .context("TUNNEL_BIND_ADDR must be host:port")?;

        // Single-source domain config. CHAN_DOMAIN drives both the
        // devserver and id hostnames; PUBLIC_SCHEME the URL scheme.
        // Both default dev-shaped (localtest.me / http); production
        // sets them once in the shared environment file. identity
        // derives the same hosts from the same vars, so the two cannot
        // drift (the devserver-gate `aud` must match). See
        // gateway_common::domain.
        let domains = gateway_common::domain::Domains::from_env();
        let public_scheme = read_public_scheme()?;

        // Apex + wildcard default to the derived devserver hosts;
        // override with APEX_HOST / WILDCARD_SUFFIX only for unusual
        // layouts. The wildcard suffix follows the apex unless set.
        let apex_host = match std::env::var("APEX_HOST") {
            Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
            _ => domains.devserver_apex.clone(),
        };
        if apex_host.is_empty() {
            anyhow::bail!("APEX_HOST must not be empty");
        }
        let wildcard_suffix = match std::env::var("WILDCARD_SUFFIX") {
            Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
            _ => format!(".{apex_host}"),
        };
        if !wildcard_suffix.starts_with('.') {
            anyhow::bail!(
                "WILDCARD_SUFFIX must start with a dot (got {wildcard_suffix:?}); \
                 e.g. .devserver.chan.app"
            );
        }

        let identity_url: Url = std::env::var("IDENTITY_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:7000".to_string())
            .parse()
            .context("IDENTITY_URL must be a URL")?;

        // Bearer devserver-proxy presents on identity-service's
        // /internal/v1/tokens/validate. Required; the same value is
        // configured on identity as IDENTITY_INTERNAL_TOKEN. No
        // fallback to PROFILE_AUTH_TOKEN: rotating the profile bearer
        // must never silently rotate this one (matches identity's
        // side, which also requires IDENTITY_INTERNAL_TOKEN outright).
        let identity_auth_token = std::env::var("IDENTITY_INTERNAL_TOKEN")
            .context("IDENTITY_INTERNAL_TOKEN is required")?;
        if identity_auth_token.is_empty() {
            anyhow::bail!("IDENTITY_INTERNAL_TOKEN must not be empty");
        }

        let workspace_gate_secret =
            std::env::var("WORKSPACE_GATE_SECRET").context("WORKSPACE_GATE_SECRET is required")?;
        if workspace_gate_secret.is_empty() {
            anyhow::bail!("WORKSPACE_GATE_SECRET must not be empty");
        }

        // Dashboard redirect target. Explicit env var wins; otherwise
        // derive the id host from the same base domain
        // (`{scheme}://id.<base>/workspaces`). The derived form carries
        // no port, so a dev / lab setup on a non-default id port still
        // needs DASHBOARD_URL set explicitly.
        let dashboard_url = match std::env::var("DASHBOARD_URL") {
            Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
            _ => format!("{public_scheme}://{}/workspaces", domains.id_host),
        };

        let max_workspaces_per_user: usize = match std::env::var("MAX_WORKSPACES_PER_USER") {
            Ok(v) => v
                .trim()
                .parse()
                .context("MAX_WORKSPACES_PER_USER must be a non-negative integer")?,
            Err(_) => 0,
        };

        let admin_token = std::env::var("WORKSPACE_ADMIN_TOKEN")
            .ok()
            .filter(|s| !s.is_empty());

        let max_response_bytes = parse_byte_cap("MAX_RESPONSE_BYTES", Some(100 * 1024 * 1024))?;
        let max_request_bytes = parse_byte_cap("MAX_REQUEST_BYTES", Some(100 * 1024 * 1024))?;
        let forwarded_proto = std::env::var("FORWARDED_PROTO")
            .unwrap_or_else(|_| "https".to_string())
            .trim()
            .to_string();
        if forwarded_proto != "http" && forwarded_proto != "https" {
            anyhow::bail!(
                "FORWARDED_PROTO must be \"http\" or \"https\" (got {forwarded_proto:?})"
            );
        }

        let request_timeout = match std::env::var("REQUEST_TIMEOUT_SECS") {
            Ok(v) => {
                let n: u64 = v
                    .trim()
                    .parse()
                    .context("REQUEST_TIMEOUT_SECS must be a non-negative integer")?;
                if n == 0 {
                    None
                } else {
                    Some(std::time::Duration::from_secs(n))
                }
            }
            Err(_) => Some(std::time::Duration::from_secs(60)),
        };

        Ok(Self {
            bind_addr,
            tunnel_bind_addr,
            apex_host,
            wildcard_suffix,
            identity_url,
            identity_auth_token,
            dashboard_url,
            workspace_gate_secret,
            max_workspaces_per_user,
            admin_token,
            max_response_bytes,
            max_request_bytes,
            request_timeout,
            forwarded_proto,
        })
    }

    /// Parse `{user}` out of a Host header, or `None` if the Host is
    /// the apex (no user prefix) or doesn't match this gateway's
    /// hostnames. The strip trims any optional `:port` suffix so
    /// dev lookups against `127.0.0.1:7002` still work.
    ///
    /// The prefix must be a single DNS label: lowercase ASCII
    /// alphanumerics plus `-`, no internal dots. Multi-label
    /// prefixes (e.g. `evil.alice` against `*.devserver.chan.app`) and
    /// non-label characters are rejected so the resulting "username"
    /// matches the shape username validators downstream accept.
    pub fn parse_wildcard_user(&self, host: &str) -> Option<String> {
        let host = host.split(':').next()?;
        if host.eq_ignore_ascii_case(&self.apex_host) {
            return None;
        }
        let suffix = self.wildcard_suffix.as_str();
        if host.len() <= suffix.len() {
            return None;
        }
        let (prefix, tail) = host.split_at(host.len() - suffix.len());
        if !tail.eq_ignore_ascii_case(suffix) {
            return None;
        }
        if prefix.is_empty() {
            return None;
        }
        if !prefix
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        {
            return None;
        }
        Some(prefix.to_ascii_lowercase())
    }

    /// True when the Host header names this gateway's apex.
    pub fn is_apex(&self, host: &str) -> bool {
        host.split(':')
            .next()
            .map(|h| h.eq_ignore_ascii_case(&self.apex_host))
            .unwrap_or(false)
    }
}

/// Read the shared PUBLIC_SCHEME (http/https), defaulting to the
/// dev-shaped `http`. Production sets `https` once in the shared
/// environment file. Kept identical to identity-service's reader so
/// the two derive matching public URLs.
fn read_public_scheme() -> anyhow::Result<String> {
    let scheme = std::env::var("PUBLIC_SCHEME")
        .unwrap_or_else(|_| gateway_common::domain::DEFAULT_PUBLIC_SCHEME.to_string())
        .trim()
        .to_string();
    if scheme != "http" && scheme != "https" {
        anyhow::bail!("PUBLIC_SCHEME must be \"http\" or \"https\"; got {scheme:?}");
    }
    Ok(scheme)
}

fn parse_byte_cap(name: &str, default: Option<usize>) -> anyhow::Result<Option<usize>> {
    match std::env::var(name) {
        Ok(v) => {
            let n: usize = v
                .trim()
                .parse()
                .with_context(|| format!("{name} must be a non-negative integer"))?;
            Ok((n != 0).then_some(n))
        }
        Err(_) => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> Config {
        Config {
            bind_addr: "127.0.0.1:7002".parse().unwrap(),
            tunnel_bind_addr: "127.0.0.1:7100".parse().unwrap(),
            apex_host: "devserver.chan.app".into(),
            wildcard_suffix: ".devserver.chan.app".into(),
            identity_url: "http://127.0.0.1:7000/".parse().unwrap(),
            identity_auth_token: "x".into(),
            dashboard_url: "https://id.chan.app/workspaces".into(),
            workspace_gate_secret: "x".into(),
            max_workspaces_per_user: 0,
            admin_token: None,
            max_response_bytes: None,
            max_request_bytes: None,
            request_timeout: None,
            forwarded_proto: "https".into(),
        }
    }

    #[test]
    fn apex_returns_none() {
        let c = cfg();
        assert_eq!(c.parse_wildcard_user("devserver.chan.app"), None);
        assert_eq!(c.parse_wildcard_user("DEVSERVER.chan.app"), None);
        assert_eq!(c.parse_wildcard_user("devserver.chan.app:7002"), None);
    }

    #[test]
    fn wildcard_extracts_user() {
        let c = cfg();
        assert_eq!(
            c.parse_wildcard_user("alice.devserver.chan.app").as_deref(),
            Some("alice"),
        );
        assert_eq!(
            c.parse_wildcard_user("Alice.Devserver.Chan.App").as_deref(),
            Some("alice"),
        );
        assert_eq!(
            c.parse_wildcard_user("alice.devserver.chan.app:7002")
                .as_deref(),
            Some("alice"),
        );
    }

    #[test]
    fn unknown_host_returns_none() {
        let c = cfg();
        assert_eq!(c.parse_wildcard_user("example.com"), None);
        assert_eq!(c.parse_wildcard_user(""), None);
        assert_eq!(c.parse_wildcard_user(".devserver.chan.app"), None);
    }

    #[test]
    fn multi_label_prefix_rejected() {
        // `evil.alice.devserver.chan.app` matches the wildcard suffix
        // but must NOT resolve to username "evil.alice": the prefix
        // is required to be a single DNS label.
        let c = cfg();
        assert_eq!(c.parse_wildcard_user("evil.alice.devserver.chan.app"), None);
        // Leading dot was already excluded by the substring length
        // check + emptiness guard, but tighten the boundary explicitly.
        assert_eq!(c.parse_wildcard_user("..devserver.chan.app"), None);
        // Underscores aren't legal DNS hostname chars and are not in
        // the username alphabet either.
        assert_eq!(c.parse_wildcard_user("a_b.devserver.chan.app"), None);
    }
}
