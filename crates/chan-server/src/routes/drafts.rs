//! `fullstack-a-66` Drafts route.
//!
//! * `POST /api/drafts/new` — slice a (Cmd+N from SPA). Creates
//!   `Drafts/<next-untitled>/draft.md` + indexes it + returns
//!   the unified-path.
//!
//! Drafts live in chan-workspace metadata (`drafts_dir()`), OUTSIDE
//! the workspace root, but appear in the wire under the `Drafts/`
//! prefix per the keyspace `systacean-25` + `-26` unified.
//! `Workspace::create_draft_dir`, `next_untitled_draft_name`,
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

const NEW_DRAFT_CONTENT: &str = "# Draft\n";

#[derive(Deserialize)]
pub struct DraftPathPayload {
    /// Any unified path inside the draft workspace, usually
    /// `Drafts/<name>/draft.md`.
    pub path: String,
}

#[derive(Deserialize)]
pub struct DraftPromotePayload {
    /// Any unified path inside the draft workspace.
    pub path: String,
    /// Workspace-relative destination. Single-file drafts save to this
    /// file; workspace drafts save to this directory.
    pub target: String,
}

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

#[derive(Serialize, PartialEq, Eq, Debug)]
pub struct DraftInspectResponse {
    pub path: String,
    pub name: String,
    pub file_count: usize,
    pub dir_count: usize,
    pub total_size: u64,
    pub has_attachments: bool,
}

#[derive(Serialize, PartialEq, Eq, Debug)]
pub struct DraftPromoteResponse {
    pub path: String,
    pub name: String,
    pub mode: &'static str,
}

/// Create a fresh draft directory + a seeded `draft.md` inside.
///
/// Race-window note: `next_untitled_draft_name` + `create_draft_dir`
/// can race against another concurrent creator; if `create_draft_dir`
/// returns `AlreadyExists` we retry once with a re-resolved name.
/// The race is rare in practice (single-user / single-machine) but
/// the retry keeps the contract clean.
pub async fn api_create_draft(State(state): State<Arc<AppState>>) -> Response {
    let workspace = state.workspace().clone();
    // Note the draft path inside the blocking task, before it returns to
    // the await, so the watcher's Created event for our own draft is
    // suppressed without the post-await race (see files.rs::api_write_file).
    let self_writes = Arc::clone(&state.self_writes);
    let result = tokio::task::spawn_blocking(move || {
        let name = create_draft_sync(&workspace)?;
        self_writes.note(&format!("Drafts/{name}/draft.md"));
        Ok::<_, chan_workspace::ChanError>(name)
    })
    .await;

    let name = match result {
        Ok(Ok(name)) => name,
        Ok(Err(e)) => return err_from(&e),
        Err(join) => return err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    };

    let path = format!("Drafts/{name}/draft.md");
    Json(DraftCreateResponse { path, name }).into_response()
}

fn create_draft_sync(
    workspace: &chan_workspace::Workspace,
) -> Result<String, chan_workspace::ChanError> {
    for _ in 0..2 {
        let name = workspace.next_untitled_draft_name()?;
        match workspace.create_draft_dir(&name) {
            Ok(_) => {
                let unified = format!("Drafts/{name}/draft.md");
                workspace.write_text(&unified, NEW_DRAFT_CONTENT)?;
                return Ok(name);
            }
            Err(chan_workspace::ChanError::Io(msg)) if msg.contains("already exists") => {
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    Err(chan_workspace::ChanError::Io(
        "race condition picking next untitled draft name (retried 2x)".to_string(),
    ))
}

pub async fn api_inspect_draft(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DraftPathPayload>,
) -> Response {
    let workspace = state.workspace().clone();
    let result =
        tokio::task::spawn_blocking(move || inspect_draft_sync(&workspace, &payload.path)).await;

    match result {
        Ok(Ok(out)) => Json(out).into_response(),
        Ok(Err(e)) => err_from(&e),
        Err(join) => err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    }
}

pub async fn api_discard_draft(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DraftPathPayload>,
) -> Response {
    let workspace = state.workspace().clone();
    let path = payload.path.clone();
    // Suppress the watcher's Removed event before the blocking discard
    // (see files.rs::api_write_file).
    state.self_writes.note(&path);
    let result =
        tokio::task::spawn_blocking(move || discard_draft_sync(&workspace, &payload.path)).await;

    match result {
        Ok(Ok(())) => StatusCode::NO_CONTENT.into_response(),
        Ok(Err(e)) => err_from(&e),
        Err(join) => err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    }
}

pub async fn api_promote_draft(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DraftPromotePayload>,
) -> Response {
    let workspace = state.workspace().clone();
    let source_path = payload.path.clone();
    let target_path = payload.target.clone();
    // Suppress the discard-at-source + create-at-target events before
    // the blocking promote (see files.rs::api_write_file).
    state.self_writes.note(&source_path);
    state.self_writes.note(&target_path);
    let result = tokio::task::spawn_blocking(move || {
        promote_draft_sync(&workspace, &payload.path, &payload.target)
    })
    .await;

    match result {
        Ok(Ok(out)) => Json(out).into_response(),
        Ok(Err(e)) => err_from(&e),
        Err(join) => err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    }
}

fn inspect_draft_sync(
    workspace: &chan_workspace::Workspace,
    path: &str,
) -> Result<DraftInspectResponse, chan_workspace::ChanError> {
    let name = chan_workspace::drafts::name_from_unified_path(path)?;
    let info = workspace.inspect_draft(&name)?;
    Ok(DraftInspectResponse {
        path: format!("Drafts/{name}/draft.md"),
        name,
        file_count: info.file_count,
        dir_count: info.dir_count,
        total_size: info.total_size,
        has_attachments: info.has_attachments,
    })
}

fn discard_draft_sync(
    workspace: &chan_workspace::Workspace,
    path: &str,
) -> Result<(), chan_workspace::ChanError> {
    let name = chan_workspace::drafts::name_from_unified_path(path)?;
    workspace.discard_draft(&name)
}

fn promote_draft_sync(
    workspace: &chan_workspace::Workspace,
    path: &str,
    target: &str,
) -> Result<DraftPromoteResponse, chan_workspace::ChanError> {
    let name = chan_workspace::drafts::name_from_unified_path(path)?;
    let report = workspace.promote_draft(&name, target)?;
    Ok(DraftPromoteResponse {
        path: report.target_path,
        name: report.name,
        mode: promote_mode_label(report.mode),
    })
}

fn promote_mode_label(mode: chan_workspace::DraftPromoteMode) -> &'static str {
    match mode {
        chan_workspace::DraftPromoteMode::File => "file",
        chan_workspace::DraftPromoteMode::DirectoryCreated => "directory_created",
        chan_workspace::DraftPromoteMode::DirectoryMerged => "directory_merged",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_workspace() -> (TempDir, TempDir, std::sync::Arc<chan_workspace::Workspace>) {
        let cfg = TempDir::new().unwrap();
        let root = TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        (cfg, root, workspace)
    }

    #[test]
    fn create_draft_sync_seeds_title() {
        let (_cfg, _root, workspace) = make_workspace();

        let name = create_draft_sync(&workspace).unwrap();
        let path = format!("Drafts/{name}/draft.md");

        assert_eq!(name, "untitled");
        assert_eq!(workspace.read_text(&path).unwrap(), NEW_DRAFT_CONTENT);
    }

    #[test]
    fn inspect_draft_sync_reports_workspace_shape() {
        let (_cfg, _root, workspace) = make_workspace();
        workspace.create_draft_dir("untitled-1").unwrap();
        workspace
            .write_text("Drafts/untitled-1/draft.md", "# draft\n")
            .unwrap();
        workspace
            .write_bytes("Drafts/untitled-1/pasted.png", &[1, 2, 3])
            .unwrap();

        let out = inspect_draft_sync(&workspace, "Drafts/untitled-1/draft.md").unwrap();

        assert_eq!(out.name, "untitled-1");
        assert_eq!(out.path, "Drafts/untitled-1/draft.md");
        assert_eq!(out.file_count, 2);
        assert!(out.has_attachments);
    }

    #[test]
    fn promote_draft_sync_returns_target_path_and_mode() {
        let (_cfg, root, workspace) = make_workspace();
        std::fs::create_dir_all(root.path().join("notes")).unwrap();
        workspace.create_draft_dir("untitled-1").unwrap();
        workspace
            .write_text("Drafts/untitled-1/draft.md", "# draft\n")
            .unwrap();

        let out =
            promote_draft_sync(&workspace, "Drafts/untitled-1/draft.md", "notes/draft.md").unwrap();

        assert_eq!(out.name, "untitled-1");
        assert_eq!(out.path, "notes/draft.md");
        assert_eq!(out.mode, "file");
        assert_eq!(
            std::fs::read_to_string(root.path().join("notes/draft.md")).unwrap(),
            "# draft\n"
        );
    }

    #[test]
    fn discard_draft_sync_removes_workspace() {
        let (_cfg, _root, workspace) = make_workspace();
        workspace.create_draft_dir("untitled-1").unwrap();
        workspace
            .write_text("Drafts/untitled-1/draft.md", "# draft\n")
            .unwrap();

        discard_draft_sync(&workspace, "Drafts/untitled-1/draft.md").unwrap();

        assert!(!workspace.drafts_dir().join("untitled-1").exists());
    }
}
