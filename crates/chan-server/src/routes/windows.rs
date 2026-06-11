//! `GET /api/windows` — enumerate the windows this tenant knows about.
//!
//! A "window" here is a per-window session id: the `?w=<id>` that keys
//! the `/api/session` layout blob and tags the window's `/ws` socket.
//! The response is the union of both sources:
//!
//!   * `saved`: a session blob exists (the window persisted a layout
//!     at some point) — on disk for workspace tenants, in the
//!     ephemeral map for terminal tenants.
//!   * `connected`: a `/ws` socket tagged with this id is live RIGHT
//!     NOW, somewhere — any browser tab or desktop webview. Note a
//!     hidden (buried) chan-desktop window keeps its webview and
//!     therefore stays `connected`; the server cannot distinguish
//!     hidden from visible, so it deliberately doesn't claim to.
//!
//! chan-desktop polls this on remote attachments (outbound / tunnel)
//! to offer `saved && !connected` windows in its Window menu for
//! ad-hoc reopening; `cs window list` serves the same view in a
//! terminal.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use crate::error::err_from;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct WindowInfo {
    /// Per-window session id (the `?w=` value; for chan-desktop
    /// windows this is the Tauri window label).
    pub id: String,
    /// A `/ws` socket tagged with this id is currently live.
    pub connected: bool,
    /// A session blob exists for this id.
    pub saved: bool,
}

/// Join saved blob keys and live socket ids into one sorted list.
/// BTreeMap so the response order is deterministic (id-sorted).
/// pub(crate): the `cs window list` control-socket handler serves the
/// same rows.
pub(crate) fn join_windows(saved: Vec<String>, connected: Vec<String>) -> Vec<WindowInfo> {
    let mut by_id: BTreeMap<String, (bool, bool)> = BTreeMap::new();
    for id in saved {
        by_id.entry(id).or_insert((false, false)).1 = true;
    }
    for id in connected {
        by_id.entry(id).or_insert((false, false)).0 = true;
    }
    by_id
        .into_iter()
        .map(|(id, (connected, saved))| WindowInfo {
            id,
            connected,
            saved,
        })
        .collect()
}

pub async fn api_list_windows(State(state): State<Arc<AppState>>) -> Response {
    let connected = state.window_presence.connected_ids();
    let Ok(workspace) = state.try_workspace() else {
        // Workspace-less terminal tenant: blobs live in memory.
        let saved: Vec<String> = state
            .ephemeral_sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect();
        return Json(join_windows(saved, connected)).into_response();
    };
    // Same spawn_blocking posture as the session routes: list_sessions
    // is sync disk I/O.
    match tokio::task::spawn_blocking(move || workspace.list_sessions()).await {
        Ok(Ok(saved)) => Json(join_windows(saved, connected)).into_response(),
        Ok(Err(e)) => err_from(&e),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("list windows task panicked: {e}"),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_is_a_sorted_union_with_per_source_flags() {
        let joined = join_windows(
            vec!["w-b".into(), "w-a".into()],
            vec!["w-c".into(), "w-b".into()],
        );
        let view: Vec<(&str, bool, bool)> = joined
            .iter()
            .map(|w| (w.id.as_str(), w.connected, w.saved))
            .collect();
        assert_eq!(
            view,
            [
                ("w-a", false, true),
                ("w-b", true, true),
                ("w-c", true, false),
            ],
        );
    }

    // Wire pin: the desktop and `cs window list` parse these exact
    // field names; a rename is a runtime break the type checker can't
    // see across the HTTP boundary.
    #[test]
    fn window_info_serializes_id_connected_saved() {
        let json = serde_json::to_string(&WindowInfo {
            id: "workspace-aa-0".into(),
            connected: true,
            saved: false,
        })
        .unwrap();
        assert_eq!(
            json,
            r#"{"id":"workspace-aa-0","connected":true,"saved":false}"#,
        );
    }
}
