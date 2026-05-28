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
    /// Bearer workspace-proxy presents on /internal/v1/tokens/validate.
    /// Required and distinct from PROFILE_AUTH_TOKEN; rotating one
    /// does not rotate the other.
    pub internal_auth_token: String,
    /// Wildcard suffix used to mint workspace-gate entry tokens. Each
    /// workspace opens at `{user}{wildcard_suffix}/{workspace}/`, e.g.
    /// `alice.workspace.chan.app/blog/`. Default `.workspace.chan.app`.
    pub workspace_wildcard_suffix: String,
    /// Scheme of the workspace-gate redirect URL (`https` in prod,
    /// `http` for local dev where `*.workspace.localtest.me` resolves
    /// to 127.0.0.1 without TLS).
    pub workspace_public_scheme: String,
    /// Optional `:port` suffix appended to the redirect URL. Empty
    /// in prod; `:7002` in local dev where workspace-proxy binds the
    /// axum listener on a non-443 port.
    pub workspace_public_port: String,
    /// Pre-built admin client for workspace-proxy. Required when
    /// `WORKSPACE_ADMIN_TOKEN` is set; identity uses it on PAT revoke,
    /// account delete, and `/api/me` (dashboard reads). `None` only
    /// in dev / lab setups; the dashboard renders empty workspace lists
    /// and revoke / delete skip the tunnel-kill best-effort hop.
    pub workspace_admin: Option<WorkspaceAdminClient>,
    /// HMAC-SHA256 secret used to mint workspace-gate entry tokens.
    /// Same value also configured on workspace-proxy. Required.
    pub workspace_gate_secret: String,
    pub providers: Vec<Arc<dyn Provider>>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let bind_addr: SocketAddr = std::env::var("BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:7000".to_string())
            .parse()
            .context("BIND_ADDR must be host:port")?;

        let base_url: Url = std::env::var("BASE_URL")
            .unwrap_or_else(|_| "http://localhost:7000".to_string())
            .parse()
            .context("BASE_URL must be a URL")?;

        let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL is required")?;

        let cookie_secure = parse_bool_env("COOKIE_SECURE", false)?;

        let profile_url: Url = std::env::var("PROFILE_SERVICE_URL")
            .context("PROFILE_SERVICE_URL is required")?
            .parse()
            .context("PROFILE_SERVICE_URL must be a URL")?;
        let profile_token =
            std::env::var("PROFILE_AUTH_TOKEN").context("PROFILE_AUTH_TOKEN is required")?;
        let profile_client = ProfileClient::new(profile_url, profile_token)?;

        // Required. No back-compat fallback: workspace-proxy holds the
        // matching value via the same env var (or one of its legacy
        // aliases), but on the identity side the only acceptable
        // source is IDENTITY_INTERNAL_TOKEN. Rotating PROFILE_AUTH_TOKEN
        // must never accidentally rotate the internal validate bearer.
        let internal_auth_token = std::env::var("IDENTITY_INTERNAL_TOKEN")
            .context("IDENTITY_INTERNAL_TOKEN is required")?;
        if internal_auth_token.is_empty() {
            anyhow::bail!("IDENTITY_INTERNAL_TOKEN must not be empty");
        }

        // Wildcard suffix used to stitch the entry-token's `aud`
        // claim and the redirect Location. `.workspace.chan.app` in prod.
        // Override for lab / dev with WORKSPACE_WILDCARD_SUFFIX.
        let workspace_wildcard_suffix = std::env::var("WORKSPACE_WILDCARD_SUFFIX")
            .unwrap_or_else(|_| ".workspace.chan.app".to_string())
            .trim()
            .to_string();
        if !workspace_wildcard_suffix.starts_with('.') {
            anyhow::bail!(
                "WORKSPACE_WILDCARD_SUFFIX must start with a dot (got \
                 {workspace_wildcard_suffix:?}); e.g. .workspace.chan.app"
            );
        }

        // Scheme + port for the workspace-gate redirect. Defaults are
        // prod-shaped; local dev exports `http` and (typically)
        // `:7002`. An empty port string is the prod case (URL has
        // no explicit port, browser uses 443 implicitly).
        let workspace_public_scheme = std::env::var("WORKSPACE_PUBLIC_SCHEME")
            .unwrap_or_else(|_| "https".to_string())
            .trim()
            .to_string();
        if workspace_public_scheme != "http" && workspace_public_scheme != "https" {
            anyhow::bail!(
                "WORKSPACE_PUBLIC_SCHEME must be \"http\" or \"https\"; got \
                 {workspace_public_scheme:?}"
            );
        }
        let workspace_public_port = match std::env::var("WORKSPACE_PUBLIC_PORT") {
            Ok(p) => {
                let p = p.trim();
                if p.is_empty() {
                    String::new()
                } else if let Some(rest) = p.strip_prefix(':') {
                    // Sanity-check the port shape; reject anything
                    // that isn't a positive integer so we don't ship
                    // a malformed URL.
                    rest.parse::<u16>().with_context(|| {
                        format!("WORKSPACE_PUBLIC_PORT (after `:`) must be a u16; got {rest:?}")
                    })?;
                    format!(":{rest}")
                } else {
                    p.parse::<u16>().with_context(|| {
                        format!("WORKSPACE_PUBLIC_PORT must be a u16 or `:u16`; got {p:?}")
                    })?;
                    format!(":{p}")
                }
            }
            Err(_) => String::new(),
        };

        // Same shape as before: WORKSPACE_ADMIN_TOKEN enables the
        // admin-side calls (revoke, delete, /api/me workspaces merge).
        // WORKSPACE_ADMIN_URL is now required when the token is set
        // because there is no `WORKSPACES_URL` to fall back on (the
        // dashboard surface is on identity, not on workspace-proxy).
        let workspace_admin = std::env::var("WORKSPACE_ADMIN_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())
            .map(|tok| -> anyhow::Result<WorkspaceAdminClient> {
                let admin_url: Url = std::env::var("WORKSPACE_ADMIN_URL")
                    .context("WORKSPACE_ADMIN_URL is required when WORKSPACE_ADMIN_TOKEN is set")?
                    .parse()
                    .context("WORKSPACE_ADMIN_URL must be a URL")?;
                WorkspaceAdminClient::new(admin_url, tok)
            })
            .transpose()?;

        let workspace_gate_secret =
            std::env::var("WORKSPACE_GATE_SECRET").context("WORKSPACE_GATE_SECRET is required")?;
        if workspace_gate_secret.is_empty() {
            anyhow::bail!("WORKSPACE_GATE_SECRET must not be empty");
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
            workspace_wildcard_suffix,
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

    /// Build the wildcard host for a username:
    /// `{user}{workspace_wildcard_suffix}` (e.g. `alice.workspace.chan.app`).
    pub fn workspace_host_for(&self, username: &str) -> String {
        // `.workspace.chan.app` already starts with a dot; the dot is
        // the separator between the username and the suffix.
        format!("{username}{}", &self.workspace_wildcard_suffix[..])
            .trim_start_matches('.')
            .to_string()
    }
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
    fn workspace_host_for_basic() {
        let cfg = Config {
            bind_addr: "127.0.0.1:7000".parse().unwrap(),
            base_url: "http://localhost:7000".parse().unwrap(),
            database_url: "x".into(),
            cookie_secure: false,
            profile_client: ProfileClient::new("http://x/".parse().unwrap(), "x".into()).unwrap(),
            internal_auth_token: "x".into(),
            workspace_wildcard_suffix: ".workspace.chan.app".into(),
            workspace_public_scheme: "https".into(),
            workspace_public_port: String::new(),
            workspace_admin: None,
            workspace_gate_secret: "x".into(),
            providers: vec![],
        };
        assert_eq!(cfg.workspace_host_for("alice"), "alice.workspace.chan.app");
        assert_eq!(
            cfg.workspace_host_for("USER-1"),
            "USER-1.workspace.chan.app"
        );
    }
}
