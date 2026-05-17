//! Session blobs (`/api/session*`).
//!
//! chan-drive owns the I/O (Drive::{put,get,list,delete}_session).
//! chan-server is a thin HTTP shell; the JSON schema of session blobs
//! (window/pane layout) lives in the frontend, not here.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

use crate::error::err_from;
use crate::state::AppState;
use crate::util::raw_json_response;

/// Window id query param (`?w=<id>`) for session routes.
#[derive(Deserialize)]
pub struct SessionQuery {
    w: String,
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
