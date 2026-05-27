//! `/api/workspace` - workspace metadata + the cloud-workspaces detection helper.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use super::preferences::{preferences_view, PreferencesView};
use crate::error::err;
use crate::state::AppState;

#[derive(Serialize)]
struct WorkspaceInfo {
    /// Absolute workspace root, POSIX-style on every platform so the
    /// JSON shape stays stable. Empty string on `--tunnel-public`
    /// runs: the absolute path of the owner's workspace would otherwise
    /// reveal the owner's username and filesystem layout to every
    /// anonymous visitor.
    root: String,
    /// Path-derived label for compact UI surfaces. It is not stored
    /// in the registry and cannot be edited through `/api/workspace`.
    label: Option<String>,
    /// Stable metadata storage key under `~/.chan/workspaces/`.
    metadata_key: Option<String>,
    /// Per-device preferences view. The frontend uses this to seed
    /// the editor (fonts, theme, line spacing) without a follow-up
    /// /api/config round-trip. Same shape as
    /// `GlobalConfig.preferences`; assembled by joining EditorPrefs
    /// and ServerConfig.
    preferences: PreferencesView,
    /// Non-fatal workspace boot warnings. Empty on healthy workspaces.
    warnings: Vec<WorkspaceWarning>,
}

#[derive(Serialize)]
struct WorkspaceWarning {
    kind: &'static str,
    path: String,
    message: String,
}

pub async fn api_get_workspace(State(state): State<Arc<AppState>>) -> Response {
    drive_info_response(state, "workspace info").await
}

async fn drive_info_response(state: Arc<AppState>, label: &'static str) -> Response {
    let result = tokio::task::spawn_blocking(move || drive_info(&state)).await;
    match result {
        Ok(Ok(info)) => Json(info).into_response(),
        Ok(Err(message)) => err(StatusCode::INTERNAL_SERVER_ERROR, message),
        Err(e) => err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("{label} task panicked: {e}"),
        ),
    }
}

/// `GET /api/workspace/bootstrap` - the structural spine the SPA renders
/// before any index / report job runs. Stat-only filtered walk of the
/// workspace root: immediate files + directories, each directory carrying
/// its recursive subtree file count and byte total, plus the
/// whole-workspace aggregate. Deeper levels load lazily via the existing
/// `/api/files?dir=` path on File Browser expand / Graph depth.
///
/// Runs on the blocking pool: the walk is synchronous filesystem I/O
/// and must not block the async runtime (a large workspace is a non-
/// trivial stat sweep).
pub async fn api_workspace_bootstrap(State(state): State<Arc<AppState>>) -> Response {
    let workspace = match state.try_drive() {
        Ok(workspace) => workspace,
        Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    match tokio::task::spawn_blocking(move || workspace.bootstrap()).await {
        Ok(Ok(tree)) => Json(tree).into_response(),
        Ok(Err(e)) => err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        Err(e) => err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("bootstrap task panicked: {e}"),
        ),
    }
}

pub async fn api_patch_workspace(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    // Kept as a compatibility endpoint while the frontend drops its
    // former workspace-name editor. Local workspace names are no longer a
    // mutable registry field.
    if body.get("name").is_some() {
        return (StatusCode::BAD_REQUEST, "workspace names are not supported").into_response();
    }
    drive_info_response(state, "workspace patch").await
}

#[derive(Serialize)]
struct CloudDriveJson {
    provider: String,
    provider_root: String,
    suggested_root: String,
}

pub async fn api_cloud_workspaces(State(state): State<Arc<AppState>>) -> Response {
    // The detection walks the owner's home dir for Dropbox / iCloud
    // / Google Drive / OneDrive locations. The result reveals which
    // cloud providers the owner is signed into and the absolute
    // paths of their sync roots. Anonymous visitors get an empty
    // list; the SPA's "register a workspace" picker is unreachable
    // anyway when Settings is locked, so the only consumer is the
    // owner running locally.
    if state.tunnel_public {
        return Json(Vec::<CloudDriveJson>::new()).into_response();
    }
    match tokio::task::spawn_blocking(move || {
        let out: Vec<CloudDriveJson> = chan_workspace::paths::detected_cloud_drives()
            .into_iter()
            .map(|c| CloudDriveJson {
                provider: c.provider,
                provider_root: c.provider_root.to_string_lossy().into_owned(),
                suggested_root: c.suggested_root.to_string_lossy().into_owned(),
            })
            .collect();
        Json(out).into_response()
    })
    .await
    {
        Ok(response) => response,
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("cloud workspaces task panicked: {e}"),
        )
            .into_response(),
    }
}

/// Build a `WorkspaceInfo` from current registry state.
///
/// `root` is blanked on `--tunnel-public` runs so the owner's
/// absolute filesystem path does not leak to anonymous visitors.
/// The SPA tolerates an empty `root`: it only uses the field for
/// the Settings panel's "Workspace root" line, which is unreachable
/// in tunnel mode anyway.
fn drive_info(state: &AppState) -> Result<WorkspaceInfo, String> {
    let workspaces = state.library.list_workspaces();
    // Snapshot the live workspace once: each call to `state.workspace()`
    // takes the `drive_cell` RwLock and clones the Arc. Two calls
    // worked fine; one call reads slightly cleaner and survives a
    // hypothetical reset-in-flight where the cell could swap
    // between the registry lookup and the path serialization.
    let workspace = state.workspace();
    let drive_root = workspace.root();
    let entry = workspaces
        .iter()
        .find(|d| d.root_path.as_path() == drive_root);
    let root = if state.tunnel_public {
        String::new()
    } else {
        drive_root.to_string_lossy().into_owned()
    };
    Ok(WorkspaceInfo {
        root,
        label: entry
            .and_then(|e| e.root_path.file_name())
            .and_then(|name| name.to_str())
            .map(str::to_string),
        metadata_key: entry.map(|e| e.metadata_key.clone()),
        preferences: preferences_view(state).map_err(|e| e.to_string())?,
        warnings: drive_warnings(&workspace),
    })
}

fn drive_warnings(workspace: &chan_workspace::Workspace) -> Vec<WorkspaceWarning> {
    let mut warnings = match workspace.draft_preflight() {
        Ok(issues) => issues
            .into_iter()
            .map(|issue| WorkspaceWarning {
                kind: "broken_draft",
                path: format!("Drafts/{}", issue.name),
                message: issue.message,
            })
            .collect(),
        Err(e) => vec![WorkspaceWarning {
            kind: "draft_preflight_failed",
            path: "Drafts".to_string(),
            message: e.to_string(),
        }],
    };
    match workspace.rich_prompt_preflight() {
        Ok(issues) => warnings.extend(issues.into_iter().map(|issue| WorkspaceWarning {
            kind: "broken_rich_prompt",
            path: format!("Drafts/{}", issue.name),
            message: issue.message,
        })),
        Err(e) => warnings.push(WorkspaceWarning {
            kind: "rich_prompt_preflight_failed",
            path: "Drafts".to_string(),
            message: e.to_string(),
        }),
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::drive_warnings;

    #[test]
    fn drive_warnings_report_broken_drafts() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        let draft = workspace.create_draft_dir("untitled-1").unwrap();
        std::fs::write(draft.abs.join("note.md"), "not draft.md").unwrap();

        let warnings = drive_warnings(&workspace);

        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].kind, "broken_draft");
        assert_eq!(warnings[0].path, "Drafts/untitled-1");
        assert_eq!(warnings[0].message, "missing draft.md");
    }
}
