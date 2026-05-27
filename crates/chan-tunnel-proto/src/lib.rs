//! chan-tunnel wire types.
//!
//! The transport between `chan serve` (client) and the tunnel
//! terminator (server) is a single HTTP/2 bidirectional stream.
//! The first message in each direction is a length-prefixed JSON
//! control frame; after that, both sides hand the byte stream to
//! yamux.
//!
//! This crate is pure data: framing helpers and serde types. No
//! I/O, no async. Both client and server depend on it.

#![forbid(unsafe_code)]

mod control;
mod frame;
mod h2_duplex;
mod io;
mod workspace_name;

pub use control::{error_code, Hello, HelloAck, HelloAckErr, HelloAckOk, ProtocolVersion};
pub use frame::{decode_frame, encode_frame, FrameError};
pub use h2_duplex::H2Duplex;
pub use io::{read_frame, write_frame, IoFrameError};
pub use workspace_name::{
    is_valid_username, is_valid_workspace_name, sanitize_workspace_name, MAX_USERNAME_LEN,
    MAX_WORKSPACE_NAME_LEN,
};

/// Path the client POSTs to on the public tunnel host. Stable
/// across versions; protocol version is negotiated inside the Hello
/// frame, not via a path bump.
pub const TUNNEL_PATH: &str = "/v1/tunnel";

/// Maximum size of a single control frame in bytes. Control frames
/// are tiny; this guards against a malicious or buggy peer trying to
/// allocate gigabytes before yamux even starts.
pub const MAX_CONTROL_FRAME_BYTES: usize = 64 * 1024;
