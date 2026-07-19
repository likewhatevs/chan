//! Shared helpers for the chan-gateway crates.
//!
//! Modules:
//!
//!   * `profile_client`: typed HTTP client for profile-service. Used
//!     by identity-service. Owns its own error enum (`ProfileError`);
//!     the consumer maps it onto its local axum error via a `From`
//!     impl.
//!   * `shutdown`: graceful-shutdown future (SIGTERM or Ctrl-C) used
//!     by every service binary.
//!   * `static_files`: rust-embed-backed SPA-fallback handler. Each
//!     consumer keeps its own `#[derive(Embed)]` (rust-embed resolves
//!     the `#[folder]` path relative to the deriving crate) and calls
//!     `static_files::serve::<Assets>(uri, banner)`. Used only by
//!     identity-service; devserver-proxy ships no SPA.
//!   * `token_bucket`: per-fingerprint token bucket with a bounded
//!     map, plus the shared default limits. Backs the brute-force
//!     throttle in `devserver_proxy::throttle_validator` and
//!     `identity::token_throttle`; both wrap this primitive in a
//!     thin trait-level adapter.
//!   * `validators`: username shape validation and the lifetime
//!     rename cap, shared by identity, profile, and devserver-proxy.
//!   * `workspace_admin_client`: typed HTTP client for devserver-proxy's
//!     `/admin/v1/*` tree. Used by identity-service (on revoke /
//!     delete / dashboard reads) and profile-service (on admin
//!     block) so a state change in the DB also tears down the
//!     in-process yamux registrations devserver-proxy holds for the
//!     user.
//!   * `devserver_gate`: shared JWT envelope and HS256 encode/decode
//!     helpers for the devserver-gate handoff. identity mints entry
//!     tokens; devserver-proxy verifies entry tokens and mints session
//!     tokens. Same envelope, same secret (DEVSERVER_GATE_SECRET),
//!     distinct `typ` claim.

pub mod devserver_gate;
pub mod profile_client;
pub mod shutdown;
pub mod static_files;
pub mod token_bucket;
pub mod validators;
pub mod workspace_admin_client;

pub use shutdown::shutdown_signal;
