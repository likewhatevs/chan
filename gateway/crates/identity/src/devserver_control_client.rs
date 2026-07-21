//! Re-export of the shared devserver-control admin client.
//!
//! Implementation lives in `gateway_common::devserver_control_client`;
//! this shim keeps the `crate::devserver_control_client::*` import
//! path stable for the rest of identity-service.
//! `From<DevserverControlError> for Error` is in `crate::error` so
//! request handlers can `?` an admin call straight through.

pub use gateway_common::devserver_control_client::*;
