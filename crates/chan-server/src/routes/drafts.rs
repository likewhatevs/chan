//! Drafts route.
//!
//! * `POST /api/drafts/new` — Cmd+N from the SPA. Creates
//!   `<drafts_dir>/<next-untitled>/draft.md` + indexes it + returns
//!   the real in-root path.
//!
//! Drafts are real in-root files under the configured drafts
//! directory (default `.Drafts`), named by `Workspace::drafts_dir_name`.
//! Public paths are plain relpaths like `.Drafts/<name>/draft.md`, so
//! `create_draft_dir`, `next_untitled_draft_name`, and `write_text`
//! route through the normal workspace path machinery with no special
//! casing.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::{err, err_from};
use crate::state::AppState;

const NEW_DRAFT_CONTENT: &str = "# Draft\n";

/// Seed for a brand-new diagram: a valid, non-empty Excalidraw scene so
/// the board opens cleanly and is not treated as an empty file that
/// auto-discards on close. `ExcalidrawCanvas` parses it as an empty
/// board. The frontend mirrors this exact string as its diagram seed so
/// a never-drawn board still discards silently on close.
const NEW_DIAGRAM_CONTENT: &str =
    r#"{"type":"excalidraw","version":2,"source":"chan","elements":[],"appState":{},"files":{}}"#;

/// Extract the draft leaf name from a draft public path.
///
/// A draft path is `<drafts_dir>/<name>/...`, so strip the configured
/// `<drafts_dir>/` prefix and take the first path segment. Errors when
/// the path is not under the drafts directory or carries no leaf.
fn draft_name_from_path(
    workspace: &chan_workspace::Workspace,
    path: &str,
) -> Result<String, chan_workspace::ChanError> {
    let dir = workspace.drafts_dir_name();
    let trimmed = path.trim_matches('/');
    let rest = trimmed
        .strip_prefix(dir)
        .and_then(|r| r.strip_prefix('/'))
        .ok_or_else(|| {
            chan_workspace::ChanError::Io(format!(
                "path `{path}` is not under the drafts directory `{dir}`"
            ))
        })?;
    let name = rest.split('/').next().unwrap_or("");
    if name.is_empty() {
        return Err(chan_workspace::ChanError::Io(format!(
            "path `{path}` carries no draft name under `{dir}`"
        )));
    }
    Ok(name.to_string())
}

#[derive(Deserialize)]
pub struct DraftPathPayload {
    /// Any path inside the draft directory, usually
    /// `<drafts_dir>/<name>/draft.md`.
    pub path: String,
}

#[derive(Deserialize)]
pub struct DraftPromotePayload {
    /// Any path inside the draft directory.
    pub path: String,
    /// Workspace-relative destination. Single-file drafts save to this
    /// file; workspace drafts save to this directory.
    pub target: String,
}

#[derive(Serialize)]
pub struct DraftCreateResponse {
    /// In-root path for the new draft.md: `<drafts_dir>/<name>/draft.md`.
    /// SPA `openInActivePane(path)` routes through
    /// `/api/files/<drafts_dir>/<name>/draft.md`, a normal in-root read.
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
        self_writes.note(&format!("{}/{name}/draft.md", workspace.drafts_dir_name()));
        Ok::<_, chan_workspace::ChanError>((name, workspace.drafts_dir_name().to_string()))
    })
    .await;

    let (name, dir) = match result {
        Ok(Ok(pair)) => pair,
        Ok(Err(e)) => return err_from(&e),
        Err(join) => return err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    };

    let path = format!("{dir}/{name}/draft.md");
    Json(DraftCreateResponse { path, name }).into_response()
}

/// Create a fresh draft directory + a seeded `<name>.excalidraw` board
/// inside, mirroring `api_create_draft`. The diagram is a real draft
/// (promotable + discardable) whose primary file is the Excalidraw
/// scene rather than `draft.md`.
pub async fn api_create_diagram(State(state): State<Arc<AppState>>) -> Response {
    let workspace = state.workspace().clone();
    // Note the diagram path inside the blocking task, before it returns
    // to the await, so the watcher's Created event for our own write is
    // suppressed without the post-await race (see files.rs::api_write_file).
    let self_writes = Arc::clone(&state.self_writes);
    let result = tokio::task::spawn_blocking(move || {
        let (name, path) = create_diagram_sync(&workspace)?;
        self_writes.note(&path);
        Ok::<_, chan_workspace::ChanError>((name, path))
    })
    .await;

    let (name, path) = match result {
        Ok(Ok(pair)) => pair,
        Ok(Err(e)) => return err_from(&e),
        Err(join) => return err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    };

    Json(DraftCreateResponse { path, name }).into_response()
}

fn create_diagram_sync(
    workspace: &chan_workspace::Workspace,
) -> Result<(String, String), chan_workspace::ChanError> {
    for _ in 0..2 {
        let name = workspace.next_untitled_draft_name()?;
        match workspace.create_draft_dir(&name) {
            Ok(_) => {
                let path = format!("{}/{name}/{name}.excalidraw", workspace.drafts_dir_name());
                workspace.write_text(&path, NEW_DIAGRAM_CONTENT)?;
                return Ok((name, path));
            }
            Err(chan_workspace::ChanError::Io(msg)) if msg.contains("already exists") => {
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    Err(chan_workspace::ChanError::Io(
        "race condition picking next untitled diagram name (retried 2x)".to_string(),
    ))
}

fn create_draft_sync(
    workspace: &chan_workspace::Workspace,
) -> Result<String, chan_workspace::ChanError> {
    for _ in 0..2 {
        let name = workspace.next_untitled_draft_name()?;
        match workspace.create_draft_dir(&name) {
            Ok(_) => {
                let path = format!("{}/{name}/draft.md", workspace.drafts_dir_name());
                workspace.write_text(&path, NEW_DRAFT_CONTENT)?;
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
    let name = draft_name_from_path(workspace, path)?;
    let info = workspace.inspect_draft(&name)?;
    Ok(DraftInspectResponse {
        path: format!("{}/{name}/draft.md", workspace.drafts_dir_name()),
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
    let name = draft_name_from_path(workspace, path)?;
    workspace.discard_draft(&name)
}

fn promote_draft_sync(
    workspace: &chan_workspace::Workspace,
    path: &str,
    target: &str,
) -> Result<DraftPromoteResponse, chan_workspace::ChanError> {
    let name = draft_name_from_path(workspace, path)?;
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
        let path = format!(".Drafts/{name}/draft.md");

        assert_eq!(name, "untitled");
        assert_eq!(workspace.read_text(&path).unwrap(), NEW_DRAFT_CONTENT);
    }

    #[test]
    fn create_diagram_sync_seeds_a_valid_board_that_inspects_and_promotes() {
        let (_cfg, root, workspace) = make_workspace();

        let (name, path) = create_diagram_sync(&workspace).unwrap();

        assert_eq!(name, "untitled");
        assert_eq!(path, ".Drafts/untitled/untitled.excalidraw");

        // The seed is non-empty valid JSON and classifies as editable
        // text, so the editor opens it as a board.
        let content = workspace.read_text(&path).unwrap();
        assert_eq!(content, NEW_DIAGRAM_CONTENT);
        assert!(!content.is_empty());
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["type"], "excalidraw");
        assert_eq!(
            chan_workspace::fs_ops::classify(&path),
            chan_workspace::FileClass::Text
        );

        // It is a real single-file draft: inspects cleanly (no
        // "missing draft.md" broken) and promotes to an .excalidraw file.
        let info = workspace.inspect_draft(&name).unwrap();
        assert!(!info.has_attachments);
        std::fs::create_dir_all(root.path().join("boards")).unwrap();
        let promoted = workspace
            .promote_draft(&name, "boards/diagram.excalidraw")
            .unwrap();
        assert_eq!(promoted.target_path, "boards/diagram.excalidraw");
        assert_eq!(
            std::fs::read_to_string(root.path().join("boards/diagram.excalidraw")).unwrap(),
            NEW_DIAGRAM_CONTENT
        );
    }

    #[test]
    fn inspect_draft_sync_reports_workspace_shape() {
        let (_cfg, _root, workspace) = make_workspace();
        workspace.create_draft_dir("untitled-1").unwrap();
        workspace
            .write_text(".Drafts/untitled-1/draft.md", "# draft\n")
            .unwrap();
        workspace
            .write_bytes(".Drafts/untitled-1/pasted.png", &[1, 2, 3])
            .unwrap();

        let out = inspect_draft_sync(&workspace, ".Drafts/untitled-1/draft.md").unwrap();

        assert_eq!(out.name, "untitled-1");
        assert_eq!(out.path, ".Drafts/untitled-1/draft.md");
        assert_eq!(out.file_count, 2);
        assert!(out.has_attachments);
    }

    #[test]
    fn promote_draft_sync_returns_target_path_and_mode() {
        let (_cfg, root, workspace) = make_workspace();
        std::fs::create_dir_all(root.path().join("notes")).unwrap();
        workspace.create_draft_dir("untitled-1").unwrap();
        workspace
            .write_text(".Drafts/untitled-1/draft.md", "# draft\n")
            .unwrap();

        let out = promote_draft_sync(&workspace, ".Drafts/untitled-1/draft.md", "notes/draft.md")
            .unwrap();

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
            .write_text(".Drafts/untitled-1/draft.md", "# draft\n")
            .unwrap();

        discard_draft_sync(&workspace, ".Drafts/untitled-1/draft.md").unwrap();

        assert!(!workspace.drafts_dir().join("untitled-1").exists());
    }

    // ---- Draft-banner backend stress test -----------------------------
    //
    // The false "unsaved changes from a previous session" banner
    // is a frontend bug, but it
    // can only be exercised cleanly if the backend invariants it stands
    // on hold under load. This is the backend half of the e2e stress
    // test: it hammers create-draft -> autosave (CAS) -> re-read over
    // many iterations against the REAL self-write suppression
    // (self_writes.rs) + watcher bridge (bus.rs) the server wires up,
    // and asserts:
    //   1. self-write suppression holds: every own write (and notify's
    //      2-3 event burst per write) is recognized as a self-echo and
    //      never forwarded to the /ws fan-out as an external edit -- the
    //      path that would otherwise drive the banner.
    //   2. a genuine external edit still surfaces (suppression is not a
    //      blanket mute).
    //   3. the CAS mtime_ns token round-trips: each autosave's returned
    //      token is valid for the next write; a stale token conflicts.
    //   4. no spurious DraftBroken / "missing draft.md": inspect_draft +
    //      re-read stay healthy across the whole loop.
    #[test]
    fn draft_autosave_loop_holds_suppression_cas_and_no_broken_draft() {
        use crate::bus::{make_watch_bridge, ScopeRegistry};
        use crate::self_writes::SelfWrites;
        use chan_workspace::{WatchCallback, WatchEvent, WatchKind};
        use std::sync::Arc;
        use tokio::sync::broadcast;

        let (_cfg, _root, workspace) = make_workspace();

        // Wire the real suppression + bridge the server uses (not a
        // mock). `events_tx` is the /ws fan-out the editor's
        // external-edit banner listens on; a self-write must never land
        // there.
        let self_writes = Arc::new(SelfWrites::new());
        let (events_tx, mut events_rx) = broadcast::channel::<String>(1024);
        let (index_tx, _index_rx) = broadcast::channel::<WatchEvent>(1024);
        let scopes = Arc::new(ScopeRegistry::new());
        let bridge = make_watch_bridge(&events_tx, &index_tx, &self_writes, &scopes);

        let echo = |bridge: &Arc<dyn WatchCallback>, kind, path: &str| {
            bridge.on_event(WatchEvent {
                kind,
                path: Some(path.to_string()),
                to: None,
            });
        };

        // Create the draft the way api_create_draft does: seed draft.md,
        // note the path so the Created event is suppressed.
        let name = create_draft_sync(&workspace).unwrap();
        let path = format!(".Drafts/{name}/draft.md");
        self_writes.note(&path);
        echo(&bridge, WatchKind::Created, &path);

        // Hammer the autosave loop. Track the CAS token across writes.
        let mut token_ns = workspace.stat(&path).unwrap().mtime_ns;
        for i in 0..200 {
            let body = format!("# Draft\n\nautosave {i}\n");
            // api_write_file notes BEFORE the blocking write; mirror it.
            self_writes.note(&path);
            workspace
                .write_text_if_unchanged(&path, token_ns, &body)
                .unwrap_or_else(|e| panic!("autosave {i} failed: {e:?}"));

            // CAS token must round-trip: the post-write mtime_ns is the
            // valid token for the next write.
            let stat = workspace.stat(&path).unwrap();
            assert!(stat.mtime_ns.is_some(), "autosave {i}: mtime_ns missing");
            token_ns = stat.mtime_ns;

            // notify often emits 2-3 events per logical write; every one
            // must be suppressed (no consume-on-match).
            echo(&bridge, WatchKind::Modified, &path);
            echo(&bridge, WatchKind::Modified, &path);
            echo(&bridge, WatchKind::Created, &path);

            // Re-read + inspect: never DraftBroken / missing draft.md.
            assert_eq!(
                workspace.read_text(&path).unwrap(),
                body,
                "autosave {i}: re-read mismatch"
            );
            let inspected = workspace
                .inspect_draft(&name)
                .unwrap_or_else(|e| panic!("autosave {i}: inspect_draft broke: {e:?}"));
            assert!(inspected.file_count >= 1, "autosave {i}: draft.md vanished");
        }

        // Not one self-write should have reached the /ws fan-out.
        assert!(
            matches!(
                events_rx.try_recv(),
                Err(broadcast::error::TryRecvError::Empty)
            ),
            "a self-write leaked to the editor as an external edit",
        );

        // A genuine external edit (a path we never noted) still surfaces.
        echo(&bridge, WatchKind::Modified, "notes/external.md");
        let frame = events_rx
            .try_recv()
            .expect("external edit must surface on /ws");
        assert!(
            frame.contains("external.md"),
            "unexpected /ws frame: {frame}",
        );

        // A stale CAS token must conflict (lock-step token contract).
        let err = workspace
            .write_text_if_unchanged(&path, Some(1), "# stale\n")
            .unwrap_err();
        assert!(
            matches!(err, chan_workspace::ChanError::WriteConflict { .. }),
            "stale token did not conflict: {err:?}",
        );
    }
}
