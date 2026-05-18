//! Per-file CRUD: list, read (text or binary), write (with optional
//! CAS), create (file or dir), delete, move.

use std::sync::Arc;

use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::{err, err_from};
use crate::state::AppState;
use crate::static_assets::content_type_for;

/// Tree entry shape on the wire. Adds a `kind` discriminator on top
/// of chan-drive's `TreeEntry` so the file browser, search overlay,
/// and graph inspector can render the right glyph + chip without a
/// per-file resolve round-trip. Five kinds (`document`, `contact`,
/// `text`, `media`, `binary`) for regular files; absent on directory
/// entries (the frontend keys off `is_dir` for those).
///
/// Mapping (see `project_kind` below):
///   - `FileClass::EditableText` + contact frontmatter -> `contact`
///   - `FileClass::EditableText`                       -> `document`
///   - `FileClass::Text`                               -> `text`
///   - `FileClass::Image` / `FileClass::Pdf`           -> `media`
///   - `FileClass::Other`                              -> `binary`
///
/// PDFs are media: the frontend's fullscreen viewer (state/pdfViewer.ts)
/// handles them via `<embed type="application/pdf">`. chan-drive keeps
/// `FileClass::Pdf` as a distinct variant so a future iteration that
/// renders PDFs differently from images (per-page extract, OCR, ...)
/// can re-distinguish without revisiting the wire shape.
#[derive(Serialize)]
struct TreeEntryView {
    path: String,
    is_dir: bool,
    mtime: Option<i64>,
    size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    path_class: Option<chan_drive::PathClass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<&'static str>,
}

/// Map a regular-file path (and its contact flag) to the wire kind
/// string. Returns `None` for directories so the existing serializer
/// drops the field on dir entries.
fn project_kind(path: &str, is_dir: bool, is_contact: bool) -> Option<&'static str> {
    if is_dir {
        return None;
    }
    if is_contact {
        return Some("contact");
    }
    Some(match chan_drive::fs_ops::classify(path) {
        chan_drive::FileClass::EditableText => "document",
        chan_drive::FileClass::Text => "text",
        chan_drive::FileClass::Image | chan_drive::FileClass::Pdf => "media",
        chan_drive::FileClass::Other => "binary",
    })
}

#[derive(Deserialize)]
pub struct ListFilesQuery {
    /// Optional directory to list non-recursively. Missing preserves
    /// the legacy recursive listing for callers that still need a
    /// whole-drive snapshot.
    #[serde(default)]
    dir: Option<String>,
}

pub async fn api_list_files(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListFilesQuery>,
) -> Response {
    let drive = state.drive();
    let tree = if let Some(dir) = query.dir.as_deref() {
        match list_dir_entries(&drive, dir) {
            Ok(t) => t,
            Err(e) => return err_from(&e),
        }
    } else {
        // The browser still reflects live disk, but it should not
        // recursively enumerate build/dependency trees that the drive's
        // own indexing policy already treats as noise (`target/`,
        // `node_modules/`, ...). Repo roots can otherwise spend startup
        // walking hundreds of thousands of uninteresting files before the
        // user sees anything.
        match chan_drive::fs_ops::list_tree_filtered(drive.root(), drive.walk_filter()) {
            Ok(t) => t,
            Err(e) => return err_from(&e),
        }
    };
    // Pull the contact-kind set in one shot; a single SQL scan beats N
    // per-path node_kind lookups on big drives.
    let contact_paths: std::collections::HashSet<String> = match drive.contacts() {
        Ok(rows) => rows.into_iter().map(|c| c.rel_path).collect(),
        Err(_) => std::collections::HashSet::new(),
    };
    let out: Vec<TreeEntryView> = tree
        .into_iter()
        .map(|e| TreeEntryView {
            kind: project_kind(&e.path, e.is_dir, contact_paths.contains(&e.path)),
            path_class: path_class_for_wire(&drive, &e.path),
            path: e.path,
            is_dir: e.is_dir,
            mtime: e.mtime,
            size: e.size,
        })
        .collect();
    Json(out).into_response()
}

fn list_dir_entries(
    drive: &chan_drive::Drive,
    dir: &str,
) -> chan_drive::Result<Vec<chan_drive::TreeEntry>> {
    let rel = normalize_dir_query(dir)?;
    let children = drive.list(&rel)?;
    let mut out = Vec::with_capacity(children.len());
    for child in children {
        if child.is_dir && drive.walk_filter().is_excluded(&child.name) {
            continue;
        }
        let path = join_rel(&rel, &child.name);
        let stat = match drive.stat(&path) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(%path, ?e, "list_dir_entries: stat failed; skipping");
                continue;
            }
        };
        out.push(chan_drive::TreeEntry {
            path,
            is_dir: stat.is_dir,
            mtime: stat.mtime,
            size: if stat.is_dir { 0 } else { stat.size },
        });
    }
    Ok(out)
}

fn normalize_dir_query(dir: &str) -> chan_drive::Result<String> {
    let trimmed = dir.trim_matches('/');
    if trimmed.is_empty() || trimmed == "." {
        return Ok(String::new());
    }
    chan_drive::fs_ops::validate_rel(trimmed)?;
    Ok(trimmed.to_string())
}

fn join_rel(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}/{name}")
    }
}

#[derive(Serialize)]
struct FileResponse {
    path: String,
    content: String,
    mtime: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path_class: Option<chan_drive::PathClass>,
    /// Filesystem-level writability. False when the path lacks the
    /// user-write bit (e.g. `chmod -w`); the editor uses this to
    /// lock the per-tab read mode regardless of user choice. Sourced
    /// from `metadata().permissions().readonly()` on the resolved
    /// drive-internal path so symlink escapes are still refused
    /// upstream by chan-drive.
    writable: bool,
}

fn path_class_for_wire(drive: &chan_drive::Drive, rel: &str) -> Option<chan_drive::PathClass> {
    match chan_drive::fs_ops::classify_path(drive.root(), rel) {
        Ok(class) => Some(class),
        Err(e) => {
            tracing::warn!(%rel, ?e, "path classification failed");
            None
        }
    }
}

/// Check the user-write bit on a drive-relative path. Returns true
/// when the path can't be safely resolved (matches read_text's own
/// behavior of failing later) so we don't surface a misleading
/// "locked" lamp on a path that's actually broken; callers get the
/// real error from `read_text` instead.
fn fs_writable(state: &AppState, rel: &str) -> bool {
    let abs = match chan_drive::fs_ops::resolve_safe_strict(state.drive().root(), rel) {
        Ok(p) => p,
        Err(_) => return true,
    };
    match std::fs::symlink_metadata(&abs) {
        Ok(m) => !m.permissions().readonly(),
        Err(_) => true,
    }
}

pub async fn api_read_file(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
) -> Response {
    // Editable-text files (.md / .txt) come back as FileResponse
    // JSON since the frontend's editor wants the content as a
    // string. Anything else (images, attachments) comes back as
    // raw bytes with a sniffed Content-Type so `<img src=...>`
    // pointing at /api/files/<path> resolves correctly.
    if chan_drive::fs_ops::is_editable_text(&path) {
        let content = match state.drive().read_text(&path) {
            Ok(c) => c,
            Err(e) => return err_from(&e),
        };
        let mtime = state.drive().stat(&path).ok().and_then(|s| s.mtime);
        let writable = fs_writable(&state, &path);
        return Json(FileResponse {
            path_class: path_class_for_wire(state.drive().as_ref(), &path),
            path,
            content,
            mtime,
            writable,
        })
        .into_response();
    }
    match state.drive().read(&path) {
        Ok(bytes) => ([(header::CONTENT_TYPE, content_type_for(&path))], bytes).into_response(),
        Err(e) => err_from(&e),
    }
}

#[derive(Deserialize)]
pub struct WriteBody {
    content: String,
    /// CAS token: the mtime the client thinks the file currently
    /// has on disk. When present, the server uses
    /// Drive::write_text_if_unchanged and rejects with 409 if the
    /// disk-side mtime differs. When absent, the write is
    /// last-write-wins (Drive::write_text), preserving the
    /// pre-CAS contract for callers that don't care
    /// (bulk imports, scripts).
    #[serde(default)]
    expected_mtime: Option<i64>,
}

#[derive(Serialize)]
struct WriteResponse {
    /// Mtime after the write. Frontend stores this as the next
    /// CAS token for subsequent saves so the client and disk stay
    /// in lock-step without an extra stat round-trip.
    mtime: Option<i64>,
}

#[derive(Serialize)]
struct WriteConflictBody {
    /// Mtime currently on disk, returned so the client knows what
    /// token to use on a follow-up "overwrite" attempt without a
    /// separate stat call. None when the file disappeared between
    /// the client's last fetch and now (rare; treat as "create
    /// fresh" on the retry).
    current_mtime: Option<i64>,
}

pub async fn api_write_file(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
    Json(body): Json<WriteBody>,
) -> Response {
    // chan-drive moved the CAS check to nanosecond precision
    // (`expected_mtime_ns`) to catch sub-second races between two
    // writers. Our wire format still surfaces seconds-precision
    // mtimes to the editor (an i64-as-JSON-number representation
    // for nanoseconds would lose precision past 2^53). We do the
    // seconds-precision compare ourselves here, then defer to
    // `write_text_if_unchanged` with the freshly-stat'd ns so the
    // actual rename is still gated atomically inside chan-drive.
    // Sub-second race protection is therefore a TODO until the
    // wire moves to ns-as-string; document the regression here so
    // the next reader knows it's a known gap, not a bug.
    let result = if body.expected_mtime.is_some() {
        let pre = state.drive().stat(&path).ok();
        let cur_secs = pre.as_ref().and_then(|s| s.mtime);
        let cur_ns = pre.as_ref().and_then(|s| s.mtime_ns);
        if body.expected_mtime != cur_secs {
            return (
                StatusCode::CONFLICT,
                Json(WriteConflictBody {
                    current_mtime: cur_secs,
                }),
            )
                .into_response();
        }
        state
            .drive()
            .write_text_if_unchanged(&path, cur_ns, &body.content)
    } else {
        state.drive().write_text(&path, &body.content)
    };
    if let Err(e) = result {
        if let chan_drive::ChanError::WriteConflict { current_mtime_ns } = e {
            return (
                StatusCode::CONFLICT,
                Json(WriteConflictBody {
                    current_mtime: current_mtime_ns.map(|ns| ns / 1_000_000_000),
                }),
            )
                .into_response();
        }
        return err_from(&e);
    }
    state.self_writes.note(&path);
    let mtime = state.drive().stat(&path).ok().and_then(|s| s.mtime);
    Json(WriteResponse { mtime }).into_response()
}

#[derive(Deserialize)]
pub struct CreateBody {
    path: String,
    is_dir: bool,
    /// Optional initial contents for files. Ignored for directories.
    content: Option<String>,
}

pub async fn api_create_file(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateBody>,
) -> Response {
    if state.drive().exists(&body.path) {
        return err(StatusCode::CONFLICT, "already exists".into());
    }
    if body.is_dir {
        match state.drive().create_dir(&body.path) {
            Ok(()) => {
                state.self_writes.note(&body.path);
                StatusCode::CREATED.into_response()
            }
            Err(e) => err_from(&e),
        }
    } else {
        let content = body.content.unwrap_or_default();
        match state.drive().write_text(&body.path, &content) {
            Ok(()) => {
                state.self_writes.note(&body.path);
                StatusCode::CREATED.into_response()
            }
            Err(e) => err_from(&e),
        }
    }
}

pub async fn api_delete_file(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
) -> Response {
    // chan-drive's Drive::remove handles files and EMPTY directories.
    // Recursive deletion of a non-empty directory is a deliberate
    // foot-gun guard; supporting it here would require either a new
    // chan-drive API (`Drive::remove_recursive`) or a server-side walk
    // that issues per-leaf removes. Tracked for a follow-up; current
    // behavior is "error out, frontend resolves the leaves itself".
    match state.drive().remove(&path) {
        Ok(()) => {
            state.self_writes.note(&path);
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => err_from(&e),
    }
}

#[derive(Deserialize)]
pub struct MoveBody {
    from: String,
    to: String,
}

pub async fn api_move(State(state): State<Arc<AppState>>, Json(body): Json<MoveBody>) -> Response {
    // Run the rename + link-rewrite pass on a blocking thread; the
    // rewrite walks N source files synchronously and can take a few
    // hundred ms on big directory moves. Keeping it off the tokio
    // worker pool avoids blocking other requests during the walk.
    let drive = state.drive().clone();
    let from = body.from.clone();
    let to = body.to.clone();
    let outcome =
        match tokio::task::spawn_blocking(move || drive.rename_with_link_rewrite(&from, &to)).await
        {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => return err_from(&e),
            Err(join) => return err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
        };
    // Rename emits two notify events on most kernels (a Removed at
    // `from` and a Created at `to`); the rewrite pass also touches
    // every rewritten source. Note them all so neither half of any
    // pair fires an external-edit prompt.
    state.self_writes.note(&body.from);
    state.self_writes.note(&body.to);
    for path in &outcome.rewritten {
        state.self_writes.note(path);
    }
    Json(MoveResponse {
        renamed: outcome.renamed,
        rewritten: outcome.rewritten,
        conflicts: outcome.conflicts,
    })
    .into_response()
}

#[derive(Serialize)]
struct MoveResponse {
    renamed: Vec<(String, String)>,
    rewritten: Vec<String>,
    conflicts: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_response_serializes_path_class_for_inspector_payload() {
        let response = FileResponse {
            path: "notes/a.md".to_string(),
            content: "hello".to_string(),
            mtime: Some(1),
            path_class: Some(chan_drive::PathClass {
                kind: chan_drive::PathKind::RegularFile,
                permission: chan_drive::PathPermission::ReadWrite,
                link_count: 2,
                target: None,
                target_escapes_drive: false,
            }),
            writable: true,
        };

        let value = serde_json::to_value(response).unwrap();
        assert_eq!(value["path_class"]["kind"], "regular_file");
        assert_eq!(value["path_class"]["permission"], "read_write");
        assert_eq!(value["path_class"]["link_count"], 2);
    }

    #[test]
    fn tree_entry_serializes_path_class_for_file_browser_inspector() {
        let entry = TreeEntryView {
            path: "alias.md".to_string(),
            is_dir: false,
            mtime: None,
            size: 0,
            path_class: Some(chan_drive::PathClass {
                kind: chan_drive::PathKind::Symlink,
                permission: chan_drive::PathPermission::ReadWrite,
                link_count: 1,
                target: Some(std::path::PathBuf::from("/etc/hosts")),
                target_escapes_drive: true,
            }),
            kind: Some("binary"),
        };

        let value = serde_json::to_value(entry).unwrap();
        assert_eq!(value["path_class"]["kind"], "symlink");
        assert_eq!(value["path_class"]["target"], "/etc/hosts");
        assert_eq!(value["path_class"]["target_escapes_drive"], true);
    }

    #[cfg(unix)]
    #[test]
    fn directory_listing_keeps_symlink_with_path_class() {
        use std::os::unix::fs::symlink;

        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_drive::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_drive(root.path(), Some("files-test".into()))
            .unwrap();
        let drive = lib.open_drive(root.path()).unwrap();
        std::fs::write(root.path().join("note.md"), "hi").unwrap();
        symlink("note.md", root.path().join("alias.md")).unwrap();

        let entries = list_dir_entries(&drive, "").unwrap();
        assert!(entries.iter().any(|entry| entry.path == "alias.md"));
        let class = path_class_for_wire(&drive, "alias.md").expect("symlink path class");
        assert_eq!(class.kind, chan_drive::PathKind::Symlink);
    }
}
