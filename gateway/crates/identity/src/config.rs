use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use url::Url;

use crate::profile_client::ProfileClient;
use crate::providers::{
    github::GitHubProvider, gitlab::GitLabProvider, google::GoogleProvider, Provider,
};
use crate::workspace_admin_client::WorkspaceAdminClient;

/// Runtime config sourced from environment variables.
#[derive(Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub base_url: Url,
    pub database_url: String,
    pub cookie_secure: bool,
    pub profile_client: ProfileClient,
    /// Bearer devserver-proxy presents on /internal/v1/tokens/validate.
    /// Required and distinct from PROFILE_AUTH_TOKEN; rotating one
    /// does not rotate the other.
    pub internal_auth_token: String,
    /// Bearer gating the operator surface under /admin/v1/*. Empty
    /// (the default) disables the surface outright: the routes answer
    /// 404 as if they did not exist. chan-gateway-admin presents this
    /// via CHAN_ADMIN_TOKEN.
    pub identity_admin_token: String,
    /// Wildcard suffix used to mint devserver-gate entry tokens. Each
    /// tenant opens at `{user}{wildcard_suffix}/{workspace}/`, e.g.
    /// `alice.devserver.chan.app/blog/`, where `{user}` is the devserver
    /// host and `{workspace}` is tenant routing. Derived from
    /// `CHAN_DOMAIN` (`.devserver.<base>`) unless `DEVSERVER_WILDCARD_SUFFIX`
    /// is set.
    pub devserver_wildcard_suffix: String,
    /// Scheme of the devserver-gate redirect URL (`https` in prod,
    /// `http` for local dev where `*.devserver.localtest.me` resolves
    /// to 127.0.0.1 without TLS).
    pub workspace_public_scheme: String,
    /// Optional `:port` suffix appended to the redirect URL. Empty
    /// in prod; `:7002` in local dev where devserver-proxy binds the
    /// axum listener on a non-443 port.
    pub workspace_public_port: String,
    /// Pre-built admin client for devserver-proxy. Required when
    /// `DEVSERVER_ADMIN_TOKEN` is set; identity uses it on PAT revoke,
    /// account delete, and `/api/me` (dashboard reads). `None` only
    /// in dev / lab setups; the dashboard renders empty workspace lists
    /// and revoke / delete skip the tunnel-kill best-effort hop.
    pub workspace_admin: Option<WorkspaceAdminClient>,
    /// HMAC-SHA256 secret used to mint workspace-gate entry tokens.
    /// Same value also configured on devserver-proxy. Required.
    pub workspace_gate_secret: String,
    pub providers: Vec<Arc<dyn Provider>>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let bind_addr: SocketAddr = std::env::var("BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:7000".to_string())
            .parse()
            .context("BIND_ADDR must be host:port")?;

        // Single-source domain config. CHAN_DOMAIN drives both the id
        // and workspace hostnames; PUBLIC_SCHEME the URL scheme. Both
        // default dev-shaped (localtest.me / http); production sets
        // them once in the shared environment file. devserver-proxy
        // derives the same hosts from the same vars, so the two cannot
        // drift (the workspace-gate `aud` must match). See
        // gateway_common::domain.
        let domains = gateway_common::domain::Domains::from_env();
        let public_scheme = read_public_scheme()?;

        // identity-service's own public origin, used to build the
        // OAuth callback redirect URIs registered with each provider.
        // Defaults to the derived id host; override with BASE_URL when
        // the origin differs (e.g. a dev port:
        // http://id.localtest.me:7000).
        let base_url: Url = match std::env::var("BASE_URL") {
            Ok(v) if !v.trim().is_empty() => v.trim().parse().context("BASE_URL must be a URL")?,
            _ => format!("{public_scheme}://{}", domains.id_host)
                .parse()
                .context("derived BASE_URL must be a URL")?,
        };

        let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL is required")?;

        let cookie_secure = parse_bool_env("COOKIE_SECURE", false)?;

        let profile_url: Url = std::env::var("PROFILE_SERVICE_URL")
            .context("PROFILE_SERVICE_URL is required")?
            .parse()
            .context("PROFILE_SERVICE_URL must be a URL")?;
        let profile_token =
            std::env::var("PROFILE_AUTH_TOKEN").context("PROFILE_AUTH_TOKEN is required")?;
        let profile_client = ProfileClient::new(profile_url, profile_token)?;

        // Required, no fallback. devserver-proxy holds the matching
        // value via the same env var. Rotating PROFILE_AUTH_TOKEN
        // must never accidentally rotate the internal validate bearer.
        let internal_auth_token = std::env::var("IDENTITY_INTERNAL_TOKEN")
            .context("IDENTITY_INTERNAL_TOKEN is required")?;
        if internal_auth_token.is_empty() {
            anyhow::bail!("IDENTITY_INTERNAL_TOKEN must not be empty");
        }

        // Optional on purpose: most deployments never mint PATs from
        // the CLI, and an unset/empty token keeps the admin surface
        // disabled rather than guarded by an empty string.
        let identity_admin_token = std::env::var("IDENTITY_ADMIN_TOKEN").unwrap_or_default();

        // Wildcard suffix used to stitch the entry-token's `aud`
        // claim and the redirect Location. Defaults to the derived
        // `.devserver.<base>`; override with DEVSERVER_WILDCARD_SUFFIX
        // only for unusual layouts.
        let devserver_wildcard_suffix = match std::env::var("DEVSERVER_WILDCARD_SUFFIX") {
            Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
            _ => domains.devserver_wildcard_suffix.clone(),
        };
        if !devserver_wildcard_suffix.starts_with('.') {
            anyhow::bail!(
                "DEVSERVER_WILDCARD_SUFFIX must start with a dot (got \
                 {devserver_wildcard_suffix:?}); e.g. .devserver.chan.app"
            );
        }

        // Scheme of the workspace-gate redirect. Defaults to the
        // shared PUBLIC_SCHEME; override with DEVSERVER_PUBLIC_SCHEME
        // only when the workspace redirect scheme differs from the id
        // origin's (rare).
        let workspace_public_scheme = match std::env::var("DEVSERVER_PUBLIC_SCHEME") {
            Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
            _ => public_scheme.clone(),
        };
        if workspace_public_scheme != "http" && workspace_public_scheme != "https" {
            anyhow::bail!(
                "DEVSERVER_PUBLIC_SCHEME must be \"http\" or \"https\"; got \
                 {workspace_public_scheme:?}"
            );
        }
        let workspace_public_port = match std::env::var("DEVSERVER_PUBLIC_PORT") {
            Ok(p) => {
                let p = p.trim();
                if p.is_empty() {
                    String::new()
                } else if let Some(rest) = p.strip_prefix(':') {
                    // Sanity-check the port shape; reject anything
                    // that isn't a positive integer so we don't ship
                    // a malformed URL.
                    rest.parse::<u16>().with_context(|| {
                        format!("DEVSERVER_PUBLIC_PORT (after `:`) must be a u16; got {rest:?}")
                    })?;
                    format!(":{rest}")
                } else {
                    p.parse::<u16>().with_context(|| {
                        format!("DEVSERVER_PUBLIC_PORT must be a u16 or `:u16`; got {p:?}")
                    })?;
                    format!(":{p}")
                }
            }
            Err(_) => String::new(),
        };

        // DEVSERVER_ADMIN_TOKEN enables the admin-side calls (revoke,
        // delete, /api/me workspaces merge); DEVSERVER_ADMIN_URL is
        // required whenever the token is set.
        let workspace_admin = std::env::var("DEVSERVER_ADMIN_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())
            .map(|tok| -> anyhow::Result<WorkspaceAdminClient> {
                let admin_url: Url = std::env::var("DEVSERVER_ADMIN_URL")
                    .context("DEVSERVER_ADMIN_URL is required when DEVSERVER_ADMIN_TOKEN is set")?
                    .parse()
                    .context("DEVSERVER_ADMIN_URL must be a URL")?;
                WorkspaceAdminClient::new(admin_url, tok)
            })
            .transpose()?;

        let workspace_gate_secret =
            std::env::var("DEVSERVER_GATE_SECRET").context("DEVSERVER_GATE_SECRET is required")?;
        if workspace_gate_secret.is_empty() {
            anyhow::bail!("DEVSERVER_GATE_SECRET must not be empty");
        }

        let mut providers: Vec<Arc<dyn Provider>> = Vec::new();
        if let (Ok(id), Ok(secret)) = (
            std::env::var("GITHUB_CLIENT_ID"),
            std::env::var("GITHUB_CLIENT_SECRET"),
        ) {
            providers.push(Arc::new(GitHubProvider::new(id, secret)?));
        }
        if let (Ok(id), Ok(secret)) = (
            std::env::var("GOOGLE_CLIENT_ID"),
            std::env::var("GOOGLE_CLIENT_SECRET"),
        ) {
            providers.push(Arc::new(GoogleProvider::new(id, secret)?));
        }
        if let (Ok(id), Ok(secret)) = (
            std::env::var("GITLAB_CLIENT_ID"),
            std::env::var("GITLAB_CLIENT_SECRET"),
        ) {
            providers.push(Arc::new(GitLabProvider::new(id, secret)?));
        }
        if providers.is_empty() {
            anyhow::bail!(
                "no providers configured (set at least one of GITHUB / GOOGLE / GITLAB CLIENT_ID + CLIENT_SECRET)"
            );
        }

        Ok(Self {
            bind_addr,
            base_url,
            database_url,
            cookie_secure,
            profile_client,
            internal_auth_token,
            identity_admin_token,
            devserver_wildcard_suffix,
            workspace_public_scheme,
            workspace_public_port,
            workspace_admin,
            workspace_gate_secret,
            providers,
        })
    }

    pub fn provider(&self, name: &str) -> Option<Arc<dyn Provider>> {
        self.providers.iter().find(|p| p.name() == name).cloned()
    }

    /// Redirect URI registered with each provider's OAuth app.
    /// Same path for every provider keeps app registration uniform.
    pub fn redirect_uri(&self, provider: &str) -> Url {
        let mut u = self.base_url.clone();
        u.set_path(&format!("/auth/{provider}/callback"));
        u
    }

    /// Build the disc wildcard host addressing one devserver:
    /// `{user}--{disc}{devserver_wildcard_suffix}` (e.g.
    /// `alice--0123456789ab.devserver.chan.app`), where `disc` is the
    /// first 12 chars of the devserver id. Both parts are lowercased
    /// explicitly: devserver-proxy lowercases the label on ingest and
    /// requires the disc to be lowercase hex, and the minted host
    /// must equal the canonical `aud` the proxy verifies. Ids shorter
    /// than 12 chars (test fixtures) are used whole; production ids
    /// are 64 hex chars.
    pub fn devserver_host_for(&self, username: &str, devserver_id: &str) -> String {
        let disc: String = devserver_id.chars().take(12).collect();
        // `.devserver.chan.app` already starts with a dot; the dot is
        // the separator between the label and the suffix.
        format!(
            "{}--{}{}",
            username.to_ascii_lowercase(),
            disc.to_ascii_lowercase(),
            &self.devserver_wildcard_suffix[..]
        )
        .trim_start_matches('.')
        .to_string()
    }
}

/// Read the shared PUBLIC_SCHEME (http/https), defaulting to the
/// dev-shaped `http`. Production sets `https` once in the shared
/// environment file.
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

fn parse_bool_env(name: &str, default: bool) -> anyhow::Result<bool> {
    match std::env::var(name) {
        Ok(v) => match v.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" | "" => Ok(false),
            other => anyhow::bail!("{name} must be true/false, got {other:?}"),
        },
        Err(_) => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devserver_host_for_basic() {
        let cfg = Config {
            bind_addr: "127.0.0.1:7000".parse().unwrap(),
            base_url: "http://localhost:7000".parse().unwrap(),
            database_url: "x".into(),
            cookie_secure: false,
            profile_client: ProfileClient::new("http://x/".parse().unwrap(), "x".into()).unwrap(),
            internal_auth_token: "x".into(),
            identity_admin_token: String::new(),
            devserver_wildcard_suffix: ".devserver.chan.app".into(),
            workspace_public_scheme: "https".into(),
            workspace_public_port: String::new(),
            workspace_admin: None,
            workspace_gate_secret: "x".into(),
            providers: vec![],
        };
        assert_eq!(
            cfg.devserver_host_for("alice", "0123456789abcdef0123456789abcdef"),
            "alice--0123456789ab.devserver.chan.app"
        );
        // Lowercased explicitly on both parts: the proxy's host parse
        // and the canonical aud are lowercase.
        assert_eq!(
            cfg.devserver_host_for("USER-1", "ABCDEFABCDEFABCDEF"),
            "user-1--abcdefabcdef.devserver.chan.app"
        );
        // Short (test-fixture) ids are used whole.
        assert_eq!(
            cfg.devserver_host_for("alice", "abc123"),
            "alice--abc123.devserver.chan.app"
        );
    }
}
