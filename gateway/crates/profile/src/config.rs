use std::net::SocketAddr;

use anyhow::Context;
use gateway_common::devserver_control_client::DevserverControlClient;
use url::Url;

/// Runtime config sourced from environment variables.
///
/// Keep the surface tiny: anything not in here is either a
/// compile-time choice or belongs in a future config file. Env-only
/// so deploys can override per-environment without baking secrets
/// into images.
#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub auth_token: String,
    /// Bearer for the /v1/admin/* tree. Distinct from `auth_token`
    /// so admin access can be rotated without touching identity /
    /// devserver-proxy. Optional: when unset, the admin tree returns
    /// 401 for every request, which is the safe default.
    pub admin_token: Option<String>,
    /// Pre-built admin client for devserver-control. `None` when
    /// `DEVSERVER_ADMIN_TOKEN` is unset, in which case admin block
    /// skips the tunnel-kill call (the live substreams stay alive
    /// until they reconnect and the next validate refuses them) and
    /// the registry sweeper does not spawn (no live-tunnel source).
    pub workspace_admin: Option<DevserverControlClient>,
    /// Devserver registry sweeper retention, from
    /// `DEVSERVER_RETENTION_MINUTES`: rows offline longer than this are
    /// deleted. Absent or empty = 15 minutes; `0` = sweeping disabled
    /// (`None`); anything unparseable fails startup -- a typo must not
    /// silently pick a policy that deletes rows.
    pub devserver_retention: Option<std::time::Duration>,
}

/// Parse `DEVSERVER_RETENTION_MINUTES` (pre-filtered so empty is
/// `None` input, matching how the other optional envs treat empty).
fn parse_retention_minutes(raw: Option<&str>) -> anyhow::Result<Option<std::time::Duration>> {
    let Some(raw) = raw else {
        return Ok(Some(std::time::Duration::from_secs(15 * 60)));
    };
    let minutes: u64 = raw.trim().parse().with_context(|| {
        format!("DEVSERVER_RETENTION_MINUTES must be a whole number of minutes (0 disables), got {raw:?}")
    })?;
    Ok((minutes > 0).then(|| std::time::Duration::from_secs(minutes * 60)))
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let bind_addr: SocketAddr = std::env::var("BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:7001".to_string())
            .parse()
            .context("BIND_ADDR must be host:port")?;

        let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL is required")?;
        let auth_token =
            std::env::var("PROFILE_AUTH_TOKEN").context("PROFILE_AUTH_TOKEN is required")?;
        if auth_token.is_empty() {
            anyhow::bail!("PROFILE_AUTH_TOKEN must not be empty");
        }

        let admin_token = std::env::var("PROFILE_ADMIN_TOKEN")
            .ok()
            .filter(|s| !s.is_empty());

        // DEVSERVER_ADMIN_URL points at devserver-control's admin
        // listener (the aggregate `/admin/v1/*` tree, default port
        // 7003). Unset is OK in lab / one-machine setups: block-user
        // still works, the live tunnel just lingers until reconnect.
        let workspace_admin = std::env::var("DEVSERVER_ADMIN_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())
            .map(|tok| -> anyhow::Result<DevserverControlClient> {
                let url: Url = std::env::var("DEVSERVER_ADMIN_URL")
                    .context("DEVSERVER_ADMIN_URL is required when DEVSERVER_ADMIN_TOKEN is set")?
                    .parse()
                    .context("DEVSERVER_ADMIN_URL must be a URL")?;
                DevserverControlClient::new(url, tok)
            })
            .transpose()?;

        let devserver_retention = parse_retention_minutes(
            std::env::var("DEVSERVER_RETENTION_MINUTES")
                .ok()
                .filter(|s| !s.is_empty())
                .as_deref(),
        )?;

        Ok(Self {
            bind_addr,
            database_url,
            auth_token,
            admin_token,
            workspace_admin,
            devserver_retention,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retention_defaults_to_fifteen_minutes_when_absent() {
        let parsed = parse_retention_minutes(None).expect("absent parses");
        assert_eq!(parsed, Some(std::time::Duration::from_secs(15 * 60)));
    }

    #[test]
    fn retention_zero_disables_sweeping() {
        let parsed = parse_retention_minutes(Some("0")).expect("zero parses");
        assert_eq!(parsed, None);
        // Whitespace tolerated, value honored.
        let parsed = parse_retention_minutes(Some(" 45 ")).expect("45 parses");
        assert_eq!(parsed, Some(std::time::Duration::from_secs(45 * 60)));
    }

    #[test]
    fn retention_garbage_fails_startup() {
        for garbage in ["abc", "-5", "1.5", "15m"] {
            assert!(
                parse_retention_minutes(Some(garbage)).is_err(),
                "{garbage:?} must bail rather than pick a deletion policy",
            );
        }
    }
}
