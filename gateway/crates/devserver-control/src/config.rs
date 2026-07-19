use std::net::SocketAddr;

use anyhow::Context;
use devserver_control_proto::ProxyOriginTemplate;

#[derive(Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub proxy_bind_addr: SocketAddr,
    pub admin_token: String,
    pub proxy_token: String,
    pub proxy_base_url_template: ProxyOriginTemplate,
    pub max_devservers_per_user: usize,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let bind_addr = std::env::var("BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:7003".into())
            .parse()
            .context("BIND_ADDR must be host:port")?;
        let proxy_bind_addr = std::env::var("PROXY_BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:7101".into())
            .parse()
            .context("PROXY_BIND_ADDR must be host:port")?;
        let admin_token = required_secret("DEVSERVER_ADMIN_TOKEN")?;
        let proxy_token = required_secret("DEVSERVER_PROXY_TOKEN")?;
        let template = std::env::var("DEVSERVER_PROXY_BASE_URL_TEMPLATE")
            .context("DEVSERVER_PROXY_BASE_URL_TEMPLATE is required")?;
        let proxy_base_url_template = ProxyOriginTemplate::parse(template.trim())
            .context("DEVSERVER_PROXY_BASE_URL_TEMPLATE is invalid")?;
        let max_devservers_per_user = std::env::var("MAX_DEVSERVERS_PER_USER")
            .unwrap_or_else(|_| "100".into())
            .trim()
            .parse()
            .context("MAX_DEVSERVERS_PER_USER must be a non-negative integer")?;
        Ok(Self {
            bind_addr,
            proxy_bind_addr,
            admin_token,
            proxy_token,
            proxy_base_url_template,
            max_devservers_per_user,
        })
    }
}

fn required_secret(name: &str) -> anyhow::Result<String> {
    let value = std::env::var(name).with_context(|| format!("{name} is required"))?;
    if value.is_empty() {
        anyhow::bail!("{name} must not be empty");
    }
    Ok(value)
}
