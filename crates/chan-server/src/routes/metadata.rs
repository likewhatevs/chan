//! Chan metadata archive routes.
//!
//! The CLI owns path-based import/export commands. The web surface
//! exposes a browser-safe export endpoint so users can download a
//! manifest-first `.tar.zst` archive without giving the browser host
//! filesystem paths.

use std::path::Path;
use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use chan_drive::{Library, MetadataExportOptions};

use crate::error::{err, err_from};
use crate::state::AppState;

struct MetadataExportDownload {
    bytes: Vec<u8>,
    filename: String,
    files: usize,
    size: u64,
}

pub async fn api_metadata_export(State(state): State<Arc<AppState>>) -> Response {
    let library = state.library.clone();
    let drive_root = state.drive_root.clone();
    let result =
        tokio::task::spawn_blocking(move || export_metadata_download(&library, &drive_root)).await;

    match result {
        Ok(Ok(download)) => metadata_download_response(download),
        Ok(Err(e)) => err_from(&e),
        Err(join) => err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    }
}

fn export_metadata_download(
    library: &Library,
    drive_root: &Path,
) -> chan_drive::Result<MetadataExportDownload> {
    let tmp = tempfile::tempdir()?;
    let archive = tmp.path().join("chan-metadata.tar.zst");
    let report = library.export_metadata_archive(
        drive_root,
        &archive,
        MetadataExportOptions {
            chan_version: env!("CARGO_PKG_VERSION").to_string(),
        },
    )?;
    let bytes = std::fs::read(&archive)?;
    Ok(MetadataExportDownload {
        bytes,
        filename: format!(
            "chan-metadata-{}.tar.zst",
            safe_filename_fragment(&report.manifest.source_metadata_key)
        ),
        files: report.files,
        size: report.bytes,
    })
}

fn metadata_download_response(download: MetadataExportDownload) -> Response {
    let mut response = download.bytes.into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/zstd"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", download.filename))
            .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
    );
    if let Ok(value) = HeaderValue::from_str(&download.files.to_string()) {
        headers.insert("x-chan-metadata-files", value);
    }
    if let Ok(value) = HeaderValue::from_str(&download.size.to_string()) {
        headers.insert("x-chan-metadata-bytes", value);
    }
    response
}

fn safe_filename_fragment(value: &str) -> String {
    let out: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "drive".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_filename_fragment_strips_path_characters() {
        assert_eq!(safe_filename_fragment("/tmp/drive root"), "tmp-drive-root");
        assert_eq!(safe_filename_fragment(""), "drive");
    }

    #[test]
    fn export_metadata_download_returns_archive_bytes() {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_drive::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_drive(root.path()).unwrap();
        let drive = lib.open_drive(root.path()).unwrap();
        drive.write_text("note.md", "hello").unwrap();
        drop(drive);

        let download = export_metadata_download(&lib, root.path()).unwrap();

        assert!(download.filename.ends_with(".tar.zst"));
        assert!(!download.bytes.is_empty());
    }
}
