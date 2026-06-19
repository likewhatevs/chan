//! Deterministic route-prefix allocation for hosted tenants.
//!
//! A workspace mounts at a stable, content-derived prefix so the same root
//! always maps to the same route across restarts — and so the window feed can
//! name an OFF workspace's prefix without a live tenant (O-W2). The
//! window-record assembly on `WorkspaceHost` calls [`allocate_workspace_prefix`]
//! for that off-workspace case; the devserver calls it on mount.

use std::hash::{Hash, Hasher};
use std::path::Path;

use crate::serve_config::sanitize_prefix;
use crate::Error;

/// Allocate a workspace's mount prefix from its root: `/api/{slug}-{hash}`,
/// deterministic so the same root always maps to the same route — the
/// workspace re-mounts at the same prefix across a restart, and an OFF
/// workspace has a stable prefix without a live tenant.
pub fn allocate_workspace_prefix(root: &Path) -> Result<String, Error> {
    let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    canonical.hash(&mut hasher);
    let hash = hasher.finish();
    let slug = workspace_slug(root);
    sanitize_prefix(&format!("/api/{slug}-{hash:x}")).map_err(Error::Config)
}

/// Sanitize a path's final component into a legible `[a-z0-9-]` slug for a
/// prefix: lowercase, non-alphanumerics to `-`, collapsed and trimmed, length
/// capped, with a fallback for an empty result.
pub fn workspace_slug(root: &Path) -> String {
    let raw = root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let mut slug: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    let trimmed: String = slug.trim_matches('-').chars().take(24).collect();
    let trimmed = trimmed.trim_matches('-');
    if trimmed.is_empty() {
        "workspace".to_string()
    } else {
        trimmed.to_string()
    }
}
