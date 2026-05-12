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
    ///
    /// Setting this to `true` is a privilege-escalation request:
    /// the server gates it on an extra token scope
    /// (`chan_tunnel_server::TUNNEL_PUBLIC_SCOPE`). Clients without
    /// that scope are refused with `MissingPublicScope`, so the
    /// client cannot unilaterally make its drive anonymous-readable.
    #[serde(default)]
    pub public: bool,
}

/// First frame, server -> client. Either confirms the
/// registration and tells the client where on the public host its
/// drive will be served, or refuses the handshake with a
/// structured reason so the client can render something better
/// than "transport closed".
///
/// Pre-audit the refusal case was a bare transport disconnect
/// after the 200 response; clients could not distinguish
/// "TooManyDrives" from "TLS reset". The tagged enum gives the
/// server one place to write a structured refusal in the same
/// stream the success ack would have used.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HelloAck {
    /// Registration accepted; carries the assigned public path
    /// prefix the client uses to wire its router.
    Ok(HelloAckOk),
    /// Registration refused after the token + Hello were
    /// validated; carries a stable `code` for client-side matching
    /// plus a human-readable `message`.
    Refused(HelloAckErr),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloAckOk {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloAckErr {
    pub protocol: ProtocolVersion,
    /// Stable, machine-readable refusal code. Clients match on
    /// these to produce specific UI; see `error_code` constants.
    pub code: String,
    /// Human-readable, operator-visible. Safe to log and surface
    /// to the user.
    pub message: String,
}

/// Stable refusal codes emitted by the server in `HelloAckErr.code`.
/// Add new codes here when introducing new pre-ack-stage failure
/// shapes; clients should fall back to a generic surface for codes
/// they do not recognise so the protocol stays additive.
pub mod error_code {
    /// `Hello.public = true` from a token without TUNNEL_PUBLIC_SCOPE.
    pub const MISSING_PUBLIC_SCOPE: &str = "missing_public_scope";
    /// Registering this drive would exceed the per-user cap.
    pub const TOO_MANY_DRIVES: &str = "too_many_drives";
    /// `Hello.drive` failed `is_valid_drive_name`.
    pub const INVALID_DRIVE_NAME: &str = "invalid_drive_name";
    /// `Hello.protocol` did not match the server's supported
    /// version. Reserved for future use; today the listener still
    /// closes the stream pre-ack for this case.
    pub const UNSUPPORTED_PROTOCOL: &str = "unsupported_protocol";
    /// Catch-all for refusals the client doesn't have a specific
    /// branch for. Treat the `message` as the only useful payload.
    pub const INTERNAL: &str = "internal";
}
