//! profile-service: internal HTTP API in front of Postgres.
//!
//! Owns the canonical user record, linked OAuth identities, and the
//! per-user list of workspace URLs (`chan serve` endpoints). Called only
//! by sibling gateway services (identity, workspace-proxy); not exposed
//! publicly. Auth is a shared bearer token in v0; replace with mTLS
//! or signed service tokens once there's a second caller.

pub mod config;
pub mod db;
pub mod error;
pub mod http;
pub mod models;

pub use config::Config;
pub use error::{Error, Result};
