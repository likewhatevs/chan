//! Filesystem graph: directories, files, symlinks, hardlinks, and
//! ghost nodes (broken or outside-workspace symlink targets, plus
//! special files like FIFOs and sockets that the content index
//! deliberately drops).
//!
//! Distinct from `/api/graph`, which describes the *semantic* graph
//! built from markdown content (file/tag/mention nodes, link/tag/
//! mention edges). This route walks the actual filesystem under the
//! workspace root and reports its shape. Same workspace sandbox invariants
//! apply: requests are lexically resolved through
//! `chan_workspace::fs_ops::resolve_safe` so `..` traversal is rejected
//! before any I/O.
//!
//! The walker uses `symlink_metadata` everywhere (lstat semantics) so
//! a symlink is never confused with the file it points at. Symlink
//! targets are classified but never traversed: their existence and
//! whether they land inside the workspace root workspaces the node kind, and
//! traversal only follows `contains` edges (parent -> child) under
//! real directories.
//!
//! Hardlinks are deduped by `(st_dev, st_ino)`. Two paths sharing the
//! same inode are surfaced as two `file` nodes joined by a
//! `hardlink` edge in addition to their parent `contains` edges.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::Metadata;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::err;
use crate::state::AppState;

/// Hard cap on `depth` for scope=directory. Six is enough for a project-
/// style workspace (a few levels of grouping) without letting a single
/// request walk a deep dependency tree disguised as a notes workspace.
const MAX_DEPTH: usize = 6;

/// Hard cap on emitted nodes. Past this the response is truncated and
/// `truncated: true` flags it on the wire so the frontend can warn
/// the user that they're looking at a partial graph.
const MAX_NODES: usize = 10_000;

#[derive(Deserialize)]
pub struct FsGraphParams {
    /// `file` or `directory`. Default `directory` so a bare
    /// `/api/fs-graph?path=...` is the common case (workspace overview /
    /// directory snapshot).
    #[serde(default = "default_scope")]
    scope: FsGraphScope,
    /// Workspace-relative target. Empty / missing / `/` means the workspace
    /// root. Path is lexical: leading slash is trimmed,
    /// `..`-traversal is rejected before any I/O.
    #[serde(default)]
    path: String,
    /// For scope=directory: how many levels of children to walk.
    /// Depth 1 means direct children only. Capped at `MAX_DEPTH`.
    /// Ignored for scope=file (always returns the file and its
    /// parent / symlink target).
    #[serde(default = "default_depth")]
    depth: usize,
}

fn default_scope() -> FsGraphScope {
    FsGraphScope::Directory
}

fn default_depth() -> usize {
    1
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FsGraphScope {
    File,
    Directory,
}

impl FsGraphScope {
    fn label(self) -> &'static str {
        match self {
            FsGraphScope::File => "file",
            FsGraphScope::Directory => "directory",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FsGraphResponse {
    /// Workspace root absolute path, identical to what `/api/workspace`
    /// reports. Included so the frontend can render breadcrumbs
    /// without a follow-up call.
    pub root: String,
    pub scope: &'static str,
    /// The request's `path` after lexical normalization (empty means
    /// workspace root).
    pub path: String,
    /// The effective directory depth used. For scope=file this is always
    /// 0 (the file plus its parent / target are the response).
    pub depth: usize,
    pub nodes: Vec<NodeView>,
    pub edges: Vec<EdgeView>,
    /// True when the walker hit `MAX_NODES` and stopped early. Callers
    /// should narrow the scope or reduce depth.
    pub truncated: bool,
}

/// Node identifier shape:
///
///   - In-workspace entries: workspace-relative POSIX path. Workspace root is
///     the empty string.
///   - Outside-workspace symlink targets: `outside:<symlink-src>` where
///     `<symlink-src>` is the workspace-relative source path. Stable
///     within a response so the frontend can hang labels off it; not
///     suitable as a long-term identifier across responses since the
///     symlink's target may change.
///   - In-workspace missing targets: workspace-relative POSIX path of the
///     would-be file. Marked `broken: true`.
#[derive(Debug, Clone, Serialize)]
pub struct NodeView {
    pub id: String,
    pub kind: &'static str,
    /// Basename for the file / directory. For ghost-outside nodes
    /// this is the literal `readlink` target so the frontend can
    /// show something meaningful.
    pub name: String,
    /// Workspace-relative path (POSIX). Same as `id` for in-workspace nodes;
    /// empty for outside-workspace ghosts.
    pub path: String,
    /// File size in bytes (regular files only; 0 for everything
    /// else).
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_class: Option<chan_workspace::PathClass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<chan_workspace::PathPermission>,
    #[serde(skip_serializing_if = "is_one")]
    pub link_count: u64,
    /// Last-modified time in unix seconds, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtime: Option<i64>,
    /// Raw `readlink` target for symlink nodes. None for other kinds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// True for symlink targets that point outside the workspace root.
    /// Never traversed.
    #[serde(skip_serializing_if = "is_false")]
    pub outside: bool,
    /// True for ghost nodes that represent missing in-workspace targets
    /// (broken `readlink`) or unreadable entries.
    #[serde(skip_serializing_if = "is_false")]
    pub broken: bool,
    /// True for symlink nodes whose target resolves outside the workspace.
    #[serde(skip_serializing_if = "is_false")]
    pub target_escapes_workspace: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn is_one(n: &u64) -> bool {
    *n == 1
}

#[derive(Debug, Clone, Serialize)]
pub struct EdgeView {
    pub source: String,
    pub target: String,
    pub kind: &'static str,
}

#[derive(Debug)]
pub struct FsGraphError {
    status: StatusCode,
    message: String,
}

impl FsGraphError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub(crate) fn into_response(self) -> Response {
        err(self.status, self.message)
    }
}

impl std::fmt::Display for FsGraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for FsGraphError {}

pub async fn api_fs_graph(
    State(state): State<Arc<AppState>>,
    Query(p): Query<FsGraphParams>,
) -> Response {
    let workspace = state.workspace();
    let result =
        tokio::task::spawn_blocking(move || build_fs_graph(&workspace, p.scope, &p.path, p.depth))
            .await;
    match result {
        Ok(Ok(response)) => Json(response).into_response(),
        Ok(Err(e)) => err(e.status, e.message),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("filesystem graph task panicked: {e}"),
        )
            .into_response(),
    }
}

pub fn build_fs_graph(
    workspace: &chan_workspace::Workspace,
    scope: FsGraphScope,
    path: &str,
    requested_depth: usize,
) -> Result<FsGraphResponse, FsGraphError> {
    let root: PathBuf = workspace.root().to_path_buf();
    let rel = normalize_rel(path);
    let abs = if rel.is_empty() {
        root.clone()
    } else {
        match chan_workspace::fs_ops::resolve_safe(&root, &rel) {
            Ok(a) => a,
            Err(e) => return Err(FsGraphError::new(StatusCode::BAD_REQUEST, e.to_string())),
        }
    };

    // Mid-path symlink escape guard. `resolve_safe` is LEXICAL only:
    // if the request path traverses through an in-workspace symlink that
    // points outside the workspace (`escape-link -> /etc`), the join
    // gives `<workspace>/escape-link/hosts`, which `symlink_metadata`
    // happily resolves to `/etc/hosts` because intermediate components
    // are followed during path resolution (lstat only spares the
    // leaf). Canonicalize the parent and verify it stays under the
    // workspace's canonical root. The leaf itself can still be a symlink;
    // the walker classifies symlink leaves via readlink without
    // following them, so an in-workspace symlink to an outside file
    // surfaces correctly as a ghost node.
    ensure_parent_inside_drive(&root, &abs, &rel)?;

    let meta = match std::fs::symlink_metadata(&abs) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(FsGraphError::new(
                StatusCode::NOT_FOUND,
                format!("no such path: {rel}"),
            ));
        }
        Err(e) => {
            return Err(FsGraphError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("stat: {e}"),
            ));
        }
    };

    let depth = requested_depth.clamp(1, MAX_DEPTH);

    // Reject scope=directory against a non-directory up front so the
    // walker stays infallible; the wire error shape matches the
    // 400 we use elsewhere.
    if scope == FsGraphScope::Directory && !meta.is_dir() {
        return Err(FsGraphError::new(
            StatusCode::BAD_REQUEST,
            format!("scope=directory requires a directory; {rel} is not"),
        ));
    }

    let mut walker = FsGraphWalker::new(root.clone(), workspace.walk_filter().clone());
    match scope {
        FsGraphScope::File => walker.walk_file(&rel, &abs, &meta),
        FsGraphScope::Directory => walker.walk_directory(&rel, &abs, &meta, depth),
    }

    let (nodes, edges, truncated) = walker.finish();
    Ok(FsGraphResponse {
        root: root.display().to_string(),
        scope: scope.label(),
        path: rel,
        depth: match scope {
            FsGraphScope::File => 0,
            FsGraphScope::Directory => depth,
        },
        nodes,
        edges,
        truncated,
    })
}

/// Verify that the parent of the joined request path resolves
/// inside the workspace root. Catches `path=alias-to-outside/x.md`
/// where `alias-to-outside` is an in-workspace symlink whose target
/// escapes the workspace — `resolve_safe` is lexical and lets that
/// through, but the kernel will follow the intermediate symlink on
/// `symlink_metadata` / `read_dir`. Workspace root requests skip the
/// check (the request resolves to the workspace root itself; no parent
/// to verify).
fn ensure_parent_inside_drive(root: &Path, abs: &Path, rel: &str) -> Result<(), FsGraphError> {
    if rel.is_empty() {
        return Ok(());
    }
    let parent = match abs.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => return Ok(()),
    };
    // Canonicalize the workspace root once per request. Cheap on local
    // filesystems; pricier on cloud-synced mounts (iCloud / Dropbox)
    // but still bounded by a single FS-provider round trip.
    let root_canon = match root.canonicalize() {
        Ok(c) => c,
        // The workspace root must canonicalize. If it doesn't (deleted
        // out from under us, broken mount) we cannot serve the
        // request safely — surface as 500 rather than silently
        // allowing the lexical check.
        Err(e) => {
            return Err(FsGraphError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("canonicalize workspace root: {e}"),
            ));
        }
    };
    // Parent equal to workspace root is the common case (top-level file
    // request); skip the canonicalize round trip.
    if parent == root || parent == root_canon {
        return Ok(());
    }
    let parent_canon = match parent.canonicalize() {
        Ok(c) => c,
        // Parent dir does not exist. Fall through to the caller's
        // `symlink_metadata` call, which will surface the standard
        // NOT_FOUND error for the leaf.
        Err(_) => return Ok(()),
    };
    if !parent_canon.starts_with(&root_canon) {
        return Err(FsGraphError::new(
            StatusCode::BAD_REQUEST,
            format!("path escapes workspace root via mid-path symlink: {rel}"),
        ));
    }
    Ok(())
}

/// Trim a leading slash and collapse `.` segments. Pure-`..` requests
/// pass through unchanged; `resolve_safe` will reject them with the
/// standard error wire shape so this helper does not have to.
fn normalize_rel(requested: &str) -> String {
    let trimmed = requested.trim_start_matches('/');
    if trimmed.is_empty() || trimmed == "." {
        return String::new();
    }
    let mut out = PathBuf::new();
    for c in Path::new(trimmed).components() {
        match c {
            Component::Normal(s) => out.push(s),
            Component::CurDir => {}
            _ => return trimmed.to_owned(),
        }
    }
    out.to_string_lossy().replace('\\', "/")
}

#[cfg(unix)]
fn inode_key(meta: &Metadata) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    Some((meta.dev(), meta.ino()))
}

#[cfg(not(unix))]
fn inode_key(_meta: &Metadata) -> Option<(u64, u64)> {
    None
}

#[cfg(unix)]
fn nlink_of(meta: &Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    meta.nlink()
}

#[cfg(not(unix))]
fn nlink_of(_meta: &Metadata) -> u64 {
    1
}

fn mtime_of(meta: &Metadata) -> Option<i64> {
    meta.modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}

fn basename_of(rel: &str, abs: &Path) -> String {
    if rel.is_empty() {
        abs.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "/".into())
    } else {
        rel.rsplit('/').next().unwrap_or(rel).to_owned()
    }
}

fn node_kind_from_class(
    class: Option<&chan_workspace::PathClass>,
    ft: &std::fs::FileType,
) -> &'static str {
    match class.map(|c| c.kind) {
        Some(chan_workspace::PathKind::Symlink) => "symlink",
        Some(chan_workspace::PathKind::Directory) => "directory",
        Some(chan_workspace::PathKind::RegularFile) => "file",
        Some(
            chan_workspace::PathKind::Fifo
            | chan_workspace::PathKind::Socket
            | chan_workspace::PathKind::BlockDevice
            | chan_workspace::PathKind::CharDevice
            | chan_workspace::PathKind::Other,
        ) => "ghost",
        None if ft.is_symlink() => "symlink",
        None if ft.is_dir() => "directory",
        None if ft.is_file() => "file",
        None => "ghost",
    }
}

/// Internal accumulator. Holds the nodes/edges plus dedup tables so
/// each path emits a single node and each pair of hardlinks emits a
/// single `hardlink` edge.
struct FsGraphWalker {
    root: PathBuf,
    root_canon: Option<PathBuf>,
    /// Directory-name blocklist (the per-workspace `WalkFilter`). The
    /// child walk skips any directory whose basename is excluded, at
    /// any depth, so the filesystem graph never plots `node_modules/`
    /// / `target/` / `venv/` dependency trees. Matches the index and
    /// File Browser spine, which exclude the same set.
    filter: chan_workspace::fs_ops::WalkFilter,
    nodes: BTreeMap<String, NodeView>,
    edges: Vec<EdgeView>,
    edge_set: HashSet<(String, String, &'static str)>,
    /// Paths grouped by `(dev, ino)`. Filled as we visit files; used
    /// to emit a single `hardlink` edge between any two paths sharing
    /// the same inode.
    inode_paths: HashMap<(u64, u64), Vec<String>>,
    truncated: bool,
}

impl FsGraphWalker {
    fn new(root: PathBuf, filter: chan_workspace::fs_ops::WalkFilter) -> Self {
        let root_canon = root.canonicalize().ok();
        Self {
            root,
            root_canon,
            filter,
            nodes: BTreeMap::new(),
            edges: Vec::new(),
            edge_set: HashSet::new(),
            inode_paths: HashMap::new(),
            truncated: false,
        }
    }

    fn finish(mut self) -> (Vec<NodeView>, Vec<EdgeView>, bool) {
        // Emit one `hardlink` edge per pair of paths sharing an inode.
        // Sort the group so the wire output is stable; `source <
        // target` lexicographically.
        let inode_paths = std::mem::take(&mut self.inode_paths);
        let groups: Vec<Vec<String>> = inode_paths.into_values().filter(|v| v.len() >= 2).collect();
        for mut group in groups {
            group.sort();
            for i in 0..group.len() {
                for j in (i + 1)..group.len() {
                    self.push_edge(group[i].clone(), group[j].clone(), "hardlink");
                }
            }
        }
        let nodes: Vec<NodeView> = self.nodes.into_values().collect();
        (nodes, self.edges, self.truncated)
    }

    fn push_edge(&mut self, source: String, target: String, kind: &'static str) {
        let key = (source.clone(), target.clone(), kind);
        if self.edge_set.insert(key) {
            self.edges.push(EdgeView {
                source,
                target,
                kind,
            });
        }
    }

    fn insert_node(&mut self, node: NodeView) {
        if self.nodes.len() >= MAX_NODES && !self.nodes.contains_key(&node.id) {
            self.truncated = true;
            return;
        }
        self.nodes.entry(node.id.clone()).or_insert(node);
    }

    /// File-scope walk: emit the requested path, its ancestor
    /// directory chain up to the workspace root, and, if the path is a
    /// symlink, classify its target. Directory targets are NOT walked
    /// here; we stay shallow so `scope=file` is cheap.
    fn walk_file(&mut self, rel: &str, abs: &Path, meta: &Metadata) {
        self.emit_ancestor_chain(rel);
        self.visit_entry(rel, abs, meta);
        if !rel.is_empty() {
            let parent_rel = parent_rel(rel);
            let parent_abs = abs
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| self.root.clone());
            if let Ok(parent_meta) = std::fs::symlink_metadata(&parent_abs) {
                self.visit_entry(&parent_rel, &parent_abs, &parent_meta);
                self.push_edge(parent_rel, rel.to_owned(), "contains");
            }
        }
    }

    /// Directory-scope walk: emit the directory node, then walk its
    /// children up to `depth` levels. Depth 1 = direct children
    /// only. Caller MUST have verified that `meta.is_dir()`; the
    /// route enforces this before invoking the walker.
    fn walk_directory(&mut self, rel: &str, abs: &Path, meta: &Metadata, depth: usize) {
        debug_assert!(meta.is_dir(), "walk_directory called on non-directory");
        self.emit_ancestor_chain(rel);
        self.visit_entry(rel, abs, meta);
        if meta.permissions().readonly() {
            return;
        }
        let mut visited_dirs: HashSet<(u64, u64)> = HashSet::new();
        if let Some(key) = inode_key(meta) {
            visited_dirs.insert(key);
        }
        self.walk_dir(rel, abs, depth, &mut visited_dirs);
    }

    /// Emit the root-to-leaf `contains` chain for a scoped file or
    /// directory before walking its local neighbourhood. The normal
    /// depth walk expands downward from the scope; this pass supplies
    /// the upstream filesystem spine so callers can always show how a
    /// scoped node attaches back to the workspace root.
    fn emit_ancestor_chain(&mut self, rel: &str) {
        let root_abs = self.root.clone();
        if let Ok(root_meta) = std::fs::symlink_metadata(&root_abs) {
            self.visit_entry("", &root_abs, &root_meta);
        }

        let parts: Vec<&str> = rel.split('/').filter(|part| !part.is_empty()).collect();
        let mut parent_rel = String::new();
        for idx in 0..parts.len() {
            let child_rel = parts[..=idx].join("/");
            let child_abs = self.root.join(&child_rel);
            if let Ok(child_meta) = std::fs::symlink_metadata(&child_abs) {
                self.visit_entry(&child_rel, &child_abs, &child_meta);
                self.push_edge(parent_rel.clone(), child_rel.clone(), "contains");
            }
            parent_rel = child_rel;
        }
    }

    fn walk_dir(
        &mut self,
        parent_rel: &str,
        parent_abs: &Path,
        depth_remaining: usize,
        visited_dirs: &mut HashSet<(u64, u64)>,
    ) {
        if depth_remaining == 0 || self.nodes.len() >= MAX_NODES {
            if depth_remaining > 0 {
                self.truncated = true;
            }
            return;
        }
        let read = match std::fs::read_dir(parent_abs) {
            Ok(r) => r,
            Err(_) => return,
        };
        // Collect + sort by basename so the wire output is stable
        // (read_dir order is platform-defined).
        let mut entries: Vec<(std::ffi::OsString, PathBuf)> = read
            .filter_map(|r| r.ok())
            .map(|e| (e.file_name(), e.path()))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));

        for (name, child_abs) in entries {
            let name_str = name.to_string_lossy();
            // Skip workspace-internal state and the per-workspace blocklist
            // dirs at ANY depth. Mirrors chan-workspace's `walk_drive`
            // (`.chan`/`.git` invariants) plus the `WalkFilter`
            // (`node_modules`, `target`, `venv`, ...) so the
            // filesystem graph excludes the same set the index, the
            // File Browser spine, and the watcher feed exclude. A
            // repo-root workspace otherwise plots its whole dependency
            // tree (60K-131K nodes). The skip applies regardless of
            // the entry's file type so a `.git` symlink can't slip a
            // dependency tree back in.
            if name_str == ".chan" || name_str == ".git" || self.filter.is_excluded(&name_str) {
                continue;
            }
            let child_rel = if parent_rel.is_empty() {
                name_str.to_string()
            } else {
                format!("{parent_rel}/{name_str}")
            };
            let child_meta = match std::fs::symlink_metadata(&child_abs) {
                Ok(m) => m,
                Err(_) => {
                    let ghost = NodeView {
                        id: child_rel.clone(),
                        kind: "ghost",
                        name: name_str.to_string(),
                        path: child_rel.clone(),
                        size: 0,
                        path_class: None,
                        permission: None,
                        link_count: 1,
                        mtime: None,
                        target: None,
                        outside: false,
                        broken: true,
                        target_escapes_workspace: false,
                    };
                    self.insert_node(ghost);
                    self.push_edge(parent_rel.to_owned(), child_rel, "contains");
                    continue;
                }
            };
            self.visit_entry(&child_rel, &child_abs, &child_meta);
            self.push_edge(parent_rel.to_owned(), child_rel.clone(), "contains");

            if child_meta.is_dir()
                && !child_meta.file_type().is_symlink()
                && !child_meta.permissions().readonly()
            {
                if let Some(key) = inode_key(&child_meta) {
                    if !visited_dirs.insert(key) {
                        // Already walked this inode; skip to avoid
                        // double-counting on platforms that allow
                        // hardlinked dirs.
                        continue;
                    }
                }
                self.walk_dir(&child_rel, &child_abs, depth_remaining - 1, visited_dirs);
            }
        }
    }

    /// Emit a node for the path at `rel` with metadata `meta`. Records
    /// hardlink candidates so `finish()` can emit dedup edges.
    fn visit_entry(&mut self, rel: &str, abs: &Path, meta: &Metadata) {
        let ft = meta.file_type();
        let class = chan_workspace::fs_ops::classify_abs(&self.root, abs).ok();
        let kind = node_kind_from_class(class.as_ref(), &ft);

        let mut node = NodeView {
            id: rel.to_owned(),
            kind,
            name: basename_of(rel, abs),
            path: rel.to_owned(),
            size: if ft.is_file() { meta.len() } else { 0 },
            path_class: class.clone(),
            permission: class.as_ref().map(|c| c.permission),
            link_count: class.as_ref().map(|c| c.link_count).unwrap_or(1),
            mtime: mtime_of(meta),
            target: None,
            outside: false,
            broken: false,
            target_escapes_workspace: class
                .as_ref()
                .map(|c| c.target_escapes_workspace)
                .unwrap_or(false),
        };

        if ft.is_symlink() {
            match std::fs::read_link(abs) {
                Ok(target) => {
                    node.target = Some(target.to_string_lossy().into_owned());
                    self.insert_node(node);
                    self.emit_symlink_target(rel, abs, &target);
                    return;
                }
                Err(_) => {
                    node.broken = true;
                    self.insert_node(node);
                    return;
                }
            }
        }

        if ft.is_file() {
            if let Some(key) = inode_key(meta) {
                if nlink_of(meta) > 1 {
                    self.inode_paths
                        .entry(key)
                        .or_default()
                        .push(rel.to_owned());
                }
            }
        }

        self.insert_node(node);
    }

    /// Classify a symlink target and emit the corresponding node +
    /// `symlink` edge. Targets are NEVER traversed; we only stat them
    /// to decide whether they exist, and emit one classification node.
    fn emit_symlink_target(&mut self, src_rel: &str, src_abs: &Path, target: &Path) {
        let target_abs: PathBuf = if target.is_absolute() {
            target.to_path_buf()
        } else {
            let parent = src_abs.parent().unwrap_or(&self.root);
            parent.join(target)
        };

        if !self.target_is_inside_drive(&target_abs) {
            let ghost_id = format!("outside:{src_rel}");
            let ghost = NodeView {
                id: ghost_id.clone(),
                kind: "ghost",
                name: target.to_string_lossy().into_owned(),
                path: String::new(),
                size: 0,
                path_class: None,
                permission: None,
                link_count: 1,
                mtime: None,
                target: Some(target.to_string_lossy().into_owned()),
                outside: true,
                broken: false,
                target_escapes_workspace: false,
            };
            self.insert_node(ghost);
            self.push_edge(src_rel.to_owned(), ghost_id, "symlink");
            return;
        }

        let target_rel = match self.drive_relative_target(&target_abs) {
            Some(s) => s,
            None => {
                // Could not pin the relative form. Treat as broken
                // rather than outside-workspace: the lexical check above
                // already ruled out escape, so the most useful signal
                // is "we can't find it".
                let ghost_id = format!("broken:{src_rel}");
                let ghost = NodeView {
                    id: ghost_id.clone(),
                    kind: "ghost",
                    name: target.to_string_lossy().into_owned(),
                    path: String::new(),
                    size: 0,
                    path_class: None,
                    permission: None,
                    link_count: 1,
                    mtime: None,
                    target: Some(target.to_string_lossy().into_owned()),
                    outside: false,
                    broken: true,
                    target_escapes_workspace: false,
                };
                self.insert_node(ghost);
                self.push_edge(src_rel.to_owned(), ghost_id, "symlink");
                return;
            }
        };

        let target_abs_in_root = self.root.join(&target_rel);
        match std::fs::symlink_metadata(&target_abs_in_root) {
            Ok(target_meta) => {
                // Emit a node for the target. We deliberately do NOT
                // recurse into `visit_entry` here: a chain
                // `a -> b -> a` would otherwise re-enter
                // `emit_symlink_target` for `b` and loop. The walker
                // will reach the actual target through its own
                // parent-`contains` descent and classify it fully
                // there; from a symlink's perspective we only need
                // the immediate target node + the edge.
                self.insert_target_node(&target_rel, &target_abs_in_root, &target_meta);
                self.push_edge(src_rel.to_owned(), target_rel, "symlink");
            }
            Err(_) => {
                let ghost = NodeView {
                    id: target_rel.clone(),
                    kind: "ghost",
                    name: target_rel
                        .rsplit('/')
                        .next()
                        .unwrap_or(&target_rel)
                        .to_owned(),
                    path: target_rel.clone(),
                    size: 0,
                    path_class: None,
                    permission: None,
                    link_count: 1,
                    mtime: None,
                    target: Some(target.to_string_lossy().into_owned()),
                    outside: false,
                    broken: true,
                    target_escapes_workspace: false,
                };
                self.insert_node(ghost);
                self.push_edge(src_rel.to_owned(), target_rel, "symlink");
            }
        }
    }

    /// Insert a node for a symlink's resolved target without
    /// recursing through its symlink chain. Mirrors the node shape
    /// `visit_entry` produces but stops at the target itself; if the
    /// target is also a symlink, we leave full classification to
    /// whatever later visit reaches it as a parent-`contains` child
    /// (or to a direct file-scope query against that path).
    fn insert_target_node(&mut self, rel: &str, abs: &Path, meta: &Metadata) {
        if self.nodes.contains_key(rel) {
            return;
        }
        let ft = meta.file_type();
        let class = chan_workspace::fs_ops::classify_abs(&self.root, abs).ok();
        let kind = node_kind_from_class(class.as_ref(), &ft);
        let target_readlink = if ft.is_symlink() {
            std::fs::read_link(abs)
                .ok()
                .map(|t| t.to_string_lossy().into_owned())
        } else {
            None
        };
        let node = NodeView {
            id: rel.to_owned(),
            kind,
            name: basename_of(rel, abs),
            path: rel.to_owned(),
            size: if ft.is_file() { meta.len() } else { 0 },
            path_class: class.clone(),
            permission: class.as_ref().map(|c| c.permission),
            link_count: class.as_ref().map(|c| c.link_count).unwrap_or(1),
            mtime: mtime_of(meta),
            target: target_readlink,
            outside: false,
            broken: false,
            target_escapes_workspace: class
                .as_ref()
                .map(|c| c.target_escapes_workspace)
                .unwrap_or(false),
        };
        self.insert_node(node);
    }

    fn target_is_inside_drive(&self, target_abs: &Path) -> bool {
        // Canonicalize the deepest existing ancestor of `target_abs`
        // (`canonicalize` fails on missing leaves; we mirror what the
        // kernel will do on `open`). Compare against the canonical
        // root when available.
        let mut probe: &Path = target_abs;
        let canon_target = loop {
            match probe.canonicalize() {
                Ok(c) => break Some(c),
                Err(_) => match probe.parent() {
                    Some(p) => probe = p,
                    None => break None,
                },
            }
        };

        match (&self.root_canon, canon_target) {
            (Some(root_canon), Some(t)) => t.starts_with(root_canon),
            // Fall back to a conservative lexical check. This keeps
            // missing in-workspace targets visible as ghosts when the root
            // cannot be canonicalized, while refusing paths that only
            // appear under the root because they contain `..`.
            _ => lexical_path_inside_root(target_abs, &self.root),
        }
    }

    fn drive_relative_target(&self, target_abs: &Path) -> Option<String> {
        if let Some(root_canon) = &self.root_canon {
            if let Ok(canon_target) = target_abs.canonicalize() {
                if let Ok(stripped) = canon_target.strip_prefix(root_canon) {
                    return Some(posix_rel(stripped));
                }
            }
        }
        target_abs.strip_prefix(&self.root).ok().map(posix_rel)
    }
}

fn lexical_path_inside_root(path: &Path, root: &Path) -> bool {
    let Ok(stripped) = path.strip_prefix(root) else {
        return false;
    };
    stripped
        .components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn posix_rel(stripped: &Path) -> String {
    let mut out = String::new();
    for (i, c) in stripped.components().enumerate() {
        if let Component::Normal(s) = c {
            if i > 0 {
                out.push('/');
            }
            out.push_str(&s.to_string_lossy());
        }
    }
    out
}

fn parent_rel(rel: &str) -> String {
    match rel.rsplit_once('/') {
        Some((parent, _)) => parent.to_owned(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    use tempfile::TempDir;

    fn write(p: &Path, body: &str) {
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, body).unwrap();
    }

    fn walk(root: &Path, scope: FsGraphScope, rel: &str, depth: usize) -> FsGraphResponse {
        walk_with_filter(
            root,
            scope,
            rel,
            depth,
            chan_workspace::fs_ops::WalkFilter::default(),
        )
    }

    fn walk_with_filter(
        root: &Path,
        scope: FsGraphScope,
        rel: &str,
        depth: usize,
        filter: chan_workspace::fs_ops::WalkFilter,
    ) -> FsGraphResponse {
        let mut walker = FsGraphWalker::new(root.to_path_buf(), filter);
        let abs = if rel.is_empty() {
            root.to_path_buf()
        } else {
            root.join(rel)
        };
        let meta = std::fs::symlink_metadata(&abs).expect("stat scope");
        match scope {
            FsGraphScope::Directory => walker.walk_directory(rel, &abs, &meta, depth),
            FsGraphScope::File => walker.walk_file(rel, &abs, &meta),
        }
        let (nodes, edges, truncated) = walker.finish();
        FsGraphResponse {
            root: root.display().to_string(),
            scope: scope.label(),
            path: rel.to_owned(),
            depth: match scope {
                FsGraphScope::File => 0,
                FsGraphScope::Directory => depth,
            },
            nodes,
            edges,
            truncated,
        }
    }

    fn node_kind<'a>(resp: &'a FsGraphResponse, id: &str) -> Option<&'a str> {
        resp.nodes.iter().find(|n| n.id == id).map(|n| n.kind)
    }

    #[cfg(unix)]
    fn node<'a>(resp: &'a FsGraphResponse, id: &str) -> Option<&'a NodeView> {
        resp.nodes.iter().find(|n| n.id == id)
    }

    #[cfg(unix)]
    fn node_path_kind(resp: &FsGraphResponse, id: &str) -> Option<chan_workspace::PathKind> {
        node(resp, id).and_then(|n| n.path_class.as_ref().map(|class| class.kind))
    }

    fn has_edge(resp: &FsGraphResponse, src: &str, dst: &str, kind: &str) -> bool {
        resp.edges
            .iter()
            .any(|e| e.source == src && e.target == dst && e.kind == kind)
    }

    #[test]
    fn lexical_fallback_rejects_parent_escape() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("missing-root");
        assert!(lexical_path_inside_root(
            &root.join("notes/missing.md"),
            &root
        ));
        assert!(!lexical_path_inside_root(
            &root.join("../outside.md"),
            &root
        ));
    }

    #[test]
    fn directory_scope_depth_one_lists_direct_children() {
        let tmp = TempDir::new().unwrap();
        write(&tmp.path().join("top.md"), "# top");
        write(&tmp.path().join("sub/nested.md"), "# n");
        write(&tmp.path().join("sub/deep/deep.md"), "# d");

        let resp = walk(tmp.path(), FsGraphScope::Directory, "", 1);
        assert_eq!(node_kind(&resp, "top.md"), Some("file"));
        assert_eq!(node_kind(&resp, "sub"), Some("directory"));
        // Depth=1 must NOT enumerate sub's contents.
        assert!(
            node_kind(&resp, "sub/nested.md").is_none(),
            "depth=1 leaked grandchildren: {:?}",
            resp.nodes.iter().map(|n| &n.id).collect::<Vec<_>>()
        );
        assert!(has_edge(&resp, "", "top.md", "contains"));
        assert!(has_edge(&resp, "", "sub", "contains"));
    }

    #[test]
    fn walk_filter_excludes_blocklisted_dirs_at_any_depth() {
        let tmp = TempDir::new().unwrap();
        write(&tmp.path().join("real.md"), "# real");
        write(&tmp.path().join("notes/today.md"), "# today");
        // Dependency-tree noise the graph must never plot.
        write(&tmp.path().join("node_modules/pkg/index.js"), "x");
        write(&tmp.path().join("target/debug/build.rs"), "x");
        write(&tmp.path().join(".venv/lib/site.py"), "x");
        // Nested blocklist dir below a real dir (any depth).
        write(&tmp.path().join("notes/node_modules/dep/a.js"), "x");

        let filter = chan_workspace::fs_ops::WalkFilter::new([
            "node_modules".to_string(),
            "target".to_string(),
            ".venv".to_string(),
        ]);
        let resp = walk_with_filter(tmp.path(), FsGraphScope::Directory, "", 6, filter);

        // Real content present.
        assert_eq!(node_kind(&resp, "real.md"), Some("file"));
        assert_eq!(node_kind(&resp, "notes"), Some("directory"));
        assert_eq!(node_kind(&resp, "notes/today.md"), Some("file"));

        // Blocklisted dirs and everything under them are absent, at
        // top level and nested.
        for absent in [
            "node_modules",
            "node_modules/pkg",
            "node_modules/pkg/index.js",
            "target",
            "target/debug",
            "target/debug/build.rs",
            ".venv",
            ".venv/lib/site.py",
            "notes/node_modules",
            "notes/node_modules/dep/a.js",
        ] {
            assert!(
                node_kind(&resp, absent).is_none(),
                "blocklisted path leaked into fs-graph: {absent}; nodes={:?}",
                resp.nodes.iter().map(|n| &n.id).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn directory_scope_deeper_includes_grandchildren() {
        let tmp = TempDir::new().unwrap();
        write(&tmp.path().join("sub/nested.md"), "# n");
        write(&tmp.path().join("sub/deep/deep.md"), "# d");

        let resp = walk(tmp.path(), FsGraphScope::Directory, "sub", 2);
        assert_eq!(node_kind(&resp, "sub/nested.md"), Some("file"));
        assert_eq!(node_kind(&resp, "sub/deep"), Some("directory"));
        assert_eq!(node_kind(&resp, "sub/deep/deep.md"), Some("file"));
    }

    #[cfg(unix)]
    #[test]
    fn read_only_directory_is_a_dead_end() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new().unwrap();
        write(&tmp.path().join("locked/hidden.md"), "# hidden");
        fs::set_permissions(tmp.path().join("locked"), fs::Permissions::from_mode(0o555)).unwrap();

        let resp = walk(tmp.path(), FsGraphScope::Directory, "", 2);
        let locked = node(&resp, "locked").expect("locked directory node");
        assert_eq!(locked.kind, "directory");
        assert_eq!(
            locked.permission,
            Some(chan_workspace::PathPermission::ReadOnly)
        );
        assert!(
            node_kind(&resp, "locked/hidden.md").is_none(),
            "read-only directory should not be expanded"
        );
    }

    #[test]
    fn drive_internal_dirs_are_hidden() {
        let tmp = TempDir::new().unwrap();
        write(&tmp.path().join("top.md"), "# t");
        write(&tmp.path().join(".chan/lock"), "x");
        write(&tmp.path().join(".git/HEAD"), "x");

        let resp = walk(tmp.path(), FsGraphScope::Directory, "", 2);
        assert!(node_kind(&resp, ".chan").is_none(), "saw .chan node");
        assert!(node_kind(&resp, ".git").is_none(), "saw .git node");
        assert_eq!(node_kind(&resp, "top.md"), Some("file"));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_in_drive_target_existing() {
        let tmp = TempDir::new().unwrap();
        write(&tmp.path().join("top.md"), "# t");
        symlink("top.md", tmp.path().join("alias.md")).unwrap();

        let resp = walk(tmp.path(), FsGraphScope::Directory, "", 1);
        assert_eq!(node_kind(&resp, "alias.md"), Some("symlink"));
        assert_eq!(
            node_path_kind(&resp, "alias.md"),
            Some(chan_workspace::PathKind::Symlink)
        );
        assert_eq!(node_kind(&resp, "top.md"), Some("file"));
        assert!(
            has_edge(&resp, "alias.md", "top.md", "symlink"),
            "missing in-workspace symlink edge: {:?}",
            resp.edges
                .iter()
                .map(|e| (&e.source, &e.target, e.kind))
                .collect::<Vec<_>>()
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_broken_emits_ghost() {
        let tmp = TempDir::new().unwrap();
        symlink("does-not-exist.md", tmp.path().join("broken.md")).unwrap();

        let resp = walk(tmp.path(), FsGraphScope::Directory, "", 1);
        assert_eq!(node_kind(&resp, "broken.md"), Some("symlink"));
        let ghost = resp
            .nodes
            .iter()
            .find(|n| n.id == "does-not-exist.md")
            .expect("missing ghost node");
        assert_eq!(ghost.kind, "ghost");
        assert!(ghost.broken);
        assert!(has_edge(&resp, "broken.md", "does-not-exist.md", "symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_outside_drive_emits_outside_ghost() {
        let tmp = TempDir::new().unwrap();
        symlink("/etc/hosts", tmp.path().join("escape.md")).unwrap();

        let resp = walk(tmp.path(), FsGraphScope::Directory, "", 1);
        assert_eq!(node_kind(&resp, "escape.md"), Some("symlink"));
        assert_eq!(
            node_path_kind(&resp, "escape.md"),
            Some(chan_workspace::PathKind::Symlink)
        );
        let ghost = resp
            .nodes
            .iter()
            .find(|n| n.id == "outside:escape.md")
            .expect("missing outside ghost");
        assert_eq!(ghost.kind, "ghost");
        assert!(ghost.outside);
        assert!(!ghost.broken);
        assert!(has_edge(&resp, "escape.md", "outside:escape.md", "symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn fifo_and_socket_surface_as_ghost_with_path_class() {
        use std::os::unix::net::UnixListener;

        let tmp = TempDir::new().unwrap();
        let fifo_path = tmp.path().join("pipe.fifo");
        // Shell out to `mkfifo` rather than pulling in libc just for
        // this test. On every platform the build supports the binary
        // is in PATH; if it's missing we skip the assertion so test
        // runs on minimal containers stay green.
        let status = std::process::Command::new("mkfifo")
            .arg(&fifo_path)
            .status();
        match status {
            Ok(s) if s.success() => {}
            _ => return,
        }
        let _listener = UnixListener::bind(tmp.path().join("sock")).unwrap();

        let resp = walk(tmp.path(), FsGraphScope::Directory, "", 1);
        assert_eq!(
            node_kind(&resp, "pipe.fifo"),
            Some("ghost"),
            "FIFO must surface as a ghost, not be silently dropped"
        );
        assert_eq!(
            node_path_kind(&resp, "pipe.fifo"),
            Some(chan_workspace::PathKind::Fifo)
        );
        assert_eq!(
            node_kind(&resp, "sock"),
            Some("ghost"),
            "socket must surface as a ghost, not be silently dropped"
        );
        assert_eq!(
            node_path_kind(&resp, "sock"),
            Some(chan_workspace::PathKind::Socket)
        );
    }

    #[cfg(unix)]
    #[test]
    fn hardlink_emits_hardlink_edge() {
        use std::fs::hard_link;
        let tmp = TempDir::new().unwrap();
        write(&tmp.path().join("top.md"), "# t");
        hard_link(tmp.path().join("top.md"), tmp.path().join("twin.md")).unwrap();

        let resp = walk(tmp.path(), FsGraphScope::Directory, "", 1);
        assert_eq!(node_kind(&resp, "top.md"), Some("file"));
        assert_eq!(node_kind(&resp, "twin.md"), Some("file"));
        // Sorted lexicographically: "top.md" < "twin.md".
        assert!(
            has_edge(&resp, "top.md", "twin.md", "hardlink"),
            "missing hardlink edge: {:?}",
            resp.edges
                .iter()
                .map(|e| (&e.source, &e.target, e.kind))
                .collect::<Vec<_>>()
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_loop_terminates() {
        // a -> b -> a. The walker must terminate (we never traverse
        // symlinks during the directory walk; we only classify their
        // targets) and emit both as symlink nodes pointing at each
        // other.
        let tmp = TempDir::new().unwrap();
        symlink("b.md", tmp.path().join("a.md")).unwrap();
        symlink("a.md", tmp.path().join("b.md")).unwrap();

        let resp = walk(tmp.path(), FsGraphScope::Directory, "", 1);
        assert_eq!(node_kind(&resp, "a.md"), Some("symlink"));
        assert_eq!(node_kind(&resp, "b.md"), Some("symlink"));
        assert!(has_edge(&resp, "a.md", "b.md", "symlink"));
        assert!(has_edge(&resp, "b.md", "a.md", "symlink"));
    }

    #[test]
    fn file_scope_emits_parent_contains_edge() {
        let tmp = TempDir::new().unwrap();
        write(&tmp.path().join("sub/nested.md"), "# n");

        let resp = walk(tmp.path(), FsGraphScope::File, "sub/nested.md", 0);
        assert_eq!(node_kind(&resp, ""), Some("directory"));
        assert_eq!(node_kind(&resp, "sub/nested.md"), Some("file"));
        assert_eq!(node_kind(&resp, "sub"), Some("directory"));
        assert!(has_edge(&resp, "", "sub", "contains"));
        assert!(has_edge(&resp, "sub", "sub/nested.md", "contains"));
    }

    #[test]
    fn file_scope_emits_full_ancestor_chain() {
        let tmp = TempDir::new().unwrap();
        write(&tmp.path().join("a/b/c/deep.md"), "# deep");

        let resp = walk(tmp.path(), FsGraphScope::File, "a/b/c/deep.md", 0);
        assert_eq!(node_kind(&resp, ""), Some("directory"));
        assert_eq!(node_kind(&resp, "a"), Some("directory"));
        assert_eq!(node_kind(&resp, "a/b"), Some("directory"));
        assert_eq!(node_kind(&resp, "a/b/c"), Some("directory"));
        assert_eq!(node_kind(&resp, "a/b/c/deep.md"), Some("file"));
        assert!(has_edge(&resp, "", "a", "contains"));
        assert!(has_edge(&resp, "a", "a/b", "contains"));
        assert!(has_edge(&resp, "a/b", "a/b/c", "contains"));
        assert!(has_edge(&resp, "a/b/c", "a/b/c/deep.md", "contains"));
    }

    #[test]
    fn directory_scope_emits_full_ancestor_chain() {
        let tmp = TempDir::new().unwrap();
        write(&tmp.path().join("a/b/c/deep.md"), "# deep");

        let resp = walk(tmp.path(), FsGraphScope::Directory, "a/b/c", 1);
        assert_eq!(node_kind(&resp, ""), Some("directory"));
        assert_eq!(node_kind(&resp, "a"), Some("directory"));
        assert_eq!(node_kind(&resp, "a/b"), Some("directory"));
        assert_eq!(node_kind(&resp, "a/b/c"), Some("directory"));
        assert!(has_edge(&resp, "", "a", "contains"));
        assert!(has_edge(&resp, "a", "a/b", "contains"));
        assert!(has_edge(&resp, "a/b", "a/b/c", "contains"));
    }

    #[test]
    fn normalize_rel_strips_leading_slash_and_dot() {
        assert_eq!(normalize_rel(""), "");
        assert_eq!(normalize_rel("/"), "");
        assert_eq!(normalize_rel("."), "");
        assert_eq!(normalize_rel("/notes/a.md"), "notes/a.md");
        assert_eq!(normalize_rel("notes/./a.md"), "notes/a.md");
    }

    /// Workspace-bootstrapped tests for the public `build_fs_graph` entry
    /// point — the CLI's `chan graph --scope file|directory` now calls
    /// this directly, so its rejection contract needs explicit
    /// coverage in addition to the walker-only tests above.
    fn open_workspace() -> (TempDir, TempDir, std::sync::Arc<chan_workspace::Workspace>) {
        let cfg = TempDir::new().unwrap();
        let drive_root = TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(drive_root.path()).unwrap();
        let workspace = lib.open_workspace(drive_root.path()).unwrap();
        workspace.write_text("notes/a.md", "# a\n").unwrap();
        (cfg, drive_root, workspace)
    }

    #[test]
    fn build_fs_graph_rejects_escape_path() {
        let (_cfg, _root, workspace) = open_workspace();
        let err = build_fs_graph(&workspace, FsGraphScope::Directory, "../etc", 1).unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert!(
            err.message.contains("escape"),
            "expected escape rejection, got: {}",
            err.message
        );
    }

    #[test]
    fn build_fs_graph_rejects_missing_path() {
        let (_cfg, _root, workspace) = open_workspace();
        let err =
            build_fs_graph(&workspace, FsGraphScope::File, "notes/no-such-file.md", 1).unwrap_err();
        assert_eq!(err.status, StatusCode::NOT_FOUND);
        assert!(
            err.message.contains("no such path"),
            "expected missing-path rejection, got: {}",
            err.message
        );
    }

    #[test]
    fn build_fs_graph_rejects_directory_scope_on_file() {
        let (_cfg, _root, workspace) = open_workspace();
        let err = build_fs_graph(&workspace, FsGraphScope::Directory, "notes/a.md", 1).unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert!(
            err.message.contains("requires a directory"),
            "expected directory-required rejection, got: {}",
            err.message
        );
    }

    #[cfg(unix)]
    #[test]
    fn build_fs_graph_rejects_mid_path_symlink_escape() {
        // syseng's design-snapshot flagged this: an in-workspace symlink
        // pointing OUTSIDE the workspace root used to be silently
        // followed when it appeared as a mid-path component, because
        // `resolve_safe` is lexical only. A request like
        // `path=alias/inside.md` (alias -> /etc) leaked /etc/inside.md
        // metadata under a workspace-relative id. ensure_parent_inside_drive
        // closes that.
        let (_cfg, root, workspace) = open_workspace();
        // Build a symlink whose target is OUTSIDE the workspace root,
        // pointing at a directory that definitely exists on every
        // posix system.
        symlink("/etc", root.path().join("escape-link")).unwrap();

        // Directory scope through the escape link: hostnames dir on
        // macOS, hosts dir on Linux. Pick a path that's almost
        // certainly present.
        let err =
            build_fs_graph(&workspace, FsGraphScope::Directory, "escape-link/ssl", 1).unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert!(
            err.message.contains("escapes workspace root"),
            "expected mid-path escape rejection, got: {}",
            err.message
        );

        // File scope through the escape link: any single file under
        // /etc. `hosts` is the canonical pick.
        let err =
            build_fs_graph(&workspace, FsGraphScope::File, "escape-link/hosts", 1).unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert!(
            err.message.contains("escapes workspace root"),
            "expected mid-path escape rejection, got: {}",
            err.message
        );
    }

    #[cfg(unix)]
    #[test]
    fn build_fs_graph_allows_in_drive_symlink_leaf_to_outside() {
        // The mid-path guard must NOT reject when the LEAF itself is
        // an in-workspace symlink pointing outside the workspace. The walker
        // classifies that leaf via readlink and emits an outside-
        // ghost node — that's the documented behavior, and it's the
        // whole point of having a graph route over filesystems with
        // symlinks.
        let (_cfg, root, workspace) = open_workspace();
        symlink("/etc/hosts", root.path().join("alias-outside.md")).unwrap();

        let resp = build_fs_graph(&workspace, FsGraphScope::File, "alias-outside.md", 0)
            .expect("in-workspace symlink leaf must be accepted");
        // Expect the symlink node + an outside ghost target.
        assert!(
            resp.nodes
                .iter()
                .any(|n| n.id == "alias-outside.md" && n.kind == "symlink"),
            "missing symlink node: {:?}",
            resp.nodes.iter().map(|n| &n.id).collect::<Vec<_>>()
        );
        assert!(
            resp.nodes
                .iter()
                .any(|n| n.id == "outside:alias-outside.md" && n.outside),
            "missing outside-workspace ghost node: {:?}",
            resp.nodes.iter().map(|n| &n.id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn build_fs_graph_root_scope_returns_drive_root() {
        let (_cfg, _root, workspace) = open_workspace();
        let resp = build_fs_graph(&workspace, FsGraphScope::Directory, "", 1).unwrap();
        assert_eq!(resp.scope, "directory");
        assert_eq!(resp.path, "");
        // Workspace root is keyed by the empty string; depth 1 lists the
        // top-level `notes/` directory.
        assert!(
            resp.nodes
                .iter()
                .any(|n| n.id.is_empty() && n.kind == "directory"),
            "workspace root node missing from response"
        );
        assert!(
            resp.nodes
                .iter()
                .any(|n| n.id == "notes" && n.kind == "directory"),
            "notes/ should be a direct child at depth 1"
        );
    }
}
