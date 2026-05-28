use std::net::SocketAddr;

use anyhow::Context;
use gateway_common::drive_admin_client::DriveAdminClient;
use url::Url;

/// Runtime config sourced from environment variables.
///
/// Keep the surface tiny: anything not in here is either a
/// compile-time choice or belongs in a future config file. v0 is
/// env-only so deploys can override per-environment without baking
/// secrets into images.
#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub auth_token: String,
    /// Bearer for the /v1/admin/* tree. Distinct from `auth_token`
    /// so admin access can be rotated without touching identity /
    /// drive-proxy. Optional: when unset, the admin tree returns
    /// 401 for every request, which is the safe default.
    pub admin_token: Option<String>,
    /// Pre-built admin client for drive-proxy. `None` when
    /// `DRIVE_ADMIN_TOKEN` is unset, in which case admin block
    /// skips the tunnel-kill call (the live substreams stay alive
    /// until they reconnect and the next validate refuses them).
    pub drive_admin: Option<DriveAdminClient>,
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

        // DRIVE_ADMIN_URL points at drive-proxy's public listener; in
        // single-listener deployments that's the same `drive.chan.app`
        // host. Unset is OK in lab / one-machine setups: block-user
        // still works, the live tunnel just lingers until reconnect.
        let drive_admin = std::env::var("DRIVE_ADMIN_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())
            .map(|tok| -> anyhow::Result<DriveAdminClient> {
                let url: Url = std::env::var("DRIVE_ADMIN_URL")
                    .context("DRIVE_ADMIN_URL is required when DRIVE_ADMIN_TOKEN is set")?
                    .parse()
                    .context("DRIVE_ADMIN_URL must be a URL")?;
                DriveAdminClient::new(url, tok)
            })
            .transpose()?;

        Ok(Self {
            bind_addr,
            database_url,
            auth_token,
            admin_token,
            drive_admin,
        })
    }
}
