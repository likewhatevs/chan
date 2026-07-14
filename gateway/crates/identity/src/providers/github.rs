//! GitHub OAuth2 provider.
//!
//! GitHub is not OIDC, so user info comes from REST endpoints:
//! - `GET /user`: id, name, login, email-or-null.
//! - `GET /user/emails`: list of `{ email, primary, verified }`.
//!   Used when the primary email is private. Requires the
//!   `user:email` scope.

use async_trait::async_trait;
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use reqwest::header;
use serde::Deserialize;
use url::Url;

use crate::error::{Error, Result};
use crate::providers::{Provider, UserInfo, OAUTH_EXCHANGE_TIMEOUT};

const AUTH_URL: &str = "https://github.com/login/oauth/authorize";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const USER_API: &str = "https://api.github.com/user";
const EMAILS_API: &str = "https://api.github.com/user/emails";
const USER_AGENT: &str = "chan-gateway-identity";

/// Override for tests / future enterprise GitHub support.
#[derive(Clone)]
pub struct GitHubEndpoints {
    pub auth: String,
    pub token: String,
    pub user: String,
    pub emails: String,
}

impl Default for GitHubEndpoints {
    fn default() -> Self {
        Self {
            auth: AUTH_URL.into(),
            token: TOKEN_URL.into(),
            user: USER_API.into(),
            emails: EMAILS_API.into(),
        }
    }
}

#[derive(Clone)]
pub struct GitHubProvider {
    client_id: ClientId,
    client_secret: ClientSecret,
    endpoints: GitHubEndpoints,
    http: reqwest::Client,
}

impl GitHubProvider {
    pub fn new(client_id: String, client_secret: String) -> anyhow::Result<Self> {
        Self::with_endpoints(client_id, client_secret, GitHubEndpoints::default())
    }

    pub fn with_endpoints(
        client_id: String,
        client_secret: String,
        endpoints: GitHubEndpoints,
    ) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(std::time::Duration::from_secs(10))
            .connect_timeout(std::time::Duration::from_secs(2))
            .build()?;
        Ok(Self {
            client_id: ClientId::new(client_id),
            client_secret: ClientSecret::new(client_secret),
            endpoints,
            http,
        })
    }

    fn oauth_client(&self, redirect_uri: &Url) -> anyhow::Result<BasicClient> {
        let client = BasicClient::new(
            self.client_id.clone(),
            Some(self.client_secret.clone()),
            AuthUrl::new(self.endpoints.auth.clone())?,
            Some(TokenUrl::new(self.endpoints.token.clone())?),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_uri.to_string())?);
        Ok(client)
    }
}

#[async_trait]
impl Provider for GitHubProvider {
    fn name(&self) -> &'static str {
        "github"
    }

    fn authorize_url(&self, redirect_uri: &Url) -> Result<(Url, CsrfToken, PkceCodeVerifier)> {
        let client = self.oauth_client(redirect_uri).map_err(Error::Anyhow)?;
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let (url, csrf) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("read:user".into()))
            .add_scope(Scope::new("user:email".into()))
            .set_pkce_challenge(challenge)
            .url();
        Ok((url, csrf, verifier))
    }

    async fn exchange(
        &self,
        code: &str,
        verifier: PkceCodeVerifier,
        redirect_uri: &Url,
    ) -> Result<UserInfo> {
        let client = self.oauth_client(redirect_uri).map_err(Error::Anyhow)?;
        let exchange = client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .set_pkce_verifier(verifier)
            .request_async(async_http_client);
        let token = tokio::time::timeout(OAUTH_EXCHANGE_TIMEOUT, exchange)
            .await
            .map_err(|_| {
                tracing::warn!("github token exchange timed out");
                Error::Upstream("oauth token exchange".to_string())
            })?
            .map_err(|e| {
                tracing::warn!(error = ?e, "github token exchange failed");
                Error::Upstream("oauth token exchange".to_string())
            })?;
        let access = token.access_token().secret();

        let user: GhUser = self
            .http
            .get(&self.endpoints.user)
            .header(header::AUTHORIZATION, format!("Bearer {access}"))
            .header(header::ACCEPT, "application/vnd.github+json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let email = match user.email {
            Some(e) => Some(e),
            None => self.fetch_primary_email(access).await?,
        };

        Ok(UserInfo {
            provider_subject: user.id.to_string(),
            email,
            display_name: user.name.or(Some(user.login)),
            picture_url: user.avatar_url,
        })
    }
}

impl GitHubProvider {
    async fn fetch_primary_email(&self, access: &str) -> Result<Option<String>> {
        let emails: Vec<GhEmail> = self
            .http
            .get(&self.endpoints.emails)
            .header(header::AUTHORIZATION, format!("Bearer {access}"))
            .header(header::ACCEPT, "application/vnd.github+json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(emails
            .into_iter()
            .find(|e| e.primary && e.verified)
            .map(|e| e.email))
    }
}

#[derive(Debug, Deserialize)]
struct GhUser {
    id: u64,
    login: String,
    name: Option<String>,
    email: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GhEmail {
    email: String,
    primary: bool,
    verified: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_endpoints_are_stock_github() {
        // IDENTITY_OAUTH_ENDPOINTS_BASE absent must mean exactly
        // these origins; the config override is for test harnesses
        // only and silence keeps production on the real GitHub.
        let e = GitHubEndpoints::default();
        assert_eq!(e.auth, "https://github.com/login/oauth/authorize");
        assert_eq!(e.token, "https://github.com/login/oauth/access_token");
        assert_eq!(e.user, "https://api.github.com/user");
        assert_eq!(e.emails, "https://api.github.com/user/emails");
    }
}
