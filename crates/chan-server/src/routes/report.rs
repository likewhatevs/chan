//! Per-file / per-prefix code report (chan-report).
//!
//! Surfaces the rolled-up chan-report data the file inspector renders
//! alongside size / mtime. Two read-only endpoints, both auth-gated by
//! the standard middleware:
//!
//!   - `GET /api/report/file?path=<rel>` returns the single
//!     `FileStats` row for one file, or 404 if the path is not in the
//!     index (binary, gitignored, language unknown to tokei).
//!   - `GET /api/report/prefix?path=<rel>` returns the per-folder
//!     roll-up: totals, by_language, cocomo. The per-file `files`
//!     array is dropped from the response since folders can fan out
//!     to thousands of rows; the inspector only needs the summary.
//!     Empty `path` means the entire drive.
//!
//! Both endpoints lazily trigger chan-drive's initial report scan on
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
    let report = match state
        .drive()
        .report_for_files(std::slice::from_ref(&p.path))
    {
        Ok(r) => r,
        Err(e) => return err_from(&e),
    };
    match report.files.into_iter().find(|f| f.path == p.path) {
        Some(stats) => Json(stats).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Folder roll-up: totals + per-language + COCOMO. The per-file
/// `files` array is dropped since folders can fan out to thousands of
/// rows and the inspector renders only the summary. Empty `path`
/// returns the whole-drive roll-up.
pub async fn api_report_prefix(
    State(state): State<Arc<AppState>>,
    Query(p): Query<ReportPathParams>,
) -> Response {
    let report = match if p.path.is_empty() {
        state.drive().report()
    } else {
        state.drive().report_for_prefix(&p.path)
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
}
