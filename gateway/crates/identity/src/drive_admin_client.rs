//! Re-export of the shared drive-proxy admin client.
//!
//! Implementation lives in `gateway_common::drive_admin_client`; this
//! shim keeps the `crate::drive_admin_client::*` import path stable
//! for the rest of identity-service. `From<DriveAdminError> for
//! Error` is in `crate::error` so request handlers can `?` an admin
//! call straight through.

pub use gateway_common::drive_admin_client::*;
