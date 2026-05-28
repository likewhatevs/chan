//! GitLab OAuth2 provider (gitlab.com by default; the endpoints
//! struct supports self-hosted instances later).
//!
//! Subject = numeric user id (stable across renames). Email comes
//! straight from `/api/v4/user`; GitLab marks unverified emails as
//! `confirmed_at: null` so we filter on that.

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

const AUTH_URL: &str = "https://gitlab.com/oauth/authorize";
const TOKEN_URL: &str = "https://gitlab.com/oauth/token";
const USERINFO_URL: &str = "https://gitlab.com/api/v4/user";

#[derive(Clone)]
pub struct GitLabEndpoints {
    pub auth: String,
    pub token: String,
    pub userinfo: String,
}

impl Default for GitLabEndpoints {
    fn default() -> Self {
        Self {
            auth: AUTH_URL.into(),
            token: TOKEN_URL.into(),
            userinfo: USERINFO_URL.into(),
        }
    }
}

#[derive(Clone)]
pub struct GitLabProvider {
    client_id: ClientId,
    client_secret: ClientSecret,
    endpoints: GitLabEndpoints,
    http: reqwest::Client,
}

impl GitLabProvider {
    pub fn new(client_id: String, client_secret: String) -> anyhow::Result<Self> {
        Self::with_endpoints(client_id, client_secret, GitLabEndpoints::default())
    }

    pub fn with_endpoints(
        client_id: String,
        client_secret: String,
        endpoints: GitLabEndpoints,
    ) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
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
        Ok(BasicClient::new(
            self.client_id.clone(),
            Some(self.client_secret.clone()),
            AuthUrl::new(self.endpoints.auth.clone())?,
            Some(TokenUrl::new(self.endpoints.token.clone())?),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_uri.to_string())?))
    }
}

#[async_trait]
impl Provider for GitLabProvider {
    fn name(&self) -> &'static str {
        "gitlab"
    }

    fn authorize_url(&self, redirect_uri: &Url) -> Result<(Url, CsrfToken, PkceCodeVerifier)> {
        let client = self.oauth_client(redirect_uri).map_err(Error::Anyhow)?;
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let (url, csrf) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("read_user".into()))
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
                tracing::warn!("gitlab token exchange timed out");
                Error::Upstream("oauth token exchange".to_string())
            })?
            .map_err(|e| {
                tracing::warn!(error = ?e, "gitlab token exchange failed");
                Error::Upstream("oauth token exchange".to_string())
            })?;
        let access = token.access_token().secret();

        let user: GLUser = self
            .http
            .get(&self.endpoints.userinfo)
            .header(header::AUTHORIZATION, format!("Bearer {access}"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        // Refuse the email if GitLab hasn't confirmed it; matches
        // our policy across providers.
        let email = user
            .confirmed_at
            .is_some()
            .then(|| user.email.clone())
            .flatten();

        let picture_url = user.avatar_url.clone();
        Ok(UserInfo {
            provider_subject: user.id.to_string(),
            email,
            display_name: user.name.or(Some(user.username)),
            picture_url,
        })
    }
}

#[derive(Debug, Deserialize)]
struct GLUser {
    id: u64,
    username: String,
    name: Option<String>,
    email: Option<String>,
    confirmed_at: Option<String>,
    avatar_url: Option<String>,
}
