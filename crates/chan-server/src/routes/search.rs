//! Filename + content search and indexer status/rebuild.
//!
//! `/api/search/files` is a server-side substring scan of `list_tree`
//! (chan-drive has no built-in filename index; the cost is linear and
//! the drive size budget is small). `/api/search/content` defers to
//! `Drive::search` (BM25 today, hybrid when the `embeddings` feature
//! is on). `/api/index/status` and `/api/index/rebuild` surface the
//! background indexer's state machine.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chan_drive::{classify, fs_ops, FileClass, NodeKind, SearchOpts, TreeEntry};
use serde::{Deserialize, Serialize};

use crate::error::{err_from, err_state};
use crate::indexer::IndexStatus;
use crate::state::AppState;

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
    blocking_response(
        move || {
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
        },
        "file search",
    )
    .await
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
    let drive = state.drive();
    let query = p.q;
    blocking_response(
        move || {
            let results = match drive.search(&query, &opts) {
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
        },
        "content search",
    )
    .await
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
    match state.try_indexer() {
        Ok(indexer) => Json(indexer.snapshot()).into_response(),
        Err(e) => err_state(&e),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IndexingStateResponse {
    root: String,
    nodes: Vec<IndexingStateNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IndexingStateNode {
    path: String,
    state: IndexingDirectoryState,
    children_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexingDirectoryState {
    Indexed,
    Indexing,
    Pending,
}

#[derive(Default)]
struct DirectoryStateAccum {
    children_count: usize,
    indexable_files: usize,
    indexed_files: usize,
    indexing: bool,
}

/// Dir-only indexing state for the empty-pane carousel. The server
/// derives the view from the same filtered tree used by reindexing
/// plus the persisted BM25 path snapshot, avoiding any parse/embed
/// work on the request path.
pub async fn api_indexing_state(State(state): State<Arc<AppState>>) -> Response {
    let drive = match state.try_drive() {
        Ok(drive) => drive,
        Err(e) => return err_state(&e),
    };
    let indexer = match state.try_indexer() {
        Ok(indexer) => indexer,
        Err(e) => return err_state(&e),
    };
    let current_file = current_index_file(indexer.snapshot());
    blocking_response(
        move || {
            let entries = match fs_ops::list_tree_filtered(drive.root(), drive.walk_filter()) {
                Ok(entries) => entries,
                Err(e) => return err_from(&e),
            };
            let indexed_paths = match drive.indexed_paths() {
                Ok(paths) => paths.into_iter().collect::<BTreeSet<_>>(),
                Err(e) => {
                    tracing::warn!(error = %e, "indexing-state: failed to snapshot indexed paths");
                    BTreeSet::new()
                }
            };
            Json(build_indexing_state(
                &entries,
                &indexed_paths,
                current_file.as_deref(),
            ))
            .into_response()
        },
        "indexing state",
    )
    .await
}

fn current_index_file(status: IndexStatus) -> Option<String> {
    match status {
        IndexStatus::Building { file, .. } | IndexStatus::Reindexing { file } => Some(file),
        IndexStatus::Idle { .. } | IndexStatus::Error { .. } => None,
    }
}

fn build_indexing_state(
    entries: &[TreeEntry],
    indexed_paths: &BTreeSet<String>,
    current_file: Option<&str>,
) -> IndexingStateResponse {
    let mut dirs = BTreeMap::<String, DirectoryStateAccum>::new();
    dirs.insert(String::new(), DirectoryStateAccum::default());

    for entry in entries.iter().filter(|entry| entry.is_dir) {
        dirs.entry(entry.path.clone()).or_default();
        dirs.entry(parent_dir(&entry.path).to_string())
            .or_default()
            .children_count += 1;
    }

    for entry in entries
        .iter()
        .filter(|entry| !entry.is_dir && fs_ops::is_indexable_text(&entry.path))
    {
        for dir in ancestor_dirs_for_file(&entry.path) {
            let accum = dirs.entry(dir).or_default();
            accum.indexable_files += 1;
            if indexed_paths.contains(&entry.path) {
                accum.indexed_files += 1;
            }
            if current_file == Some(entry.path.as_str()) {
                accum.indexing = true;
            }
        }
    }

    let nodes = dirs
        .into_iter()
        .map(|(path, accum)| {
            let state = if accum.indexing {
                IndexingDirectoryState::Indexing
            } else if accum.indexable_files > 0 && accum.indexed_files == accum.indexable_files {
                IndexingDirectoryState::Indexed
            } else {
                IndexingDirectoryState::Pending
            };
            IndexingStateNode {
                path,
                state,
                children_count: accum.children_count,
            }
        })
        .collect();

    IndexingStateResponse {
        root: String::new(),
        nodes,
    }
}

fn ancestor_dirs_for_file(path: &str) -> Vec<String> {
    let mut dirs = vec![String::new()];
    let mut rel = path;
    while let Some((parent, _name)) = rel.rsplit_once('/') {
        dirs.push(parent.to_string());
        rel = parent;
    }
    dirs
}

fn parent_dir(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(parent, _name)| parent)
}

/// Trigger a full reindex of the drive (search + graph). Routed
/// through the background indexer's coordinator so the request
/// coalesces with anything already queued and the status
/// snapshot transitions cleanly through Building -> Idle.
/// Returns 202 Accepted: the work runs in the background and
/// progress is observable via `/api/index/status`.
pub async fn api_index_rebuild(State(state): State<Arc<AppState>>) -> Response {
    match state.try_indexer() {
        Ok(indexer) => {
            indexer.request_rebuild();
            (
                StatusCode::ACCEPTED,
                Json(serde_json::json!({"queued": true})),
            )
                .into_response()
        }
        Err(e) => err_state(&e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;
    use std::sync::{Mutex, RwLock};

    use axum::body::Body;
    use axum::http::{header, Request, StatusCode};
    use chan_drive::{NoProgress, SearchAggression};
    use tempfile::TempDir;
    use tokio::sync::{broadcast, watch};
    use tower::ServiceExt;

    use crate::self_writes::SelfWrites;
    use crate::state::{AppState, DriveCell};
    use crate::terminal_sessions::{Registry as TerminalRegistry, RegistryConfig};
    use crate::{EditorPrefs, ServerConfig};

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

    fn tree_entry(path: &str, is_dir: bool) -> TreeEntry {
        TreeEntry {
            path: path.to_string(),
            is_dir,
            mtime: None,
            size: 0,
        }
    }

    #[test]
    fn indexing_state_shape_uses_dir_states_only() {
        let entries = vec![
            tree_entry("indexed", true),
            tree_entry("indexed/done.md", false),
            tree_entry("indexing", true),
            tree_entry("indexing/live.md", false),
            tree_entry("pending", true),
            tree_entry("pending/todo.md", false),
            tree_entry("assets", true),
            tree_entry("assets/logo.png", false),
        ];
        let indexed_paths = BTreeSet::from([
            "indexed/done.md".to_string(),
            "indexing/live.md".to_string(),
        ]);

        let response = build_indexing_state(&entries, &indexed_paths, Some("indexing/live.md"));

        assert_eq!(response.root, "");
        assert!(response
            .nodes
            .iter()
            .all(|node| !node.path.ends_with(".md")));
        assert_eq!(
            response
                .nodes
                .iter()
                .find(|node| node.path == "indexed")
                .map(|node| node.state),
            Some(IndexingDirectoryState::Indexed)
        );
        assert_eq!(
            response
                .nodes
                .iter()
                .find(|node| node.path == "indexing")
                .map(|node| node.state),
            Some(IndexingDirectoryState::Indexing)
        );
        assert_eq!(
            response
                .nodes
                .iter()
                .find(|node| node.path == "pending")
                .map(|node| node.state),
            Some(IndexingDirectoryState::Pending)
        );
        assert_eq!(
            response
                .nodes
                .iter()
                .find(|node| node.path.is_empty())
                .map(|node| node.children_count),
            Some(4)
        );

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["root"], "");
        assert_eq!(json["nodes"][0]["path"], "");
        assert!(matches!(
            json["nodes"][0]["state"].as_str(),
            Some("indexed" | "indexing" | "pending")
        ));
        assert!(json["nodes"][0]["children_count"].is_u64());
    }

    struct RouteTestApp {
        _cfg: TempDir,
        _root: TempDir,
        state: Arc<AppState>,
    }

    fn route_test_app() -> RouteTestApp {
        let cfg = TempDir::new().unwrap();
        let root = TempDir::new().unwrap();
        let lib = chan_drive::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_drive(root.path()).unwrap();
        let drive = lib.open_drive(root.path()).unwrap();
        drive.write_text("notes/done.md", "# done\n").unwrap();
        drive.write_text("notes/todo.md", "# todo\n").unwrap();
        drive.index_file("notes/done.md").unwrap();

        let (events_tx, _) = broadcast::channel::<String>(1);
        let (index_events_tx, _) = broadcast::channel::<chan_drive::WatchEvent>(1);
        let indexer = Arc::new(crate::indexer::Indexer::spawn(
            drive.clone(),
            index_events_tx.subscribe(),
            false,
            SearchAggression::Conservative,
            Arc::new(NoProgress),
        ));
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        std::mem::forget(shutdown_tx);

        let state = Arc::new(AppState {
            library: lib,
            drive_root: root.path().to_path_buf(),
            drive_cell: Arc::new(RwLock::new(Some(DriveCell {
                drive,
                watch_handle: None,
                indexer,
            }))),
            token: Some("secret".to_string()),
            prefix: Arc::new(RwLock::new(String::new())),
            settings_disabled: false,
            tunnel_public: false,
            last_activity: Arc::new(AtomicU64::new(0)),
            events_tx,
            index_events_tx,
            server_config: Mutex::new(ServerConfig::default()),
            editor_prefs: Mutex::new(EditorPrefs::default()),
            self_writes: Arc::new(SelfWrites::new()),
            terminal_sessions: Arc::new(TerminalRegistry::new(RegistryConfig {
                drive_root: root.path().to_path_buf(),
                mcp_socket_path: None,
                control_socket_path: None,
                terminal: ServerConfig::default().terminal,
            })),
            shutdown_rx,
            loaded_teams: std::sync::Mutex::new(std::collections::HashMap::new()),
            scope_registry: std::sync::Arc::new(crate::bus::ScopeRegistry::new()),
        });

        RouteTestApp {
            _cfg: cfg,
            _root: root,
            state,
        }
    }

    #[tokio::test]
    async fn indexing_state_endpoint_requires_auth() {
        let app = route_test_app();
        let router = crate::router(app.state);
        let request = Request::builder()
            .uri("/api/indexing/state")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn indexing_state_endpoint_returns_dir_nodes() {
        let app = route_test_app();
        let router = crate::router(app.state);
        let request = Request::builder()
            .uri("/api/indexing/state")
            .header(header::AUTHORIZATION, "Bearer secret")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["root"], "");
        let nodes = json["nodes"].as_array().unwrap();
        assert!(nodes.iter().any(|node| node["path"] == ""));
        assert!(nodes.iter().any(|node| node["path"] == "notes"));
        assert!(nodes.iter().all(|node| {
            matches!(
                node["state"].as_str(),
                Some("indexed" | "indexing" | "pending")
            )
        }));
        assert!(nodes.iter().all(|node| node["children_count"].is_u64()));
    }
}
