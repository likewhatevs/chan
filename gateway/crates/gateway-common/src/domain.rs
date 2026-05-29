//! Single-source domain derivation.
//!
//! identity-service and workspace-proxy must agree on the public
//! hostnames, or the workspace-gate handoff breaks: the entry/session
//! JWT `aud` claim is the inbound host, and workspace-proxy parses the
//! `{user}` label out of the Host header. Both services derive every
//! hostname from one base domain (`CHAN_DOMAIN`) through this module,
//! so the id and workspace hosts cannot drift apart.
//!
//! This replaces the previous arrangement, where each service carried
//! its own `workspace.chan.app` literal and workspace-proxy
//! reconstructed the id host by string-swapping the `workspace.`
//! prefix for `id.`.
//!
//! std-only on purpose: this type sits in the data layer next to the
//! other dependency-light helpers, so consumers map any validation
//! into their own error type.

/// Default base domain when `CHAN_DOMAIN` is unset. Dev-shaped:
/// `*.localtest.me` resolves to 127.0.0.1, so a local stack works
/// with no domain configuration. Production sets `CHAN_DOMAIN`
/// (e.g. `chan.app`) in the shared environment file.
pub const DEFAULT_BASE_DOMAIN: &str = "localtest.me";

/// Default public URL scheme when `PUBLIC_SCHEME` is unset. Dev-shaped
/// (`http`); production sets `https` in the shared environment file.
pub const DEFAULT_PUBLIC_SCHEME: &str = "http";

/// Public hostnames derived from a single base domain (e.g.
/// `chan.app`). The `id.` / `workspace.` prefixes are the fixed chan
/// convention; both services agree on them through this one type.
#[derive(Debug, Clone)]
pub struct Domains {
    /// Base domain, normalized (e.g. `chan.app`).
    pub base: String,
    /// identity-service host (e.g. `id.chan.app`).
    pub id_host: String,
    /// workspace-proxy apex host (e.g. `workspace.chan.app`).
    pub workspace_apex: String,
    /// Wildcard suffix, leading dot included
    /// (e.g. `.workspace.chan.app`).
    pub workspace_wildcard_suffix: String,
}

impl Domains {
    /// Derive the chan hostnames from a base domain. Surrounding
    /// whitespace and leading/trailing dots are trimmed, so
    /// `chan.app`, ` chan.app `, and `.chan.app.` normalize the same.
    pub fn from_base(base: &str) -> Self {
        let base = base.trim().trim_matches('.').to_string();
        Self {
            id_host: format!("id.{base}"),
            workspace_apex: format!("workspace.{base}"),
            workspace_wildcard_suffix: format!(".workspace.{base}"),
            base,
        }
    }

    /// Derive from `CHAN_DOMAIN`, falling back to
    /// [`DEFAULT_BASE_DOMAIN`] when unset or empty.
    pub fn from_env() -> Self {
        let base = std::env::var("CHAN_DOMAIN")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_DOMAIN.to_string());
        Self::from_base(&base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_prod_hosts() {
        let d = Domains::from_base("chan.app");
        assert_eq!(d.base, "chan.app");
        assert_eq!(d.id_host, "id.chan.app");
        assert_eq!(d.workspace_apex, "workspace.chan.app");
        assert_eq!(d.workspace_wildcard_suffix, ".workspace.chan.app");
    }

    #[test]
    fn normalizes_dots_and_space() {
        let d = Domains::from_base("  .chan.app.  ");
        assert_eq!(d.base, "chan.app");
        assert_eq!(d.id_host, "id.chan.app");
        assert_eq!(d.workspace_wildcard_suffix, ".workspace.chan.app");
    }

    #[test]
    fn dev_default_base() {
        let d = Domains::from_base(DEFAULT_BASE_DOMAIN);
        assert_eq!(d.workspace_apex, "workspace.localtest.me");
    }
}
