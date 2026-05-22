//! `fullstack-a-66` Drafts route. SPA Cmd+N → `POST /api/drafts/new`
//! → creates `Drafts/<next-untitled>/draft.md` + indexes it +
//! returns the unified-path so the SPA can open it via the
//! existing `/api/files/Drafts/.../draft.md` route.
//!
//! Drafts live in chan-drive metadata (`drafts_dir()`), OUTSIDE
//! the drive root, but appear in the wire under the `Drafts/`
//! prefix per the keyspace `systacean-25` + `-26` unified.
//! `Drive::create_draft_dir`, `next_untitled_draft_name`,
//! `write_text`, and `index_draft_file` (called via the unified
//! `write_text` after `-26`) all route correctly.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use crate::error::{err, err_from};
use crate::state::AppState;

#[derive(Serialize)]
pub struct DraftCreateResponse {
    /// Unified-path for the new draft.md: `Drafts/<name>/draft.md`.
    /// SPA `openInActivePane(path)` routes through
    /// `/api/files/Drafts/<name>/draft.md` which post-`-26` reads
    /// from the drafts dir transparently.
    pub path: String,
    /// Bare draft name (e.g. `"untitled"` or `"untitled-3"`), in
    /// case the SPA wants to show it separately from the path.
    pub name: String,
}

/// Create a fresh draft directory + an empty `draft.md` inside.
///
/// Race-window note: `next_untitled_draft_name` + `create_draft_dir`
/// can race against another concurrent creator; if `create_draft_dir`
/// returns `AlreadyExists` we retry once with a re-resolved name.
/// The race is rare in practice (single-user / single-machine) but
/// the retry keeps the contract clean.
pub async fn api_create_draft(
    State(state): State<Arc<AppState>>,
) -> Response {
    let drive = state.drive().clone();
    let result = tokio::task::spawn_blocking(move || -> Result<String, chan_drive::ChanError> {
        for _ in 0..2 {
            let name = drive.next_untitled_draft_name()?;
            match drive.create_draft_dir(&name) {
                Ok(_) => {
                    let unified = format!("Drafts/{name}/draft.md");
                    drive.write_text(&unified, "")?;
                    return Ok(name);
                }
                Err(chan_drive::ChanError::Io(msg)) if msg.contains("already exists") => {
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        Err(chan_drive::ChanError::Io(
            "race condition picking next untitled draft name (retried 2x)".to_string(),
        ))
    })
    .await;

    let name = match result {
        Ok(Ok(name)) => name,
        Ok(Err(e)) => return err_from(&e),
        Err(join) => return err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    };

    let path = format!("Drafts/{name}/draft.md");
    state.self_writes.note(&path);
    Json(DraftCreateResponse { path, name }).into_response()
}
