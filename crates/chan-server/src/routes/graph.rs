//! `[[ ]]` typeahead, link resolution, headings, and the unified
//! graph view.
//!
//! Two-phase typeahead UX. Phase 1: as the user types `[[Re...`, the
//! picker hits /api/link-targets to surface candidate files. Phase 2:
//! after the user picks a file (`[[recipes/pasta.md`), they may type
//! `#` to jump to a heading; the picker hits /api/headings/<rel> to
//! enumerate the file's anchors.
//!
//! The graph endpoints (links / graph / backlinks) walk chan-drive's
//! per-file accessors and stitch them into the unified `{ nodes,
//! edges }` shape the frontend visualization expects.

use std::sync::Arc;

use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chan_drive::EdgeKind;
use serde::{Deserialize, Serialize};

use crate::error::err_from;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct LinkTargetsParams {
    #[serde(default)]
    q: String,
    #[serde(default = "default_link_limit")]
    limit: u32,
}

fn default_link_limit() -> u32 {
    20
}

pub async fn api_link_targets(
    State(state): State<Arc<AppState>>,
    Query(p): Query<LinkTargetsParams>,
) -> Response {
    match state.drive().link_targets(&p.q, p.limit) {
        Ok(targets) => Json(targets).into_response(),
        Err(e) => err_from(&e),
    }
}

#[derive(Deserialize)]
pub struct ResolveLinkParams {
    /// Wiki-link target as written, e.g. `recipes/pasta` or
    /// `recipes/pasta#ingredients`. Pass through verbatim from
    /// the editor; chan-drive handles the .md / .txt extension
    /// fallback and the anchor split.
    target: String,
}

/// Resolve a wiki-link target to an existing drive file. 404
/// when no file matches the candidates; this lets the editor's
/// click handler render a "broken link / create?" affordance.
pub async fn api_resolve_link(
    State(state): State<Arc<AppState>>,
    Query(p): Query<ResolveLinkParams>,
) -> Response {
    match state.drive().resolve_link(&p.target) {
        Some(resolved) => Json(resolved).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn api_headings(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
) -> Response {
    let drive = state.drive();
    let graph = match drive.graph() {
        Ok(g) => g,
        Err(e) => return err_from(&e),
    };
    match graph.headings_of(&path) {
        Ok(headings) => Json(headings).into_response(),
        Err(e) => err_from(&e),
    }
}

// chan-drive's GraphView exposes per-file accessors (neighbors,
// backlinks, headings_of) and bulk reads (files, tags). It does
// NOT expose an "all edges" call, so /api/links and /api/graph
// walk the file list and accumulate. For typical drive sizes the
// O(n) sqlite round-trip is fine; if profiles show this hot we
// add a chan-drive helper.

/// All link-kind edges in the drive. Mention and tag edges are
/// excluded; the graph view fetches those via /api/graph. The
/// shape is `[Edge]` so the frontend can render the link-only
/// view without a follow-up request.
pub async fn api_links(State(state): State<Arc<AppState>>) -> Response {
    let drive = state.drive();
    let graph = match drive.graph() {
        Ok(g) => g,
        Err(e) => return err_from(&e),
    };
    let files = match graph.files() {
        Ok(f) => f,
        Err(e) => return err_from(&e),
    };
    let mut edges = Vec::new();
    for f in &files {
        match graph.neighbors(f) {
            Ok(es) => edges.extend(es.into_iter().filter(|e| matches!(e.kind, EdgeKind::Link))),
            Err(e) => return err_from(&e),
        }
    }
    Json(edges).into_response()
}

/// `/api/graph` view. Frontend's `GraphView` type is unified
/// `{ nodes, edges }`; chan-drive exposes per-kind primitives
/// (files / tags / neighbors). This handler walks the graph DB and
/// emits the unified shape so the visualization can render without
/// per-kind glue on the frontend side.
///
/// Node kinds: file (one per indexed path), tag (#name), mention
/// (@@name). Date nodes from the typescript type aren't emitted;
/// chan-drive's EdgeKind has no date variant today.
#[derive(Serialize)]
struct GraphViewResponse {
    nodes: Vec<GraphNodeView>,
    edges: Vec<GraphEdgeView>,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum GraphNodeView {
    File {
        id: String,
        label: String,
        path: String,
        /// True for ghost nodes synthesized as the target of a
        /// broken link. Frontend renders them muted.
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        missing: bool,
    },
    Tag {
        id: String,
        label: String,
    },
    Mention {
        id: String,
        label: String,
    },
}

#[derive(Serialize)]
struct GraphEdgeView {
    source: String,
    target: String,
    /// "link" | "tag" | "mention". Lowercase to match the
    /// frontend's GraphViewEdgeKind type.
    kind: &'static str,
    /// Only meaningful for link edges: true when the link resolves
    /// to a missing file. Other kinds skip the field.
    #[serde(skip_serializing_if = "Option::is_none")]
    broken: Option<bool>,
}

fn edge_kind_tag(k: EdgeKind) -> &'static str {
    match k {
        EdgeKind::Link => "link",
        EdgeKind::Tag => "tag",
        EdgeKind::Mention => "mention",
    }
}

/// Derive the file-node label from a drive-relative path. Strips
/// the `.md` / `.txt` extension and uses the basename so the graph
/// renders "recipes/pasta" as just "pasta" without losing the path
/// (the file node carries the full path on its `path` field).
fn file_label(rel: &str) -> String {
    let stem = std::path::Path::new(rel)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| rel.to_string());
    stem
}

/// Image-extension predicate. Mirrors the frontend's classifyFile
/// regex (`png|jpe?g|gif|webp|svg|avif|bmp`). The graph route uses
/// this to enrich the file set with image files referenced by
/// markdown so a `![](pic.png)` lands on a real file node instead
/// of a ghost. Keep both predicates in sync.
fn is_image_path(rel: &str) -> bool {
    let ext = std::path::Path::new(rel)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase());
    matches!(
        ext.as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "avif" | "bmp")
    )
}

/// Resolve a markdown link-edge target to an indexed drive file when
/// possible. chan-drive stores link targets verbatim from the source
/// (e.g. `[link](my%20note.md)` -> dst = `"my%20note.md"`); without
/// this rewrite, every URL-encoded or source-relative target ends up
/// as a non-clickable "ghost" node in the inspector.
///
/// Resolution order, first hit wins:
///   1. Decoded target as drive-relative (with `.md` / `.txt` /
///      exact tries), matching wiki-style link semantics.
///   2. Decoded target joined to the source file's parent directory
///      (handles `./peer.md`, `../sibling/note.md`, and bare leaves
///      authored relative to the source).
///
/// On miss, returns the percent-decoded target so the ghost node
/// gets a clean label ("my note") instead of "my%20note".
fn resolve_link_dst(src: &str, target: &str, files: &std::collections::BTreeSet<&str>) -> String {
    use percent_encoding::percent_decode_str;
    use std::path::Path;

    let decoded = percent_decode_str(target).decode_utf8_lossy().into_owned();
    let stripped = decoded.trim_start_matches('/');

    let mut candidates: Vec<String> = vec![stripped.to_string()];
    if let Some(parent) = Path::new(src).parent() {
        if !parent.as_os_str().is_empty() {
            let joined = parent.join(stripped);
            if let Some(norm) = normalize_drive_rel(&joined) {
                candidates.push(norm);
            }
        }
    }
    for cand in &candidates {
        for try_path in [cand.clone(), format!("{cand}.md"), format!("{cand}.txt")] {
            if files.contains(try_path.as_str()) {
                return try_path;
            }
        }
    }
    decoded
}

/// Collapse `.` / `..` components against a drive-relative path.
/// Returns None if the result would escape the drive root or if the
/// path includes an absolute prefix. Always emits `/` separators so
/// the result matches drive-relative file-set keys on Windows too,
/// where `PathBuf::to_string_lossy` would otherwise yield `\`.
fn normalize_drive_rel(p: &std::path::Path) -> Option<String> {
    use std::path::Component;
    let mut parts: Vec<String> = Vec::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::Normal(s) => parts.push(s.to_string_lossy().into_owned()),
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(parts.join("/"))
}

/// Collect drive image files (non-directory, image extension).
/// Returns an empty set on `list_tree` failure so callers degrade to
/// the old ghost-rendering path instead of failing the request.
fn drive_image_files(drive: &chan_drive::Drive) -> std::collections::BTreeSet<String> {
    match drive.list_tree() {
        Ok(entries) => entries
            .into_iter()
            .filter(|e| !e.is_dir && is_image_path(&e.path))
            .map(|e| e.path)
            .collect(),
        Err(_) => std::collections::BTreeSet::new(),
    }
}

pub async fn api_graph(State(state): State<Arc<AppState>>) -> Response {
    let drive = state.drive();
    let graph = match drive.graph() {
        Ok(g) => g,
        Err(e) => return err_from(&e),
    };
    let files = match graph.files() {
        Ok(f) => f,
        Err(e) => return err_from(&e),
    };
    let tags = match graph.tags() {
        Ok(t) => t,
        Err(e) => return err_from(&e),
    };
    let mut all_edges = Vec::new();
    for f in &files {
        match graph.neighbors(f) {
            Ok(es) => all_edges.extend(es),
            Err(e) => return err_from(&e),
        }
    }

    // Image files aren't graph nodes (the indexer skips non-text
    // files), so a markdown `![alt](pic.png)` would otherwise resolve
    // to a ghost. Image files merged into the resolution set so an
    // existing image lands on a real file node (the frontend then
    // styles file-kind nodes by extension via classifyFile).
    let image_files = drive_image_files(&drive);

    let mut file_set: std::collections::BTreeSet<&str> = files.iter().map(String::as_str).collect();
    for img in &image_files {
        file_set.insert(img.as_str());
    }

    // Rewrite link-edge targets so URL-encoded / source-relative
    // markdown links land on the real file node (clickable in the
    // inspector). Genuine ghosts get the decoded form so the label
    // reads "my note" instead of "my%20note".
    for e in all_edges.iter_mut() {
        if matches!(e.kind, EdgeKind::Link) {
            e.dst = resolve_link_dst(&e.src, &e.dst, &file_set);
        }
    }

    // Track which image files are actually referenced by an edge so
    // we only emit nodes for images that participate in the graph.
    // Unreferenced images would inflate the node count without
    // adding any edges — purely visual noise.
    let mut referenced_images: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    for e in &all_edges {
        if matches!(e.kind, EdgeKind::Link) && image_files.contains(&e.dst) {
            referenced_images.insert(e.dst.clone());
        }
    }

    // Build the node list. File nodes for every indexed path; tag
    // nodes per #tag; mention nodes per distinct @@name. Image
    // file nodes for any image actually referenced by an edge (the
    // indexer skips images, so they aren't in `files` even when
    // they exist on disk). Ghost file nodes for unresolved link
    // targets so the graph shows broken links as dangling muted
    // nodes.
    let mut nodes: Vec<GraphNodeView> = Vec::new();
    for path in &files {
        nodes.push(GraphNodeView::File {
            id: path.clone(),
            label: file_label(path),
            path: path.clone(),
            missing: false,
        });
    }
    for img in &referenced_images {
        nodes.push(GraphNodeView::File {
            id: img.clone(),
            label: file_label(img),
            path: img.clone(),
            missing: false,
        });
    }
    for tag in &tags {
        nodes.push(GraphNodeView::Tag {
            id: format!("#{}", tag.name),
            label: format!("#{}", tag.name),
        });
    }
    let mut mention_set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut ghost_set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for e in &all_edges {
        // Defensive: skip edges that resolved to an empty dst.
        // Cytoscape rejects nodes with an empty string id; without
        // this filter a legacy database with an empty-target edge
        // (pre-empty-target-skip extractor) blocks the whole graph
        // render with "Can not create element with invalid string
        // ID ``".
        if e.dst.is_empty() {
            continue;
        }
        match e.kind {
            EdgeKind::Mention => {
                mention_set.insert(e.dst.clone());
            }
            EdgeKind::Link => {
                if !file_set.contains(e.dst.as_str()) {
                    ghost_set.insert(e.dst.clone());
                }
            }
            EdgeKind::Tag => {}
        }
    }
    for m in &mention_set {
        nodes.push(GraphNodeView::Mention {
            id: m.clone(),
            label: m.clone(),
        });
    }
    for ghost in &ghost_set {
        nodes.push(GraphNodeView::File {
            id: ghost.clone(),
            label: file_label(ghost),
            path: ghost.clone(),
            missing: true,
        });
    }

    let edges: Vec<GraphEdgeView> = all_edges
        .iter()
        // Same defensive guard as the node-set above: an edge with
        // an empty endpoint would point at a node we never created
        // (ghosts / mentions / tags filter empty dsts), and
        // Cytoscape errors on empty source/target ids the same as
        // empty node ids.
        .filter(|e| !e.src.is_empty() && !e.dst.is_empty())
        .map(|e| GraphEdgeView {
            source: e.src.clone(),
            // chan-drive stores the leading `#` / `@@` sigil on the
            // tag/mention edge's dst already (Drive::build_edges
            // does the formatting), and the matching tag node ids
            // we emit above use the same `#name` shape. So the
            // wire-shape target is the plain dst with no extra
            // prefix; the previous format!("#{}", e.dst) for tag
            // edges was double-prefixing into "##name" and orphaning
            // every tag edge.
            target: e.dst.clone(),
            kind: edge_kind_tag(e.kind),
            broken: match e.kind {
                EdgeKind::Link => Some(!file_set.contains(e.dst.as_str())),
                _ => None,
            },
        })
        .collect();

    Json(GraphViewResponse { nodes, edges }).into_response()
}

/// Incoming link edges for one file. The frontend uses this for
/// the "linked from" panel. chan-drive's `backlinks` filters to
/// link-kind edges already; we just pass through.
/// Backlinks payload shape: matches `ApiEdge` (lowercase `kind`)
/// so the frontend's `GraphEdge` type doesn't have to special-case
/// PascalCase versus lowercase across endpoints. `Edge.kind`'s
/// default `Serialize` would emit `"Link"` / `"Mention"` / `"Tag"`,
/// which `FileInfoBody`'s `kind === "link"` filter then rejects
/// — surfacing as "0 linked from" in the inspector.
#[derive(serde::Serialize)]
struct ApiBacklinkEdge {
    src: String,
    dst: String,
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    anchor: Option<String>,
}

pub async fn api_backlinks(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
) -> Response {
    let drive = state.drive();
    let graph = match drive.graph() {
        Ok(g) => g,
        Err(e) => return err_from(&e),
    };

    // chan-drive stores the verbatim authored target on each link
    // edge ("./img.png", "attachments/pic.png", "../foo/x.md"), so
    // its SQL backlinks(dst=?) query misses every source-relative
    // reference. We mirror api_graph's resolution: walk all
    // outgoing edges, resolve link dsts against the drive's file
    // set, then keep the ones that land on `path`. Slightly more
    // expensive than the SQL filter, but on the same order as the
    // graph load the inspector just ran.
    let files = match graph.files() {
        Ok(f) => f,
        Err(e) => return err_from(&e),
    };
    let image_files = drive_image_files(&drive);
    let mut file_set: std::collections::BTreeSet<&str> =
        files.iter().map(String::as_str).collect();
    for img in &image_files {
        file_set.insert(img.as_str());
    }

    let mut out: Vec<ApiBacklinkEdge> = Vec::new();
    for f in &files {
        let edges = match graph.neighbors(f) {
            Ok(es) => es,
            Err(e) => return err_from(&e),
        };
        for e in edges {
            if !matches!(e.kind, EdgeKind::Link) {
                continue;
            }
            let resolved = resolve_link_dst(&e.src, &e.dst, &file_set);
            if resolved == path {
                out.push(ApiBacklinkEdge {
                    src: e.src,
                    dst: resolved,
                    kind: edge_kind_tag(e.kind),
                    anchor: e.anchor,
                });
            }
        }
    }
    out.sort_by(|a, b| a.src.cmp(&b.src));
    Json(out).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_link_dst_decodes_percent_encoded_to_real_file() {
        let files: std::collections::BTreeSet<&str> = ["recipes/my note.md", "recipes/intro.md"]
            .into_iter()
            .collect();
        // [link](my%20note.md) inside recipes/intro.md.
        assert_eq!(
            resolve_link_dst("recipes/intro.md", "my%20note.md", &files),
            "recipes/my note.md"
        );
    }

    #[test]
    fn resolve_link_dst_drive_relative_match_wins() {
        // Wiki-style targets store no extension; resolver tries .md
        // and lands on the indexed file at drive root.
        let files: std::collections::BTreeSet<&str> =
            ["pasta.md", "recipes/pasta.md"].into_iter().collect();
        assert_eq!(
            resolve_link_dst("recipes/intro.md", "pasta", &files),
            "pasta.md",
        );
    }

    #[test]
    fn resolve_link_dst_dot_relative_to_source() {
        let files: std::collections::BTreeSet<&str> = ["recipes/peer.md", "recipes/intro.md"]
            .into_iter()
            .collect();
        assert_eq!(
            resolve_link_dst("recipes/intro.md", "./peer.md", &files),
            "recipes/peer.md",
        );
    }

    #[test]
    fn resolve_link_dst_parent_relative_to_source() {
        let files: std::collections::BTreeSet<&str> =
            ["sibling.md", "recipes/intro.md"].into_iter().collect();
        assert_eq!(
            resolve_link_dst("recipes/intro.md", "../sibling.md", &files),
            "sibling.md",
        );
    }

    #[test]
    fn resolve_link_dst_unresolved_returns_decoded() {
        let files: std::collections::BTreeSet<&str> = ["intro.md"].into_iter().collect();
        // Genuine broken link: decoded form surfaces a clean ghost
        // label without %20 noise.
        assert_eq!(
            resolve_link_dst("intro.md", "my%20missing.md", &files),
            "my missing.md",
        );
    }

    #[test]
    fn resolve_link_dst_strips_leading_slash() {
        let files: std::collections::BTreeSet<&str> = ["recipes/pasta.md"].into_iter().collect();
        assert_eq!(
            resolve_link_dst("intro.md", "/recipes/pasta.md", &files),
            "recipes/pasta.md",
        );
    }

    #[test]
    fn resolve_link_dst_image_attachments_drive_relative() {
        // `![](attachments/pic.png)` from any source resolves to the
        // image at the drive root. Backlinks for the image now find
        // this edge instead of returning the stale "linked from: 0"
        // that the SQL `dst = "attachments/pic.png"` query produced
        // when the source authored it as a drive-relative path.
        let files: std::collections::BTreeSet<&str> =
            ["attachments/pic.png", "notes/journal.md"]
                .into_iter()
                .collect();
        assert_eq!(
            resolve_link_dst("notes/journal.md", "attachments/pic.png", &files),
            "attachments/pic.png",
        );
    }

    #[test]
    fn resolve_link_dst_image_source_relative_dot() {
        // `![](./img.png)` inside notes/journal.md should land on the
        // sibling image, not stay verbatim.
        let files: std::collections::BTreeSet<&str> =
            ["notes/img.png", "notes/journal.md"].into_iter().collect();
        assert_eq!(
            resolve_link_dst("notes/journal.md", "./img.png", &files),
            "notes/img.png",
        );
    }

    #[test]
    fn resolve_link_dst_parent_escape_falls_back() {
        // `../../escape` from a one-level source escapes the drive
        // root: normalize_drive_rel returns None, so only the
        // verbatim drive-relative candidate is tried; both miss and
        // we surface the decoded original.
        let files: std::collections::BTreeSet<&str> = ["intro.md"].into_iter().collect();
        assert_eq!(
            resolve_link_dst("intro.md", "../../escape.md", &files),
            "../../escape.md",
        );
    }
}
