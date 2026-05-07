//! Control frames exchanged once before yamux takes over.

use serde::{Deserialize, Serialize};

/// Wire-format protocol version. Bumped only on incompatible
/// changes; additive fields are tolerated via `#[serde(default)]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProtocolVersion(pub u16);

impl ProtocolVersion {
    pub const V1: ProtocolVersion = ProtocolVersion(1);
}

/// First frame, client -> server. Sent right after the HTTP/2
/// stream opens. The token itself rides in the `Authorization`
/// header on the POST; Hello carries client-identifying metadata
/// that helps with logs and future capability negotiation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
    pub protocol: ProtocolVersion,
    /// chan version string (e.g. "chan/0.4.0"). Server-side logs
    /// only; not used for routing.
    pub client_version: String,
    /// Optional drive name hint. The token is the source of truth;
    /// when both are present and disagree, the server rejects the
    /// connection rather than silently picking one.
    #[serde(default)]
    pub drive_hint: Option<String>,
}

/// First frame, server -> client. Tells the client where on the
/// public host its drive will be served, so `chan serve` can wire
/// the prefix into its router without the user passing --prefix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloAck {
    pub protocol: ProtocolVersion,
    /// Public path prefix, e.g. `/u/alice/notes`. Always starts
    /// with `/` and never ends with one.
    pub prefix: String,
    pub user: String,
    pub drive: String,
}
