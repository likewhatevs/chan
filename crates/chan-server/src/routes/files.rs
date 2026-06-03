//! Per-file CRUD: list, read (text or binary), write (with optional
//! CAS), create (file or dir), delete, move.

use std::{convert::Infallible, io::Cursor, sync::Arc};

use axum::body::{Body, Bytes};
use axum::extract::{Multipart, Path as AxumPath, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::error::{err, err_from, err_state};
use crate::state::AppState;
use crate::static_assets::content_type_for;

enum ReadFileResult {
    Text {
        content: String,
        mtime: Option<i64>,
        mtime_ns: Option<i64>,
        writable: bool,
        path_class: Option<chan_workspace::PathClass>,
    },
    Binary(Vec<u8>),
}

/// Tree entry shape on the wire. Adds a `kind` discriminator on top
/// of chan-workspace's `TreeEntry` so the file browser, search overlay,
/// and graph inspector can render the right glyph + chip without a
/// per-file resolve round-trip. Six kinds (`document`, `contact`,
/// `text`, `media`, `binary`, `pending`) for regular files; absent on
/// directory entries (the frontend keys off `is_dir` for those).
///
/// Mapping (see `project_kind` below):
///   - `FileClass::EditableText` + contact frontmatter -> `contact`
///   - `FileClass::EditableText` + Markdown (`.md`)     -> `document`
///   - `FileClass::EditableText` non-Markdown (`.txt`)  -> `text`
///   - `FileClass::Text`                               -> `text`
///   - `FileClass::Image` / `FileClass::Pdf`           -> `media`
///   - `FileClass::Other` -> `pending`; a content sniff in
///     `list_files_sync` then resolves it to `text` (valid UTF-8, no
///     NUL) or `binary` for per-directory listings.
///
/// PDFs are media: the frontend's fullscreen viewer (state/pdfViewer.ts)
/// handles them via `<embed type="application/pdf">`. chan-workspace keeps
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
    path_class: Option<chan_workspace::PathClass>,
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
    Some(match chan_workspace::fs_ops::classify(path) {
        // Only Markdown (.md) is a graph "document" (graphed + wikilinked).
        // .txt stays editable + BM25-searchable but is not a document node,
        // so it rides the "text" wire kind alongside source/config text.
        // Keyed off `is_markdown_file` to stay in lockstep with the graph
        // ingest gate (`Workspace::rebuild_graph` / `index_file_inner`).
        chan_workspace::FileClass::EditableText
            if chan_workspace::fs_ops::is_markdown_file(path) =>
        {
            "document"
        }
        chan_workspace::FileClass::EditableText | chan_workspace::FileClass::Text => "text",
        chan_workspace::FileClass::Image | chan_workspace::FileClass::Pdf => "media",
        // Unknown extension/basename: the path alone can't tell text
        // from binary. Emit "pending" rather than prejudging "binary";
        // per-directory listings resolve it with a content sniff (see
        // `list_files_sync`), so the file browser still shows a final
        // "text"/"binary" kind. Only the recursive whole-tree listing
        // (image picker) leaves it "pending", and that caller reads
        // media kinds only.
        chan_workspace::FileClass::Other => "pending",
    })
}

#[derive(Deserialize)]
pub struct ListFilesQuery {
    /// Optional directory to list non-recursively. Missing preserves
    /// the legacy recursive listing for callers that still need a
    /// whole-workspace snapshot.
    #[serde(default)]
    dir: Option<String>,
}

pub async fn api_list_files(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListFilesQuery>,
) -> Response {
    let workspace = match state.try_workspace() {
        Ok(workspace) => workspace,
        Err(e) => return err_state(&e),
    };
    let result = tokio::task::spawn_blocking(move || list_files_sync(&workspace, query)).await;

    match result {
        Ok(Ok(out)) => Json(out).into_response(),
        Ok(Err(e)) => err_from(&e),
        Err(join) => err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    }
}

fn list_files_sync(
    workspace: &chan_workspace::Workspace,
    query: ListFilesQuery,
) -> chan_workspace::Result<Vec<TreeEntryView>> {
    let tree = if let Some(dir) = query.dir.as_deref() {
        list_dir_entries(workspace, dir)?
    } else {
        // The browser still reflects live disk, but it should not
        // recursively enumerate build/dependency trees that the workspace's
        // own indexing policy already treats as noise (`target/`,
        // `node_modules/`, ...). Repo roots can otherwise spend startup
        // walking hundreds of thousands of uninteresting files before the
        // user sees anything.
        chan_workspace::fs_ops::list_tree_filtered(workspace.root(), workspace.walk_filter())?
    };
    // Pull the contact-kind set in one shot; a single SQL scan beats N
    // per-path node_kind lookups on big workspaces.
    let contact_paths: std::collections::HashSet<String> = match workspace.contacts() {
        Ok(rows) => rows.into_iter().map(|c| c.rel_path).collect(),
        Err(_) => std::collections::HashSet::new(),
    };
    let mut out: Vec<TreeEntryView> = tree
        .into_iter()
        .map(|e| TreeEntryView {
            kind: project_kind(&e.path, e.is_dir, contact_paths.contains(&e.path)),
            path_class: path_class_for_wire(workspace, &e.path),
            path: e.path,
            is_dir: e.is_dir,
            mtime: e.mtime,
            size: e.size,
        })
        .collect();
    // Resolve the path-only "pending" kind with a bounded content
    // sniff, but only for per-directory listings (the file browser).
    // It lists one directory at a time, so this stays a handful of
    // 8 KiB reads per expand. The recursive whole-tree listing (no
    // `dir`, used by the image picker) is left untouched so we never
    // sniff the entire tree; its consumer reads media kinds only.
    if query.dir.is_some() {
        for entry in out.iter_mut() {
            if entry.kind == Some("pending") {
                entry.kind = Some(if workspace.sniff_is_text(&entry.path) {
                    "text"
                } else {
                    "binary"
                });
            }
        }
    }
    Ok(out)
}

fn list_dir_entries(
    workspace: &chan_workspace::Workspace,
    dir: &str,
) -> chan_workspace::Result<Vec<chan_workspace::TreeEntry>> {
    let rel = normalize_dir_query(dir)?;
    if chan_workspace::drafts::is_unified_drafts_path(&rel) {
        return Err(chan_workspace::ChanError::Io(
            "not found: Drafts is hidden from File Browser".to_string(),
        ));
    }
    let children = workspace.list(&rel)?;
    let mut out = Vec::with_capacity(children.len());
    for child in children {
        if child.is_dir && workspace.walk_filter().is_excluded(&child.name) {
            continue;
        }
        let path = join_rel(&rel, &child.name);
        let stat = match workspace.stat(&path) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(%path, ?e, "list_dir_entries: stat failed; skipping");
                continue;
            }
        };
        out.push(chan_workspace::TreeEntry {
            path,
            is_dir: stat.is_dir,
            mtime: stat.mtime,
            size: if stat.is_dir { 0 } else { stat.size },
        });
    }
    Ok(out)
}

fn normalize_dir_query(dir: &str) -> chan_workspace::Result<String> {
    let trimmed = dir.trim_matches('/');
    if trimmed.is_empty() || trimmed == "." {
        return Ok(String::new());
    }
    chan_workspace::fs_ops::validate_rel(trimmed)?;
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
    mtime_ns: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path_class: Option<chan_workspace::PathClass>,
    /// Filesystem-level writability. False when the path lacks the
    /// user-write bit (e.g. `chmod -w`); the editor uses this to
    /// lock the per-tab read mode regardless of user choice. Sourced
    /// from `metadata().permissions().readonly()` on the resolved
    /// workspace-internal path so symlink escapes are still refused
    /// upstream by chan-workspace.
    writable: bool,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum FileStreamEvent<'a> {
    Meta {
        path: &'a str,
        size: u64,
        mtime: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        mtime_ns: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        path_class: Option<chan_workspace::PathClass>,
        writable: bool,
    },
    Chunk {
        content: &'a str,
        bytes: usize,
    },
    Done,
    Error {
        error: String,
    },
}

enum FileStreamMessage {
    Data(Bytes),
    Error(chan_workspace::ChanError),
}

fn path_class_for_wire(
    workspace: &chan_workspace::Workspace,
    rel: &str,
) -> Option<chan_workspace::PathClass> {
    match chan_workspace::fs_ops::classify_path(workspace.root(), rel) {
        Ok(class) => Some(class),
        Err(e) => {
            tracing::warn!(%rel, ?e, "path classification failed");
            None
        }
    }
}

/// Check the user-write bit on a workspace-relative path. Returns true when
/// the path can't be safely resolved (matches read_text's own behavior
/// of failing later) so we don't surface a misleading "locked" lamp on a
/// path that's actually broken; callers get the real error from
/// `read_text` instead.
fn fs_writable(workspace: &chan_workspace::Workspace, rel: &str) -> bool {
    let abs = match chan_workspace::fs_ops::resolve_safe_strict(workspace.root(), rel) {
        Ok(p) => p,
        Err(_) => return true,
    };
    match std::fs::symlink_metadata(&abs) {
        Ok(m) => !m.permissions().readonly(),
        Err(_) => true,
    }
}

fn read_file_sync(
    workspace: &chan_workspace::Workspace,
    path: &str,
) -> chan_workspace::Result<ReadFileResult> {
    // `read_text_with_stat` applies the content-aware editable gate, so
    // an extensionless / odd-suffix text file (`.zshrc`, `*.service`)
    // reads as text here. A genuinely binary file fails the gate with
    // `NotEditableText`; that is the only error we swallow into a binary
    // read. Any other error (invalid UTF-8 deeper than the sniff window,
    // I/O failure) propagates so the editor sees the real cause.
    match workspace.read_text_with_stat(path) {
        Ok((content, stat)) => Ok(ReadFileResult::Text {
            content,
            mtime: stat.mtime,
            mtime_ns: stat.mtime_ns,
            writable: fs_writable(workspace, path),
            path_class: path_class_for_wire(workspace, path),
        }),
        Err(chan_workspace::ChanError::NotEditableText(_)) => {
            workspace.read(path).map(ReadFileResult::Binary)
        }
        Err(e) => Err(e),
    }
}

fn ndjson_bytes(event: &FileStreamEvent<'_>) -> Result<Bytes, serde_json::Error> {
    let mut line = serde_json::to_vec(event)?;
    line.push(b'\n');
    Ok(Bytes::from(line))
}

fn ndjson_error_bytes(error: String) -> Bytes {
    match ndjson_bytes(&FileStreamEvent::Error { error }) {
        Ok(bytes) => bytes,
        Err(e) => Bytes::from(format!(
            "{{\"type\":\"error\",\"error\":\"failed to encode stream error: {e}\"}}\n"
        )),
    }
}

fn stream_read_file_sync<F>(
    workspace: &chan_workspace::Workspace,
    path: &str,
    mut emit: F,
) -> chan_workspace::Result<()>
where
    F: FnMut(Bytes) -> bool,
{
    let mut encode_error = None;
    let result = workspace.read_text_with_stat_chunked(
        path,
        chan_workspace::TEXT_READ_CHUNK_SIZE,
        |event| {
            let event = match event {
                chan_workspace::TextReadEvent::Meta(stat) => FileStreamEvent::Meta {
                    path,
                    size: stat.size,
                    mtime: stat.mtime,
                    mtime_ns: stat.mtime_ns.map(|ns| ns.to_string()),
                    path_class: path_class_for_wire(workspace, path),
                    writable: fs_writable(workspace, path),
                },
                chan_workspace::TextReadEvent::Chunk(content) => FileStreamEvent::Chunk {
                    content,
                    bytes: content.len(),
                },
                chan_workspace::TextReadEvent::Done => FileStreamEvent::Done,
            };
            match ndjson_bytes(&event) {
                Ok(bytes) => emit(bytes),
                Err(e) => {
                    encode_error = Some(chan_workspace::ChanError::Io(format!(
                        "failed to encode file stream event: {e}"
                    )));
                    false
                }
            }
        },
    );
    result?;
    if let Some(e) = encode_error {
        Err(e)
    } else {
        Ok(())
    }
}

enum DownloadPayload {
    File(Vec<u8>),
    DirectoryTar(Vec<u8>),
}

fn download_path_sync(
    workspace: &chan_workspace::Workspace,
    path: &str,
) -> chan_workspace::Result<DownloadPayload> {
    let stat = workspace.stat(path)?;
    if stat.is_dir {
        let bytes = archive_directory_sync(workspace, path)?;
        Ok(DownloadPayload::DirectoryTar(bytes))
    } else {
        workspace.read(path).map(DownloadPayload::File)
    }
}

fn download_filename(path: &str) -> String {
    let raw = path
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .unwrap_or("download");
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch == '"' || ch == '\\' || ch == ':' || ch.is_control() {
            out.push('_');
        } else {
            out.push(ch);
        }
    }
    if out.trim().is_empty() {
        "download".to_string()
    } else {
        out
    }
}

fn content_disposition_attachment(path: &str) -> String {
    format!("attachment; filename=\"{}\"", download_filename(path))
}

fn download_archive_filename(path: &str) -> String {
    let name = download_filename(path);
    if name.to_ascii_lowercase().ends_with(".tar") {
        name
    } else {
        format!("{name}.tar")
    }
}

fn content_disposition_archive(path: &str) -> String {
    format!(
        "attachment; filename=\"{}\"",
        download_archive_filename(path)
    )
}

fn archive_directory_sync(
    workspace: &chan_workspace::Workspace,
    path: &str,
) -> chan_workspace::Result<Vec<u8>> {
    let root_name = download_filename(path);
    let mut builder = tar::Builder::new(Vec::new());
    append_dir_to_archive(&mut builder, workspace, path, &root_name)?;
    builder.finish()?;
    Ok(builder.into_inner()?)
}

fn append_dir_to_archive(
    builder: &mut tar::Builder<Vec<u8>>,
    workspace: &chan_workspace::Workspace,
    source_rel: &str,
    archive_rel: &str,
) -> chan_workspace::Result<()> {
    append_archive_dir(builder, archive_rel)?;
    for child in workspace.list(source_rel)? {
        let child_source = join_rel(source_rel.trim_matches('/'), &child.name);
        let child_archive = join_rel(archive_rel, &child.name);
        if child.is_dir {
            append_dir_to_archive(builder, workspace, &child_source, &child_archive)?;
        } else {
            let bytes = workspace.read(&child_source)?;
            append_archive_file(builder, &child_archive, bytes)?;
        }
    }
    Ok(())
}

fn append_archive_dir(
    builder: &mut tar::Builder<Vec<u8>>,
    archive_rel: &str,
) -> chan_workspace::Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Directory);
    header.set_size(0);
    header.set_mode(0o755);
    header.set_cksum();
    builder.append_data(&mut header, archive_rel, std::io::empty())?;
    Ok(())
}

fn append_archive_file(
    builder: &mut tar::Builder<Vec<u8>>,
    archive_rel: &str,
    bytes: Vec<u8>,
) -> chan_workspace::Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder.append_data(&mut header, archive_rel, Cursor::new(bytes))?;
    Ok(())
}

#[derive(Default, Deserialize)]
pub struct ReadFileQuery {
    #[serde(default)]
    download: Option<String>,
    #[serde(default)]
    stream: Option<String>,
}

fn query_flag(value: &Option<String>) -> bool {
    matches!(
        value.as_deref(),
        Some("1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON")
    )
}

pub async fn api_read_file(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
    Query(query): Query<ReadFileQuery>,
) -> Response {
    // Editable-text files (.md / .txt) come back as FileResponse
    // JSON since the frontend's editor wants the content as a
    // string. Anything else (images, attachments) comes back as
    // raw bytes with a sniffed Content-Type so `<img src=...>`
    // pointing at /api/files/<path> resolves correctly.
    let workspace = match state.try_workspace() {
        Ok(workspace) => workspace,
        Err(e) => return err_state(&e),
    };
    if query_flag(&query.download) {
        let path_for_download = path.clone();
        let result =
            tokio::task::spawn_blocking(move || download_path_sync(&workspace, &path_for_download))
                .await;
        return match result {
            Ok(Ok(DownloadPayload::File(bytes))) => (
                [
                    (header::CONTENT_TYPE, content_type_for(&path).to_string()),
                    (
                        header::CONTENT_DISPOSITION,
                        content_disposition_attachment(&path),
                    ),
                ],
                bytes,
            )
                .into_response(),
            Ok(Ok(DownloadPayload::DirectoryTar(bytes))) => (
                [
                    (header::CONTENT_TYPE, "application/x-tar".to_string()),
                    (
                        header::CONTENT_DISPOSITION,
                        content_disposition_archive(&path),
                    ),
                ],
                bytes,
            )
                .into_response(),
            Ok(Err(e)) => err_from(&e),
            Err(join) => err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
        };
    }

    if query_flag(&query.stream) {
        return stream_read_file_response(workspace, path).await;
    }

    let path_for_read = path.clone();
    let result =
        tokio::task::spawn_blocking(move || read_file_sync(&workspace, &path_for_read)).await;

    match result {
        Ok(Ok(ReadFileResult::Text {
            content,
            mtime,
            mtime_ns,
            writable,
            path_class,
        })) => Json(FileResponse {
            path_class,
            path,
            content,
            mtime,
            mtime_ns: mtime_ns.map(|ns| ns.to_string()),
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

async fn stream_read_file_response(
    workspace: Arc<chan_workspace::Workspace>,
    path: String,
) -> Response {
    let (tx, mut rx) = mpsc::channel::<FileStreamMessage>(8);
    let path_for_read = path.clone();
    tokio::task::spawn_blocking(move || {
        let result = stream_read_file_sync(&workspace, &path_for_read, |bytes| {
            tx.blocking_send(FileStreamMessage::Data(bytes)).is_ok()
        });
        if let Err(e) = result {
            let _ = tx.blocking_send(FileStreamMessage::Error(e));
        }
    });

    let first = match rx.recv().await {
        Some(FileStreamMessage::Data(bytes)) => bytes,
        Some(FileStreamMessage::Error(e)) => return err_from(&e),
        None => {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                "file stream ended before metadata".into(),
            )
        }
    };
    let rest = stream::unfold(rx, |mut rx| async {
        rx.recv().await.map(|message| {
            let bytes = match message {
                FileStreamMessage::Data(bytes) => bytes,
                FileStreamMessage::Error(e) => ndjson_error_bytes(e.to_string()),
            };
            (Ok::<Bytes, Infallible>(bytes), rx)
        })
    });
    let body =
        Body::from_stream(stream::once(async move { Ok::<Bytes, Infallible>(first) }).chain(rest));
    ([(header::CONTENT_TYPE, "application/x-ndjson")], body).into_response()
}

#[derive(Deserialize)]
pub struct WriteBody {
    content: String,
    /// CAS token: the mtime the client thinks the file currently
    /// has on disk. When present, the server uses
    /// Workspace::write_text_if_unchanged and rejects with 409 if the
    /// disk-side mtime differs. When absent, the write is
    /// last-write-wins (Workspace::write_text), preserving the
    /// pre-CAS contract for callers that don't care
    /// (bulk imports, scripts).
    #[serde(default)]
    expected_mtime: Option<i64>,
    /// Nanosecond CAS token as a decimal string. Preferred over
    /// `expected_mtime` when present, because JSON numbers cannot
    /// represent nanosecond mtimes exactly in browser clients.
    #[serde(default)]
    expected_mtime_ns: Option<String>,
}

#[derive(Serialize)]
struct WriteResponse {
    /// Mtime after the write. Frontend stores this as the next
    /// CAS token for subsequent saves so the client and disk stay
    /// in lock-step without an extra stat round-trip.
    mtime: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mtime_ns: Option<String>,
}

#[derive(Serialize)]
struct WriteConflictBody {
    /// Mtime currently on disk, returned so the client knows what
    /// token to use on a follow-up "overwrite" attempt without a
    /// separate stat call. None when the file disappeared between
    /// the client's last fetch and now (rare; treat as "create
    /// fresh" on the retry).
    current_mtime: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_mtime_ns: Option<String>,
}

pub async fn api_write_file(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
    Json(body): Json<WriteBody>,
) -> Response {
    let expected_mtime_ns = match parse_optional_mtime_ns(body.expected_mtime_ns.as_deref()) {
        Ok(mtime_ns) => mtime_ns,
        Err(message) => return err(StatusCode::BAD_REQUEST, message),
    };
    let workspace = match state.try_workspace() {
        Ok(workspace) => workspace,
        Err(e) => return err_state(&e),
    };
    // Record the self-write BEFORE the blocking write runs. The fs
    // watcher runs on its own thread and can deliver the resulting
    // notify event the instant the write lands; noting after the
    // spawn_blocking await left a window where that event reached
    // should_suppress() before the path was recorded, so the editor's
    // own autosave surfaced as a phantom "external edit" mid-typing.
    // Recording up front closes the window. Noting a path whose write
    // then fails (CAS conflict / IO error) is harmless and within the
    // module's documented over-suppression trade-off.
    state.self_writes.note(&path);
    let path_for_write = path.clone();
    let result = tokio::task::spawn_blocking(move || {
        write_file_sync(
            &workspace,
            &path_for_write,
            body.expected_mtime,
            expected_mtime_ns,
            &body.content,
        )
    })
    .await;

    let (mtime, mtime_ns) = match result {
        Ok(Ok(mtime)) => mtime,
        Ok(Err(e)) => {
            if let chan_workspace::ChanError::WriteConflict { current_mtime_ns } = e {
                return (
                    StatusCode::CONFLICT,
                    Json(WriteConflictBody {
                        current_mtime: current_mtime_ns.map(|ns| ns / 1_000_000_000),
                        current_mtime_ns: current_mtime_ns.map(|ns| ns.to_string()),
                    }),
                )
                    .into_response();
            }
            return err_from(&e);
        }
        Err(join) => return err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    };
    Json(WriteResponse {
        mtime,
        mtime_ns: mtime_ns.map(|ns| ns.to_string()),
    })
    .into_response()
}

fn write_file_sync(
    workspace: &chan_workspace::Workspace,
    path: &str,
    expected_mtime: Option<i64>,
    expected_mtime_ns: Option<i64>,
    content: &str,
) -> chan_workspace::Result<(Option<i64>, Option<i64>)> {
    if let Some(ns) = expected_mtime_ns {
        workspace.write_text_if_unchanged(path, Some(ns), content)?;
    } else if expected_mtime.is_some() {
        let pre = workspace.stat(path).ok();
        let cur_secs = pre.as_ref().and_then(|s| s.mtime);
        let cur_ns = pre.as_ref().and_then(|s| s.mtime_ns);
        if expected_mtime != cur_secs {
            return Err(chan_workspace::ChanError::WriteConflict {
                current_mtime_ns: cur_ns,
            });
        }
        workspace.write_text_if_unchanged(path, cur_ns, content)?;
    } else {
        workspace.write_text(path, content)?;
    }
    let stat = workspace.stat(path).ok();
    Ok((
        stat.as_ref().and_then(|s| s.mtime),
        stat.as_ref().and_then(|s| s.mtime_ns),
    ))
}

fn parse_optional_mtime_ns(value: Option<&str>) -> Result<Option<i64>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Err("expected_mtime_ns must be a decimal nanosecond timestamp".into());
    }
    value
        .parse::<i64>()
        .map(Some)
        .map_err(|_| "expected_mtime_ns must be a decimal nanosecond timestamp".into())
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
    let workspace = match state.try_workspace() {
        Ok(workspace) => workspace,
        Err(e) => return err_state(&e),
    };
    let path = body.path.clone();
    // Record the self-write before the blocking create so the
    // watcher's echo is suppressed without racing the await; see
    // api_write_file for the full rationale.
    state.self_writes.note(&path);
    let result = tokio::task::spawn_blocking(move || create_file_sync(&workspace, body)).await;
    match result {
        Ok(Ok(())) => StatusCode::CREATED.into_response(),
        Ok(Err(e)) => err_from(&e),
        Err(join) => err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    }
}

fn create_file_sync(
    workspace: &chan_workspace::Workspace,
    body: CreateBody,
) -> chan_workspace::Result<()> {
    if create_target_exists(workspace, &body.path) {
        return Err(chan_workspace::ChanError::PathAlreadyExists(body.path));
    }
    if body.is_dir {
        workspace.create_dir(&body.path)
    } else {
        workspace.write_text(&body.path, &body.content.unwrap_or_default())
    }
}

fn create_target_exists(workspace: &chan_workspace::Workspace, path: &str) -> bool {
    workspace.stat(path).is_ok()
}

#[derive(Debug, Serialize)]
struct UploadFileResponse {
    path: String,
    size: u64,
}

pub async fn api_upload_file(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Response {
    let mut chosen: Option<(String, Vec<u8>)> = None;
    let mut dir = String::new();
    let mut replace_path: Option<String> = None;
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                let name = field.name().unwrap_or("").to_owned();
                match name.as_str() {
                    "file" if chosen.is_none() => {
                        let filename = field.file_name().unwrap_or("").to_owned();
                        let bytes = match field.bytes().await {
                            Ok(b) => b.to_vec(),
                            Err(e) => {
                                return err(
                                    StatusCode::BAD_REQUEST,
                                    format!("multipart read: {e}"),
                                );
                            }
                        };
                        chosen = Some((filename, bytes));
                    }
                    "dir" => match field.text().await {
                        Ok(s) => dir = s,
                        Err(e) => {
                            return err(StatusCode::BAD_REQUEST, format!("multipart read: {e}"));
                        }
                    },
                    "path" => match field.text().await {
                        Ok(s) => replace_path = Some(s),
                        Err(e) => {
                            return err(StatusCode::BAD_REQUEST, format!("multipart read: {e}"));
                        }
                    },
                    _ => {}
                }
            }
            Ok(None) => break,
            Err(e) => return err(StatusCode::BAD_REQUEST, format!("multipart parse: {e}")),
        }
    }

    let Some((filename, bytes)) = chosen else {
        return err(
            StatusCode::BAD_REQUEST,
            "missing `file` part in multipart body".into(),
        );
    };

    let workspace = match state.try_workspace() {
        Ok(workspace) => workspace,
        Err(e) => return err_state(&e),
    };
    // The destination path is computed inside the blocking task (the
    // collision-avoidance loop picks a free name), so we can't note it
    // before the spawn. Clone the suppression handle in and record the
    // self-write inside the task, before it returns to the await, so
    // the watcher's echo is suppressed without the race that surfaced
    // server writes as phantom external edits. See api_write_file.
    let self_writes = Arc::clone(&state.self_writes);
    let result = tokio::task::spawn_blocking(move || {
        let upload = if let Some(path) = replace_path {
            replace_file_sync(&workspace, &path, &bytes)
        } else {
            upload_file_sync(&workspace, &dir, &filename, &bytes)
        }?;
        self_writes.note(&upload.path);
        Ok::<_, chan_workspace::ChanError>(upload)
    })
    .await;
    match result {
        Ok(Ok(upload)) => Json(upload).into_response(),
        Ok(Err(e)) => err_from(&e),
        Err(e) => err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("file upload task panicked: {e}"),
        ),
    }
}

fn replace_file_sync(
    workspace: &chan_workspace::Workspace,
    path: &str,
    bytes: &[u8],
) -> chan_workspace::Result<UploadFileResponse> {
    let trimmed = path.trim_matches('/');
    chan_workspace::fs_ops::validate_rel(trimmed)?;
    let stat = workspace.stat(trimmed)?;
    if stat.is_dir {
        return Err(chan_workspace::ChanError::Io(format!(
            "not a file: {trimmed}"
        )));
    }
    workspace.write_bytes(trimmed, bytes)?;
    Ok(UploadFileResponse {
        path: trimmed.to_string(),
        size: bytes.len() as u64,
    })
}

fn upload_file_sync(
    workspace: &chan_workspace::Workspace,
    dir: &str,
    original_name: &str,
    bytes: &[u8],
) -> chan_workspace::Result<UploadFileResponse> {
    let dir = normalize_dir_query(dir)?;
    if !dir.is_empty() {
        let stat = workspace.stat(&dir)?;
        if !stat.is_dir {
            return Err(chan_workspace::ChanError::Io(format!(
                "not a directory: {dir}"
            )));
        }
    }
    let filename = upload_leaf_filename(original_name)?;
    let rel = join_rel(&dir, &filename);
    if create_target_exists(workspace, &rel) {
        return Err(chan_workspace::ChanError::PathAlreadyExists(rel));
    }
    workspace.write_bytes(&rel, bytes)?;
    Ok(UploadFileResponse {
        path: rel,
        size: bytes.len() as u64,
    })
}

fn upload_leaf_filename(original_name: &str) -> chan_workspace::Result<String> {
    let leaf = original_name
        .trim()
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .trim();
    if leaf.is_empty() {
        return Err(chan_workspace::ChanError::PathEmpty);
    }
    if leaf == "." || leaf == ".." || leaf.contains('\0') {
        return Err(chan_workspace::ChanError::PathEscape);
    }
    chan_workspace::fs_ops::validate_rel(leaf)?;
    Ok(leaf.to_string())
}

#[cfg(test)]
mod file_browser_listing_tests {
    use super::{
        create_target_exists, list_dir_entries, list_files_sync, replace_file_sync,
        upload_file_sync, upload_leaf_filename, ListFilesQuery,
    };

    #[test]
    fn list_files_sync_keeps_drafts_out_of_root_dir_query() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        std::fs::write(root.path().join("note.md"), "hi").unwrap();
        workspace
            .write_text("Drafts/untitled-1/draft.md", "# draft\n")
            .unwrap();

        let entries = list_files_sync(
            &workspace,
            ListFilesQuery {
                dir: Some(String::new()),
            },
        )
        .unwrap();

        assert!(!entries.iter().any(|entry| entry.path == "Drafts"));
        assert!(entries.iter().any(|entry| entry.path == "note.md"));
    }

    #[test]
    fn list_dir_entries_rejects_drafts_namespace_for_file_browser() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        workspace
            .write_text("Drafts/untitled-1/draft.md", "# draft\n")
            .unwrap();

        let err = list_dir_entries(&workspace, "Drafts").unwrap_err();

        assert!(
            err.to_string().contains("hidden from File Browser"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn create_target_exists_counts_directories_as_collisions() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        workspace.create_dir("notes").unwrap();

        assert!(create_target_exists(&workspace, "notes"));
        assert!(!create_target_exists(&workspace, "missing"));
    }

    #[test]
    fn upload_file_sync_writes_binary_with_original_leaf_name() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        workspace.create_dir("assets").unwrap();

        let uploaded = upload_file_sync(&workspace, "assets", "photo 1.PNG", &[1, 2, 3]).unwrap();

        assert_eq!(uploaded.path, "assets/photo 1.PNG");
        assert_eq!(uploaded.size, 3);
        assert_eq!(workspace.read("assets/photo 1.PNG").unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn upload_file_sync_rejects_existing_target() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        workspace.write_bytes("same.bin", b"old").unwrap();

        let err = upload_file_sync(&workspace, "", "same.bin", b"new").unwrap_err();

        assert!(matches!(err, chan_workspace::ChanError::PathAlreadyExists(p) if p == "same.bin"));
        assert_eq!(workspace.read("same.bin").unwrap(), b"old");
    }

    #[test]
    fn replace_file_sync_overwrites_existing_file() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        workspace.write_text("same.md", "old").unwrap();

        let uploaded = replace_file_sync(&workspace, "same.md", b"new").unwrap();

        assert_eq!(uploaded.path, "same.md");
        assert_eq!(uploaded.size, 3);
        assert_eq!(workspace.read_text("same.md").unwrap(), "new");
    }

    #[test]
    fn replace_file_sync_rejects_non_utf8_for_text_file() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        workspace.write_text("same.md", "old").unwrap();

        let err = replace_file_sync(&workspace, "same.md", &[0xff, 0xfe]).unwrap_err();

        assert!(err
            .to_string()
            .contains("non-UTF-8 bytes to editable text file"));
        assert_eq!(workspace.read_text("same.md").unwrap(), "old");
    }

    #[test]
    fn replace_file_sync_rejects_directory_target() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        workspace.create_dir("notes").unwrap();

        let err = replace_file_sync(&workspace, "notes", b"new").unwrap_err();

        assert!(err.to_string().contains("not a file: notes"));
    }

    #[test]
    fn upload_leaf_filename_uses_basename_and_rejects_empty_names() {
        assert_eq!(
            upload_leaf_filename(r"C:\tmp\report.pdf").unwrap(),
            "report.pdf"
        );
        assert!(matches!(
            upload_leaf_filename(""),
            Err(chan_workspace::ChanError::PathEmpty)
        ));
        assert!(matches!(
            upload_leaf_filename(".."),
            Err(chan_workspace::ChanError::PathEscape)
        ));
    }
}

#[cfg(test)]
mod write_tests {
    use super::*;

    #[test]
    fn read_file_sync_returns_editable_text_metadata() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        workspace.write_text("note.md", "hello").unwrap();

        let result = read_file_sync(&workspace, "note.md").unwrap();

        match result {
            ReadFileResult::Text {
                content,
                mtime,
                mtime_ns,
                writable,
                path_class,
            } => {
                assert_eq!(content, "hello");
                assert!(mtime.is_some());
                assert!(mtime_ns.is_some());
                assert!(writable);
                assert_eq!(
                    path_class.map(|class| class.kind),
                    Some(chan_workspace::PathKind::RegularFile)
                );
            }
            ReadFileResult::Binary(_) => panic!("expected editable text result"),
        }
    }

    #[test]
    fn read_file_sync_returns_binary_bytes() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        std::fs::write(root.path().join("image.bin"), [0, 1, 2, 3]).unwrap();

        let result = read_file_sync(&workspace, "image.bin").unwrap();

        match result {
            ReadFileResult::Binary(bytes) => assert_eq!(bytes, vec![0, 1, 2, 3]),
            ReadFileResult::Text { .. } => panic!("expected binary result"),
        }
    }

    #[test]
    fn read_file_sync_sniffs_unknown_extension_text_as_text() {
        // B11: an odd-suffix text file the extension classifier can't
        // type (here `.service`) must still open in the editor. Created
        // via std::fs because write_text only creates known-text paths.
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        std::fs::write(
            root.path().join("deploy.service"),
            "[Unit]\nDescription=demo\n",
        )
        .unwrap();

        match read_file_sync(&workspace, "deploy.service").unwrap() {
            ReadFileResult::Text { content, .. } => {
                assert_eq!(content, "[Unit]\nDescription=demo\n");
            }
            ReadFileResult::Binary(_) => panic!("expected sniffed text result"),
        }
    }

    #[test]
    fn list_files_sync_resolves_pending_kind_per_dir() {
        // Per-directory listings sniff Other-class files so the file
        // browser shows a final text/binary kind, never "pending".
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        std::fs::create_dir(root.path().join("cfg")).unwrap();
        std::fs::write(root.path().join("cfg/zshrc-like"), "export A=1\n").unwrap();
        std::fs::write(root.path().join("cfg/blob"), [0u8, 1, 2, 0]).unwrap();

        let out = list_files_sync(
            &workspace,
            ListFilesQuery {
                dir: Some("cfg".to_string()),
            },
        )
        .unwrap();

        let kind_of = |name: &str| {
            out.iter()
                .find(|e| e.path == format!("cfg/{name}"))
                .and_then(|e| e.kind)
        };
        assert_eq!(kind_of("zshrc-like"), Some("text"));
        assert_eq!(kind_of("blob"), Some("binary"));
    }

    #[test]
    fn stream_read_file_sync_emits_meta_chunks_done_in_order() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        workspace.write_text("note.md", "hello").unwrap();
        let mut lines = Vec::new();

        stream_read_file_sync(&workspace, "note.md", |bytes| {
            lines.push(String::from_utf8(bytes.to_vec()).unwrap());
            true
        })
        .unwrap();

        let events: Vec<serde_json::Value> = lines
            .iter()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(events[0]["type"], "meta");
        assert_eq!(events[0]["path"], "note.md");
        assert_eq!(events[0]["size"], 5);
        assert_eq!(events[1]["type"], "chunk");
        assert_eq!(events[1]["content"], "hello");
        assert_eq!(events[2]["type"], "done");
    }

    #[test]
    fn stream_read_file_sync_stops_when_emit_returns_false() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        workspace.write_text("note.md", "hello").unwrap();
        let mut lines = 0usize;

        stream_read_file_sync(&workspace, "note.md", |_| {
            lines += 1;
            false
        })
        .unwrap();

        assert_eq!(lines, 1);
    }

    #[test]
    fn query_flag_accepts_stream_one() {
        assert!(query_flag(&Some("1".to_string())));
        assert!(query_flag(&Some("true".to_string())));
        assert!(!query_flag(&None));
        assert!(!query_flag(&Some("0".to_string())));
    }

    #[test]
    fn download_path_sync_returns_editable_text_bytes() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        workspace.write_text("notes/readme.md", "hello\n").unwrap();

        let payload = download_path_sync(&workspace, "notes/readme.md").unwrap();

        match payload {
            DownloadPayload::File(bytes) => assert_eq!(bytes, b"hello\n"),
            DownloadPayload::DirectoryTar(_) => panic!("expected file download"),
        }
    }

    #[test]
    fn download_path_sync_archives_directory_tree() {
        use std::collections::BTreeMap;
        use std::io::Read;

        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        workspace.create_dir("notes").unwrap();
        workspace.create_dir("notes/deep").unwrap();
        workspace.write_text("notes/readme.md", "hello\n").unwrap();
        workspace
            .write_text("notes/deep/todo.txt", "todo\n")
            .unwrap();

        let payload = download_path_sync(&workspace, "notes").unwrap();

        let DownloadPayload::DirectoryTar(bytes) = payload else {
            panic!("expected directory archive");
        };
        let mut archive = tar::Archive::new(std::io::Cursor::new(bytes));
        let mut files = BTreeMap::new();
        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            if !entry.header().entry_type().is_file() {
                continue;
            }
            let path = entry.path().unwrap().to_string_lossy().into_owned();
            let mut body = String::new();
            entry.read_to_string(&mut body).unwrap();
            files.insert(path, body);
        }

        assert_eq!(
            files.get("notes/readme.md").map(String::as_str),
            Some("hello\n")
        );
        assert_eq!(
            files.get("notes/deep/todo.txt").map(String::as_str),
            Some("todo\n")
        );
    }

    #[test]
    fn download_content_disposition_uses_safe_basename() {
        assert_eq!(
            content_disposition_attachment("notes/readme.md"),
            "attachment; filename=\"readme.md\"",
        );
        assert_eq!(
            content_disposition_attachment("notes/bad\"name.md"),
            "attachment; filename=\"bad_name.md\"",
        );
        assert_eq!(
            content_disposition_archive("notes/bad:name"),
            "attachment; filename=\"bad_name.tar\"",
        );
    }

    #[test]
    fn api_read_file_wraps_sync_workspace_reads_in_spawn_blocking() {
        let source = include_str!("files.rs");

        assert!(source.contains(
            "tokio::task::spawn_blocking(move || read_file_sync(&workspace, &path_for_read))"
        ));
        assert!(source.contains(
            "tokio::task::spawn_blocking(move || download_path_sync(&workspace, &path_for_download))"
        ));
    }

    #[test]
    fn api_list_files_wraps_sync_workspace_walk_in_spawn_blocking() {
        let source = include_str!("files.rs");

        assert!(source
            .contains("tokio::task::spawn_blocking(move || list_files_sync(&workspace, query))"));
    }

    #[test]
    fn api_create_and_delete_wrap_sync_workspace_io_in_spawn_blocking() {
        let source = include_str!("files.rs");

        assert!(source
            .contains("tokio::task::spawn_blocking(move || create_file_sync(&workspace, body))"));
        assert!(source
            .contains("tokio::task::spawn_blocking(move || workspace.remove(&path_for_remove))"));
    }

    #[test]
    fn create_file_sync_rejects_existing_directory_collision() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        workspace.create_dir("notes").unwrap();

        let err = create_file_sync(
            &workspace,
            CreateBody {
                path: "notes".to_string(),
                is_dir: false,
                content: Some("body".to_string()),
            },
        )
        .unwrap_err();

        assert!(matches!(
            err,
            chan_workspace::ChanError::PathAlreadyExists(_)
        ));
    }

    #[test]
    fn write_file_sync_reports_seconds_conflict() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        workspace.write_text("note.md", "v1").unwrap();

        let err = write_file_sync(&workspace, "note.md", Some(0), None, "v2").unwrap_err();

        assert!(matches!(
            err,
            chan_workspace::ChanError::WriteConflict {
                current_mtime_ns: Some(_)
            }
        ));
        assert_eq!(workspace.read_text("note.md").unwrap(), "v1");
    }

    #[test]
    fn write_file_sync_reports_nanosecond_conflict() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        workspace.write_text("note.md", "v1").unwrap();

        let err = write_file_sync(&workspace, "note.md", None, Some(0), "v2").unwrap_err();

        assert!(matches!(
            err,
            chan_workspace::ChanError::WriteConflict {
                current_mtime_ns: Some(_)
            }
        ));
        assert_eq!(workspace.read_text("note.md").unwrap(), "v1");
    }

    #[test]
    fn write_file_sync_returns_new_mtime() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();

        let (mtime, mtime_ns) = write_file_sync(&workspace, "note.md", None, None, "v1").unwrap();

        assert!(mtime.is_some());
        assert!(mtime_ns.is_some());
        assert_eq!(workspace.read_text("note.md").unwrap(), "v1");
    }

    #[test]
    fn write_file_sync_accepts_matching_nanosecond_token() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        workspace.write_text("note.md", "v1").unwrap();
        let ns = workspace.stat("note.md").unwrap().mtime_ns.unwrap();

        let (_mtime, mtime_ns) =
            write_file_sync(&workspace, "note.md", Some(0), Some(ns), "v2").unwrap();

        assert!(mtime_ns.is_some());
        assert_eq!(workspace.read_text("note.md").unwrap(), "v2");
    }

    #[test]
    fn parse_optional_mtime_ns_rejects_bad_values() {
        assert_eq!(parse_optional_mtime_ns(None).unwrap(), None);
        assert_eq!(parse_optional_mtime_ns(Some("123")).unwrap(), Some(123));
        assert!(parse_optional_mtime_ns(Some("")).is_err());
        assert!(parse_optional_mtime_ns(Some("nope")).is_err());
    }
}

pub async fn api_delete_file(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
) -> Response {
    // chan-workspace's Workspace::remove handles files and EMPTY directories.
    // Recursive deletion of a non-empty directory is a deliberate
    // foot-gun guard; supporting it here would require either a new
    // chan-workspace API (`Workspace::remove_recursive`) or a server-side walk
    // that issues per-leaf removes. Tracked for a follow-up; current
    // behavior is "error out, frontend resolves the leaves itself".
    let workspace = match state.try_workspace() {
        Ok(workspace) => workspace,
        Err(e) => return err_state(&e),
    };
    // Register the self-write before the blocking remove so the
    // watcher's Removed event is suppressed without racing the await
    // (see api_write_file - noting after the await leaks a phantom
    // external-edit/removal event).
    state.self_writes.note(&path);
    let path_for_remove = path.clone();
    match tokio::task::spawn_blocking(move || workspace.remove(&path_for_remove)).await {
        Ok(Ok(())) => StatusCode::NO_CONTENT.into_response(),
        Ok(Err(e)) => err_from(&e),
        Err(join) => err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
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
    let workspace = match state.try_workspace() {
        Ok(workspace) => workspace,
        Err(e) => return err_state(&e),
    };
    let from = body.from.clone();
    let to = body.to.clone();
    // Rename emits two notify events on most kernels (a Removed at
    // `from` and a Created at `to`); the rewrite pass also touches
    // every rewritten source. Note the endpoints before the blocking
    // rename (paths known up front) and the rewritten sources inside
    // the task as the rewrite reports them - all BEFORE the await
    // returns, so neither half of any pair fires a phantom external-
    // edit prompt (noting after the await raced the watcher; see
    // api_write_file).
    state.self_writes.note(&body.from);
    state.self_writes.note(&body.to);
    let self_writes = Arc::clone(&state.self_writes);
    let outcome = match tokio::task::spawn_blocking(move || {
        let outcome = workspace.rename_with_link_rewrite(&from, &to)?;
        for path in &outcome.rewritten {
            self_writes.note(path);
        }
        Ok::<_, chan_workspace::ChanError>(outcome)
    })
    .await
    {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => return err_from(&e),
        Err(join) => return err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    };
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

/// Multi-entry move/copy for the File Browser clipboard + multi-drag
/// (FB capabilities). `op` selects move (cut/paste, drag) vs copy
/// (copy/paste); `sources` are the workspace-rooted POSIX paths of the
/// selection; `dest_dir` is the target directory ("" = workspace root).
#[derive(Deserialize)]
pub struct TransferBody {
    op: TransferOp,
    sources: Vec<String>,
    dest_dir: String,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransferOp {
    Move,
    Copy,
}

#[derive(Serialize, Default)]
struct TransferResponse {
    /// Per-source outcome, in request order: the final destination path
    /// each source landed at (after collision suffixing) plus the op.
    moved: Vec<TransferItem>,
    /// Sources skipped because the destination equals the source's
    /// current parent (a no-op move) or the source escaped the workspace.
    skipped: Vec<String>,
    /// Link-rewrite CAS conflicts accumulated across all moved entries.
    conflicts: Vec<String>,
}

#[derive(Serialize)]
struct TransferItem {
    from: String,
    to: String,
}

/// Basename of a workspace-rooted POSIX path.
fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Parent dir of a workspace-rooted POSIX path ("" for a top-level entry).
fn parent_dir(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..i],
        None => "",
    }
}

pub async fn api_fs_transfer(
    State(state): State<Arc<AppState>>,
    Json(body): Json<TransferBody>,
) -> Response {
    let workspace = match state.try_workspace() {
        Ok(workspace) => workspace,
        Err(e) => return err_state(&e),
    };
    let dest_dir = body.dest_dir.trim_end_matches('/').to_string();
    let op = body.op;
    let sources = body.sources.clone();

    // The whole batch runs on a blocking thread: each move does a
    // synchronous link-rewrite walk and each copy reads + writes N
    // files, both off the tokio worker pool.
    let dest_for_task = dest_dir.clone();
    // Note every created/moved/rewritten path INSIDE the blocking task,
    // as each workspace op reports it, so the watcher's Created/Removed
    // events are suppressed before the await returns. Noting after the
    // await (the old behavior) raced the watcher into firing phantom
    // external-edit prompts on files the user may have open. The
    // watcher still emits the events; the scoped `fs` registry routes
    // them to subscribed File Browser instances + the Graph.
    let self_writes = Arc::clone(&state.self_writes);
    let result = tokio::task::spawn_blocking(move || {
        let mut resp = TransferResponse::default();
        for src in &sources {
            let name = basename(src);
            // A move into the source's own current parent is a no-op
            // (and would otherwise resolve a needless " copy" suffix).
            if op == TransferOp::Move && parent_dir(src) == dest_for_task {
                resp.skipped.push(src.clone());
                continue;
            }
            // Resolve a non-colliding destination name; both copy and a
            // cut-into-a-collision get a Finder-style " copy" suffix so
            // we never overwrite.
            let dest = match workspace.resolve_free_name(&dest_for_task, name) {
                Ok(d) => d,
                Err(e) => return Err(e),
            };
            match op {
                TransferOp::Move => {
                    let outcome = workspace.rename_with_link_rewrite(src, &dest)?;
                    for (from, to) in &outcome.renamed {
                        self_writes.note(from);
                        self_writes.note(to);
                    }
                    for path in &outcome.rewritten {
                        self_writes.note(path);
                    }
                    resp.conflicts.extend(outcome.conflicts);
                }
                TransferOp::Copy => {
                    let outcome = workspace.copy(src, &dest)?;
                    for path in &outcome.created {
                        self_writes.note(path);
                    }
                }
            }
            self_writes.note(src);
            self_writes.note(&dest);
            resp.moved.push(TransferItem {
                from: src.clone(),
                to: dest,
            });
        }
        Ok::<_, chan_workspace::ChanError>(resp)
    })
    .await;

    let resp = match result {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => return err_from(&e),
        Err(join) => return err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    };
    Json(resp).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only Markdown (.md) is the `document` wire kind; .txt is editable +
    /// searchable but rides `text` alongside source/config files. Contacts
    /// and directories take their own branches ahead of the classifier.
    #[test]
    fn project_kind_marks_only_markdown_as_document() {
        assert_eq!(project_kind("notes/a.md", false, false), Some("document"));
        assert_eq!(project_kind("notes/plain.txt", false, false), Some("text"));
        assert_eq!(project_kind("src/main.rs", false, false), Some("text"));
        assert_eq!(project_kind("logo.png", false, false), Some("media"));
        // Unknown extension is "pending" from the path alone; the
        // per-directory listing sniff (list_files_sync) resolves it to
        // text/binary. project_kind is path-only and never sniffs.
        assert_eq!(project_kind("archive.zip", false, false), Some("pending"));
        // Contact frontmatter wins over the .md document mapping.
        assert_eq!(
            project_kind("contacts/alex.md", false, true),
            Some("contact")
        );
        // Directories carry no wire kind.
        assert_eq!(project_kind("notes", true, false), None);
    }

    #[test]
    fn file_response_serializes_path_class_for_inspector_payload() {
        let response = FileResponse {
            path: "notes/a.md".to_string(),
            content: "hello".to_string(),
            mtime: Some(1),
            mtime_ns: Some("1000000000".to_string()),
            path_class: Some(chan_workspace::PathClass {
                kind: chan_workspace::PathKind::RegularFile,
                permission: chan_workspace::PathPermission::ReadWrite,
                link_count: 2,
                target: None,
                target_escapes_workspace: false,
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
            path_class: Some(chan_workspace::PathClass {
                kind: chan_workspace::PathKind::Symlink,
                permission: chan_workspace::PathPermission::ReadWrite,
                link_count: 1,
                target: Some(std::path::PathBuf::from("/etc/hosts")),
                target_escapes_workspace: true,
            }),
            kind: Some("binary"),
        };

        let value = serde_json::to_value(entry).unwrap();
        assert_eq!(value["path_class"]["kind"], "symlink");
        assert_eq!(value["path_class"]["target"], "/etc/hosts");
        assert_eq!(value["path_class"]["target_escapes_workspace"], true);
    }

    #[test]
    fn transfer_body_deserializes_the_fb_clipboard_wire_shape() {
        // The FB clipboard + multi-drag posts this shape; pin it so a
        // wire change is an explicit edit, not silent client breakage.
        let body: TransferBody = serde_json::from_value(serde_json::json!({
            "op": "copy",
            "sources": ["notes/a.md", "notes/sub"],
            "dest_dir": "archive"
        }))
        .unwrap();
        assert!(matches!(body.op, TransferOp::Copy));
        assert_eq!(body.sources, vec!["notes/a.md", "notes/sub"]);
        assert_eq!(body.dest_dir, "archive");

        let mv: TransferBody = serde_json::from_value(serde_json::json!({
            "op": "move",
            "sources": ["x.md"],
            "dest_dir": ""
        }))
        .unwrap();
        assert!(matches!(mv.op, TransferOp::Move));
    }

    #[test]
    fn basename_and_parent_dir_split_workspace_rooted_paths() {
        assert_eq!(basename("notes/sub/a.md"), "a.md");
        assert_eq!(basename("top.md"), "top.md");
        assert_eq!(parent_dir("notes/sub/a.md"), "notes/sub");
        assert_eq!(parent_dir("top.md"), "");
    }

    #[cfg(unix)]
    #[test]
    fn directory_listing_keeps_symlink_with_path_class() {
        use std::os::unix::fs::symlink;

        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        std::fs::write(root.path().join("note.md"), "hi").unwrap();
        symlink("note.md", root.path().join("alias.md")).unwrap();

        let entries = list_dir_entries(&workspace, "").unwrap();
        assert!(entries.iter().any(|entry| entry.path == "alias.md"));
        let class = path_class_for_wire(&workspace, "alias.md").expect("symlink path class");
        assert_eq!(class.kind, chan_workspace::PathKind::Symlink);
    }
}
