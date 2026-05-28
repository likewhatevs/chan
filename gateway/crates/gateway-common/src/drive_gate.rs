//! Drive-gate JWT envelope shared by identity-service (mints entry
//! tokens) and drive-proxy (verifies entry tokens, mints + verifies
//! session cookies). HS256 only; `alg: none` is hard-rejected by the
//! validation config.
//!
//! Token shapes:
//!
//! * `typ: "entry"` (issued by identity, lives in `?t=` URL param,
//!   30s exp). After successful verification drive-proxy mints a
//!   session token of the same envelope but with `typ: "session"`,
//!   sets it as a `Path=/<drive>/` host-only cookie, and 303s to the
//!   clean URL.
//! * `typ: "session"` (issued and verified by drive-proxy, lives in
//!   the `drive_gate` cookie, 24h exp).
//!
//! Both shapes carry the same envelope so the verify path can decode
//! once and dispatch on `typ`. `aud` binds the token to the wildcard
//! host (`alice.drive.chan.app`) so a token minted for one user is
//! not accepted on another user's subdomain. `drv` binds it to one
//! drive slug for the same reason.
//!
//! Why HS256 (symmetric): both services run in the same trust zone
//! and share the same secret already (`DRIVE_GATE_SECRET`). HS256
//! gives the smallest token footprint and the simplest key rotation
//! (one secret rotation invalidates every live token). Asymmetric
//! buys nothing here.

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
    /// Token issuer. Always `id.chan.app` for entry tokens; always
    /// `drive.chan.app` for session tokens. Verified opportunistically
    /// (we trust the signature; `iss` is a debug aid in logs).
    pub iss: String,
    /// `users.id` of the drive owner.
    pub sub: Uuid,
    /// Drive slug.
    pub drv: String,
    /// Wildcard host the token is bound to (e.g. `alice.drive.chan.app`).
    pub aud: String,
    /// `"entry"` or `"session"`. See module doc.
    pub typ: String,
    pub iat: i64,
    pub exp: i64,
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
pub enum DriveGateError {
    /// Signature failed HMAC verify, or the wire shape is bad.
    /// Library-level decode errors collapse here; the only thing we
    /// surface upstream is "token bad."
    #[error("invalid drive-gate token: {0}")]
    Decode(String),

    /// `exp` is in the past. Common case for an expired session
    /// cookie; the caller should treat this the same way as "no
    /// cookie at all" (404 on the proxy path) so existence does not
    /// leak.
    #[error("drive-gate token expired")]
    Expired,

    /// `aud` claim does not match the expected host.
    #[error("drive-gate token audience mismatch")]
    WrongAudience,

    /// `drv` claim does not match the requested drive slug.
    #[error("drive-gate token drive mismatch")]
    WrongDrive,

    /// `typ` claim does not match the verify-call's expectation.
    /// Defensive: prevents an "entry" token being replayed as a
    /// session cookie or vice versa. `got` is attacker-controlled
    /// (any string the caller put in the JWT), so the Display form
    /// only surfaces `want` to avoid echoing arbitrary content into
    /// any future log site that formats the error. Operators who
    /// need the observed value can read it directly off the variant.
    #[error("drive-gate token type mismatch (want {want:?})")]
    WrongType { got: String, want: &'static str },
}

pub type DriveGateResult<T> = Result<T, DriveGateError>;

/// Mint an entry token (30s exp).
pub fn encode_entry(secret: &[u8], sub: Uuid, drv: &str, aud: &str) -> DriveGateResult<String> {
    encode(
        secret,
        sub,
        drv,
        aud,
        TokenType::Entry,
        Duration::seconds(30),
    )
}

/// Mint a session token (24h exp).
pub fn encode_session(secret: &[u8], sub: Uuid, drv: &str, aud: &str) -> DriveGateResult<String> {
    encode(
        secret,
        sub,
        drv,
        aud,
        TokenType::Session,
        Duration::hours(24),
    )
}

fn encode(
    secret: &[u8],
    sub: Uuid,
    drv: &str,
    aud: &str,
    typ: TokenType,
    lifetime: Duration,
) -> DriveGateResult<String> {
    let now = Utc::now();
    let claims = Claims {
        iss: match typ {
            TokenType::Entry => "id.chan.app".to_string(),
            TokenType::Session => "drive.chan.app".to_string(),
        },
        sub,
        drv: drv.to_string(),
        aud: aud.to_string(),
        typ: typ.as_str().to_string(),
        iat: now.timestamp(),
        exp: (now + lifetime).timestamp(),
    };
    jwt_encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .map_err(|e| DriveGateError::Decode(format!("encode: {e}")))
}

/// Verify a token and return the claims. `expected_typ` hard-fails if
/// the token's `typ` does not match (an entry token cannot ride in
/// the cookie slot and vice versa). `expected_aud` and `expected_drv`
/// bind the verification to the host and drive the request actually
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
) -> DriveGateResult<Claims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    // We match `aud` ourselves below so the error mapping is clean.
    // jsonwebtoken's aud check returns `InvalidAudience`, which we'd
    // collapse into the same WrongAudience anyway.
    validation.validate_aud = false;

    let data = jwt_decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => DriveGateError::Expired,
            _ => DriveGateError::Decode(format!("{e}")),
        })?;
    let claims = data.claims;
    if claims.aud != expected_aud {
        return Err(DriveGateError::WrongAudience);
    }
    if claims.drv != expected_drv {
        return Err(DriveGateError::WrongDrive);
    }
    if claims.typ != expected_typ.as_str() {
        return Err(DriveGateError::WrongType {
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
        let t = encode_entry(SECRET, sample_uuid(), "blog", "alice.drive.chan.app").unwrap();
        let c = decode(SECRET, &t, TokenType::Entry, "alice.drive.chan.app", "blog").unwrap();
        assert_eq!(c.sub, sample_uuid());
        assert_eq!(c.drv, "blog");
        assert_eq!(c.aud, "alice.drive.chan.app");
        assert_eq!(c.typ, "entry");
        assert_eq!(c.iss, "id.chan.app");
    }

    #[test]
    fn session_roundtrip_ok() {
        let t = encode_session(SECRET, sample_uuid(), "blog", "alice.drive.chan.app").unwrap();
        let c = decode(
            SECRET,
            &t,
            TokenType::Session,
            "alice.drive.chan.app",
            "blog",
        )
        .unwrap();
        assert_eq!(c.iss, "drive.chan.app");
        assert_eq!(c.typ, "session");
    }

    #[test]
    fn cross_type_replay_rejected() {
        // An entry token must not be accepted in the session slot.
        // Defensive: even if someone exfiltrates an entry token, it
        // can only ride the URL leg, not the cookie leg.
        let entry = encode_entry(SECRET, sample_uuid(), "blog", "alice.drive.chan.app").unwrap();
        let err = decode(
            SECRET,
            &entry,
            TokenType::Session,
            "alice.drive.chan.app",
            "blog",
        )
        .unwrap_err();
        assert!(matches!(err, DriveGateError::WrongType { .. }));
    }

    #[test]
    fn aud_mismatch_rejected() {
        let t = encode_entry(SECRET, sample_uuid(), "blog", "alice.drive.chan.app").unwrap();
        let err = decode(SECRET, &t, TokenType::Entry, "bob.drive.chan.app", "blog").unwrap_err();
        assert!(matches!(err, DriveGateError::WrongAudience));
    }

    #[test]
    fn drv_mismatch_rejected() {
        // Critical isolation property: a token minted for alice/blog
        // must not be accepted on alice/journal even on the same
        // subdomain.
        let t = encode_session(SECRET, sample_uuid(), "blog", "alice.drive.chan.app").unwrap();
        let err = decode(
            SECRET,
            &t,
            TokenType::Session,
            "alice.drive.chan.app",
            "journal",
        )
        .unwrap_err();
        assert!(matches!(err, DriveGateError::WrongDrive));
    }

    #[test]
    fn wrong_secret_rejected() {
        let t = encode_entry(SECRET, sample_uuid(), "blog", "alice.drive.chan.app").unwrap();
        let err = decode(
            b"different-secret-32-bytes-long-ab",
            &t,
            TokenType::Entry,
            "alice.drive.chan.app",
            "blog",
        )
        .unwrap_err();
        assert!(matches!(err, DriveGateError::Decode(_)));
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
                "drv":"blog","aud":"alice.drive.chan.app","typ":"entry",
                "iat":0,"exp":9999999999}"#,
        );
        let token = format!("{header}.{payload}.");
        let err = decode(
            SECRET,
            &token,
            TokenType::Entry,
            "alice.drive.chan.app",
            "blog",
        )
        .unwrap_err();
        assert!(matches!(err, DriveGateError::Decode(_)));
    }

    fn base64_url(s: &str) -> String {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;
        URL_SAFE_NO_PAD.encode(s.as_bytes())
    }
}
