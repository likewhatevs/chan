//! Session blobs (`/api/session*`), assistant conversation blobs
//! (`/api/assistant/conversation*`), and saved answers (`/api/answers`).
//!
//! chan-drive owns the I/O (Drive::{put,get,list,delete}_session +
//! _assistant + clear_assistant). chan-server is a thin HTTP shell;
//! the JSON schema of session blobs (window/pane layout) and
//! assistant blobs (chat turns) lives in the frontend, not here.
//!
//! Answers are different: the user picks a directory inside the
//! drive (`server.toml` -> answers_dir) and we land each saved
//! answer as a `.md` file there via Drive::write_text. Same path
//! sandbox + special-file refusal apply.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::err_from;
use crate::state::AppState;
use crate::util::{extract_h1, raw_json_response, slugify_for_filename, timestamp_slug};

/// Window id query param (`?w=<id>`) for session routes.
#[derive(Deserialize)]
pub struct SessionQuery {
    w: String,
}

/// Conversation key query param (`?path=<key>`) for the assistant
/// blob routes. The key is either a file path (per-file
/// conversation) or a synthetic group key (per-window-pane group);
/// the server treats it as opaque since the chunking is the
/// frontend's concern.
#[derive(Deserialize)]
pub struct ConversationQuery {
    path: String,
}

pub async fn api_get_session(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SessionQuery>,
) -> Response {
    match state.drive().get_session(&q.w) {
        Ok(Some(bytes)) => raw_json_response(bytes),
        // 204 NO_CONTENT, not 404: "no session yet" is the normal
        // first-launch state. transport.ts treats an empty 2xx body
        // as `undefined`; the api wrapper coerces that to `null`.
        Ok(None) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_from(&e),
    }
}

pub async fn api_put_session(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SessionQuery>,
    body: axum::body::Bytes,
) -> Response {
    match state.drive().put_session(&q.w, &body) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_from(&e),
    }
}

pub async fn api_delete_session(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SessionQuery>,
) -> Response {
    match state.drive().delete_session(&q.w) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_from(&e),
    }
}

pub async fn api_list_sessions(State(state): State<Arc<AppState>>) -> Response {
    match state.drive().list_sessions() {
        Ok(keys) => Json(keys).into_response(),
        Err(e) => err_from(&e),
    }
}

pub async fn api_get_assistant(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ConversationQuery>,
) -> Response {
    match state.drive().get_assistant(&q.path) {
        Ok(Some(bytes)) => raw_json_response(bytes),
        // 204 NO_CONTENT, not 404: same reasoning as get_session.
        Ok(None) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_from(&e),
    }
}

pub async fn api_put_assistant(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ConversationQuery>,
    body: axum::body::Bytes,
) -> Response {
    match state.drive().put_assistant(&q.path, &body) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_from(&e),
    }
}

pub async fn api_delete_assistant(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ConversationQuery>,
) -> Response {
    match state.drive().delete_assistant(&q.path) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_from(&e),
    }
}

pub async fn api_list_assistant(State(state): State<Arc<AppState>>) -> Response {
    match state.drive().list_assistant() {
        Ok(keys) => Json(keys).into_response(),
        Err(e) => err_from(&e),
    }
}

pub async fn api_clear_assistant(State(state): State<Arc<AppState>>) -> Response {
    match state.drive().clear_assistant() {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_from(&e),
    }
}

#[derive(Deserialize)]
pub struct AnswerBody {
    /// Markdown content to save. Becomes a new `.md` file under
    /// the configured `answers_dir`. Filename is derived from the
    /// body's first heading or, failing that, a timestamp slug.
    content: String,
    /// Optional override for the filename stem (no extension; the
    /// server appends `.md`). Useful when the frontend generates
    /// its own stable id for a saved answer.
    #[serde(default)]
    name: Option<String>,
}

#[derive(Serialize)]
struct AnswerSaved {
    /// Drive-relative POSIX path the answer landed at.
    path: String,
}

pub async fn api_post_answer(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AnswerBody>,
) -> Response {
    let dir = state.server_config.lock().unwrap().answers_dir.clone();
    let stem = body
        .name
        .as_deref()
        .map(slugify_for_filename)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            body.content
                .lines()
                .find_map(extract_h1)
                .map(|s| slugify_for_filename(&s))
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(timestamp_slug);
    let rel = format!("{dir}/{stem}.md");
    match state.drive().write_text(&rel, &body.content) {
        Ok(()) => {
            state.self_writes.note(&rel);
            Json(AnswerSaved { path: rel }).into_response()
        }
        Err(e) => err_from(&e),
    }
}
