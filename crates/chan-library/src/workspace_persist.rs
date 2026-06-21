//! The persisted workspace on/off overlay shared by every chan-library
//! deployment.
//!
//! A chan-library's existence source is its registry (`chan workspace ls` /
//! `Library::list_workspaces`) — the set of workspaces it owns. What the
//! registry does NOT record is which of those a given deployment had MOUNTED
//! (`on`) versus registered-but-unmounted (`off`) at its last save. That on/off
//! state is this overlay: a [`PersistedWorkspace`] row keyed by path, persisted
//! by both the desktop-local library (`~/.chan/desktop/config.json`) and the
//! headless devserver (`~/.chan/devserver/config.json`) so a restart comes back
//! serving exactly what was on.
//!
//! The route `prefix` a workspace mounts at is deliberately NOT persisted: it is
//! a pure function of the root path, derived per library by that library's own
//! scheme (the devserver's gateway-legible slug via
//! [`allocate_workspace_prefix`](crate::allocate_workspace_prefix); a hashed
//! window label for the local desktop). Persisting it would pin one library's
//! scheme into a shape the other reads — so each library re-derives its own
//! prefix at restore.
//!
//! The TYPE is shared; the STORE is not — each library keeps this overlay in its
//! own config file (the desktop's `config.json`, the devserver's `config.json`).
//! A future step could lift the whole overlay into one library-owned store both
//! sides route through (the [`WindowRegistry`](crate::windows::WindowRegistry)
//! pattern), if the duplication ever earns it; today the shared shape is enough.

use serde::{Deserialize, Serialize};

/// One workspace's persisted on/off state: the `path` that identifies it (the
/// registry key) and whether it was mounted (`on`) at the last save. The
/// registry is the existence source; this is the on/off overlay over it. A row
/// absent from the overlay defaults to off — the registry still surfaces it.
/// The mount prefix is re-derived per library at restore, not stored here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistedWorkspace {
    /// Filesystem path identifying the workspace (the registry key).
    pub path: String,
    /// Whether the workspace was mounted (`on`) at the last save.
    pub on: bool,
}
