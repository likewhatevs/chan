use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unauthorized")]
    Unauthorized,

    #[error("not found")]
    NotFound,

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("upstream error: {0}")]
    Upstream(String),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Error::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".to_string()),
            Error::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
            Error::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            // Upstream detail (hyper / yamux / reqwest message) stays in the
            // server log; the public body is intentionally fixed so a probe
            // cannot enumerate failure modes by reading the response.
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
