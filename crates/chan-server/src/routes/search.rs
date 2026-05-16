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
use chan_drive::{classify, FileClass, NodeKind, SearchOpts};
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
    let drive = state.drive();
    let tree = match drive.list_tree() {
        Ok(t) => t,
        Err(e) => return err_from(&e),
    };
    // Contact-kind notes have their own picker (`@<query>`), so skip
    // them from the `[[` autocomplete. `graph()` may be unavailable
    // very early in the lifecycle (index not yet open); in that case
    // we fall back to returning all matches rather than blocking the
    // search.
    let graph = drive.graph().ok();
    let needle = p.q.to_lowercase();
    let mut hits = Vec::new();
    // Two-pass collection so editable-text notes (.md / .txt) sort
    // ahead of binary assets. Linking a `[](image.png)` is legal
    // markdown and we allow it, but the [[ picker's primary use is
    // navigating between notes; surfacing those first keeps the
    // picker feeling note-shaped without hiding any file.
    let mut notes = Vec::new();
    let mut others = Vec::new();
    for entry in tree {
        if entry.is_dir {
            continue;
        }
        // Match against the full path (lowercased) so directory names
        // count as a prefix the user can type. Typing "reci" finds
        // every file under "Recipes/" even when the basename doesn't
        // contain "reci".
        let full = entry.path.to_lowercase();
        if !needle.is_empty() && !full.contains(&needle) {
            continue;
        }
        if let Some(g) = &graph {
            if let Ok(Some(NodeKind::Contact)) = g.node_kind(&entry.path) {
                continue;
            }
        }
        if matches!(classify(&entry.path), FileClass::EditableText) {
            notes.push(entry);
        } else if others.len() < p.limit {
            // Once we have `limit` non-note candidates buffered there
            // is no way more of them survive the final truncate, so
            // skip buffering further to bound memory.
            others.push(entry);
        }
        if notes.len() >= p.limit {
            // Enough notes to fill the response on their own; no need
            // to keep scanning for fallback candidates.
            break;
        }
    }
    hits.extend(notes);
    hits.extend(others);
    hits.truncate(p.limit);
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

/// `/api/search/content` view. The search index can return multiple
/// matching chunks/headings per file; the UI wants one row per file,
/// carrying the best-ranked heading/snippet for that path.
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

#[derive(Debug, Clone, PartialEq, Serialize)]
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
    let response_limit = normalized_content_limit(p.limit);
    let opts = SearchOpts {
        limit: expanded_content_candidate_limit(response_limit),
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
    let hits = collapse_hits_by_file(
        results.hits.into_iter().map(ContentHit::from),
        response_limit,
    );
    Json(ContentSearchResponse {
        ready: results.ready,
        mode: results.mode,
        hits,
    })
    .into_response()
}

impl From<chan_drive::Hit> for ContentHit {
    fn from(h: chan_drive::Hit) -> Self {
        Self {
            path: h.path,
            chunk_id: h.chunk_id,
            heading: h.heading,
            start_line: u32::try_from(h.start_line).unwrap_or(u32::MAX),
            snippet: h.snippet,
            score: h.score,
        }
    }
}

fn normalized_content_limit(limit: u32) -> u32 {
    if limit == 0 {
        default_content_limit()
    } else {
        limit
    }
}

fn expanded_content_candidate_limit(limit: u32) -> u32 {
    let widened = limit.saturating_mul(8);
    let cap = limit.max(200);
    widened.min(cap)
}

/// Collapse score-descending search hits to the best hit per file.
fn collapse_hits_by_file<I>(hits: I, limit: u32) -> Vec<ContentHit>
where
    I: IntoIterator<Item = ContentHit>,
{
    let mut out: Vec<ContentHit> = Vec::new();
    for hit in hits {
        if out.iter().any(|existing| existing.path == hit.path) {
            continue;
        }
        out.push(hit);
        if out.len() >= limit as usize {
            break;
        }
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(path: &str, heading: &str, score: f32) -> ContentHit {
        ContentHit {
            path: path.to_string(),
            chunk_id: format!("{path}:{heading}"),
            heading: heading.to_string(),
            start_line: 1,
            snippet: heading.to_string(),
            score,
        }
    }

    #[test]
    fn collapse_hits_by_file_keeps_first_ranked_heading() {
        let hits = collapse_hits_by_file(
            vec![
                hit("a.md", "best", 10.0),
                hit("b.md", "only", 8.0),
                hit("a.md", "lower", 2.0),
            ],
            20,
        );

        assert_eq!(
            hits,
            vec![hit("a.md", "best", 10.0), hit("b.md", "only", 8.0)]
        );
    }

    #[test]
    fn collapse_hits_by_file_honors_limit_after_dedup() {
        let hits = collapse_hits_by_file(
            vec![
                hit("a.md", "best", 10.0),
                hit("a.md", "lower", 2.0),
                hit("b.md", "next", 1.0),
            ],
            1,
        );

        assert_eq!(hits, vec![hit("a.md", "best", 10.0)]);
    }

    #[test]
    fn expanded_content_candidate_limit_broadens_small_queries() {
        assert_eq!(normalized_content_limit(0), default_content_limit());
        assert_eq!(expanded_content_candidate_limit(20), 160);
        assert_eq!(expanded_content_candidate_limit(50), 200);
        assert_eq!(expanded_content_candidate_limit(500), 500);
    }
}
