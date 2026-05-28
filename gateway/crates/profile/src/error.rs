use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not found")]
    NotFound,

    #[error("conflict: {0}")]
    Conflict(&'static str),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("unauthorized")]
    Unauthorized,

    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Error::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
            Error::Conflict(m) => (StatusCode::CONFLICT, (*m).to_string()),
            Error::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            Error::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".to_string()),
            Error::Db(e) => {
                // Postgres unique-violation surfaces here when callers race; map
                // it to 409 so callers can retry the lookup-or-create flow
                // without inspecting our error wire format.
                if let Some(dbe) = e.as_database_error() {
                    if dbe.code().as_deref() == Some("23505") {
                        return (StatusCode::CONFLICT, Json(json!({"error": "conflict"})))
                            .into_response();
                    }
                }
                tracing::error!(error = ?e, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal error".to_string(),
                )
            }
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
