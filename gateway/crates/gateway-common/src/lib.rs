//! Shared helpers for the chan-gateway crates.
//!
//! Five modules:
//!
//!   * `profile_client`: typed HTTP client for profile-service. Used
//!     by identity-service and drive-proxy. Owns its own error enum
//!     (`ProfileError`); each consumer maps it onto its local axum
//!     error via a `From` impl.
//!   * `drive_admin_client`: typed HTTP client for drive-proxy's
//!     `/admin/v1/*` tree. Used by identity-service (on revoke /
//!     delete / dashboard reads) and profile-service (on admin
//!     block) so a state change in the DB also tears down the
//!     in-process yamux registrations drive-proxy holds for the
//!     user.
//!   * `drive_gate`: shared JWT envelope and HS256 encode/decode
//!     helpers for the drive-gate handoff. identity mints entry
//!     tokens; drive-proxy verifies entry tokens and mints session
//!     tokens. Same envelope, same secret (DRIVE_GATE_SECRET),
//!     distinct `typ` claim.
//!   * `static_files`: rust-embed-backed SPA-fallback handler. Each
//!     consumer keeps its own `#[derive(Embed)]` (rust-embed resolves
//!     the `#[folder]` path relative to the deriving crate) and calls
//!     `static_files::serve::<Assets>(uri, banner)`. Used today only
//!     by identity-service; drive-proxy ships no SPA.
//!   * `token_bucket`: per-fingerprint token bucket with a bounded
//!     map. Backs the brute-force throttle in
//!     `drive_proxy::throttle_validator` and
//!     `identity::token_throttle`; both wrap this primitive in a
//!     thin trait-level adapter.

pub mod drive_admin_client;
pub mod drive_gate;
pub mod profile_client;
pub mod shutdown;
pub mod static_files;
pub mod token_bucket;
pub mod validators;

pub use shutdown::shutdown_signal;
