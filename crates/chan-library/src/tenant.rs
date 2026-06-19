//! The seams between the library core and the route layer that builds tenants.
//!
//! Two traits keep `chan-server → chan-library` acyclic while the host and the
//! control socket live here:
//!
//! - [`WorkspaceCellHandle`] — how the host and the control socket reach a
//!   tenant's live workspace + its indexer, without naming `chan-server`'s
//!   `WorkspaceCell` / `Indexer` (which stay in the route layer). The route
//!   layer hands back an `Arc<dyn WorkspaceCellHandle>` over the shared cell.
//! - [`HostControl`] — the slice of the host a control-socket connection
//!   reaches through a `Weak` back-reference: unmount a tenant (`chan unserve`)
//!   and read the library window set (`cs window list`). Lets the control
//!   socket hold `Weak<dyn HostControl>` instead of a concrete `WorkspaceHost`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};

use chan_workspace::Workspace;

use crate::windows::WindowRecord;
use crate::Error;

/// A handle to one tenant's live workspace cell, owned by the route layer
/// (it wraps `chan-server`'s `WorkspaceCell`, which holds the search indexer).
/// The host drives tenant teardown + reindex cancellation through this without
/// depending on the route layer's concrete cell type.
pub trait WorkspaceCellHandle: Send + Sync {
    /// The live `Arc<Workspace>`, or `None` for a terminal-only tenant or the
    /// brief `/api/storage/reset` swap window.
    fn workspace(&self) -> Option<Arc<Workspace>>;

    /// Cancel any in-flight reindex (host shutdown / `cancel_all_reindex`).
    fn cancel_reindex(&self);

    /// Tear the cell down — cancel the indexer, drop the watcher + the strong
    /// `Arc<Workspace>` — and return a `Weak` to the workspace plus its lock
    /// directory so the host can wait for the per-workspace flock to release
    /// before an immediate reopen. `None` when the cell was already cleared.
    fn clear(&self) -> Option<(Weak<Workspace>, PathBuf)>;
}

/// The host operations a control-socket connection reaches through a `Weak`
/// back-reference (`WorkspaceHost` registers itself via `install_self`). Held
/// as `Weak<dyn HostControl>` so the control socket never names the concrete
/// host type.
pub trait HostControl: Send + Sync {
    /// Unmount the hosted tenant whose root matches `root` (the `chan unserve`
    /// over-the-host path). `Ok(false)` when no tenant owns that path.
    fn close_workspace_for_root(&self, root: &Path) -> Result<bool, Error>;

    /// The full library window set — every window across every tenant, as the
    /// authoritative records `cs window list` and the launcher render. Assembled
    /// from the window registry + live tenant + presence state.
    fn assemble_window_records(&self) -> Vec<WindowRecord>;
}
