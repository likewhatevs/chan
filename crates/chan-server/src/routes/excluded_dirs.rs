//! `GET` / `PUT /api/index/excluded-dirs`: the per-workspace directory
//! blocklist.
//!
//! Hybrid model: the global machine-wide baseline
//! (`Registry::index_excluded_dirs`) is READ-ONLY here; each workspace adds
//! its own `excluded_dirs` (in the per-workspace `IndexConfig`), and the walk
//! the index + graph rebuild use is `effective = union(defaults, additions)`.
//! This route edits ONLY the per-workspace additions.
//!
//! Names are exact directory BASENAMES matched at any depth, case-insensitive
//! (no globs, no paths). On `PUT` the re-walk is heavy filesystem + CPU work,
//! so it runs OFF the tokio executor: we persist the new set (a small config
//! write) and queue a rebuild via the indexer (`request_rebuild`, the same
//! off-executor path `POST /api/index/rebuild` uses); the handler returns
//! promptly rather than blocking the runtime on the re-walk.

use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::{err, err_from, err_state};
use crate::state::AppState;

#[derive(Debug, Serialize)]
struct ExcludedDirsView {
    /// Global machine-wide baseline (`Registry::index_excluded_dirs`).
    /// Read-only on this route; shown so the UI can render what the
    /// per-workspace additions sit on top of.
    defaults: Vec<String>,
    /// This workspace's own additions (the editable set).
    workspace: Vec<String>,
    /// `union(defaults, workspace)`: what the index + graph walk actually
    /// skips for this workspace.
    effective: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct PutBody {
    /// The full replacement set of per-workspace additions.
    workspace: Vec<String>,
}

fn view(ws: &chan_workspace::Workspace) -> Result<ExcludedDirsView, chan_workspace::ChanError> {
    Ok(ExcludedDirsView {
        defaults: ws.global_excluded_dirs(),
        workspace: ws.excluded_dirs()?,
        effective: ws.effective_excluded_dirs()?,
    })
}

pub async fn api_excluded_dirs_get(State(state): State<Arc<AppState>>) -> Response {
    let workspace = match state.try_workspace() {
        Ok(w) => w,
        Err(e) => return err_state(&e),
    };
    match view(&workspace) {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_from(&e),
    }
}

/// Normalize the requested set: trim, drop blanks, reject path separators
/// (a name, not a path), lower-case (matching is case-insensitive), dedupe.
/// Returns the clean set, or the offending raw entry on a hard reject.
fn normalize(raw: &[String]) -> Result<Vec<String>, String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for entry in raw {
        let name = entry.trim();
        if name.is_empty() {
            continue;
        }
        if name.contains('/') || name.contains('\\') {
            return Err(entry.clone());
        }
        let lower = name.to_ascii_lowercase();
        if seen.insert(lower.clone()) {
            out.push(lower);
        }
    }
    Ok(out)
}

pub async fn api_excluded_dirs_put(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PutBody>,
) -> Response {
    let dirs = match normalize(&body.workspace) {
        Ok(d) => d,
        Err(bad) => {
            return err(
                StatusCode::BAD_REQUEST,
                format!("excluded dir must be a bare name, not a path: {bad:?}"),
            )
        }
    };
    let workspace = match state.try_workspace() {
        Ok(w) => w,
        Err(e) => return err_state(&e),
    };
    if let Err(e) = workspace.set_excluded_dirs(dirs) {
        return err_from(&e);
    }
    // Re-walk OFF the tokio loop: queue a rebuild (the indexer spawn_blocks
    // it). The reindex re-derives the effective filter from the now-persisted
    // config, so the new blocklist takes effect on the walk. A workspace
    // without an indexer (shouldn't happen for a served workspace) just skips
    // the re-walk; the config is still persisted + applies on next open.
    if let Ok(indexer) = state.try_indexer() {
        indexer.request_rebuild();
    }
    match view(&workspace) {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_from(&e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_trims_drops_blanks_lowercases_and_dedupes() {
        let got = normalize(&[
            "  Vendor ".to_string(),
            "vendor".to_string(),
            "".to_string(),
            "  ".to_string(),
            "NodeModules".to_string(),
        ])
        .unwrap();
        assert_eq!(got, vec!["vendor".to_string(), "nodemodules".to_string()]);
    }

    #[test]
    fn normalize_rejects_path_separators() {
        assert!(normalize(&["a/b".to_string()]).is_err());
        assert!(normalize(&["a\\b".to_string()]).is_err());
    }
}
