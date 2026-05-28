//! Re-export of the shared workspace-proxy admin client.
//!
//! Implementation lives in `gateway_common::workspace_admin_client`; this
//! shim keeps the `crate::workspace_admin_client::*` import path stable
//! for the rest of identity-service. `From<WorkspaceAdminError> for
//! Error` is in `crate::error` so request handlers can `?` an admin
//! call straight through.

pub use gateway_common::workspace_admin_client::*;
