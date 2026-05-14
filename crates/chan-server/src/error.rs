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

/// Refusal returned by `tunnel_guard::settings_guard` when the
/// server was started with `settings_disabled = true`
/// (`--tunnel-public` runs, or any future caller that opts in).
/// 403 because the request is well-formed; the host policy just
/// forbids the operation. Single source of truth for the error
/// body so SPA error toasts stay consistent.
pub fn err_settings_locked() -> Response {
    err(
        StatusCode::FORBIDDEN,
        "settings are disabled while this drive is shared publicly; \
         configuration changes are only allowed on a local (loopback) \
         serve or an OAuth-gated tunnel"
            .into(),
    )
}

/// Refusal returned by `tunnel_guard::tunnel_public_guard` for
/// cost-bearing routes that must not be reachable from anonymous
/// visitors when the server is running with `--tunnel-public`. The
/// drive owner's LLM tokens, the indexer's CPU/IO budget, and the
/// keychain backends are all attached to the owner's machine; an
/// unauthenticated visitor cannot be allowed to draw on them.
pub fn err_tunnel_public_locked() -> Response {
    err(
        StatusCode::FORBIDDEN,
        "this operation requires an authenticated session; the \
         public tunnel does not authenticate visitors. Use a \
         loopback serve or a private (non-public) tunnel."
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    async fn body_json(r: Response) -> serde_json::Value {
        let (parts, body) = r.into_parts();
        let bytes = to_bytes(body, 8192).await.expect("read body");
        // Sanity: error bodies are tiny, way under 8 KiB.
        assert_eq!(parts.status, StatusCode::FORBIDDEN);
        serde_json::from_slice(&bytes).expect("error body is JSON")
    }

    #[tokio::test]
    async fn err_settings_locked_shape() {
        let v = body_json(err_settings_locked()).await;
        let msg = v
            .get("error")
            .and_then(|x| x.as_str())
            .expect("error field");
        assert!(
            msg.contains("settings"),
            "wrong message: {msg:?}, must reference 'settings' so the SPA \
             toast is recognisable"
        );
    }

    #[tokio::test]
    async fn err_tunnel_public_locked_shape() {
        let v = body_json(err_tunnel_public_locked()).await;
        let msg = v
            .get("error")
            .and_then(|x| x.as_str())
            .expect("error field");
        // The exact text is a UX choice and can drift, but the body
        // MUST carry a `error` string field at 403 — that's the
        // wire contract every chan-server refusal shares.
        assert!(!msg.is_empty());
    }
}
