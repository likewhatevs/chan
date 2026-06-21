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
//!         "scopes": ["tunnel", ...] }
//!   401 if the token is unknown / revoked / expired
//!
//! Scopes are per-token: identity stores them in `api_tokens.scopes`
//! and emits them in the validate response. devserver-proxy forwards
//! them verbatim into `Validated::scopes`; chan-tunnel-server gates the
//! `tunnel` (base dial) scope against the validated list.

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
    /// Token-resolved devserver identity (lowercase hex SHA-256 of the
    /// PAT). identity computes and pins it; the tunnel-server keys the
    /// registration on this value, so the registry's second key is the
    /// devserver id.
    devserver_id: String,
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
                    devserver_id: body.devserver_id,
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
/// reverse-proxy auth gate reads back the cached user_id as metadata
/// for admin tooling; without this cache devserver-proxy would have to
/// round-trip to profile-service on every reverse-proxy request.
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

#[cfg(test)]
mod tests {
    use super::*;

    // Consumer byte-pin (W1): the validate-response field identity produces
    // is `devserver_id`, and the registry keys on it. A one-sided rename of
    // this wire field must fail here, not silently key the registry on an
    // empty string.
    #[test]
    fn validate_response_reads_devserver_id() {
        let json = r#"{
            "user_id": "11111111-1111-4111-8111-111111111111",
            "username": "alice",
            "devserver_id": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "scopes": ["tunnel"]
        }"#;
        let parsed: ValidateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(
            parsed.devserver_id,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        );
        assert_eq!(parsed.username, "alice");
    }

    #[test]
    fn validate_response_without_devserver_id_is_rejected() {
        // devserver_id is required (not defaulted): a response missing it is
        // a contract break, not a silent empty key.
        let json = r#"{"user_id":"11111111-1111-4111-8111-111111111111","username":"a"}"#;
        assert!(serde_json::from_str::<ValidateResponse>(json).is_err());
    }
}
