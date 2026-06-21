//! Deterministic route-prefix allocation for hosted tenants.
//!
//! A workspace mounts at its PUBLIC slug `/{slug}` (the last path segment,
//! sanitized) so the same root always maps to the same route across restarts —
//! and so the window feed can name an OFF workspace's prefix without a live
//! tenant (O-W2). The slug IS the public path the gateway forwards
//! (`{user}.devserver.chan.app/{slug}/`), so the proxy hands the devserver the
//! public path unchanged and the devserver routes the tenant by it. Slug
//! uniqueness within a devserver is required: two roots with the same basename
//! collide and the second is rejected at mount time. The window-record assembly
//! on `WorkspaceHost` calls [`allocate_workspace_prefix`] for the off-workspace
//! case; the devserver calls it on mount.

use std::path::Path;

use crate::serve_config::sanitize_prefix;
use crate::Error;

/// Allocate a workspace's mount prefix from its root: `/{slug}`, where `slug`
/// is the sanitized last path segment. Deterministic so the same root always
/// maps to the same route — the workspace re-mounts at the same prefix across a
/// restart, and an OFF workspace has a stable prefix without a live tenant. The
/// prefix is the PUBLIC slug the gateway forwards; two same-basename roots map
/// to the same slug and the devserver rejects the second at mount time.
pub fn allocate_workspace_prefix(root: &Path) -> Result<String, Error> {
    let slug = workspace_slug(root);
    sanitize_prefix(&format!("/{slug}")).map_err(Error::Config)
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
