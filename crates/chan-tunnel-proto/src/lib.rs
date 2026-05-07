//! chan-tunnel wire types.
//!
//! The transport between `chan serve` (client) and chan-tunneld
//! (server) is a single HTTP/2 bidirectional stream. The first
//! message in each direction is a length-prefixed JSON control
//! frame; after that, both sides hand the byte stream to yamux.
//!
//! This crate is pure data: framing helpers and serde types. No
//! I/O, no async. Both client and server depend on it.

#![forbid(unsafe_code)]

mod control;
mod frame;
mod io;

pub use control::{Hello, HelloAck, ProtocolVersion};
pub use frame::{decode_frame, encode_frame, FrameError};
pub use io::{read_frame, write_frame, IoFrameError};

/// Path the client POSTs to on the public tunnel host. Stable
/// across versions; protocol version is negotiated inside the Hello
/// frame, not via a path bump.
pub const TUNNEL_PATH: &str = "/v1/tunnel";

/// Maximum size of a single control frame in bytes. Control frames
/// are tiny; this guards against a malicious or buggy peer trying to
/// allocate gigabytes before yamux even starts.
pub const MAX_CONTROL_FRAME_BYTES: usize = 64 * 1024;
