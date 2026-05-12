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
/// stream opens. The token in the `Authorization` header
/// authenticates the user; this frame names the drive the client
/// wants to expose. Tokens are user-scoped, not drive-scoped, so
/// the same token can register `(user, notes)` and `(user,
/// journal)` from two separate `chan serve` instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
    pub protocol: ProtocolVersion,
    /// chan version string (e.g. "chan/0.4.0"). Server-side logs
    /// only; not used for routing.
    pub client_version: String,
    /// Drive name to register under. Combined with the token's
    /// user to form the public path `/{user}/{drive}/...`.
    pub drive: String,
    /// When true, the public proxy lets anonymous visitors reach
    /// this drive without an OAuth round-trip. When false (default),
    /// only the drive owner's signed-in session can reach it.
    /// Additive field; older clients omitting it default to false.
    #[serde(default)]
    pub public: bool,
}

/// First frame, server -> client. Tells the client where on the
/// public host its drive will be served, so `chan serve` can wire
/// the prefix into its router without the user passing --prefix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloAck {
    pub protocol: ProtocolVersion,
    /// Public path prefix on the gateway's wildcard subdomain.
    /// Shape: `/{drive}` (one leading slash, no trailing slash).
    /// The username lives in the host (`{user}.drive.chan.app`),
    /// not in the path; chan-server uses this value as
    /// `<meta name="chan-prefix">` so the SPA's relative URLs
    /// resolve under that drive.
    pub prefix: String,
    pub user: String,
    pub drive: String,
}
