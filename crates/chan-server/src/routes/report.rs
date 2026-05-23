//! Per-file / per-prefix / per-directory code report (chan-report).
//!
//! Surfaces the rolled-up chan-report data the file inspector renders
//! alongside size / mtime. Three read-only endpoints, all auth-gated
//! by the standard middleware:
//!
//!   - `GET /api/report/file?path=<rel>` returns the single
//!     `FileStats` row for one file, or 404 if the path is not in the
//!     index (binary, gitignored, language unknown to tokei).
//!   - `GET /api/report/prefix?path=<rel>` returns the per-directory
//!     roll-up: totals, by_language, cocomo. Walks the file map every
//!     call (O(N) over the matching prefix). Use this when the caller
//!     wants prefix-string semantics (e.g. "everything starting with
//!     'src/lib'" — including the file `src/lib.rs` itself if it
//!     exists). The per-file `files` array is dropped from the
//!     response since directories can fan out to thousands of rows;
//!     the inspector only needs the summary. Empty `path` means the
//!     entire drive.
//!   - `GET /api/report/dir?path=<rel>` returns the same shape as
//!     `prefix`, but reads from the O(1) maintained directory
//!     aggregation cache. Strict directory semantics: only files
//!     under the directory contribute (a file named identically to
//!     a directory would not). Empty `path` is the drive root.
//!     Responds 404 when no tracked file lives at or under `path`.
//!     This is the endpoint the graph overhaul's directory inspector
//!     (G3) consumes per-click without re-walking the file map.
//!
//! All endpoints lazily trigger chan-drive's initial report scan on
//! first call; subsequent calls hit the warm in-memory index.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chan_drive::{CocomoSummary, ReportLanguageStats, ReportTotals};
use serde::{Deserialize, Serialize};

use crate::error::err_from;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ReportPathParams {
    #[serde(default)]
    path: String,
}

#[derive(Serialize)]
pub struct PrefixReport {
    totals: ReportTotals,
    by_language: Vec<ReportLanguageStats>,
    cocomo: CocomoSummary,
}

async fn blocking_response(
    f: impl FnOnce() -> Response + Send + 'static,
    label: &'static str,
) -> Response {
    match tokio::task::spawn_blocking(f).await {
        Ok(response) => response,
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("{label} task panicked: {e}"),
        )
            .into_response(),
    }
}

/// Per-file report row. 404 when the file is not indexed; an empty
/// `path` is rejected with 400 since the file endpoint is path-keyed
/// (use `/api/report/prefix` with an empty path for the whole-drive
/// roll-up).
pub async fn api_report_file(
    State(state): State<Arc<AppState>>,
    Query(p): Query<ReportPathParams>,
) -> Response {
    if p.path.trim().is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let drive = state.drive();
    blocking_response(
        move || {
            let report = match drive.report_for_files(std::slice::from_ref(&p.path)) {
                Ok(r) => r,
                Err(e) => return err_from(&e),
            };
            match report.files.into_iter().find(|f| f.path == p.path) {
                Some(stats) => Json(stats).into_response(),
                None => StatusCode::NOT_FOUND.into_response(),
            }
        },
        "report file",
    )
    .await
}

/// Directory roll-up: totals + per-language + COCOMO. The per-file
/// `files` array is dropped since directories can fan out to thousands of
/// rows and the inspector renders only the summary. Empty `path`
/// returns the whole-drive roll-up.
pub async fn api_report_prefix(
    State(state): State<Arc<AppState>>,
    Query(p): Query<ReportPathParams>,
) -> Response {
    let drive = state.drive();
    blocking_response(
        move || {
            let report = match if p.path.is_empty() {
                drive.report()
            } else {
                drive.report_for_prefix(&p.path)
            } {
                Ok(r) => r,
                Err(e) => return err_from(&e),
            };
            Json(PrefixReport {
                totals: report.totals,
                by_language: report.by_language,
                cocomo: report.cocomo,
            })
            .into_response()
        },
        "report prefix",
    )
    .await
}

/// Per-directory roll-up via the maintained O(1) cache. Same
/// response shape as `api_report_prefix` but reads from the cache
/// instead of walking the file map. 404 when the directory has no
/// tracked files. Empty `path` returns the drive root.
pub async fn api_report_dir(
    State(state): State<Arc<AppState>>,
    Query(p): Query<ReportPathParams>,
) -> Response {
    let drive = state.drive();
    blocking_response(
        move || {
            let report = match drive.report_for_dir(&p.path) {
                Ok(Some(r)) => r,
                Ok(None) => return StatusCode::NOT_FOUND.into_response(),
                Err(e) => return err_from(&e),
            };
            Json(PrefixReport {
                totals: report.totals,
                by_language: report.by_language,
                cocomo: report.cocomo,
            })
            .into_response()
        },
        "report dir",
    )
    .await
}
