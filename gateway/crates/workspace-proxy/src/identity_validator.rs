//! Token validator backed by identity-service's
//! `/internal/v1/tokens/validate`.
//!
//! The endpoint shape:
//!
//!   POST /internal/v1/tokens/validate
//!   Authorization: Bearer <shared internal bearer>
//!   Content-Type: application/json
//!   { "token": "<chan_pat_*>" }
//!
//!   200 { "user_id": "...", "username": "...", "token_id": "...",
//!         "expires_at": "<iso8601|null>",
//!         "scopes": ["tunnel", "tunnel.public", ...] }
//!   401 if the token is unknown / revoked / expired
//!
//! Scopes are per-token: identity stores them in `api_tokens.scopes`
//! and emits them in the validate response. workspace-proxy forwards
//! them verbatim into `Validated::scopes`; chan-tunnel-server then
//! gates `tunnel` (base dial) and `tunnel.public` (anonymous-readable
//! workspace) against the validated list. A token minted without
//! `tunnel.public` cannot host a public workspace at runtime.

use async_trait::async_trait;
use chan_tunnel_server::{ServerError, Validated, Validator};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::registry::Registry;

#[derive(Clone)]
pub struct IdentityValidator {
    client: Client,
    endpoint: Url,
    auth_header: String,
}

impl IdentityValidator {
    pub fn new(identity_url: Url, auth_token: String) -> anyhow::Result<Self> {
        let endpoint = identity_url.join("/internal/v1/tokens/validate")?;
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .connect_timeout(std::time::Duration::from_secs(2))
            .build()?;
        Ok(Self {
            client,
            endpoint,
            auth_header: format!("Bearer {auth_token}"),
        })
    }
}

#[derive(Serialize)]
struct ValidateRequest<'a> {
    token: &'a str,
}

#[derive(Deserialize)]
struct ValidateResponse {
    user_id: Uuid,
    username: String,
    #[serde(default)]
    scopes: Vec<String>,
}

#[async_trait]
impl Validator for IdentityValidator {
    async fn validate(&self, token: &str) -> Result<Validated, ServerError> {
        let resp = self
            .client
            .post(self.endpoint.clone())
            .header(reqwest::header::AUTHORIZATION, &self.auth_header)
            .json(&ValidateRequest { token })
            .send()
            .await
            .map_err(|e| ServerError::Identity(format!("request: {e}")))?;

        match resp.status() {
            StatusCode::OK => {
                let body: ValidateResponse = resp
                    .json()
                    .await
                    .map_err(|e| ServerError::Identity(format!("decode: {e}")))?;
                Ok(Validated {
                    user_id: body.user_id,
                    username: body.username,
                    scopes: body.scopes,
                })
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(ServerError::InvalidToken),
            other => Err(ServerError::Identity(format!("unexpected status {other}"))),
        }
    }
}

/// Wraps a Validator and records the (username, user_id) mapping
/// in the shared `Registry` on every successful validate. The
/// reverse-proxy auth gate reads back the cached user_id when
/// matching the session against the tunnel owner; without this
/// cache workspace-proxy would have to round-trip to profile-service
/// on every public request.
pub struct CapturingValidator<V: Validator> {
    inner: V,
    registry: Registry,
}

impl<V: Validator> CapturingValidator<V> {
    pub fn new(inner: V, registry: Registry) -> Self {
        Self { inner, registry }
    }
}

#[async_trait]
impl<V: Validator> Validator for CapturingValidator<V> {
    async fn validate(&self, token: &str) -> Result<Validated, ServerError> {
        let validated = self.inner.validate(token).await?;
        self.registry
            .record_user(&validated.username, validated.user_id);
        Ok(validated)
    }
}
