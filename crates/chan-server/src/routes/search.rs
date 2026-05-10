//! Filename + content search and indexer status/rebuild.
//!
//! `/api/search/files` is a server-side substring scan of `list_tree`
//! (chan-drive has no built-in filename index; the cost is linear and
//! the drive size budget is small). `/api/search/content` defers to
//! `Drive::search` (BM25 today, hybrid when the `embeddings` feature
//! is on). `/api/index/status` and `/api/index/rebuild` surface the
//! background indexer's state machine.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chan_drive::SearchOpts;
use serde::{Deserialize, Serialize};

use crate::error::err_from;
use crate::state::AppState;

/// Filename search params. Empty `q` returns the first `limit`
/// files in the tree, mirroring the [[ picker's empty state.
#[derive(Deserialize)]
pub struct FileSearchParams {
    #[serde(default)]
    q: String,
    #[serde(default = "default_search_limit")]
    limit: usize,
}

fn default_search_limit() -> usize {
    50
}

/// Server-side filename match: walk the tree, keep regular files
/// whose basename contains `q` (case-insensitive). chan-drive has
/// no built-in filename index since the cost (scan list_tree) is
/// linear and the drive size budget is small. Revisit if profiles
/// show this hot.
pub async fn api_search_files(
    State(state): State<Arc<AppState>>,
    Query(p): Query<FileSearchParams>,
) -> Response {
    let tree = match state.drive().list_tree() {
        Ok(t) => t,
        Err(e) => return err_from(&e),
    };
    let needle = p.q.to_lowercase();
    let mut hits = Vec::new();
    for entry in tree {
        if entry.is_dir {
            continue;
        }
        let basename = std::path::Path::new(&entry.path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if needle.is_empty() || basename.contains(&needle) {
            hits.push(entry);
            if hits.len() >= p.limit {
                break;
            }
        }
    }
    Json(hits).into_response()
}

#[derive(Deserialize)]
pub struct ContentSearchParams {
    #[serde(default)]
    q: String,
    #[serde(default = "default_content_limit")]
    limit: u32,
    /// Optional subdir scope (POSIX rel path under the drive root).
    /// Mirrors chan-drive's `SearchOpts::scope`.
    #[serde(default)]
    scope: Option<String>,
}

fn default_content_limit() -> u32 {
    20
}

/// `/api/search/content` view. Frontend's `ContentSearchResponse`
/// is a flat hit list; chan-drive's `SearchResults` wraps per-file
/// hits with a sub-array of snippets. We expand each snippet to its
/// own ContentHit so the result palette can show one row per
/// matching section. start_line isn't surfaced by chan-drive today;
/// synthesized as 0 (the frontend sorts by score, not line).
#[derive(Serialize)]
struct ContentSearchResponse {
    /// True when the index is ready to serve queries. chan-drive
    /// opens the index lazily and is always ready once a drive is
    /// open; kept as an explicit field so a future "rebuilding"
    /// state can land without a contract break.
    ready: bool,
    /// Mode actually used. "bm25" today (chan-drive's tantivy
    /// search); "hybrid" / "semantic" reserved for the dense
    /// retrieval that lands with the embeddings feature.
    mode: &'static str,
    hits: Vec<ContentHit>,
}

#[derive(Serialize)]
struct ContentHit {
    path: String,
    chunk_id: String,
    heading: String,
    start_line: u32,
    snippet: String,
    score: f32,
}

pub async fn api_search_content(
    State(state): State<Arc<AppState>>,
    Query(p): Query<ContentSearchParams>,
) -> Response {
    if p.q.trim().is_empty() {
        return Json(ContentSearchResponse {
            ready: true,
            mode: "hybrid",
            hits: Vec::new(),
        })
        .into_response();
    }
    let opts = SearchOpts {
        limit: p.limit,
        scope: p.scope.clone(),
        // Mode defaults to Hybrid via SearchOpts::default; the
        // facade's BM25 fallback kicks in when the binary is built
        // without `embeddings`.
        ..Default::default()
    };
    let results = match state.drive().search(&p.q, &opts) {
        Ok(r) => r,
        Err(e) => return err_from(&e),
    };
    let hits = results
        .hits
        .into_iter()
        .map(|h| ContentHit {
            path: h.path,
            chunk_id: h.chunk_id,
            heading: h.heading,
            start_line: u32::try_from(h.start_line).unwrap_or(u32::MAX),
            snippet: h.snippet,
            score: h.score,
        })
        .collect();
    Json(ContentSearchResponse {
        ready: results.ready,
        mode: results.mode,
        hits,
    })
    .into_response()
}

/// Index status snapshot. Reads the live `IndexStatus` from the
/// background indexer; shape mirrors the frontend's IndexStatus
/// tagged union (Settings -> Search index). The indexer flips
/// the snapshot to Building / Reindexing while a pass is in
/// flight and to Idle (with chunk + vector counts plus the
/// embedding model id) when it settles.
pub async fn api_index_status(State(state): State<Arc<AppState>>) -> Response {
    Json(state.indexer().snapshot()).into_response()
}

/// Trigger a full reindex of the drive (search + graph). Routed
/// through the background indexer's coordinator so the request
/// coalesces with anything already queued and the status
/// snapshot transitions cleanly through Building -> Idle.
/// Returns 202 Accepted: the work runs in the background and
/// progress is observable via `/api/index/status`.
pub async fn api_index_rebuild(State(state): State<Arc<AppState>>) -> Response {
    state.indexer().request_rebuild();
    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"queued": true})),
    )
        .into_response()
}
