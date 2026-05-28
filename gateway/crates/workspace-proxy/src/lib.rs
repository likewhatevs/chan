//! workspace-proxy: public-facing service at workspace.chan.app (apex) and
//! *.workspace.chan.app (wildcard).
//!
//! Two TCP listeners share the process, fronted by one nginx vhost
//! per role:
//!
//!   * `bind_addr` (workspace.chan.app apex + *.workspace.chan.app wildcard):
//!     axum HTTP. The wildcard host carries the tenant reverse-proxy
//!     surface; the apex carries `/admin/v1/*` and `/healthz` only.
//!     A single router dispatches on the `Host` header. workspace-proxy
//!     reads no session cookie. The proxy gate uses a workspace-gate JWT
//!     (HS256, secret shared with identity-service) carried in the
//!     entry URL or in a host-only, path-scoped `workspace_gate` cookie.
//!
//!   * `tunnel_bind_addr` (workspace.chan.app apex, behind nginx
//!     `grpc_pass` on `/v1/tunnel`): h2c handshake for chan-tunnel
//!     clients. Embeds `chan_tunnel_server` as a library and shares
//!     the same in-process `Registry` with the public side.
//!
//! No SPA ships in this binary. The dashboard, sign-in surface and
//! workspace list live at id.chan.app.

pub mod admin;
pub mod config;
pub mod error;
pub mod http;
pub mod identity_validator;
pub mod proxy;
pub mod registry;
pub mod throttle_validator;

pub use config::Config;
pub use error::{Error, Result};
