//! The launcher SPA root surface + the `/api/library/*` serve handlers.
//!
//! `web-launcher/` is a pure `/api/library/*` HTTP client served at the
//! devserver/library root `/`. chan-library's `host_dispatch` 404s the root
//! (it only matches workspace-tenant prefixes); this module builds the router
//! the embedder installs as the host's root fallback
//! (`WorkspaceHost::install_root_fallback`) so `/` serves the launcher and
//! `/api/library/*` reaches the library handles. It lives in chan-server, not
//! chan-library, because it serves a frontend bundle and the crate dependency
//! only flows chan-server -> chan-library.

use axum::Router;

use crate::static_assets::serve_launcher;

/// The launcher router installed as the `WorkspaceHost` root fallback. Today it
/// is the launcher SPA static surface (assets + index, with the SPA fallback to
/// `index.html`). The `/api/library/*` serve handlers are layered in here as the
/// loopback gaps vs `web-launcher/src/api/library.ts` are filled — the launcher
/// already mirrors the `/api/library/windows*` handlers served in
/// `crate::devserver`, so the remaining surface is workspaces + devservers.
pub fn launcher_router() -> Router {
    Router::new().fallback(serve_launcher)
}
