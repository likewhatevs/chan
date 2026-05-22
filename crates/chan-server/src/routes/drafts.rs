//! `fullstack-a-66` Drafts route.
//!
//! * `POST /api/drafts/new` — slice a (Cmd+N from SPA). Creates
//!   `Drafts/<next-untitled>/draft.md` + indexes it + returns
//!   the unified-path.
//! * `POST /api/drafts/rich-prompt` — slice d. Body is the
//!   submitted Rich Prompt source; route picks the next
//!   `rich-prompt-N` slot under `Drafts/` + writes the source
//!   as `prompt.md` inside. Each submission lands as a fresh
//!   history entry the user can browse via the FB Drafts row.
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
use serde::{Deserialize, Serialize};

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

/// `fullstack-a-66` slice d: payload for the Rich Prompt
/// history-persistence route. The SPA POSTs the submitted
/// source verbatim; the server picks the next slot + writes
/// `prompt.md` inside.
#[derive(Deserialize)]
pub struct RichPromptCreatePayload {
    /// Submitted Rich Prompt source (the text the editor's
    /// `prompt.buffer` held at Cmd+Enter time, including the
    /// agent-submit-mode trailing chord if it was applied).
    pub content: String,
}

#[derive(Serialize)]
pub struct RichPromptCreateResponse {
    /// Unified-path for the new `prompt.md`: `Drafts/rich-prompt-N/prompt.md`.
    pub path: String,
    /// Bare draft dir name (e.g. `"rich-prompt"` or
    /// `"rich-prompt-3"`).
    pub name: String,
}

/// Create a fresh draft directory + an empty `draft.md` inside.
///
/// Race-window note: `next_untitled_draft_name` + `create_draft_dir`
/// can race against another concurrent creator; if `create_draft_dir`
/// returns `AlreadyExists` we retry once with a re-resolved name.
/// The race is rare in practice (single-user / single-machine) but
/// the retry keeps the contract clean.
pub async fn api_create_draft(State(state): State<Arc<AppState>>) -> Response {
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

/// `fullstack-a-66` slice d: persist a Rich Prompt submission
/// into `Drafts/rich-prompt-<N>/prompt.md`.
///
/// Race-window: same retry-once pattern as `api_create_draft`.
/// Concurrent submits in a single SPA window are unlikely (the
/// editor blocks the keystroke until the await returns) but
/// the retry keeps the contract clean for any future concurrent
/// caller (multiple browser tabs, an MCP-driven submit, etc.).
pub async fn api_create_rich_prompt(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RichPromptCreatePayload>,
) -> Response {
    let drive = state.drive().clone();
    let content = payload.content;
    let result = tokio::task::spawn_blocking(move || -> Result<String, chan_drive::ChanError> {
        for _ in 0..2 {
            let name = next_rich_prompt_name(&drive)?;
            match drive.create_draft_dir(&name) {
                Ok(_) => {
                    let unified = format!("Drafts/{name}/prompt.md");
                    drive.write_text(&unified, &content)?;
                    return Ok(name);
                }
                Err(chan_drive::ChanError::Io(msg)) if msg.contains("already exists") => {
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        Err(chan_drive::ChanError::Io(
            "race condition picking next rich-prompt draft name (retried 2x)".to_string(),
        ))
    })
    .await;

    let name = match result {
        Ok(Ok(name)) => name,
        Ok(Err(e)) => return err_from(&e),
        Err(join) => return err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    };

    let path = format!("Drafts/{name}/prompt.md");
    state.self_writes.note(&path);
    Json(RichPromptCreateResponse { path, name }).into_response()
}

/// `fullstack-a-66` slice d: pick the next `rich-prompt-N` slot.
/// Lives in chan-server (not chan-drive) so the prefix-pickup
/// loop stays where its consumer is + doesn't drag a
/// `next_<prefix>_name` API surface into chan-drive. The
/// existing `Drive::next_untitled_draft_name` stays untouched.
///
/// Naming: first slot is `rich-prompt`; subsequent are
/// `rich-prompt-1`, `rich-prompt-2`, ... (matches the
/// `untitled` / `untitled-1` shape `next_untitled_draft_name`
/// uses).
fn next_rich_prompt_name(drive: &chan_drive::Drive) -> Result<String, chan_drive::ChanError> {
    let existing = drive.list_drafts()?;
    let names: std::collections::HashSet<String> =
        existing.into_iter().map(|d| d.name).collect();
    if !names.contains("rich-prompt") {
        return Ok("rich-prompt".to_string());
    }
    let mut i: u32 = 1;
    loop {
        let candidate = format!("rich-prompt-{i}");
        if !names.contains(&candidate) {
            return Ok(candidate);
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // `fullstack-a-66` slice d: prefix-aware name picker should
    // ignore non-matching draft dirs (e.g. coexisting `untitled`
    // drafts from slice a) when picking the next `rich-prompt-N`
    // slot. Also test the gap-counting + the first-slot-without-
    // suffix shape.

    fn make_drive() -> (TempDir, TempDir, std::sync::Arc<chan_drive::Drive>) {
        let cfg = TempDir::new().unwrap();
        let root = TempDir::new().unwrap();
        let lib = chan_drive::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_drive(root.path(), Some("rich-prompt-test".into()))
            .unwrap();
        let drive = lib.open_drive(root.path()).unwrap();
        (cfg, root, drive)
    }

    #[test]
    fn next_rich_prompt_name_first_slot_is_unsuffixed() {
        let (_cfg, _root, drive) = make_drive();
        assert_eq!(next_rich_prompt_name(&drive).unwrap(), "rich-prompt");
    }

    #[test]
    fn next_rich_prompt_name_counts_up_through_gaps() {
        let (_cfg, _root, drive) = make_drive();
        drive.create_draft_dir("rich-prompt").unwrap();
        assert_eq!(next_rich_prompt_name(&drive).unwrap(), "rich-prompt-1");
        drive.create_draft_dir("rich-prompt-1").unwrap();
        assert_eq!(next_rich_prompt_name(&drive).unwrap(), "rich-prompt-2");
    }

    #[test]
    fn next_rich_prompt_name_ignores_untitled_drafts() {
        // Slice-a `untitled` drafts should not shift the
        // rich-prompt sequence: the picker filters by prefix.
        let (_cfg, _root, drive) = make_drive();
        drive.create_draft_dir("untitled").unwrap();
        drive.create_draft_dir("untitled-1").unwrap();
        assert_eq!(next_rich_prompt_name(&drive).unwrap(), "rich-prompt");
    }

    #[test]
    fn next_rich_prompt_name_fills_internal_gaps() {
        // If `rich-prompt-1` was deleted, the next pick should
        // still bump past the existing tail (matching the
        // `next_untitled_draft_name` shape, which monotonically
        // climbs rather than reusing released slots).
        let (_cfg, _root, drive) = make_drive();
        drive.create_draft_dir("rich-prompt").unwrap();
        drive.create_draft_dir("rich-prompt-2").unwrap();
        // Gap at `rich-prompt-1` is reused (picker walks from
        // 1 upward, returns the first missing slot).
        assert_eq!(next_rich_prompt_name(&drive).unwrap(), "rich-prompt-1");
    }
}
