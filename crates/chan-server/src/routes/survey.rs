//! Survey reply route + the `[F]` followup-file generator (the @@LaneC
//! side of `cs terminal survey`).
//!
//! A `cs terminal survey` call blocks in the control socket on a oneshot
//! parked in the [`crate::survey::SurveyBus`] (D's side) keyed by a
//! server-minted `survey_id`. The SPA renders the overlay and, when the user
//! answers, POSTs a [`SurveyReplyRequest`] here. This route turns that into a
//! [`chan_shell::SurveyReply`] and calls [`SurveyBus::complete_survey`], which
//! fires the oneshot and unblocks the CLI. Two halves of one stable
//! `complete_survey` API keep the C<->D seam off a shared file
//! (round-3-survey-contract.md + its 2026-06-01 followup amendment).
//!
//! On `[F]` the SPA cannot know the minted followup path, so it echoes back
//! the `followup { dir, from, to }` context the survey carried; THIS route
//! creates `{dir}/followups/followup-{from}-{to}-{n}.md` through the Workspace
//! sandbox and replies that path to the bus.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chan_shell::SurveyReply;
use chan_workspace::Workspace;
use serde::Deserialize;

use crate::error::err;
use crate::state::AppState;

/// Body of `POST /api/survey/reply`. Internally tagged on `kind`, camelCase
/// to match the SPA (`web/src/api/client.ts` `SurveyReplyRequest`). Distinct
/// from [`chan_shell::SurveyReply`]: for a followup the SPA sends the echoed
/// context (it cannot know the minted path), and this route synthesizes the
/// path before completing the bus oneshot.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SurveyReplyRequest {
    #[serde(rename = "option", rename_all = "camelCase")]
    Option {
        survey_id: String,
        option_index: u32,
        option_label: String,
    },
    #[serde(rename = "followup", rename_all = "camelCase")]
    Followup {
        survey_id: String,
        followup: FollowupContext,
        #[serde(default)]
        title: Option<String>,
        body_markdown: String,
    },
}

/// Team context for a `[F]` followup, originating with the surveying agent
/// (it knows its team-dir + `$CHAN_TAB_NAME`), carried command -> SurveySpec
/// -> SPA -> echoed here. camelCase single words match 1:1.
#[derive(Deserialize)]
pub struct FollowupContext {
    pub dir: String,
    pub from: String,
    pub to: String,
}

/// `POST /api/survey/reply` - complete a parked `cs terminal survey`. On
/// "option" the chosen label round-trips straight to the blocked CLI; on
/// "followup" this creates the followup file first, then replies its path.
/// 404 when no survey with that id is parked (already answered / stale id).
pub async fn api_survey_reply(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SurveyReplyRequest>,
) -> Response {
    let reply = match req {
        SurveyReplyRequest::Option {
            survey_id,
            option_index,
            option_label,
        } => SurveyReply::Option {
            survey_id,
            option_index,
            option_label,
        },
        SurveyReplyRequest::Followup {
            survey_id,
            followup,
            title,
            body_markdown,
        } => {
            // Workspace I/O is blocking; create the file off the async runtime.
            let workspace = state.workspace();
            let result = tokio::task::spawn_blocking(move || {
                create_followup_file(
                    &workspace,
                    &followup.dir,
                    &followup.from,
                    &followup.to,
                    title.as_deref(),
                    &body_markdown,
                )
            })
            .await;
            match result {
                Ok(Ok(path)) => SurveyReply::Followup {
                    survey_id,
                    followup_path: path,
                },
                Ok(Err(msg)) => return err(StatusCode::BAD_REQUEST, msg),
                Err(join) => return err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
            }
        }
    };

    let survey_id = reply.survey_id().to_string();
    if state.survey_bus.complete_survey(&survey_id, reply) {
        Json(serde_json::json!({})).into_response()
    } else {
        err(
            StatusCode::NOT_FOUND,
            format!("no survey parked with id {survey_id} (already answered or stale)"),
        )
    }
}

/// Create `{dir}/followups/followup-{from}-{to}-{n}.md` through the Workspace
/// sandbox, pre-populated per the plan, and return the workspace-relative
/// path. `n` is the next free index for that from/to pair; from/to are bare
/// (the `@@` prefix + non-filename chars are stripped) so the filename stays
/// clean, while the file body keeps the handles as given.
pub fn create_followup_file(
    workspace: &Workspace,
    dir: &str,
    from: &str,
    to: &str,
    title: Option<&str>,
    body: &str,
) -> Result<String, String> {
    let dir = dir.trim().trim_end_matches('/');
    if dir.is_empty() {
        return Err("followup dir is required".into());
    }
    if dir.starts_with('/') {
        return Err(format!("followup dir must be workspace-relative: {dir}"));
    }
    let followups_dir = format!("{dir}/followups");
    workspace
        .create_dir(&followups_dir)
        .map_err(|e| format!("cannot create {followups_dir}: {e}"))?;

    let bare_from = sanitize_handle(from);
    let bare_to = sanitize_handle(to);
    let n = next_followup_index(workspace, &followups_dir, &bare_from, &bare_to);
    let rel = format!("{followups_dir}/followup-{bare_from}-{bare_to}-{n}.md");

    let content = render_followup(title, from, to, body);
    workspace
        .write_text(&rel, &content)
        .map_err(|e| format!("cannot write {rel}: {e}"))?;
    Ok(rel)
}

/// Strip a leading `@@` and replace any char that is not ASCII alphanumeric,
/// `-`, or `_` with `-`, so a handle is a safe filename segment. Falls back
/// to "x" when nothing usable remains.
fn sanitize_handle(handle: &str) -> String {
    let trimmed = handle.trim().trim_start_matches("@@");
    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "x".to_string()
    } else {
        trimmed
    }
}

/// Next free `n` for `followup-{from}-{to}-{n}.md` in `followups_dir`: one
/// past the highest existing index, or 1 when none exist. A missing dir or a
/// listing error reads as empty (the create_dir above just made it).
fn next_followup_index(
    workspace: &Workspace,
    followups_dir: &str,
    bare_from: &str,
    bare_to: &str,
) -> u32 {
    let prefix = format!("followup-{bare_from}-{bare_to}-");
    let entries = match workspace.list(followups_dir) {
        Ok(entries) => entries,
        Err(_) => return 1,
    };
    let max = entries
        .iter()
        .filter(|e| !e.is_dir)
        .filter_map(|e| {
            let stem = e.name.strip_suffix(".md")?;
            let num = stem.strip_prefix(&prefix)?;
            num.parse::<u32>().ok()
        })
        .max()
        .unwrap_or(0);
    max + 1
}

/// The pre-populated followup body: title heading, created/from/to header,
/// the "not ready, check later" line agents key off, the original survey
/// prompt, and a comment placeholder for `to` (the host) to fill in the
/// decision. ASCII only, no em dashes.
fn render_followup(title: Option<&str>, from: &str, to: &str, body: &str) -> String {
    let heading = title
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .unwrap_or("survey");
    let created = chrono::Utc::now().to_rfc3339();
    let mut out = String::new();
    out.push_str(&format!("# Follow up: {heading}\n\n"));
    out.push_str(&format!("Created: {created}\n"));
    out.push_str(&format!("From: {from}\n"));
    out.push_str(&format!("To: {to}\n\n"));
    out.push_str("Agents: this is a follow up, not ready; check again later.\n\n");
    out.push_str("## Original prompt\n\n");
    out.push_str(body.trim_end());
    out.push_str("\n\n");
    out.push_str(&format!("## {to} comments\n\n"));
    out.push_str(&format!(
        "<!-- {to}: leave your decision here; the agent re-reads this file. -->\n"
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_workspace() -> (
        tempfile::TempDir,
        tempfile::TempDir,
        Arc<chan_workspace::Workspace>,
    ) {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        (cfg, root, workspace)
    }

    #[test]
    fn sanitize_strips_at_prefix_and_unsafe_chars() {
        assert_eq!(sanitize_handle("@@LaneC"), "LaneC");
        assert_eq!(sanitize_handle("@@Host"), "Host");
        assert_eq!(sanitize_handle("a b/c"), "a-b-c");
        assert_eq!(sanitize_handle("@@"), "x");
    }

    #[test]
    fn create_followup_writes_inside_team_dir_and_increments() {
        let (_cfg, _root, workspace) = test_workspace();

        let p1 = create_followup_file(
            &workspace,
            "new-team-1",
            "@@LaneC",
            "@@Host",
            Some("Pick a search backend"),
            "Should search use BM25 or semantic?",
        )
        .unwrap();
        assert_eq!(p1, "new-team-1/followups/followup-LaneC-Host-1.md");
        assert!(workspace.exists(&p1));

        // Same from/to pair increments n; the dir already exists.
        let p2 = create_followup_file(
            &workspace,
            "new-team-1",
            "@@LaneC",
            "@@Host",
            None,
            "Second question",
        )
        .unwrap();
        assert_eq!(p2, "new-team-1/followups/followup-LaneC-Host-2.md");

        // A different from/to pair restarts at 1.
        let p3 =
            create_followup_file(&workspace, "new-team-1", "@@LaneB", "@@Host", None, "x").unwrap();
        assert_eq!(p3, "new-team-1/followups/followup-LaneB-Host-1.md");
    }

    #[test]
    fn followup_body_carries_prompt_and_placeholders() {
        let (_cfg, _root, workspace) = test_workspace();
        let path = create_followup_file(
            &workspace,
            "team",
            "@@LaneC",
            "@@Host",
            Some("Backend choice"),
            "BM25 or semantic?",
        )
        .unwrap();
        let text = workspace.read_text(&path).unwrap();

        assert!(text.contains("# Follow up: Backend choice"));
        assert!(text.contains("From: @@LaneC"));
        assert!(text.contains("To: @@Host"));
        assert!(text.contains("Agents: this is a follow up, not ready; check again later."));
        assert!(text.contains("## Original prompt"));
        assert!(text.contains("BM25 or semantic?"));
        assert!(text.contains("## @@Host comments"));
        assert!(text.contains("Created: "), "an ISO timestamp line");
        assert!(!text.contains('\u{2014}'), "no em dashes");
    }

    #[test]
    fn missing_title_falls_back_to_survey_heading() {
        let (_cfg, _root, workspace) = test_workspace();
        let path = create_followup_file(&workspace, "team", "@@A", "@@B", None, "body").unwrap();
        let text = workspace.read_text(&path).unwrap();
        assert!(text.contains("# Follow up: survey"));
    }

    #[test]
    fn empty_dir_is_rejected() {
        let (_cfg, _root, workspace) = test_workspace();
        assert!(create_followup_file(&workspace, "  ", "@@A", "@@B", None, "x").is_err());
    }

    #[test]
    fn option_reply_request_deserializes_camel_case() {
        let json = r#"{"surveyId":"survey-3","kind":"option","optionIndex":2,"optionLabel":"Yes"}"#;
        let req: SurveyReplyRequest = serde_json::from_str(json).unwrap();
        match req {
            SurveyReplyRequest::Option {
                survey_id,
                option_index,
                option_label,
            } => {
                assert_eq!(survey_id, "survey-3");
                assert_eq!(option_index, 2);
                assert_eq!(option_label, "Yes");
            }
            _ => panic!("expected option variant"),
        }
    }

    #[test]
    fn followup_reply_request_deserializes_with_context() {
        let json = r#"{"surveyId":"survey-9","kind":"followup",
            "followup":{"dir":"new-team-1","from":"@@LaneC","to":"@@Host"},
            "title":"T","bodyMarkdown":"the question"}"#;
        let req: SurveyReplyRequest = serde_json::from_str(json).unwrap();
        match req {
            SurveyReplyRequest::Followup {
                survey_id,
                followup,
                title,
                body_markdown,
            } => {
                assert_eq!(survey_id, "survey-9");
                assert_eq!(followup.dir, "new-team-1");
                assert_eq!(followup.from, "@@LaneC");
                assert_eq!(followup.to, "@@Host");
                assert_eq!(title.as_deref(), Some("T"));
                assert_eq!(body_markdown, "the question");
            }
            _ => panic!("expected followup variant"),
        }
    }
}
