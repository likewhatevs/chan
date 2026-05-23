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

enum ReadFileResult {
    Text {
        content: String,
        mtime: Option<i64>,
        writable: bool,
        path_class: Option<chan_drive::PathClass>,
    },
    Binary(Vec<u8>),
}

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
    let drive = state.drive().clone();
    let result = tokio::task::spawn_blocking(move || list_files_sync(&drive, query)).await;

    match result {
        Ok(Ok(out)) => Json(out).into_response(),
        Ok(Err(e)) => err_from(&e),
        Err(join) => err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    }
}

fn list_files_sync(
    drive: &chan_drive::Drive,
    query: ListFilesQuery,
) -> chan_drive::Result<Vec<TreeEntryView>> {
    let tree = if let Some(dir) = query.dir.as_deref() {
        list_dir_entries(drive, dir)?
    } else {
        // The browser still reflects live disk, but it should not
        // recursively enumerate build/dependency trees that the drive's
        // own indexing policy already treats as noise (`target/`,
        // `node_modules/`, ...). Repo roots can otherwise spend startup
        // walking hundreds of thousands of uninteresting files before the
        // user sees anything.
        chan_drive::fs_ops::list_tree_filtered(drive.root(), drive.walk_filter())?
    };
    // Pull the contact-kind set in one shot; a single SQL scan beats N
    // per-path node_kind lookups on big drives.
    let contact_paths: std::collections::HashSet<String> = match drive.contacts() {
        Ok(rows) => rows.into_iter().map(|c| c.rel_path).collect(),
        Err(_) => std::collections::HashSet::new(),
    };
    let mut out: Vec<TreeEntryView> = tree
        .into_iter()
        .map(|e| TreeEntryView {
            kind: project_kind(&e.path, e.is_dir, contact_paths.contains(&e.path)),
            path_class: path_class_for_wire(drive, &e.path),
            path: e.path,
            is_dir: e.is_dir,
            mtime: e.mtime,
            size: e.size,
        })
        .collect();
    // `fullstack-a-66b`: when listing the drive root, inject a
    // synthetic `Drafts` directory entry at position 0 so the
    // file browser renders it as the first row. `Drafts`
    // lives in chan-drive metadata (drafts_dir), NOT under
    // the drive root, but appears in the wire under the
    // `Drafts/` keyspace per `systacean-25` / `-26` / `-29`.
    // Listing under `dir=Drafts/...` already routes through
    // the unified `Drive::list` thanks to `-29`.
    //
    // **Webtest-a triage (5dffa09 follow-up)**: the initial
    // implementation gated on `query.dir.is_none()` only, but
    // the SPA's `api.list` sends `?dir=` (empty string) for
    // root listings — that branch silently dropped Drafts.
    // Match both shapes: no `dir` param OR `dir` normalises
    // to root (empty / `.` / `/` after trimming).
    if is_root_listing(query.dir.as_deref()) {
        let drafts_entry = TreeEntryView {
            kind: None,
            path_class: None,
            path: "Drafts".to_string(),
            is_dir: true,
            mtime: None,
            size: 0,
        };
        out.insert(0, drafts_entry);
    }
    Ok(out)
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

/// `fullstack-a-66b` follow-up: classify the `?dir=` query
/// param as a root listing (used by `api_list_files` to gate
/// the synthetic Drafts injection). Matches every shape the
/// SPA + curl could send for "list the drive root":
///   * absent: `None`
///   * empty string: `Some("")`
///   * "/" or "./": trims to `""`
///   * "." literal: explicit current-dir form.
fn is_root_listing(dir: Option<&str>) -> bool {
    match dir {
        None => true,
        Some(s) => {
            let trimmed = s.trim_matches('/');
            trimmed.is_empty() || trimmed == "."
        }
    }
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

/// Check the user-write bit on a drive-relative path. Returns true when
/// the path can't be safely resolved (matches read_text's own behavior
/// of failing later) so we don't surface a misleading "locked" lamp on a
/// path that's actually broken; callers get the real error from
/// `read_text` instead.
fn fs_writable(drive: &chan_drive::Drive, rel: &str) -> bool {
    let abs = match chan_drive::fs_ops::resolve_safe_strict(drive.root(), rel) {
        Ok(p) => p,
        Err(_) => return true,
    };
    match std::fs::symlink_metadata(&abs) {
        Ok(m) => !m.permissions().readonly(),
        Err(_) => true,
    }
}

fn read_file_sync(drive: &chan_drive::Drive, path: &str) -> chan_drive::Result<ReadFileResult> {
    if chan_drive::fs_ops::is_editable_text(path) {
        let content = drive.read_text(path)?;
        let mtime = drive.stat(path).ok().and_then(|s| s.mtime);
        let writable = fs_writable(drive, path);
        return Ok(ReadFileResult::Text {
            content,
            mtime,
            writable,
            path_class: path_class_for_wire(drive, path),
        });
    }
    drive.read(path).map(ReadFileResult::Binary)
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
    let drive = state.drive().clone();
    let path_for_read = path.clone();
    let result = tokio::task::spawn_blocking(move || read_file_sync(&drive, &path_for_read)).await;

    match result {
        Ok(Ok(ReadFileResult::Text {
            content,
            mtime,
            writable,
            path_class,
        })) => Json(FileResponse {
            path_class,
            path,
            content,
            mtime,
            writable,
        })
        .into_response(),
        Ok(Ok(ReadFileResult::Binary(bytes))) => {
            ([(header::CONTENT_TYPE, content_type_for(&path))], bytes).into_response()
        }
        Ok(Err(e)) => err_from(&e),
        Err(join) => err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
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
    let drive = state.drive().clone();
    let path_for_write = path.clone();
    let result = tokio::task::spawn_blocking(move || {
        write_file_sync(&drive, &path_for_write, body.expected_mtime, &body.content)
    })
    .await;

    let mtime = match result {
        Ok(Ok(mtime)) => mtime,
        Ok(Err(e)) => {
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
        Err(join) => return err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    };
    state.self_writes.note(&path);
    Json(WriteResponse { mtime }).into_response()
}

fn write_file_sync(
    drive: &chan_drive::Drive,
    path: &str,
    expected_mtime: Option<i64>,
    content: &str,
) -> chan_drive::Result<Option<i64>> {
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
    if expected_mtime.is_some() {
        let pre = drive.stat(path).ok();
        let cur_secs = pre.as_ref().and_then(|s| s.mtime);
        let cur_ns = pre.as_ref().and_then(|s| s.mtime_ns);
        if expected_mtime != cur_secs {
            return Err(chan_drive::ChanError::WriteConflict {
                current_mtime_ns: cur_ns,
            });
        }
        drive.write_text_if_unchanged(path, cur_ns, content)?;
    } else {
        drive.write_text(path, content)?;
    }
    Ok(drive.stat(path).ok().and_then(|s| s.mtime))
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
        let drive = state.drive().clone();
        let path = body.path.clone();
        let result = tokio::task::spawn_blocking(move || drive.write_text(&path, &content)).await;
        match result {
            Ok(Ok(())) => {
                state.self_writes.note(&body.path);
                StatusCode::CREATED.into_response()
            }
            Ok(Err(e)) => err_from(&e),
            Err(join) => err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
        }
    }
}

#[cfg(test)]
mod is_root_listing_tests {
    use super::{is_root_listing, list_files_sync, ListFilesQuery};

    // `fullstack-a-66b` follow-up: the SPA's `api.list("")`
    // sends `?dir=` (empty string) — pre-fix the gate matched
    // only `None`, so the synthetic Drafts injection silently
    // dropped. Pin every shape the SPA / curl / tests could
    // produce.

    #[test]
    fn matches_absent_dir_param() {
        assert!(is_root_listing(None));
    }

    #[test]
    fn matches_empty_string_dir() {
        assert!(is_root_listing(Some("")));
    }

    #[test]
    fn matches_slash_only() {
        assert!(is_root_listing(Some("/")));
        assert!(is_root_listing(Some("//")));
    }

    #[test]
    fn matches_dot_form() {
        assert!(is_root_listing(Some(".")));
        assert!(is_root_listing(Some("./")));
    }

    #[test]
    fn rejects_non_root_dirs() {
        assert!(!is_root_listing(Some("docs")));
        assert!(!is_root_listing(Some("Drafts")));
        assert!(!is_root_listing(Some("Drafts/untitled-1")));
        assert!(!is_root_listing(Some("crates/chan-drive")));
    }

    #[test]
    fn list_files_sync_injects_drafts_for_root_dir_query() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_drive::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_drive(root.path(), Some("files-list-test".into()))
            .unwrap();
        let drive = lib.open_drive(root.path()).unwrap();
        std::fs::write(root.path().join("note.md"), "hi").unwrap();

        let entries = list_files_sync(
            &drive,
            ListFilesQuery {
                dir: Some(String::new()),
            },
        )
        .unwrap();

        assert_eq!(
            entries.first().map(|entry| entry.path.as_str()),
            Some("Drafts")
        );
        assert!(entries.iter().any(|entry| entry.path == "note.md"));
    }
}

#[cfg(test)]
mod write_tests {
    use super::*;

    #[test]
    fn read_file_sync_returns_editable_text_metadata() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_drive::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_drive(root.path(), Some("files-read-test".into()))
            .unwrap();
        let drive = lib.open_drive(root.path()).unwrap();
        drive.write_text("note.md", "hello").unwrap();

        let result = read_file_sync(&drive, "note.md").unwrap();

        match result {
            ReadFileResult::Text {
                content,
                mtime,
                writable,
                path_class,
            } => {
                assert_eq!(content, "hello");
                assert!(mtime.is_some());
                assert!(writable);
                assert_eq!(
                    path_class.map(|class| class.kind),
                    Some(chan_drive::PathKind::RegularFile)
                );
            }
            ReadFileResult::Binary(_) => panic!("expected editable text result"),
        }
    }

    #[test]
    fn read_file_sync_returns_binary_bytes() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_drive::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_drive(root.path(), Some("files-read-test".into()))
            .unwrap();
        let drive = lib.open_drive(root.path()).unwrap();
        std::fs::write(root.path().join("image.bin"), [0, 1, 2, 3]).unwrap();

        let result = read_file_sync(&drive, "image.bin").unwrap();

        match result {
            ReadFileResult::Binary(bytes) => assert_eq!(bytes, vec![0, 1, 2, 3]),
            ReadFileResult::Text { .. } => panic!("expected binary result"),
        }
    }

    #[test]
    fn api_read_file_wraps_sync_drive_reads_in_spawn_blocking() {
        let source = include_str!("files.rs");

        assert!(source.contains(
            "tokio::task::spawn_blocking(move || read_file_sync(&drive, &path_for_read))"
        ));
    }

    #[test]
    fn api_list_files_wraps_sync_drive_walk_in_spawn_blocking() {
        let source = include_str!("files.rs");

        assert!(
            source.contains("tokio::task::spawn_blocking(move || list_files_sync(&drive, query))")
        );
    }

    #[test]
    fn write_file_sync_reports_seconds_conflict() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_drive::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_drive(root.path(), Some("files-write-test".into()))
            .unwrap();
        let drive = lib.open_drive(root.path()).unwrap();
        drive.write_text("note.md", "v1").unwrap();

        let err = write_file_sync(&drive, "note.md", Some(0), "v2").unwrap_err();

        assert!(matches!(
            err,
            chan_drive::ChanError::WriteConflict {
                current_mtime_ns: Some(_)
            }
        ));
        assert_eq!(drive.read_text("note.md").unwrap(), "v1");
    }

    #[test]
    fn write_file_sync_returns_new_mtime() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_drive::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_drive(root.path(), Some("files-write-test".into()))
            .unwrap();
        let drive = lib.open_drive(root.path()).unwrap();

        let mtime = write_file_sync(&drive, "note.md", None, "v1").unwrap();

        assert!(mtime.is_some());
        assert_eq!(drive.read_text("note.md").unwrap(), "v1");
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
