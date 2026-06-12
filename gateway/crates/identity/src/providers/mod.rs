//! Provider trait and implementations.
//!
//! Each provider knows how to:
//! 1. produce an authorize URL with state + PKCE,
//! 2. exchange the callback code for a verified user info record.
//!
//! Wired providers: GitHub, Google, GitLab. Adding another provider
//! is one new file plus wiring in `Config::from_env`. Microsoft and
//! Apple are intentionally excluded — Microsoft because tenant
//! admins can mint unverified-email accounts that would defeat our
//! email-as-link key, Apple because the OAuth setup (signing key,
//! team id, key id, JWT client secret rotation) is high-touch for
//! the value it adds at this scale.

use async_trait::async_trait;
use oauth2::{CsrfToken, PkceCodeVerifier};
use url::Url;

use crate::error::Result;

pub mod github;
pub mod gitlab;
pub mod google;

/// Hard ceiling on a single token-exchange request, used by every
/// provider's `request_async`. `oauth2::reqwest::async_http_client`
/// builds a fresh client with no timeouts (oauth2 4.4 / reqwest 0.11),
/// so without this wrap a hung GitHub / Google / GitLab token endpoint
/// stalls the callback for as long as the OS keeps the socket open.
/// The outer `auth_callback` is itself bounded at 15s; this 8s ceiling
/// leaves room for one retry inside the callback budget.
pub(crate) const OAUTH_EXCHANGE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);

#[derive(Debug, Clone)]
pub struct UserInfo {
    /// Stable per-provider user identifier. For GitHub: numeric id
    /// (not the username, which can be renamed).
    pub provider_subject: String,
    /// Provider-verified email; we don't run our own verification.
    /// May be `None` if the user hides email and has no verified
    /// fallback.
    pub email: Option<String>,
    pub display_name: Option<String>,
    /// Provider-hosted avatar URL. GitHub: `avatar_url`. Google:
    /// `picture`. GitLab: `avatar_url`. The browser fetches the URL
    /// directly; we never proxy or cache.
    pub picture_url: Option<String>,
}

#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &'static str;

    /// Returns (authorize_url, csrf, pkce_verifier). Caller stashes
    /// csrf + verifier in the session and redirects the browser.
    /// Fallible because constructing the oauth2 client involves URL
    /// parsing that, while hardcoded today, is exposed as `Result`
    /// to keep the trait surface honest if a future provider takes
    /// runtime-configured endpoints.
    fn authorize_url(&self, redirect_uri: &Url) -> Result<(Url, CsrfToken, PkceCodeVerifier)>;

    /// Exchange the auth code for tokens, then fetch user info.
    async fn exchange(
        &self,
        code: &str,
        verifier: PkceCodeVerifier,
        redirect_uri: &Url,
    ) -> Result<UserInfo>;
}
