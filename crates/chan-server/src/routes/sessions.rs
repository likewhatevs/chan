//! Session blobs (`/api/session*`).
//!
//! chan-workspace owns the I/O (Workspace::{put,get,list,delete}_session).
//! chan-server is a thin HTTP shell; the JSON schema of session blobs
//! (window/pane layout) lives in the frontend, not here.

use std::sync::Arc;

use axum::body::Bytes;
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

async fn blocking_response(
    f: impl FnOnce() -> Response + Send + 'static,
    label: &'static str,
) -> Response {
    match tokio::task::spawn_blocking(f).await {
        Ok(response) => response,
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("{label} task panicked: {e}"),
        )
            .into_response(),
    }
}

pub async fn api_get_session(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SessionQuery>,
) -> Response {
    let workspace = state.workspace();
    let key = q.w;
    blocking_response(
        move || match workspace.get_session(&key) {
            Ok(Some(bytes)) => raw_json_response(bytes),
            // 204 NO_CONTENT, not 404: "no session yet" is the normal
            // first-launch state. transport.ts treats an empty 2xx body
            // as `undefined`; the api wrapper coerces that to `null`.
            Ok(None) => StatusCode::NO_CONTENT.into_response(),
            Err(e) => err_from(&e),
        },
        "get session",
    )
    .await
}

pub async fn api_put_session(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SessionQuery>,
    body: Bytes,
) -> Response {
    let workspace = state.workspace();
    let key = q.w;
    blocking_response(
        move || match workspace.put_session(&key, &body) {
            Ok(()) => StatusCode::NO_CONTENT.into_response(),
            Err(e) => err_from(&e),
        },
        "put session",
    )
    .await
}

pub async fn api_delete_session(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SessionQuery>,
) -> Response {
    let workspace = state.workspace();
    let key = q.w;
    blocking_response(
        move || match workspace.delete_session(&key) {
            Ok(()) => StatusCode::NO_CONTENT.into_response(),
            Err(e) => err_from(&e),
        },
        "delete session",
    )
    .await
}

pub async fn api_list_sessions(State(state): State<Arc<AppState>>) -> Response {
    let workspace = state.workspace();
    blocking_response(
        move || match workspace.list_sessions() {
            Ok(keys) => Json(keys).into_response(),
            Err(e) => err_from(&e),
        },
        "list sessions",
    )
    .await
}

#[cfg(test)]
mod tests {
    #[test]
    fn session_routes_wrap_sync_workspace_io_in_spawn_blocking() {
        let source = include_str!("sessions.rs");

        assert!(source.contains("tokio::task::spawn_blocking(f)"));
        assert!(source.contains("move || match workspace.get_session(&key)"));
        assert!(source.contains("move || match workspace.put_session(&key, &body)"));
        assert!(source.contains("move || match workspace.delete_session(&key)"));
        assert!(source.contains("move || match workspace.list_sessions()"));
    }
}
