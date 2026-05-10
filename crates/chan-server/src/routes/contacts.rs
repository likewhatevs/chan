//! Contact routes:
//!
//!   POST /api/contacts/import — multipart import of a contact CSV.
//!   GET  /api/contacts        — list (and filter) contact notes.
//!
//! The list route powers the editor `@` picker: caller passes an
//! optional `?q=` substring; we case-insensitive-match against
//! display title and basename, cap at `?limit=` (default 10), and
//! return drive-relative paths plus display labels. The wiki-link
//! the picker inserts is what re-resolves to the same Contact node
//! on the next graph pass, so the round-trip stays consistent.
//!
//! Wraps `Drive::import_contacts`. The frontend wizard (and the
//! `chan contacts import csv` CLI for parity) sends:
//!
//!   file       multipart   the CSV bytes
//!   dest_dir   text        drive-relative folder (created if absent;
//!                          empty string writes at the drive root)
//!   provider   text        "google" today; flag is forward-compat
//!   overwrite  text        "true" / "false" (default false)
//!
//! Response shape per plan §7:
//!   {
//!     "wrote":     ["Contacts/Jane Doe.md", ...],
//!     "overwrote": [...],
//!     "skipped":   [{"path": "...", "reason": "exists"}, ...],
//!     "failed":    [{"name": "...", "reason": "..."}, ...]
//!   }
//!
//! Per-file errors do not fail the request: a single bad slug
//! lands as `failed` and the rest of the batch goes through.
//! Setup-level failures (parse error, dest_dir creation refused
//! by the path sandbox) return 400.

use std::sync::Arc;

use axum::extract::{Multipart, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

use chan_drive::contacts::{google::parse_google_csv, ImportOpts, ImportOutcome, ProviderKind};

use crate::error::{err, err_from};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ContactsListQuery {
    /// Case-insensitive substring filter on display title + basename.
    /// Empty / absent returns the full alphabetical list, capped by
    /// `limit`.
    #[serde(default)]
    pub q: Option<String>,
    /// Result cap. The picker is fine with 10 by default; bump for
    /// power-user / debug callers.
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    10
}

pub async fn api_get_contacts(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ContactsListQuery>,
) -> Response {
    let needle = q.q.unwrap_or_default().trim().to_lowercase();
    let all = match state.drive().contacts() {
        Ok(v) => v,
        Err(e) => return err_from(&e),
    };
    let mut out = Vec::with_capacity(q.limit.min(all.len()));
    for c in all {
        if !needle.is_empty() {
            let title_ok = c
                .title
                .as_deref()
                .map(|t| t.to_lowercase().contains(&needle))
                .unwrap_or(false);
            let basename_ok = c.basename.to_lowercase().contains(&needle);
            if !title_ok && !basename_ok {
                continue;
            }
        }
        // Picker rows show the title primarily; basename is the
        // fallback when the imported file has no `# H1` (rare, but
        // possible if the user edited the markdown).
        let label = c.title.clone().unwrap_or_else(|| c.basename.clone());
        out.push(serde_json::json!({
            "path": c.rel_path,
            "label": label,
        }));
        if out.len() >= q.limit {
            break;
        }
    }
    Json(out).into_response()
}

pub async fn api_post_contacts_import(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Response {
    let mut csv_bytes: Option<Vec<u8>> = None;
    let mut dest_dir: Option<String> = None;
    let mut provider: Option<String> = None;
    let mut overwrite = false;

    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                let name = field.name().unwrap_or("").to_owned();
                match name.as_str() {
                    "file" if csv_bytes.is_none() => match field.bytes().await {
                        Ok(b) => csv_bytes = Some(b.to_vec()),
                        Err(e) => {
                            return err(StatusCode::BAD_REQUEST, format!("multipart read: {e}"));
                        }
                    },
                    "dest_dir" => match field.text().await {
                        Ok(s) => dest_dir = Some(s),
                        Err(e) => {
                            return err(StatusCode::BAD_REQUEST, format!("multipart read: {e}"));
                        }
                    },
                    "provider" => match field.text().await {
                        Ok(s) => provider = Some(s),
                        Err(e) => {
                            return err(StatusCode::BAD_REQUEST, format!("multipart read: {e}"));
                        }
                    },
                    "overwrite" => match field.text().await {
                        Ok(s) => {
                            // Tolerant parse: anything that isn't a
                            // recognized truthy string is false. The
                            // wizard sends "true"/"false"; curl users
                            // sending "1" / "yes" should also work.
                            let s = s.trim().to_ascii_lowercase();
                            overwrite = matches!(s.as_str(), "true" | "1" | "yes" | "on");
                        }
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

    let Some(bytes) = csv_bytes else {
        return err(
            StatusCode::BAD_REQUEST,
            "missing `file` part in multipart body".into(),
        );
    };
    let Some(dest_dir) = dest_dir else {
        return err(
            StatusCode::BAD_REQUEST,
            "missing `dest_dir` part in multipart body".into(),
        );
    };

    let provider_str = provider.unwrap_or_else(|| "google".into());
    let prov = match ProviderKind::parse(&provider_str) {
        Some(p) => p,
        None => {
            return err(
                StatusCode::BAD_REQUEST,
                format!("unknown provider: {provider_str}"),
            );
        }
    };
    if prov != ProviderKind::Google {
        return err(
            StatusCode::BAD_REQUEST,
            "only provider=google is supported today".into(),
        );
    }

    let contacts = match parse_google_csv(bytes.as_slice()) {
        Ok(v) => v,
        Err(e) => return err(StatusCode::BAD_REQUEST, format!("csv parse: {e}")),
    };

    let summary = match state
        .drive()
        .import_contacts(&dest_dir, contacts, ImportOpts { overwrite })
    {
        Ok(s) => s,
        Err(e) => return err_from(&e),
    };

    // Tell the watcher these paths were our own writes so the
    // editor doesn't see a flood of "external edit" events.
    for o in &summary.outcomes {
        match o {
            ImportOutcome::Wrote { path } | ImportOutcome::Overwrote { path } => {
                state.self_writes.note(path);
            }
            _ => {}
        }
    }

    let mut wrote = Vec::new();
    let mut overwrote = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();
    for o in summary.outcomes {
        match o {
            ImportOutcome::Wrote { path } => wrote.push(path),
            ImportOutcome::Overwrote { path } => overwrote.push(path),
            ImportOutcome::Skipped { path, reason } => {
                skipped.push(serde_json::json!({ "path": path, "reason": reason }));
            }
            ImportOutcome::Failed { name, reason } => {
                failed.push(serde_json::json!({ "name": name, "reason": reason }));
            }
        }
    }

    Json(serde_json::json!({
        "wrote": wrote,
        "overwrote": overwrote,
        "skipped": skipped,
        "failed": failed,
    }))
    .into_response()
}
