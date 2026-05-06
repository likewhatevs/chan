//! HTTP + WebSocket surface for chan.
//!
//! Wraps chan-core's `Library` and `Drive` handles in axum routes
//! and serves the embedded web editor frontend (rust-embed, wired
//! in a later iteration). Routes will be ported in successive
//! commits from the old `chan-core/src/server.rs` in `fiorix/chan`.
//!
//! The crate is intentionally a stub at the initial commit so the
//! workspace compiles end-to-end; substantive routes land per the
//! migration plan in `design.md`.

#![forbid(unsafe_code)]

use std::net::SocketAddr;

use chan_core::Drive;
use std::sync::Arc;

/// Configuration the binary hands the server at boot. Kept terse on
/// purpose; expand only when a route demands it.
#[derive(Debug, Clone)]
pub struct ServeConfig {
    pub addr: SocketAddr,
    /// When false, the server skips the per-launch token gate. Used
    /// by tests and by the desktop shell embedding the server in the
    /// same process. Loopback binds + bearer-token gate is the
    /// default; do not flip this in production.
    pub no_token: bool,
}

/// Spawn-and-serve entry point. Returns when the server is shut
/// down. The actual axum router is empty in this initial commit;
/// routes port in follow-up changes.
pub async fn serve(_drive: Arc<Drive>, _config: ServeConfig) -> Result<(), Error> {
    Err(Error::NotImplemented)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("chan-server is not implemented yet; routes port in follow-up commits")]
    NotImplemented,
    #[error("chan-core: {0}")]
    Core(#[from] chan_core::ChanError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}
