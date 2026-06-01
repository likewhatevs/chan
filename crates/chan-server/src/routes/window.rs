//! Window reply route (the SPA side of `cs pane`).
//!
//! A `cs pane` call blocks in the control socket on a oneshot parked in the
//! [`crate::window_bus::WindowBus`] keyed by a server-minted `request_id`.
//! The SPA reads its `layout`, builds the snapshot, and POSTs a
//! [`WindowReplyRequest`] here; this route calls [`crate::window_bus::WindowBus::complete`],
//! which fires the oneshot and unblocks the CLI with the snapshot. This is
//! the reply half of the window channel, mirroring `routes::survey` for the
//! survey bus.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

use crate::error::err;
use crate::state::AppState;

/// Body of `POST /api/window/reply`. camelCase to match the SPA
/// (`web/src/api/client.ts` `WindowReplyRequest`). `payload` is opaque to the
/// server: the CLI formats it. For a `cs pane` query it is the layout
/// snapshot the SPA built from its `layout` singleton.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowReplyRequest {
    pub request_id: String,
    pub payload: serde_json::Value,
}

/// `POST /api/window/reply` - complete a parked `cs pane` round-trip with the
/// SPA's payload. 404 when no request with that id is parked (already
/// answered, timed out, or a stale id).
pub async fn api_window_reply(
    State(state): State<Arc<AppState>>,
    Json(req): Json<WindowReplyRequest>,
) -> Response {
    if state.window_bus.complete(&req.request_id, req.payload) {
        Json(serde_json::json!({})).into_response()
    } else {
        err(
            StatusCode::NOT_FOUND,
            format!(
                "no window request parked with id {} (already answered, timed out, or stale)",
                req.request_id
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_reply_request_deserializes_camel_case() {
        let json = r#"{"requestId":"win-3","payload":{"activePaneId":"p1","panes":[]}}"#;
        let req: WindowReplyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.request_id, "win-3");
        assert_eq!(req.payload["activePaneId"], "p1");
        assert!(req.payload["panes"].is_array());
    }
}
