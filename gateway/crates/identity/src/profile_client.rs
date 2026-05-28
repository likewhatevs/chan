//! Re-export of the shared profile-service client.
//!
//! The implementation lives in `gateway_common::profile_client`;
//! this shim keeps the `crate::profile_client::*` import path
//! intact for the rest of identity-service. `From<ProfileError> for
//! Error` is in `crate::error` so the request handlers can `?` a
//! profile call straight through.

pub use gateway_common::profile_client::*;
