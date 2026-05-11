//! Error type and HTTP response builders.
//!
//! `Error` is the crate-wide error returned by `serve()`/`serve_via_tunnel()`.
//! The `err_*` helpers shape uniform `{"error": "..."}` JSON bodies and map
//! chan-drive / chan-llm errors onto the right HTTP status. Routes call into
//! these instead of building responses by hand so the wire shape stays
//! consistent across handlers.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chan_llm::LlmError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("chan-drive: {0}")]
    Core(#[from] chan_drive::ChanError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("config: {0}")]
    Config(String),
}

/// Wrap a status + message into the standard `{"error": "..."}` body.
pub fn err(status: StatusCode, msg: String) -> Response {
    (status, Json(serde_json::json!({"error": msg}))).into_response()
}

/// Refusal returned by every settings-area write endpoint when the
/// server was started with `settings_disabled = true` (today: any
/// tunnel run). Reads stay open; only mutating routes call this.
/// Uses 403 because the request is well-formed and authenticated by
/// the gateway, the host policy just forbids the operation.
pub fn err_settings_locked() -> Response {
    err(
        StatusCode::FORBIDDEN,
        "settings are disabled by the host: this server is running \
         in a mode that forbids configuration changes from the UI \
         (tunnel mode)"
            .into(),
    )
}

/// Map a chan-llm error to the right HTTP status. Backend HTTP errors
/// surface their upstream status when it's a valid u16; everything else
/// collapses to 500 / 400 / 501 by category.
pub fn err_llm(e: &LlmError) -> Response {
    let status = match e {
        LlmError::MissingApiKey(_) => StatusCode::BAD_REQUEST,
        LlmError::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
        LlmError::BackendError { status, .. } => {
            StatusCode::from_u16(*status).unwrap_or(StatusCode::BAD_GATEWAY)
        }
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    err(status, e.to_string())
}

/// Map chan-drive errors to HTTP statuses. The shape of the JSON
/// matches the old server so frontend error handling stays unchanged.
pub fn err_from(e: &chan_drive::ChanError) -> Response {
    use chan_drive::ChanError as C;
    let (status, msg) = match e {
        C::PathEmpty | C::PathEscape | C::SymlinkEscape(_) => {
            (StatusCode::BAD_REQUEST, e.to_string())
        }
        C::NotEditableText(_) => (StatusCode::UNSUPPORTED_MEDIA_TYPE, e.to_string()),
        C::SpecialFile { .. } => (StatusCode::UNSUPPORTED_MEDIA_TYPE, e.to_string()),
        C::DriveNotRegistered(_) | C::DriveRootMissing(_) => (StatusCode::NOT_FOUND, e.to_string()),
        C::DriveLocked => (StatusCode::CONFLICT, e.to_string()),
        C::Io(s) if s.contains("No such file") || s.contains("not found") => {
            (StatusCode::NOT_FOUND, e.to_string())
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    err(status, msg)
}
