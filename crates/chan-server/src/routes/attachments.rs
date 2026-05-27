//! POST /api/attachments: multipart upload from the editor.
//!
//! The frontend sends one part named `file`; we slugify the original
//! filename, prefix with the unix timestamp (collision resistance),
//! and write via Workspace::write_bytes (so the path sandbox + special-
//! file refusal apply). Returns the workspace-relative path the file
//! landed at, matching the frontend's `uploadAttachment` contract.
//!
//! Optional `dir` form field overrides the configured
//! `attachments_dir` so the editor can land an upload in the same
//! directory as the file being edited (markdown can then reference
//! it with a `./name` src). An empty `dir` saves at workspace root; an
//! absent `dir` falls back to `attachments_dir`. Workspace sandboxing
//! rejects `..` escape attempts so we don't validate manually here.

use std::sync::Arc;

use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::error::{err, err_from, err_state};
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
    // string for workspace root) wins; missing falls back to the
    // configured attachments_dir.
    let dir = match dir_override {
        Some(d) => d,
        None => match state.server_config.lock() {
            Ok(cfg) => cfg.attachments_dir.clone(),
            Err(_) => {
                return err(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "server config lock poisoned".into(),
                );
            }
        },
    };

    // Filename: <slugified-stem>.<ext>, kept close to what the user
    // pasted / uploaded so the markdown source reads naturally. On
    // collision in the target dir, append `-1`, `-2`, ... until we
    // find a free slot. The slug step strips path separators and
    // disallowed characters so a hostile filename can't escape the
    // chosen dir. Extension is lowercased so the browser's
    // content-type sniffer agrees with the editor's render.
    let (stem, ext) = split_filename(&original);
    let stem_slug = slugify_for_filename(stem);
    let stem_or_default = if stem_slug.is_empty() {
        "file".to_string()
    } else {
        stem_slug
    };
    let ext = ext.map(|e| e.to_ascii_lowercase()).unwrap_or_default();

    let workspace = match state.try_drive() {
        Ok(workspace) => workspace,
        Err(e) => return err_state(&e),
    };
    // Record the self-write inside the blocking task, before the
    // bytes hit disk. The fs watcher runs on its own thread and can
    // observe the write the instant it lands; noting after the
    // spawn_blocking await (the old behavior) leaves a window where
    // the watcher reaches should_suppress() before the path is
    // recorded, so an image paste surfaces as a phantom "external
    // edit". Cloning the Arc handle in lets us close that window.
    let self_writes = Arc::clone(&state.self_writes);
    let result = tokio::task::spawn_blocking(move || {
        let join_filename = |name: &str| -> String {
            if dir.is_empty() {
                name.to_owned()
            } else {
                format!("{dir}/{name}")
            }
        };
        let build_name = |suffix: Option<u32>| -> String {
            let base = match suffix {
                None => stem_or_default.clone(),
                Some(n) => format!("{stem_or_default}-{n}"),
            };
            if ext.is_empty() {
                base
            } else {
                format!("{base}.{ext}")
            }
        };
        let mut rel = join_filename(&build_name(None));
        let mut attempt: u32 = 1;
        // Hard cap on retries: if a thousand suffixes are taken the
        // user has bigger problems than this loop; bail to a unique
        // timestamp fallback rather than spinning forever.
        while workspace.exists(&rel) {
            if attempt > 1000 {
                let ts = now_unix_secs();
                let base = format!("{stem_or_default}-{ts}");
                let name = if ext.is_empty() {
                    base
                } else {
                    format!("{base}.{ext}")
                };
                rel = join_filename(&name);
                break;
            }
            rel = join_filename(&build_name(Some(attempt)));
            attempt += 1;
        }

        self_writes.note(&rel);
        workspace.write_bytes(&rel, &bytes)?;
        Ok::<_, chan_workspace::ChanError>(rel)
    })
    .await;
    let rel = match result {
        Ok(Ok(rel)) => rel,
        Ok(Err(e)) => return err_from(&e),
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("attachment write task panicked: {e}"),
            )
                .into_response();
        }
    };
    Json(serde_json::json!({ "path": rel })).into_response()
}
