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

/// Lock the workspace-less tenant's in-memory session store. Recovers from a
/// poisoned lock (the critical sections are simple map ops that never leave
/// it inconsistent) so a session request can never itself panic the server.
fn ephemeral_lock(
    state: &AppState,
) -> std::sync::MutexGuard<'_, std::collections::HashMap<String, Vec<u8>>> {
    state
        .ephemeral_sessions
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

pub async fn api_get_session(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SessionQuery>,
) -> Response {
    let key = q.w;
    let Ok(workspace) = state.try_workspace() else {
        // Workspace-less terminal tenant: tenant-scoped in-memory store.
        return match ephemeral_lock(&state).get(&key) {
            Some(bytes) => raw_json_response(bytes.clone()),
            None => StatusCode::NO_CONTENT.into_response(),
        };
    };
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
    let key = q.w;
    let Ok(workspace) = state.try_workspace() else {
        ephemeral_lock(&state).insert(key, body.to_vec());
        return StatusCode::NO_CONTENT.into_response();
    };
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
    let key = q.w;
    let Ok(workspace) = state.try_workspace() else {
        ephemeral_lock(&state).remove(&key);
        return StatusCode::NO_CONTENT.into_response();
    };
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
    let Ok(workspace) = state.try_workspace() else {
        let keys: Vec<String> = ephemeral_lock(&state).keys().cloned().collect();
        return Json(keys).into_response();
    };
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
