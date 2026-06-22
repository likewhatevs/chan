//! Workspace-less file transfer for standalone-terminal windows.
//!
//! A standalone terminal (`kind=terminal`) has no workspace, so `cs upload` /
//! `cs download` cannot anchor at a workspace root. Per the scope decision,
//! transfers resolve against the terminal session's working directory with the
//! reach of the shell's own uid — there is no extra sandbox wall, since the
//! terminal already grants that filesystem access. The `cs` CLI absolutizes the
//! path against its cwd (the session cwd) before it reaches the control socket;
//! the control socket sends that absolute path with its leading `/` stripped so
//! the SPA's existing transfer bubble builds clean `/api/files/...` URLs. These
//! handlers — mounted only on the terminal tenant — re-root that path at `/` and
//! read or write it directly, so no SPA change is needed.
//!
//! Both directions pre-flight access before doing any work (fail fast, no
//! partial artifact): download verifies the source tree is readable before
//! building the tarball; upload verifies the destination directory is writable
//! before writing. [`verify_writable_dir`] also backs the workspace upload path
//! in `files.rs`.

use std::path::{Path, PathBuf};

use axum::extract::{Multipart, Path as AxumPath, Query};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::err;
use crate::routes::files::{
    content_disposition_archive, content_disposition_attachment, download_filename, query_flag,
    upload_leaf_filename,
};
use crate::static_assets::content_type_for;

/// Re-root a terminal-tenant `*path` (the control socket strips the leading
/// `/` before sending it) at the filesystem root. A standalone-terminal
/// transfer is uid-scoped, not workspace-scoped, so the path is always
/// absolute.
fn abs_from_terminal_path(path: &str) -> PathBuf {
    PathBuf::from("/").join(path.trim_start_matches('/'))
}

/// Pre-flight for download: every file under `abs` is openable for read and
/// every directory is listable. Fails fast on the first inaccessible entry so a
/// download never starts a tarball it cannot finish. The workspace path uses a
/// sibling guard in `files.rs` that walks via `Workspace::list` to match the
/// workspace tarball's `.chan`/`.git` filtering.
pub(crate) fn verify_readable_fs(abs: &Path) -> Result<(), String> {
    let meta = std::fs::symlink_metadata(abs)
        .map_err(|e| format!("cannot access {}: {e}", abs.display()))?;
    if meta.file_type().is_symlink() {
        // The archive stores the link itself; don't follow it (and don't fault
        // on a dangling target).
        return Ok(());
    }
    if meta.is_dir() {
        let entries = std::fs::read_dir(abs)
            .map_err(|e| format!("cannot read directory {}: {e}", abs.display()))?;
        for entry in entries {
            let entry =
                entry.map_err(|e| format!("cannot read directory {}: {e}", abs.display()))?;
            verify_readable_fs(&entry.path())?;
        }
        Ok(())
    } else {
        std::fs::File::open(abs)
            .map(|_| ())
            .map_err(|e| format!("cannot read {}: {e}", abs.display()))
    }
}

/// Pre-flight for upload: `dir` exists, is a directory, and accepts a new
/// entry. The writability check probes with a temp file it removes immediately
/// — the only check that also catches read-only mounts and ACLs a mode-bit test
/// misses, and the same operation `atomic_write` performs on every real write.
/// On failure nothing is written, so the caller can bail before transferring.
pub(crate) fn verify_writable_dir(dir: &Path) -> Result<(), String> {
    let meta = std::fs::metadata(dir)
        .map_err(|e| format!("cannot access destination {}: {e}", dir.display()))?;
    if !meta.is_dir() {
        return Err(format!("destination is not a directory: {}", dir.display()));
    }
    tempfile::Builder::new()
        .prefix(".chan-upload-check-")
        .tempfile_in(dir)
        .map(|_| ())
        .map_err(|e| format!("destination is not writable: {} ({e})", dir.display()))
}

#[derive(Default, Deserialize)]
pub(crate) struct TerminalDownloadQuery {
    #[serde(default)]
    download: Option<String>,
}

#[cfg_attr(test, derive(Debug))]
enum TerminalPayload {
    File { bytes: Vec<u8>, name: String },
    DirectoryTar { bytes: Vec<u8>, name: String },
}

/// `GET /api/files/*path?download=1` on the terminal tenant: stream the cwd /
/// uid-scoped file or a tar of the directory. Mounted only on the slim terminal
/// router, so `*path` is always a filesystem-absolute target (see
/// [`abs_from_terminal_path`]).
pub async fn api_terminal_read_file(
    AxumPath(path): AxumPath<String>,
    Query(query): Query<TerminalDownloadQuery>,
) -> Response {
    // The slim terminal tenant fetches no file content inline (no editor, no
    // file browser); the only legitimate GET here is the download gesture, so a
    // bare read is refused rather than serving arbitrary bytes.
    if !query_flag(&query.download) {
        return err(
            StatusCode::BAD_REQUEST,
            "terminal file route requires ?download=1".into(),
        );
    }
    let abs = abs_from_terminal_path(&path);
    let result = tokio::task::spawn_blocking(move || terminal_download_sync(&abs)).await;
    match result {
        Ok(Ok(TerminalPayload::File { bytes, name })) => (
            [
                (header::CONTENT_TYPE, content_type_for(&name).to_string()),
                (
                    header::CONTENT_DISPOSITION,
                    content_disposition_attachment(&name),
                ),
            ],
            bytes,
        )
            .into_response(),
        Ok(Ok(TerminalPayload::DirectoryTar { bytes, name })) => (
            [
                (header::CONTENT_TYPE, "application/x-tar".to_string()),
                (
                    header::CONTENT_DISPOSITION,
                    content_disposition_archive(&name),
                ),
            ],
            bytes,
        )
            .into_response(),
        // Pre-flight / IO failures are reported before any partial archive is
        // emitted (the tar is built fully in memory and dropped on error).
        Ok(Err(message)) => err(StatusCode::BAD_REQUEST, message),
        Err(join) => err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    }
}

fn terminal_download_sync(abs: &Path) -> Result<TerminalPayload, String> {
    let meta =
        std::fs::metadata(abs).map_err(|e| format!("cannot access {}: {e}", abs.display()))?;
    let name = download_filename(&abs.to_string_lossy());
    if meta.is_dir() {
        // Pre-flight the whole tree we are about to tar so an unreadable entry
        // fails fast instead of aborting a half-built archive.
        verify_readable_fs(abs)?;
        let mut builder = tar::Builder::new(Vec::new());
        builder
            .append_dir_all(&name, abs)
            .map_err(|e| format!("archive {}: {e}", abs.display()))?;
        builder
            .finish()
            .map_err(|e| format!("archive {}: {e}", abs.display()))?;
        let bytes = builder
            .into_inner()
            .map_err(|e| format!("archive {}: {e}", abs.display()))?;
        Ok(TerminalPayload::DirectoryTar { bytes, name })
    } else {
        // A single file is read into memory; a permission error surfaces here
        // with nothing sent (no partial response body).
        let bytes =
            std::fs::read(abs).map_err(|e| format!("cannot read {}: {e}", abs.display()))?;
        Ok(TerminalPayload::File { bytes, name })
    }
}

#[derive(Serialize)]
#[cfg_attr(test, derive(Debug))]
struct TerminalUploadResponse {
    path: String,
    size: u64,
}

/// `POST /api/files/upload` on the terminal tenant: write the uploaded file into
/// the cwd / uid-scoped `dir`. No replace (`path`) flow — the slim tenant has no
/// file browser. Mounted only on the terminal router, so `dir` is absolute.
pub async fn api_terminal_upload_file(mut multipart: Multipart) -> Response {
    let mut chosen: Option<(String, Vec<u8>)> = None;
    let mut dir = String::new();
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
                                return err(StatusCode::BAD_REQUEST, format!("multipart read: {e}"))
                            }
                        };
                        chosen = Some((filename, bytes));
                    }
                    "dir" => match field.text().await {
                        Ok(s) => dir = s,
                        Err(e) => {
                            return err(StatusCode::BAD_REQUEST, format!("multipart read: {e}"))
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
    let abs_dir = abs_from_terminal_path(&dir);
    let result =
        tokio::task::spawn_blocking(move || terminal_upload_sync(&abs_dir, &filename, &bytes))
            .await;
    match result {
        Ok(Ok(response)) => Json(response).into_response(),
        Ok(Err(message)) => err(StatusCode::BAD_REQUEST, message),
        Err(join) => err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("file upload task panicked: {join}"),
        ),
    }
}

fn terminal_upload_sync(
    abs_dir: &Path,
    original_name: &str,
    bytes: &[u8],
) -> Result<TerminalUploadResponse, String> {
    // Pre-flight: bail before consuming-to-disk if the destination is not a
    // writable directory (write nothing on failure).
    verify_writable_dir(abs_dir)?;
    let leaf = upload_leaf_filename(original_name).map_err(|e| e.to_string())?;
    let target = abs_dir.join(&leaf);
    if target.exists() {
        return Err(format!("already exists: {}", target.display()));
    }
    // `atomic_write` writes a temp file and renames, so a failure leaves no
    // partial file at `target`.
    chan_workspace::fs_ops::atomic_write(&target, bytes)
        .map_err(|e| format!("cannot write {}: {e}", target.display()))?;
    Ok(TerminalUploadResponse {
        path: target.display().to_string(),
        size: bytes.len() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abs_from_terminal_path_reroots_at_filesystem_root() {
        assert_eq!(
            abs_from_terminal_path("home/u/proj/foo.txt"),
            PathBuf::from("/home/u/proj/foo.txt")
        );
        // Defensive: a leading slash (shouldn't happen — the control socket
        // strips it) is tolerated, not doubled.
        assert_eq!(
            abs_from_terminal_path("/etc/hosts"),
            PathBuf::from("/etc/hosts")
        );
    }

    #[test]
    fn verify_readable_fs_passes_a_readable_tree_and_names_an_unreadable_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), b"a").unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("b.txt"), b"b").unwrap();
        assert!(verify_readable_fs(dir.path()).is_ok());

        let missing = dir.path().join("nope.txt");
        let e = verify_readable_fs(&missing).unwrap_err();
        assert!(e.contains("nope.txt"), "error should name the path: {e}");
    }

    #[test]
    fn verify_writable_dir_rejects_a_nondirectory_and_missing_path() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("f");
        std::fs::write(&file, b"x").unwrap();
        assert!(verify_writable_dir(dir.path()).is_ok());

        let not_dir = verify_writable_dir(&file).unwrap_err();
        assert!(not_dir.contains("not a directory"), "{not_dir}");

        let missing = verify_writable_dir(&dir.path().join("gone")).unwrap_err();
        assert!(missing.contains("cannot access destination"), "{missing}");
    }

    #[test]
    fn terminal_upload_writes_into_dir_and_refuses_existing_target() {
        let dir = tempfile::tempdir().unwrap();
        let resp = terminal_upload_sync(dir.path(), "note.txt", b"hello").unwrap();
        assert_eq!(resp.size, 5);
        assert_eq!(
            std::fs::read(dir.path().join("note.txt")).unwrap(),
            b"hello"
        );
        // A second upload of the same name is refused (no silent overwrite).
        let again = terminal_upload_sync(dir.path(), "note.txt", b"world").unwrap_err();
        assert!(again.contains("already exists"), "{again}");
        assert_eq!(
            std::fs::read(dir.path().join("note.txt")).unwrap(),
            b"hello"
        );
    }

    #[test]
    fn terminal_upload_writes_nothing_when_destination_is_unwritable() {
        // A path whose parent is a file, not a directory: the dest cannot be a
        // writable directory, so the upload must fail before writing.
        let dir = tempfile::tempdir().unwrap();
        let as_file = dir.path().join("file");
        std::fs::write(&as_file, b"x").unwrap();
        let under_file = as_file.join("sub");
        let e = terminal_upload_sync(&under_file, "x.txt", b"data").unwrap_err();
        assert!(
            e.contains("cannot access destination") || e.contains("not a directory"),
            "{e}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn verify_readable_fs_rejects_an_unreadable_file_before_tarring() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let secret = dir.path().join("secret.txt");
        std::fs::write(&secret, b"x").unwrap();
        std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o000)).unwrap();
        // Root bypasses permission bits; only assert when the chmod truly denies
        // (skip under a root test runner).
        if std::fs::File::open(&secret).is_ok() {
            return;
        }
        let e = verify_readable_fs(dir.path()).unwrap_err();
        assert!(e.contains("secret.txt"), "error should name the file: {e}");
    }

    #[test]
    fn terminal_download_tars_a_directory_and_reads_a_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("one.txt"), b"one").unwrap();

        match terminal_download_sync(&dir.path().join("one.txt")).unwrap() {
            TerminalPayload::File { bytes, name } => {
                assert_eq!(bytes, b"one");
                assert_eq!(name, "one.txt");
            }
            _ => panic!("expected a file payload"),
        }
        match terminal_download_sync(dir.path()).unwrap() {
            TerminalPayload::DirectoryTar { bytes, .. } => assert!(!bytes.is_empty()),
            _ => panic!("expected a directory tar"),
        }
        let missing = terminal_download_sync(&dir.path().join("missing")).unwrap_err();
        assert!(missing.contains("cannot access"), "{missing}");
    }
}
