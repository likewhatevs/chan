//! POST /api/attachments — multipart upload from the editor.
//!
//! The frontend sends one part named `file`; we slugify the original
//! filename, prefix with the unix timestamp (collision resistance),
//! and write via Drive::write_bytes (so the path sandbox + special-
//! file refusal apply). Returns the drive-relative path the file
//! landed at, matching the frontend's `uploadAttachment` contract.
//!
//! Optional `dir` form field overrides the configured
//! `attachments_dir` so the editor can land an upload in the same
//! directory as the file being edited (markdown can then reference
//! it with a `./name` src). An empty `dir` saves at drive root; an
//! absent `dir` falls back to `attachments_dir`. Drive sandboxing
//! rejects `..` escape attempts so we don't validate manually here.

use std::sync::Arc;

use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::error::{err, err_from};
use crate::signal::now_unix_secs;
use crate::state::AppState;
use crate::util::{slugify_for_filename, split_filename};

pub async fn api_post_attachment(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Response {
    // Walk every multipart field once: we want both the file and
    // the optional `dir` override, and a streaming multipart parser
    // doesn't let us re-read parts. Order on the wire is up to the
    // client; pick the first `file` field we see and take the last
    // `dir` field (so a duplicate doesn't silently win the wrong
    // way).
    let mut chosen: Option<(String, Vec<u8>)> = None;
    let mut dir_override: Option<String> = None;
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
                        Ok(s) => dir_override = Some(s),
                        Err(e) => {
                            return err(StatusCode::BAD_REQUEST, format!("multipart read: {e}"));
                        }
                    },
                    _ => {}
                }
            }
            Ok(None) => break,
            Err(e) => {
                return err(StatusCode::BAD_REQUEST, format!("multipart parse: {e}"));
            }
        }
    }

    let Some((original, bytes)) = chosen else {
        return err(
            StatusCode::BAD_REQUEST,
            "missing `file` part in multipart body".into(),
        );
    };

    if bytes.is_empty() {
        return err(StatusCode::BAD_REQUEST, "empty file".into());
    }

    // Resolve the target dir: caller-supplied `dir` (incl. empty
    // string for drive root) wins; missing falls back to the
    // configured attachments_dir.
    let dir = match dir_override {
        Some(d) => d,
        None => state.server_config.lock().unwrap().attachments_dir.clone(),
    };

    // Filename: <unix_ts>-<slugified-stem>.<ext>. Keeping the
    // unix timestamp at the front gives natural sort + collision
    // resistance without committing to a date format the frontend
    // would parse. Extension is preserved (lowercased) so the
    // browser's content-type sniffer agrees with what the editor
    // wrote.
    let (stem, ext) = split_filename(&original);
    let stem_slug = slugify_for_filename(stem);
    let stem_or_default = if stem_slug.is_empty() {
        "file"
    } else {
        &stem_slug
    };
    let ext = ext.map(|e| e.to_ascii_lowercase()).unwrap_or_default();
    let ts = now_unix_secs();
    let saved = if ext.is_empty() {
        format!("{ts}-{stem_or_default}")
    } else {
        format!("{ts}-{stem_or_default}.{ext}")
    };
    let rel = if dir.is_empty() {
        saved
    } else {
        format!("{dir}/{saved}")
    };

    if let Err(e) = state.drive().write_bytes(&rel, &bytes) {
        return err_from(&e);
    }
    state.self_writes.note(&rel);
    Json(serde_json::json!({ "path": rel })).into_response()
}
