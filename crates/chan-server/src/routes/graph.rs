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
use chan_drive::{EdgeKind, ReportFileStats};
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
        /// `chan.kind` for the underlying file. "contact" for notes
        /// flagged with `chan.kind: contact` frontmatter; absent for
        /// regular markdown so the visualizer's default doc styling
        /// kicks in. Image files are still classified by extension on
        /// the frontend; this field is the indexer-side discriminator
        /// chan-drive carries on every file node.
        #[serde(skip_serializing_if = "Option::is_none")]
        node_kind: Option<&'static str>,
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

#[derive(Deserialize)]
pub struct LanguageGraphParams {
    #[serde(default)]
    depth: u32,
    #[serde(default)]
    language: Option<String>,
}

#[derive(Serialize)]
struct LanguageGraphResponse {
    max_depth: u32,
    nodes: Vec<LanguageGraphNode>,
    edges: Vec<LanguageGraphEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum LanguageGraphNode {
    Language {
        id: String,
        label: String,
        language: String,
        files: u64,
        code: u64,
    },
    Folder {
        id: String,
        label: String,
        path: String,
        files: u64,
        code: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct LanguageGraphEdge {
    source: String,
    target: String,
    kind: &'static str,
    rank: u32,
    files: u64,
    code: u64,
}

#[derive(Debug, Clone, Default)]
struct LanguageFolderStats {
    files: u64,
    code: u64,
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

/// True only for regular files under the drive root.
///
/// In-drive symlinks, even healthy ones, are treated as missing so
/// the graph's display truth matches what `chan-drive` would re-index
/// on the next pass under its lstat semantics.
fn indexed_file_exists(root: &std::path::Path, rel: &str) -> bool {
    std::fs::symlink_metadata(root.join(rel))
        .map(|m| m.file_type().is_file())
        .unwrap_or(false)
}

fn language_node_id(language: &str) -> String {
    format!("language:{language}")
}

fn folder_node_id(path: &str) -> String {
    format!("folder:{path}")
}

fn folder_label(path: &str) -> String {
    if path.is_empty() {
        "/".to_string()
    } else {
        std::path::Path::new(path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(path)
            .to_string()
    }
}

fn parent_folder(path: &str) -> String {
    std::path::Path::new(path)
        .parent()
        .and_then(|p| p.to_str())
        .filter(|p| !p.is_empty())
        .unwrap_or("")
        .replace('\\', "/")
}

fn build_language_graph(
    files: &[ReportFileStats],
    depth: u32,
    language_filter: Option<&str>,
) -> LanguageGraphResponse {
    let filter = language_filter.map(str::to_lowercase);
    let mut by_language: std::collections::BTreeMap<
        String,
        std::collections::BTreeMap<String, LanguageFolderStats>,
    > = std::collections::BTreeMap::new();

    for file in files {
        if file.language.trim().is_empty() {
            continue;
        }
        if let Some(filter) = &filter {
            if file.language.to_lowercase() != *filter {
                continue;
            }
        }
        let folder = parent_folder(&file.path);
        let stats = by_language
            .entry(file.language.clone())
            .or_default()
            .entry(folder)
            .or_default();
        stats.files += 1;
        stats.code += file.code;
    }

    let max_depth = by_language
        .values()
        .map(|folders| u32::try_from(folders.len()).unwrap_or(u32::MAX))
        .max()
        .unwrap_or(0);
    let effective_depth = if depth == 0 {
        max_depth
    } else {
        depth.min(max_depth)
    };

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut folder_totals: std::collections::BTreeMap<String, LanguageFolderStats> =
        std::collections::BTreeMap::new();

    for (language, folders) in &by_language {
        let mut ranked: Vec<(&String, &LanguageFolderStats)> = folders.iter().collect();
        ranked.sort_by(|(a_path, a), (b_path, b)| {
            b.files
                .cmp(&a.files)
                .then_with(|| b.code.cmp(&a.code))
                .then_with(|| a_path.cmp(b_path))
        });

        let language_files = folders.values().map(|s| s.files).sum();
        let language_code = folders.values().map(|s| s.code).sum();
        nodes.push(LanguageGraphNode::Language {
            id: language_node_id(language),
            label: language.clone(),
            language: language.clone(),
            files: language_files,
            code: language_code,
        });

        for (idx, (folder, stats)) in ranked.into_iter().enumerate() {
            let rank = u32::try_from(idx + 1).unwrap_or(u32::MAX);
            if effective_depth != 0 && rank > effective_depth {
                continue;
            }
            let totals = folder_totals.entry(folder.clone()).or_default();
            totals.files += stats.files;
            totals.code += stats.code;
            edges.push(LanguageGraphEdge {
                source: language_node_id(language),
                target: folder_node_id(folder),
                kind: "language",
                rank,
                files: stats.files,
                code: stats.code,
            });
        }
    }

    for (folder, stats) in folder_totals {
        nodes.push(LanguageGraphNode::Folder {
            id: folder_node_id(&folder),
            label: folder_label(&folder),
            path: folder,
            files: stats.files,
            code: stats.code,
        });
    }

    LanguageGraphResponse {
        max_depth,
        nodes,
        edges,
    }
}

pub async fn api_language_graph(
    State(state): State<Arc<AppState>>,
    Query(p): Query<LanguageGraphParams>,
) -> Response {
    let report = match state.drive().report() {
        Ok(r) => r,
        Err(e) => return err_from(&e),
    };
    Json(build_language_graph(
        &report.files,
        p.depth,
        p.language.as_deref(),
    ))
    .into_response()
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
    let present_files: std::collections::BTreeSet<&str> = files
        .iter()
        .filter(|path| indexed_file_exists(drive.root(), path))
        .map(String::as_str)
        .collect();

    // Contact-kind file set, used to stamp `node_kind: "contact"` on
    // file nodes so the visualizer can render `chan.kind: contact`
    // notes (Contacts/alice.md, etc.) with the contact treatment
    // rather than the generic doc shape. Single SQL scan; cheap
    // compared to N per-node `node_kind` lookups.
    //
    // Also serves as the lookup table for the @@mention -> contact
    // file rewrite below: a contact whose file_stem matches the
    // mention name (case-insensitive) gets its rel_path stamped on
    // the mention edge's dst, so `@@alice` no longer renders as a
    // standalone yellow text node alongside the Contacts/alice.md
    // file node — the two collapse into one.
    let contact_rows = drive.contacts().unwrap_or_default();
    let contact_paths: std::collections::HashSet<String> =
        contact_rows.iter().map(|c| c.rel_path.clone()).collect();
    // Maps the lowercased mention name (the bit after `@@`) to the
    // resolved contact file. The basename-stem entry is the legacy
    // resolver (pre-phase-5: `@@alice` resolves to `Contacts/alice.md`
    // by filename match). Phase 5 layers each contact's declared
    // aliases on top: a contact with `aliases: [ali, smith]` adds
    // `(ali, path)` and `(smith, path)` entries so `@@ali` resolves
    // the same way `@@alice` does. When two contacts claim the same
    // alias the last writer wins; the picker UI surfaces aliases so
    // users can disambiguate by editing the offending contact's
    // frontmatter.
    let mut mention_to_contact: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for c in &contact_rows {
        if let Some(stem) = std::path::Path::new(&c.rel_path)
            .file_stem()
            .and_then(|s| s.to_str())
        {
            mention_to_contact.insert(stem.to_lowercase(), c.rel_path.clone());
        }
        for alias in &c.aliases {
            let key = alias.trim().to_lowercase();
            if !key.is_empty() {
                mention_to_contact.insert(key, c.rel_path.clone());
            }
        }
    }

    let mut file_set: std::collections::BTreeSet<&str> = files.iter().map(String::as_str).collect();
    for img in &image_files {
        file_set.insert(img.as_str());
    }
    let mut present_file_set = present_files.clone();
    for img in &image_files {
        present_file_set.insert(img.as_str());
    }

    // Rewrite link-edge targets so URL-encoded / source-relative
    // markdown links land on the real file node (clickable in the
    // inspector). Genuine ghosts get the decoded form so the label
    // reads "my note" instead of "my%20note".
    //
    // Mention-edge targets get a similar rewrite: `@@alice` is
    // remapped to `Contacts/alice.md` when a contact file with the
    // matching file_stem exists. The two would otherwise render as
    // separate nodes (yellow `@@alice` text node + yellow rectangle
    // contact node), even though they refer to the same person.
    // Unresolved mentions keep their `@@name` dst and fall through to
    // the synthesized Mention node below.
    for e in all_edges.iter_mut() {
        match e.kind {
            EdgeKind::Link => {
                e.dst = resolve_link_dst(&e.src, &e.dst, &file_set);
            }
            EdgeKind::Mention => {
                let stripped = e.dst.strip_prefix("@@").unwrap_or(&e.dst).to_lowercase();
                if let Some(contact_path) = mention_to_contact.get(&stripped) {
                    e.dst = contact_path.clone();
                }
            }
            EdgeKind::Tag => {}
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
            node_kind: if contact_paths.contains(path) {
                Some("contact")
            } else {
                None
            },
            missing: !present_files.contains(path.as_str()),
        });
    }
    for img in &referenced_images {
        nodes.push(GraphNodeView::File {
            id: img.clone(),
            label: file_label(img),
            path: img.clone(),
            node_kind: None,
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
                // Only synthesize a standalone @@name node for
                // mentions that didn't resolve to a real contact
                // file. Resolved mentions point at the contact's
                // file node, which already exists in `files`.
                if !file_set.contains(e.dst.as_str()) {
                    mention_set.insert(e.dst.clone());
                }
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
            node_kind: None,
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
                EdgeKind::Link => Some(!present_file_set.contains(e.dst.as_str())),
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
    let mut file_set: std::collections::BTreeSet<&str> = files.iter().map(String::as_str).collect();
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

    fn report_file(path: &str, language: &str, code: u64) -> ReportFileStats {
        ReportFileStats {
            path: path.to_string(),
            language: language.to_string(),
            code,
            comments: 0,
            blanks: 0,
            complexity: 0,
            bytes: 0,
            mtime: None,
        }
    }

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
        let files: std::collections::BTreeSet<&str> = ["attachments/pic.png", "notes/journal.md"]
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

    #[test]
    fn indexed_file_exists_requires_regular_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("notes")).unwrap();
        std::fs::write(tmp.path().join("notes/live.md"), "# live\n").unwrap();
        std::fs::create_dir(tmp.path().join("notes/dir.md")).unwrap();

        assert!(indexed_file_exists(tmp.path(), "notes/live.md"));
        assert!(!indexed_file_exists(tmp.path(), "notes/missing.md"));
        assert!(!indexed_file_exists(tmp.path(), "notes/dir.md"));
    }

    #[cfg(unix)]
    #[test]
    fn indexed_file_exists_treats_symlink_as_missing() {
        use std::os::unix::fs::symlink;

        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("target.md"), "# target\n").unwrap();
        symlink("target.md", tmp.path().join("alias.md")).unwrap();

        assert!(!indexed_file_exists(tmp.path(), "alias.md"));
    }

    #[test]
    fn language_graph_ranks_folders_per_language() {
        let graph = build_language_graph(
            &[
                report_file("crates/a/src/lib.rs", "Rust", 100),
                report_file("crates/a/src/main.rs", "Rust", 40),
                report_file("crates/b/lib.rs", "Rust", 400),
                report_file("web/src/App.svelte", "Svelte", 80),
            ],
            0,
            None,
        );

        assert_eq!(graph.max_depth, 2);
        assert!(graph.nodes.contains(&LanguageGraphNode::Language {
            id: "language:Rust".to_string(),
            label: "Rust".to_string(),
            language: "Rust".to_string(),
            files: 3,
            code: 540,
        }));
        assert!(graph.nodes.contains(&LanguageGraphNode::Folder {
            id: "folder:crates/a/src".to_string(),
            label: "src".to_string(),
            path: "crates/a/src".to_string(),
            files: 2,
            code: 140,
        }));
        assert!(graph.edges.contains(&LanguageGraphEdge {
            source: "language:Rust".to_string(),
            target: "folder:crates/a/src".to_string(),
            kind: "language",
            rank: 1,
            files: 2,
            code: 140,
        }));
        assert!(graph.edges.contains(&LanguageGraphEdge {
            source: "language:Rust".to_string(),
            target: "folder:crates/b".to_string(),
            kind: "language",
            rank: 2,
            files: 1,
            code: 400,
        }));
    }

    #[test]
    fn language_graph_depth_and_language_filter_trim_edges() {
        let graph = build_language_graph(
            &[
                report_file("a/one.rs", "Rust", 10),
                report_file("b/two.rs", "Rust", 20),
                report_file("web/App.svelte", "Svelte", 30),
            ],
            1,
            Some("rust"),
        );

        assert_eq!(graph.max_depth, 2);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].source, "language:Rust");
        assert_eq!(graph.edges[0].rank, 1);
        assert!(graph.nodes.iter().any(
            |n| matches!(n, LanguageGraphNode::Language { language, .. } if language == "Rust")
        ));
        assert!(!graph.nodes.iter().any(
            |n| matches!(n, LanguageGraphNode::Language { language, .. } if language == "Svelte")
        ));
    }

    #[test]
    fn language_graph_breaks_ties_by_code_then_path() {
        let graph = build_language_graph(
            &[
                report_file("z/lib.rs", "Rust", 10),
                report_file("b/lib.rs", "Rust", 30),
                report_file("a/lib.rs", "Rust", 30),
            ],
            0,
            Some("Rust"),
        );

        let targets: Vec<&str> = graph
            .edges
            .iter()
            .map(|edge| edge.target.as_str())
            .collect();
        assert_eq!(targets, ["folder:a", "folder:b", "folder:z"]);
        assert_eq!(
            graph.edges.iter().map(|edge| edge.rank).collect::<Vec<_>>(),
            [1, 2, 3]
        );
    }

    #[test]
    fn language_graph_renders_root_folder_with_slash_label() {
        let graph = build_language_graph(&[report_file("lib.rs", "Rust", 12)], 0, None);

        assert!(graph.nodes.contains(&LanguageGraphNode::Folder {
            id: "folder:".to_string(),
            label: "/".to_string(),
            path: "".to_string(),
            files: 1,
            code: 12,
        }));
        assert!(graph.edges.contains(&LanguageGraphEdge {
            source: "language:Rust".to_string(),
            target: "folder:".to_string(),
            kind: "language",
            rank: 1,
            files: 1,
            code: 12,
        }));
    }

    #[test]
    fn language_graph_clamps_depth_to_max_depth() {
        let graph = build_language_graph(
            &[
                report_file("a/lib.rs", "Rust", 10),
                report_file("b/lib.rs", "Rust", 20),
                report_file("c/lib.rs", "Rust", 30),
            ],
            99,
            Some("Rust"),
        );

        assert_eq!(graph.max_depth, 3);
        assert_eq!(graph.edges.len(), 3);
        assert_eq!(
            graph.edges.iter().map(|edge| edge.rank).collect::<Vec<_>>(),
            [1, 2, 3]
        );
    }

    #[test]
    fn language_graph_empty_drive_returns_empty_payload() {
        let graph = build_language_graph(&[], 0, None);

        assert_eq!(graph.max_depth, 0);
        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
    }
}
