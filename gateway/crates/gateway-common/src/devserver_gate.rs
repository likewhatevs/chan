//! Devserver-gate JWT envelope shared by identity-service (mints entry
//! tokens) and devserver-proxy (verifies entry tokens, mints + verifies
//! session cookies). HS256 only; `alg: none` is hard-rejected by the
//! validation config.
//!
//! Token shapes:
//!
//! * `typ: "entry"` (issued by identity, lives in `?t=` URL param,
//!   30s exp). After successful verification devserver-proxy mints a
//!   session token of the same envelope but with `typ: "session"`,
//!   sets it as a host-only `Path=/` cookie, and 303s to the
//!   clean URL.
//! * `typ: "session"` (issued and verified by devserver-proxy, lives in
//!   the `devserver_gate` cookie, 24h exp).
//!
//! Both shapes carry the same envelope so the verify path can decode
//! once and dispatch on `typ`. `aud` binds the token to the wildcard
//! host (`alice.devserver.chan.app`) so a token minted for one user is
//! not accepted on another user's subdomain. `drv` binds it to one
//! live devserver registration for the same reason.
//!
//! Why HS256 (symmetric): both services run in the same trust zone
//! and share the same secret already (`DEVSERVER_GATE_SECRET`, a
//! cross-service secret kept generic across the rename). HS256
//! gives the smallest token footprint and the simplest key rotation
//! (one secret rotation invalidates every live token). Asymmetric
//! buys nothing here.
//!
//! Optional identity claims (`name`, `email`) are resolved by
//! identity-service at entry mint and propagated into the session
//! token unchanged, so a display-name change stays stale until the
//! next entry mint (up to the 24h session-cookie lifetime). They are
//! cosmetic (participant display strings on the devserver), never an
//! authorization input.

use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{
    decode as jwt_decode, encode as jwt_encode, Algorithm, DecodingKey, EncodingKey, Header,
    Validation,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One envelope for both shapes; `typ` discriminates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Stable logical token issuer. Verified opportunistically (we trust the
    /// signature; `iss` is a debug aid in logs), so it names the service role
    /// instead of a deployment hostname.
    pub iss: String,
    /// `users.id` of the workspace owner.
    pub sub: Uuid,
    /// Access role resolved by profile-service: `owner`, `editor`, or
    /// `viewer`.
    #[serde(default = "default_role")]
    pub role: String,
    /// Devserver id resolved from the live tunnel registration.
    pub drv: String,
    /// Exact tenant host the token is bound to.
    pub aud: String,
    /// `"entry"` or `"session"`. See module doc.
    pub typ: String,
    /// Caller display name, resolved by identity-service at entry
    /// mint. Absent when the minting service predates identity claims
    /// or the profile lookup failed. See the module doc for the
    /// staleness bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Caller email, same provenance as `name`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub iat: i64,
    pub exp: i64,
}

impl Claims {
    /// The identity bundle carried by this token, for propagating an
    /// entry token's identity into the session mint.
    pub fn identity(&self) -> CallerIdentity {
        CallerIdentity {
            name: self.name.clone(),
            email: self.email.clone(),
        }
    }
}

/// Caller identity attached to a token at mint time. `Default` is the
/// no-identity bundle for callers that have nothing to attach.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallerIdentity {
    pub name: Option<String>,
    pub email: Option<String>,
}

fn default_role() -> String {
    "viewer".to_string()
}

/// Discriminator for the verify call site so we can hard-require the
/// shape we expect at each hop. Decoupled from the `typ` string so a
/// future shape (`refresh`?) doesn't require parsing surgery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Entry,
    Session,
}

impl TokenType {
    pub fn as_str(self) -> &'static str {
        match self {
            TokenType::Entry => "entry",
            TokenType::Session => "session",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DevserverGateError {
    /// Signature failed HMAC verify, or the wire shape is bad.
    /// Library-level decode errors collapse here; the only thing we
    /// surface upstream is "token bad."
    #[error("invalid devserver-gate token: {0}")]
    Decode(String),

    /// `exp` is in the past. Common case for an expired session
    /// cookie; the caller should treat this the same way as "no
    /// cookie at all" (404 on the proxy path) so existence does not
    /// leak.
    #[error("devserver-gate token expired")]
    Expired,

    /// `aud` claim does not match the expected host.
    #[error("devserver-gate token audience mismatch")]
    WrongAudience,

    /// `drv` claim does not match the requested workspace slug.
    #[error("devserver-gate token devserver mismatch")]
    WrongWorkspace,

    /// `typ` claim does not match the verify-call's expectation.
    /// Defensive: prevents an "entry" token being replayed as a
    /// session cookie or vice versa. `got` is attacker-controlled
    /// (any string the caller put in the JWT), so the Display form
    /// only surfaces `want` to avoid echoing arbitrary content into
    /// any future log site that formats the error. Operators who
    /// need the observed value can read it directly off the variant.
    #[error("devserver-gate token type mismatch (want {want:?})")]
    WrongType { got: String, want: &'static str },
}

pub type DevserverGateResult<T> = Result<T, DevserverGateError>;

/// Mint an entry token (30s exp).
pub fn encode_entry(
    secret: &[u8],
    sub: Uuid,
    role: &str,
    drv: &str,
    aud: &str,
    identity: CallerIdentity,
) -> DevserverGateResult<String> {
    encode(secret, sub, role, drv, aud, identity, TokenType::Entry)
}

/// Mint a session token (24h exp).
pub fn encode_session(
    secret: &[u8],
    sub: Uuid,
    role: &str,
    drv: &str,
    aud: &str,
    identity: CallerIdentity,
) -> DevserverGateResult<String> {
    encode(secret, sub, role, drv, aud, identity, TokenType::Session)
}

fn encode(
    secret: &[u8],
    sub: Uuid,
    role: &str,
    drv: &str,
    aud: &str,
    identity: CallerIdentity,
    typ: TokenType,
) -> DevserverGateResult<String> {
    let now = Utc::now();
    let (iss, lifetime) = match typ {
        TokenType::Entry => ("chan-gateway-identity", Duration::seconds(30)),
        TokenType::Session => ("chan-gateway-devserver-proxy", Duration::hours(24)),
    };
    let claims = Claims {
        iss: iss.to_string(),
        sub,
        role: role.to_string(),
        drv: drv.to_string(),
        aud: aud.to_string(),
        typ: typ.as_str().to_string(),
        name: identity.name,
        email: identity.email,
        iat: now.timestamp(),
        exp: (now + lifetime).timestamp(),
    };
    jwt_encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .map_err(|e| DevserverGateError::Decode(format!("encode: {e}")))
}

/// Verify a token and return the claims. `expected_typ` hard-fails if
/// the token's `typ` does not match (an entry token cannot ride in
/// the cookie slot and vice versa). `expected_aud` and `expected_drv`
/// bind the verification to the host and workspace the request actually
/// hit; passing different values is a logic error in the caller.
///
/// The validation config:
/// * `Algorithm::HS256` only (no `alg: none`, no asymmetric algs);
/// * `validate_exp = true` (library-level expiry check);
/// * `aud` is matched in-band here against `expected_aud`.
///
/// `Validation::new(HS256)` already enforces alg + exp; we keep the
/// explicit `validate_exp(true)` for documentation.
pub fn decode(
    secret: &[u8],
    token: &str,
    expected_typ: TokenType,
    expected_aud: &str,
    expected_drv: &str,
) -> DevserverGateResult<Claims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    // We match `aud` ourselves below so the error mapping is clean.
    // jsonwebtoken's aud check returns `InvalidAudience`, which we'd
    // collapse into the same WrongAudience anyway.
    validation.validate_aud = false;

    let data = jwt_decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => DevserverGateError::Expired,
            _ => DevserverGateError::Decode(format!("{e}")),
        })?;
    let claims = data.claims;
    if claims.aud != expected_aud {
        return Err(DevserverGateError::WrongAudience);
    }
    if claims.drv != expected_drv {
        return Err(DevserverGateError::WrongWorkspace);
    }
    if claims.typ != expected_typ.as_str() {
        return Err(DevserverGateError::WrongType {
            got: claims.typ.clone(),
            want: expected_typ.as_str(),
        });
    }
    Ok(claims)
}

/// Convenience: when did this token issue?
pub fn issued_at(claims: &Claims) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(claims.iat, 0).unwrap_or_else(Utc::now)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"test-secret-must-be-long-enough-32";

    fn sample_uuid() -> Uuid {
        Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap()
    }

    #[test]
    fn entry_roundtrip_ok() {
        let t = encode_entry(
            SECRET,
            sample_uuid(),
            "owner",
            "blog",
            "alice.devserver.chan.app",
            CallerIdentity::default(),
        )
        .unwrap();
        let c = decode(
            SECRET,
            &t,
            TokenType::Entry,
            "alice.devserver.chan.app",
            "blog",
        )
        .unwrap();
        assert_eq!(c.sub, sample_uuid());
        assert_eq!(c.drv, "blog");
        assert_eq!(c.aud, "alice.devserver.chan.app");
        assert_eq!(c.typ, "entry");
        assert_eq!(c.iss, "chan-gateway-identity");
        assert_eq!(c.role, "owner");
    }

    #[test]
    fn session_roundtrip_ok() {
        let t = encode_session(
            SECRET,
            sample_uuid(),
            "editor",
            "blog",
            "alice.devserver.chan.app",
            CallerIdentity::default(),
        )
        .unwrap();
        let c = decode(
            SECRET,
            &t,
            TokenType::Session,
            "alice.devserver.chan.app",
            "blog",
        )
        .unwrap();
        assert_eq!(c.iss, "chan-gateway-devserver-proxy");
        assert_eq!(c.typ, "session");
        assert_eq!(c.role, "editor");
    }

    #[test]
    fn cross_type_replay_rejected() {
        // An entry token must not be accepted in the session slot.
        // Defensive: even if someone exfiltrates an entry token, it
        // can only ride the URL leg, not the cookie leg.
        let entry = encode_entry(
            SECRET,
            sample_uuid(),
            "owner",
            "blog",
            "alice.devserver.chan.app",
            CallerIdentity::default(),
        )
        .unwrap();
        let err = decode(
            SECRET,
            &entry,
            TokenType::Session,
            "alice.devserver.chan.app",
            "blog",
        )
        .unwrap_err();
        assert!(matches!(err, DevserverGateError::WrongType { .. }));
    }

    #[test]
    fn aud_mismatch_rejected() {
        let t = encode_entry(
            SECRET,
            sample_uuid(),
            "owner",
            "blog",
            "alice.devserver.chan.app",
            CallerIdentity::default(),
        )
        .unwrap();
        let err = decode(
            SECRET,
            &t,
            TokenType::Entry,
            "bob.devserver.chan.app",
            "blog",
        )
        .unwrap_err();
        assert!(matches!(err, DevserverGateError::WrongAudience));
    }

    #[test]
    fn drv_mismatch_rejected() {
        // Critical isolation property: a token minted for alice/blog
        // must not be accepted on alice/journal even on the same
        // subdomain.
        let t = encode_session(
            SECRET,
            sample_uuid(),
            "owner",
            "blog",
            "alice.devserver.chan.app",
            CallerIdentity::default(),
        )
        .unwrap();
        let err = decode(
            SECRET,
            &t,
            TokenType::Session,
            "alice.devserver.chan.app",
            "journal",
        )
        .unwrap_err();
        assert!(matches!(err, DevserverGateError::WrongWorkspace));
    }

    #[test]
    fn wrong_secret_rejected() {
        let t = encode_entry(
            SECRET,
            sample_uuid(),
            "owner",
            "blog",
            "alice.devserver.chan.app",
            CallerIdentity::default(),
        )
        .unwrap();
        let err = decode(
            b"different-secret-32-bytes-long-ab",
            &t,
            TokenType::Entry,
            "alice.devserver.chan.app",
            "blog",
        )
        .unwrap_err();
        assert!(matches!(err, DevserverGateError::Decode(_)));
    }

    #[test]
    fn alg_none_rejected() {
        // Defense-in-depth: even if someone hand-crafts an `alg: none`
        // header pointing at our `aud`/`drv`, the decoder must refuse
        // it because Validation::new(HS256) hard-requires HS256.
        //
        // Construction: header `{"alg":"none","typ":"JWT"}` + claims
        // with no signature trailing.
        let header = base64_url(r#"{"alg":"none","typ":"JWT"}"#);
        let payload = base64_url(
            r#"{"iss":"id.chan.app","sub":"11111111-1111-4111-8111-111111111111",
                "drv":"blog","aud":"alice.devserver.chan.app","typ":"entry",
                "iat":0,"exp":9999999999}"#,
        );
        let token = format!("{header}.{payload}.");
        let err = decode(
            SECRET,
            &token,
            TokenType::Entry,
            "alice.devserver.chan.app",
            "blog",
        )
        .unwrap_err();
        assert!(matches!(err, DevserverGateError::Decode(_)));
    }

    fn base64_url(s: &str) -> String {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;
        URL_SAFE_NO_PAD.encode(s.as_bytes())
    }

    #[test]
    fn identity_roundtrips_entry_to_session() {
        let identity = CallerIdentity {
            name: Some("Alice Doe".to_string()),
            email: Some("alice@example.com".to_string()),
        };
        let entry = encode_entry(
            SECRET,
            sample_uuid(),
            "editor",
            "blog",
            "alice.devserver.chan.app",
            identity.clone(),
        )
        .unwrap();
        let c = decode(
            SECRET,
            &entry,
            TokenType::Entry,
            "alice.devserver.chan.app",
            "blog",
        )
        .unwrap();
        assert_eq!(c.identity(), identity);

        // The proxy propagates the verified entry identity into the
        // session mint; the session token must carry it unchanged.
        let session = encode_session(SECRET, c.sub, &c.role, &c.drv, &c.aud, c.identity()).unwrap();
        let s = decode(
            SECRET,
            &session,
            TokenType::Session,
            "alice.devserver.chan.app",
            "blog",
        )
        .unwrap();
        assert_eq!(s.identity(), identity);
    }

    /// A token minted by a service that predates the identity claims
    /// has no name/email members at all; it must decode with both None.
    #[test]
    fn legacy_token_without_identity_decodes_to_none() {
        let now = Utc::now().timestamp();
        let legacy = serde_json::json!({
            "iss": "id.chan.app",
            "sub": sample_uuid(),
            "role": "owner",
            "drv": "blog",
            "aud": "alice.devserver.chan.app",
            "typ": "entry",
            "iat": now,
            "exp": now + 30,
        });
        let token = jwt_encode(
            &Header::new(Algorithm::HS256),
            &legacy,
            &EncodingKey::from_secret(SECRET),
        )
        .unwrap();
        let c = decode(
            SECRET,
            &token,
            TokenType::Entry,
            "alice.devserver.chan.app",
            "blog",
        )
        .unwrap();
        assert_eq!(c.name, None);
        assert_eq!(c.email, None);
    }

    /// A token minted by a future service with claims this build does
    /// not know must still verify; unknown members are ignored.
    #[test]
    fn unknown_claims_ignored() {
        let now = Utc::now().timestamp();
        let future = serde_json::json!({
            "iss": "id.chan.app",
            "sub": sample_uuid(),
            "role": "owner",
            "drv": "blog",
            "aud": "alice.devserver.chan.app",
            "typ": "entry",
            "name": "Alice Doe",
            "email": "alice@example.com",
            "avatar_url": "https://example.com/a.png",
            "iat": now,
            "exp": now + 30,
        });
        let token = jwt_encode(
            &Header::new(Algorithm::HS256),
            &future,
            &EncodingKey::from_secret(SECRET),
        )
        .unwrap();
        let c = decode(
            SECRET,
            &token,
            TokenType::Entry,
            "alice.devserver.chan.app",
            "blog",
        )
        .unwrap();
        assert_eq!(c.name.as_deref(), Some("Alice Doe"));
        assert_eq!(c.email.as_deref(), Some("alice@example.com"));
    }
}
