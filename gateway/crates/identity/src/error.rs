use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use gateway_common::drive_admin_client::DriveAdminError;
use gateway_common::profile_client::ProfileError;
use serde_json::json;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unauthorized")]
    Unauthorized,

    /// Authenticated but the action is refused (e.g. the account is
    /// blocked). Distinct from Unauthorized so the SPA can render a
    /// "blocked" view instead of bouncing back to sign-in.
    #[error("forbidden: {0}")]
    Forbidden(&'static str),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("not found")]
    NotFound,

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("upstream error: {0}")]
    Upstream(String),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}

/// Map profile-service client errors onto the local axum error so
/// request handlers can `?` a profile call straight through. Each
/// ProfileError variant has a 1-1 axum response: NotFound -> 404,
/// BadRequest -> 400, Conflict -> 409, Upstream/Reqwest -> 502.
impl From<ProfileError> for Error {
    fn from(e: ProfileError) -> Self {
        match e {
            ProfileError::NotFound => Error::NotFound,
            ProfileError::BadRequest(m) => Error::BadRequest(m),
            ProfileError::Conflict(m) => Error::Conflict(m),
            ProfileError::Upstream(m) => Error::Upstream(m),
            ProfileError::Reqwest(e) => Error::Reqwest(e),
        }
    }
}

/// drive-proxy admin failures only surface in the account-delete and
/// token-revoke paths. Map them to Upstream so the caller can decide
/// whether to log-and-continue (every current caller does) or to
/// `?` straight through.
impl From<DriveAdminError> for Error {
    fn from(e: DriveAdminError) -> Self {
        match e {
            DriveAdminError::Upstream(m) => Error::Upstream(m),
            DriveAdminError::Reqwest(e) => Error::Reqwest(e),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Error::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".to_string()),
            Error::Forbidden(m) => (StatusCode::FORBIDDEN, (*m).to_string()),
            Error::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            Error::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
            Error::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            // Upstream detail (oauth2 RequestTokenError, profile-service body,
            // drive-proxy admin response) stays in the server log; the public
            // body is fixed so OAuth provider errors and profile SQL fragments
            // do not leak through the 502.
            Error::Upstream(detail) => {
                tracing::warn!(detail = %detail, "upstream error");
                (StatusCode::BAD_GATEWAY, "upstream unreachable".to_string())
            }
            Error::Anyhow(e) => {
                tracing::error!(error = ?e, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal error".to_string(),
                )
            }
            Error::Reqwest(e) => {
                tracing::error!(error = ?e, "reqwest error");
                (StatusCode::BAD_GATEWAY, "upstream unreachable".to_string())
            }
        };
        (status, Json(json!({"error": message}))).into_response()
    }
}
