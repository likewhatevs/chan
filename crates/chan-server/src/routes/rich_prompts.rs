//! Rich Prompt session lifecycle routes.

use std::path::Path as FsPath;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::{err, err_from, err_tunnel_public_locked};
use crate::state::AppState;
use crate::terminal_sessions::{CloseReason, WatcherStatus};

#[derive(Debug, Deserialize)]
pub struct RichPromptCreateBody {
    pub session: String,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RichPromptStatusQuery {
    pub session: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RichPromptSubmitBody {
    pub content: String,
    pub expected_sequence: u64,
    #[serde(default)]
    pub expected_mtime_ns: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RichPromptCloseBody {
    pub session: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RichPromptPhase {
    Active,
    Broken,
    Submitted,
    Discarded,
}

#[derive(Debug, Serialize)]
pub struct RichPromptResponse {
    pub phase: RichPromptPhase,
    pub name: String,
    pub draft_path: String,
    pub workspace_path: String,
    pub events_path: String,
    pub process_path: String,
    pub workspace_abs: String,
    pub events_abs: String,
    pub submission_sequence: u64,
    pub watcher: RichPromptWatcherView,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum RichPromptWatcherView {
    MissingSession,
    Detached,
    Attached {
        dir: String,
    },
    Failed {
        #[serde(skip_serializing_if = "Option::is_none")]
        dir: Option<String>,
        message: String,
    },
}

#[derive(Debug, Serialize)]
pub struct RichPromptSubmitResponse {
    pub phase: RichPromptPhase,
    pub name: String,
    pub archived_path: String,
    pub draft_path: String,
    pub submission_sequence: u64,
}

#[derive(Debug, Serialize)]
pub struct RichPromptCloseResponse {
    pub phase: RichPromptPhase,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub async fn api_create_rich_prompt_session(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RichPromptCreateBody>,
) -> Response {
    if state.tunnel_public {
        return err_tunnel_public_locked();
    }
    let session = body.session.trim().to_string();
    if session.is_empty() {
        return err(
            StatusCode::BAD_REQUEST,
            "terminal session is required".into(),
        );
    }
    if state.terminal_sessions.watcher_status(&session).is_none() {
        return err(StatusCode::NOT_FOUND, "terminal session not found".into());
    }
    let workspace = state.workspace().clone();
    let requested = body.name.clone();
    let result = tokio::task::spawn_blocking(move || {
        workspace.create_rich_prompt_session(requested.as_deref())
    })
    .await;
    let workspace = match result {
        Ok(Ok(workspace)) => workspace,
        Ok(Err(e)) => return rich_prompt_err(&e),
        Err(join) => return err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    };

    let attach = state
        .terminal_sessions
        .set_watcher(&session, workspace.events_abs.clone());
    let (watcher, phase, error) = match attach {
        Ok(true) => watcher_view(state.terminal_sessions.watcher_status(&session)),
        Ok(false) => (
            RichPromptWatcherView::MissingSession,
            RichPromptPhase::Broken,
            Some("terminal session missing".to_string()),
        ),
        Err(e) => (
            RichPromptWatcherView::Failed {
                dir: Some(workspace.events_abs.to_string_lossy().into_owned()),
                message: e.to_string(),
            },
            RichPromptPhase::Broken,
            Some(format!("failed to start terminal watcher: {e}")),
        ),
    };
    Json(rich_prompt_response(workspace, watcher, phase, error)).into_response()
}

pub async fn api_get_rich_prompt_status(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(query): Query<RichPromptStatusQuery>,
) -> Response {
    if state.tunnel_public {
        return err_tunnel_public_locked();
    }
    let workspace = state.workspace().clone();
    let inspect_name = name.clone();
    let result =
        tokio::task::spawn_blocking(move || workspace.inspect_rich_prompt_session(&inspect_name))
            .await;
    let workspace = match result {
        Ok(Ok(workspace)) => workspace,
        Ok(Err(chan_workspace::ChanError::DraftBroken { message, .. })) => {
            return Json(broken_status_response(name, query.session, message, &state))
                .into_response();
        }
        Ok(Err(e)) => return rich_prompt_err(&e),
        Err(join) => return err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    };

    let (watcher, phase, error) =
        session_status_for_rich_prompt(query.session, &state, &workspace.events_abs);
    Json(rich_prompt_response(workspace, watcher, phase, error)).into_response()
}

pub async fn api_submit_rich_prompt(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(body): Json<RichPromptSubmitBody>,
) -> Response {
    if state.tunnel_public {
        return err_tunnel_public_locked();
    }
    let workspace = state.workspace().clone();
    // Note the archived + draft paths inside the blocking task, once the
    // submit reports them and before the await returns, so the watcher's
    // events are suppressed without the post-await race (see
    // files.rs::api_write_file).
    let self_writes = Arc::clone(&state.self_writes);
    let result = tokio::task::spawn_blocking(move || {
        let report = workspace.submit_rich_prompt_session(
            &name,
            &body.content,
            body.expected_sequence,
            body.expected_mtime_ns,
        )?;
        self_writes.note(&report.archived_path);
        self_writes.note(&report.draft_path);
        Ok::<_, chan_workspace::ChanError>(report)
    })
    .await;
    match result {
        Ok(Ok(report)) => Json(RichPromptSubmitResponse {
            phase: RichPromptPhase::Submitted,
            name: report.name,
            archived_path: report.archived_path,
            draft_path: report.draft_path,
            submission_sequence: report.submission_sequence,
        })
        .into_response(),
        Ok(Err(e)) => rich_prompt_err(&e),
        Err(join) => err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    }
}

pub async fn api_close_rich_prompt(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(body): Json<RichPromptCloseBody>,
) -> Response {
    if state.tunnel_public {
        return err_tunnel_public_locked();
    }
    let session = body.session.trim().to_string();
    if !session.is_empty() {
        state.terminal_sessions.clear_watcher(&session);
    }
    let workspace = state.workspace().clone();
    let discard_name = name.clone();
    // Suppress the watcher's Removed event before the blocking discard
    // (see files.rs::api_write_file).
    state.self_writes.note(&format!("Drafts/{name}"));
    let result =
        tokio::task::spawn_blocking(move || workspace.discard_rich_prompt_session(&discard_name))
            .await;
    match result {
        Ok(Ok(())) => {
            if !session.is_empty() {
                state
                    .terminal_sessions
                    .close(&session, CloseReason::Explicit);
            }
            Json(RichPromptCloseResponse {
                phase: RichPromptPhase::Discarded,
                name,
                error: None,
            })
            .into_response()
        }
        Ok(Err(e)) => Json(RichPromptCloseResponse {
            phase: RichPromptPhase::Broken,
            name,
            error: Some(e.to_string()),
        })
        .into_response(),
        Err(join) => err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    }
}

fn rich_prompt_err(e: &chan_workspace::ChanError) -> Response {
    if let chan_workspace::ChanError::Io(msg) = e {
        let lower = msg.to_lowercase();
        if lower.contains("rich prompt name")
            || lower.contains("must use the rich-prompt prefix")
            || lower.contains("path separators")
            || lower.contains("is reserved")
        {
            return err(StatusCode::BAD_REQUEST, msg.clone());
        }
    }
    err_from(e)
}

fn session_status(
    session: Option<String>,
    state: &Arc<AppState>,
) -> (RichPromptWatcherView, RichPromptPhase, Option<String>) {
    let Some(session) = session.filter(|s| !s.trim().is_empty()) else {
        return (
            RichPromptWatcherView::MissingSession,
            RichPromptPhase::Broken,
            Some("terminal session is required".into()),
        );
    };
    watcher_view(state.terminal_sessions.watcher_status(&session))
}

fn session_status_for_rich_prompt(
    session: Option<String>,
    state: &Arc<AppState>,
    events_abs: &FsPath,
) -> (RichPromptWatcherView, RichPromptPhase, Option<String>) {
    let Some(session) = session.filter(|s| !s.trim().is_empty()) else {
        return (
            RichPromptWatcherView::MissingSession,
            RichPromptPhase::Broken,
            Some("terminal session is required".into()),
        );
    };
    let current = state.terminal_sessions.watcher_status(&session);
    match current {
        Some(WatcherStatus::Attached { ref dir }) if FsPath::new(dir) == events_abs => {
            watcher_view(current)
        }
        Some(WatcherStatus::Failed { .. }) | None => watcher_view(current),
        Some(WatcherStatus::Detached) | Some(WatcherStatus::Attached { .. }) => {
            match state
                .terminal_sessions
                .set_watcher(&session, events_abs.to_path_buf())
            {
                Ok(true) => watcher_view(state.terminal_sessions.watcher_status(&session)),
                Ok(false) => watcher_view(None),
                Err(e) => (
                    RichPromptWatcherView::Failed {
                        dir: Some(events_abs.to_string_lossy().into_owned()),
                        message: e.to_string(),
                    },
                    RichPromptPhase::Broken,
                    Some(format!("failed to start terminal watcher: {e}")),
                ),
            }
        }
    }
}

fn watcher_view(
    status: Option<WatcherStatus>,
) -> (RichPromptWatcherView, RichPromptPhase, Option<String>) {
    match status {
        None => (
            RichPromptWatcherView::MissingSession,
            RichPromptPhase::Broken,
            Some("terminal session missing".into()),
        ),
        Some(WatcherStatus::Detached) => (
            RichPromptWatcherView::Detached,
            RichPromptPhase::Broken,
            Some("terminal watcher detached".into()),
        ),
        Some(WatcherStatus::Attached { dir }) => (
            RichPromptWatcherView::Attached { dir },
            RichPromptPhase::Active,
            None,
        ),
        Some(WatcherStatus::Failed { dir, message }) => (
            RichPromptWatcherView::Failed {
                dir,
                message: message.clone(),
            },
            RichPromptPhase::Broken,
            Some(message),
        ),
    }
}

fn rich_prompt_response(
    workspace: chan_workspace::RichPromptSession,
    watcher: RichPromptWatcherView,
    phase: RichPromptPhase,
    error: Option<String>,
) -> RichPromptResponse {
    RichPromptResponse {
        phase,
        name: workspace.name,
        draft_path: workspace.draft_path,
        workspace_path: workspace.workspace_path,
        events_path: workspace.events_path,
        process_path: workspace.process_path,
        workspace_abs: workspace.workspace_abs.to_string_lossy().into_owned(),
        events_abs: workspace.events_abs.to_string_lossy().into_owned(),
        submission_sequence: workspace.submission_sequence,
        watcher,
        error,
    }
}

fn broken_status_response(
    name: String,
    session: Option<String>,
    message: String,
    state: &Arc<AppState>,
) -> RichPromptResponse {
    let (watcher, _, watcher_error) = session_status(session, state);
    let error = Some(match watcher_error {
        Some(watcher_error) => format!("{message}; {watcher_error}"),
        None => message,
    });
    RichPromptResponse {
        phase: RichPromptPhase::Broken,
        draft_path: format!("Drafts/{name}/draft.md"),
        workspace_path: format!("Drafts/{name}"),
        events_path: format!("Drafts/{name}/spool/events"),
        process_path: format!("Drafts/{name}/spool/process.md"),
        workspace_abs: String::new(),
        events_abs: String::new(),
        submission_sequence: 0,
        name,
        watcher,
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal_sessions::{CloseReason, CreateOptions};
    use portable_pty::PtySize;

    #[test]
    fn watcher_view_maps_missing_session_to_broken() {
        let (watcher, phase, error) = watcher_view(None);

        assert!(matches!(watcher, RichPromptWatcherView::MissingSession));
        assert_eq!(phase, RichPromptPhase::Broken);
        assert_eq!(error.as_deref(), Some("terminal session missing"));
    }

    #[test]
    fn watcher_view_maps_attached_to_active() {
        let (watcher, phase, error) = watcher_view(Some(WatcherStatus::Attached {
            dir: "/tmp/events".into(),
        }));

        assert!(matches!(watcher, RichPromptWatcherView::Attached { .. }));
        assert_eq!(phase, RichPromptPhase::Active);
        assert!(error.is_none());
    }

    #[test]
    fn status_refresh_reattaches_detached_rich_prompt_watcher() {
        let state = crate::state::test_support::make_test_state(false, false);
        let handle = state
            .terminal_sessions
            .create(CreateOptions {
                size: PtySize {
                    rows: 24,
                    cols: 80,
                    pixel_width: 0,
                    pixel_height: 0,
                },
                tab_name: Some("@@Architect".into()),
                window_id: None,
                mcp_env: false,
                cwd: None,
                command: Some("sleep 5".into()),
                env: Default::default(),
                preflight: None,
            })
            .expect("terminal session");
        let id = handle.id().to_string();
        let events = tempfile::tempdir().expect("events dir");

        let (watcher, phase, error) =
            session_status_for_rich_prompt(Some(id.clone()), &state, events.path());

        assert!(matches!(watcher, RichPromptWatcherView::Attached { .. }));
        assert_eq!(phase, RichPromptPhase::Active);
        assert!(error.is_none());
        assert_eq!(
            state.terminal_sessions.watcher_dir(&id),
            Some(events.path().to_path_buf())
        );
        state.terminal_sessions.close(&id, CloseReason::Explicit);
    }
}
