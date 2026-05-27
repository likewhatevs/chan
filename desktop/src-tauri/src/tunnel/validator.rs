//! Local tunnel-server validator.
//!
//! The bearer token presented by the remote `chan serve` is opaque
//! to the protocol; this validator returns it verbatim as
//! `Validated.username`. In local-tunnel use that string IS the
//! tenant label shown in the Workspaces window and used as the first URL
//! segment routed by the tunnel server's public router. There is
//! no shared secret and no mapping table: auth is the SSH tunnel
//! plus the loopback bind of the tunnel listener, not the token.
//!
//! Both `TUNNEL_SCOPE` and `TUNNEL_PUBLIC_SCOPE` are granted. There
//! is no separate "public router with auth" downstream — every
//! per-tenant listener is loopback-only — so the public bit on the
//! Hello frame is informational (we surface it on the UI row) and
//! has no privilege-escalation meaning locally.

use async_trait::async_trait;
use chan_tunnel_proto::is_valid_username;
use chan_tunnel_server::{ServerError, Validated, Validator, TUNNEL_PUBLIC_SCOPE, TUNNEL_SCOPE};

pub struct LocalValidator;

#[async_trait]
impl Validator for LocalValidator {
    async fn validate(&self, token: &str) -> Result<Validated, ServerError> {
        // The token flows verbatim into the public URL path as the
        // first segment, so we apply the same charset check the
        // chan-tunnel-server handshake would apply defensively to
        // any validator output. Failing here returns 401 on the
        // wire, matching how chan-tunnel-server treats InvalidToken
        // (see tunnel.rs's status mapping).
        if !is_valid_username(token) {
            return Err(ServerError::InvalidToken);
        }
        Ok(Validated {
            user_id: uuid::Uuid::nil(),
            username: token.to_string(),
            scopes: vec![TUNNEL_SCOPE.to_string(), TUNNEL_PUBLIC_SCOPE.to_string()],
        })
    }
}
