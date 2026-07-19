use std::net::SocketAddr;

use anyhow::Context;
use url::Url;

/// Both-directions idle window after which a bridged WebSocket is cut.
/// Long enough that any heartbeat-carrying socket (the SPA pings every
/// 20s) never trips it; short enough that abandoned bridges do not pin
/// yamux substreams for hours.
pub const DEFAULT_WS_IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

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
    /// generic `DEVSERVER_GATE_SECRET` name (a cross-service shared
    /// secret), so the field tracks the var rather than the cookie.
    /// Required.
    pub workspace_gate_secret: String,
    /// Maximum number of distinct devservers a single user can have
    /// registered concurrently. `0` disables the cap. Reconnects of
    /// an already-registered devserver are always allowed (last-writer-
    /// wins eviction in the tunnel registry handles that). Sourced
    /// from `MAX_DEVSERVERS_PER_USER` (legacy alias
    /// `MAX_WORKSPACES_PER_USER` when unset); defaults to 100.
    pub max_devservers_per_user: usize,
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
    /// consume. `None` disables; default 60s. WebSocket bridges use
    /// the shared `ws_idle_timeout` window instead.
    pub request_timeout: Option<std::time::Duration>,
    /// Idle window for bridged WebSockets. The bridge is cut (with a
    /// proper Close frame to both halves) only after BOTH directions
    /// have been quiet this long; a frame either way resets the
    /// window, so a socket streaming one way never dies mid-stream.
    /// Always [`DEFAULT_WS_IDLE_TIMEOUT`] in production (not
    /// env-sourced); tests inject sub-second values via the struct.
    pub ws_idle_timeout: std::time::Duration,
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

        let tunnel_origin = required_origin("DEVSERVER_TUNNEL_ORIGIN")?;
        let proxy_base_url = required_origin("DEVSERVER_PROXY_BASE_URL")?;
        let apex_host = tunnel_origin
            .host_str()
            .context("DEVSERVER_TUNNEL_ORIGIN must contain a host")?
            .to_string();
        let wildcard_suffix = format!(
            ".{}",
            proxy_base_url
                .host_str()
                .context("DEVSERVER_PROXY_BASE_URL must contain a host")?
        );

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
            std::env::var("DEVSERVER_GATE_SECRET").context("DEVSERVER_GATE_SECRET is required")?;
        if workspace_gate_secret.is_empty() {
            anyhow::bail!("DEVSERVER_GATE_SECRET must not be empty");
        }

        let dashboard_url = std::env::var("DASHBOARD_URL")
            .context("DASHBOARD_URL is required")?
            .trim()
            .to_string();
        if dashboard_url.is_empty() {
            anyhow::bail!("DASHBOARD_URL must not be empty");
        }

        let max_devservers_per_user = parse_devserver_cap(
            std::env::var("MAX_DEVSERVERS_PER_USER").ok(),
            std::env::var("MAX_WORKSPACES_PER_USER").ok(),
        )?;

        let admin_token = std::env::var("DEVSERVER_ADMIN_TOKEN")
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
            max_devservers_per_user,
            admin_token,
            max_response_bytes,
            max_request_bytes,
            request_timeout,
            ws_idle_timeout: DEFAULT_WS_IDLE_TIMEOUT,
            forwarded_proto,
        })
    }

    /// Parse `(user, disc)` out of a Host header, or `None` if the
    /// Host is the apex (no user prefix) or doesn't match this
    /// gateway's hostnames. The strip trims any optional `:port`
    /// suffix so dev lookups against `127.0.0.1:7002` still work.
    ///
    /// Two wildcard forms share one DNS label:
    ///
    ///   * `{user}.devserver.<base>` -> `(user, None)`. Bare host;
    ///     the proxy resolves the devserver from the user's live set.
    ///   * `{user}--{disc}.devserver.<base>` -> `(user, Some(disc))`.
    ///     `disc` is the first 12 hex chars of the devserver id. The
    ///     double hyphen is unambiguous because `valid_username`
    ///     rejects usernames containing `--`.
    ///
    /// The label is lowercased on ingest (DNS names are case-
    /// insensitive; devserver ids are stored lowercase). A label with
    /// more than one `--`, a disc tail that is not exactly 12
    /// lowercase hex chars, a multi-label prefix (e.g. `evil.alice`
    /// against `*.devserver.chan.app`), and non-label characters are
    /// all rejected so the resulting username matches the shape the
    /// downstream validators accept.
    pub fn parse_wildcard_host(&self, host: &str) -> Option<(String, Option<String>)> {
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
        let label = prefix.to_ascii_lowercase();
        match label.match_indices("--").count() {
            0 => Some((label, None)),
            1 => {
                let (user, disc) = label.split_once("--").expect("counted one occurrence");
                if user.is_empty() {
                    return None;
                }
                let is_lower_hex = |c: u8| c.is_ascii_digit() || (b'a'..=b'f').contains(&c);
                if disc.len() != 12 || !disc.bytes().all(is_lower_hex) {
                    return None;
                }
                Some((user.to_string(), Some(disc.to_string())))
            }
            _ => None,
        }
    }

    /// True when the Host header names this gateway's apex.
    pub fn is_apex(&self, host: &str) -> bool {
        host.split(':')
            .next()
            .map(|h| h.eq_ignore_ascii_case(&self.apex_host))
            .unwrap_or(false)
    }
}

fn required_origin(name: &str) -> anyhow::Result<Url> {
    let raw = std::env::var(name).with_context(|| format!("{name} is required"))?;
    let url: Url = raw
        .trim()
        .parse()
        .with_context(|| format!("{name} must be a URL origin"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        anyhow::bail!(
            "{name} must be an http(s) origin with no credentials, path, query, or fragment"
        );
    }
    Ok(url)
}

/// Resolve the per-user devserver cap from the env pair. The new
/// `MAX_DEVSERVERS_PER_USER` name wins; the legacy
/// `MAX_WORKSPACES_PER_USER` alias applies only when the new name is
/// unset. Unset entirely defaults to 100; `0` disables the cap.
fn parse_devserver_cap(
    new_var: Option<String>,
    legacy_var: Option<String>,
) -> anyhow::Result<usize> {
    if let Some(v) = new_var {
        return v
            .trim()
            .parse()
            .context("MAX_DEVSERVERS_PER_USER must be a non-negative integer");
    }
    if let Some(v) = legacy_var {
        return v
            .trim()
            .parse()
            .context("MAX_WORKSPACES_PER_USER must be a non-negative integer");
    }
    Ok(100)
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
            max_devservers_per_user: 0,
            admin_token: None,
            max_response_bytes: None,
            max_request_bytes: None,
            request_timeout: None,
            ws_idle_timeout: DEFAULT_WS_IDLE_TIMEOUT,
            forwarded_proto: "https".into(),
        }
    }

    #[test]
    fn devserver_cap_new_var_wins_legacy_falls_back_default_100() {
        assert_eq!(
            parse_devserver_cap(Some("5".into()), Some("9".into())).unwrap(),
            5
        );
        assert_eq!(parse_devserver_cap(None, Some("9".into())).unwrap(), 9);
        assert_eq!(parse_devserver_cap(None, None).unwrap(), 100);
        assert_eq!(parse_devserver_cap(Some("0".into()), None).unwrap(), 0);
        assert!(parse_devserver_cap(Some("x".into()), None).is_err());
        assert!(parse_devserver_cap(None, Some("x".into())).is_err());
    }

    #[test]
    fn apex_returns_none() {
        let c = cfg();
        assert_eq!(c.parse_wildcard_host("devserver.chan.app"), None);
        assert_eq!(c.parse_wildcard_host("DEVSERVER.chan.app"), None);
        assert_eq!(c.parse_wildcard_host("devserver.chan.app:7002"), None);
    }

    #[test]
    fn bare_wildcard_extracts_user_without_disc() {
        let c = cfg();
        assert_eq!(
            c.parse_wildcard_host("alice.devserver.chan.app"),
            Some(("alice".into(), None)),
        );
        assert_eq!(
            c.parse_wildcard_host("Alice.Devserver.Chan.App"),
            Some(("alice".into(), None)),
        );
        assert_eq!(
            c.parse_wildcard_host("alice.devserver.chan.app:7002"),
            Some(("alice".into(), None)),
        );
        // Single interior hyphens stay part of the username.
        assert_eq!(
            c.parse_wildcard_host("a-b-c.devserver.chan.app"),
            Some(("a-b-c".into(), None)),
        );
    }

    #[test]
    fn disc_wildcard_extracts_user_and_disc() {
        let c = cfg();
        assert_eq!(
            c.parse_wildcard_host("alice--0123456789ab.devserver.chan.app"),
            Some(("alice".into(), Some("0123456789ab".into()))),
        );
        // The whole label is lowercased on ingest, disc included.
        assert_eq!(
            c.parse_wildcard_host("Alice--0123456789AB.devserver.chan.app"),
            Some(("alice".into(), Some("0123456789ab".into()))),
        );
        assert_eq!(
            c.parse_wildcard_host("a-b--abcdefabcdef.devserver.chan.app:7002"),
            Some(("a-b".into(), Some("abcdefabcdef".into()))),
        );
    }

    #[test]
    fn malformed_disc_rejected() {
        let c = cfg();
        // Tail must be exactly 12 hex chars.
        assert_eq!(
            c.parse_wildcard_host("alice--0123456789a.devserver.chan.app"),
            None
        );
        assert_eq!(
            c.parse_wildcard_host("alice--0123456789abc.devserver.chan.app"),
            None
        );
        assert_eq!(
            c.parse_wildcard_host("alice--0123456789xy.devserver.chan.app"),
            None
        );
        // More than one `--` cannot come from a valid username.
        assert_eq!(
            c.parse_wildcard_host("a--b--0123456789ab.devserver.chan.app"),
            None
        );
        assert_eq!(
            c.parse_wildcard_host("a----0123456789ab.devserver.chan.app"),
            None
        );
        // A triple hyphen leaves a `-` in the disc tail.
        assert_eq!(
            c.parse_wildcard_host("a---0123456789ab.devserver.chan.app"),
            None
        );
        // Empty user before the separator.
        assert_eq!(
            c.parse_wildcard_host("--0123456789ab.devserver.chan.app"),
            None
        );
    }

    #[test]
    fn unknown_host_returns_none() {
        let c = cfg();
        assert_eq!(c.parse_wildcard_host("example.com"), None);
        assert_eq!(c.parse_wildcard_host(""), None);
        assert_eq!(c.parse_wildcard_host(".devserver.chan.app"), None);
    }

    #[test]
    fn multi_label_prefix_rejected() {
        // `evil.alice.devserver.chan.app` matches the wildcard suffix
        // but must NOT resolve to username "evil.alice": the prefix
        // is required to be a single DNS label.
        let c = cfg();
        assert_eq!(c.parse_wildcard_host("evil.alice.devserver.chan.app"), None);
        // Leading dot was already excluded by the substring length
        // check + emptiness guard, but tighten the boundary explicitly.
        assert_eq!(c.parse_wildcard_host("..devserver.chan.app"), None);
        // Underscores aren't legal DNS hostname chars and are not in
        // the username alphabet either.
        assert_eq!(c.parse_wildcard_host("a_b.devserver.chan.app"), None);
    }
}
