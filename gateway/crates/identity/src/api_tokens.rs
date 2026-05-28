//! Personal access tokens.
//!
//! Issued from the identity service for the chan CLI / chan-tunnel.
//! Token shape: `chan_pat_<32 random b64url bytes>`. Database stores
//! only `SHA-256(token)`, so a leak of the table doesn't hand out
//! live secrets; the plaintext leaves on the create response and is
//! never persisted.
//!
//! Scope is intentionally flat: a token authenticates a user. Drive
//! ownership is enforced at the URL layer (`chan.app/{username}/...`)
//! by chan-tunnel, not via per-token bindings -- mirrors the new
//! GitHub fine-grained model.
//!
//! Every state change writes one row to `api_token_audit`. Three
//! actions in v0: `created`, `used` (validate succeeded), `revoked`.

use base64::Engine;
use chrono::{DateTime, Utc};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Error, Result};

const TOKEN_PREFIX: &str = "chan_pat_";

/// Audit-log actions. Stored as text to keep migrations simple; if
/// the set ever grows we can add a CHECK constraint.
pub const ACTION_CREATED: &str = "created";
pub const ACTION_CREATED_DESKTOP: &str = "created_via_desktop";
pub const ACTION_USED: &str = "used";
pub const ACTION_REVOKED: &str = "revoked";

/// Where a `create()` call came from. The desktop-authorize flow
/// records a distinct audit action so operators (and the user
/// themselves) can tell apart tokens minted by the SPA's "create
/// token" button from tokens minted by chan-desktop bouncing through
/// `/desktop/authorize`. Existing `created` rows continue to mean
/// SPA mint; the new `created_via_desktop` only appears for the
/// desktop flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenOrigin {
    Spa,
    Desktop,
}

impl TokenOrigin {
    pub fn audit_action(self) -> &'static str {
        match self {
            Self::Spa => ACTION_CREATED,
            Self::Desktop => ACTION_CREATED_DESKTOP,
        }
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ApiToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub label: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    /// Capabilities the token carries. Returned to drive-proxy at
    /// validate time and gates chan-tunnel-server's scope checks
    /// (`tunnel` for any dial, `tunnel.public` for anonymous-readable
    /// drives). Newly-issued tokens default to `["tunnel"]`; granting
    /// extra scopes is a deliberate act at create time.
    pub scopes: Vec<String>,
}

/// One-shot response: the only time the plaintext token is exposed.
#[derive(Debug, Serialize)]
pub struct CreatedToken {
    #[serde(flatten)]
    pub token: ApiToken,
    /// `chan_pat_...` -- shown to the user once, never persisted.
    pub secret: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AuditEntry {
    pub id: i64,
    pub ts: DateTime<Utc>,
    pub action: String,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

/// Successful validate result handed to chan-tunnel. `username` is
/// what chan.app/{username} resolves to; tunneld uses it to build
/// the public URL.
#[derive(Debug, Clone, Serialize)]
pub struct ValidatedToken {
    pub user_id: Uuid,
    pub username: String,
    pub token_id: Uuid,
    pub expires_at: Option<DateTime<Utc>>,
    /// Per-token capabilities. drive-proxy forwards these into
    /// `chan_tunnel_server::Validated::scopes`, which the listener
    /// uses to gate `tunnel` (any dial) and `tunnel.public`
    /// (anonymous-readable drive).
    pub scopes: Vec<String>,
}

/// Default scope set for a freshly-issued token. Private-only:
/// the holder can dial chan-tunnel but cannot expose a drive
/// anonymously. Granting `tunnel.public` (or any future scope) is
/// an explicit step in the token-create call.
pub const DEFAULT_TOKEN_SCOPES: &[&str] = &["tunnel"];

#[derive(Clone)]
pub struct ApiTokenService {
    pool: PgPool,
}

impl ApiTokenService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // create() is the single PAT-mint entry point and grew one more
    // parameter (origin) to differentiate SPA / desktop audit rows.
    // Bundling these into a builder/struct adds an indirection for
    // every caller without buying clarity.
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        user_id: Uuid,
        label: &str,
        expires_at: Option<DateTime<Utc>>,
        scopes: &[String],
        ip: Option<&str>,
        user_agent: Option<&str>,
        origin: TokenOrigin,
    ) -> Result<CreatedToken> {
        let label = label.trim();
        if label.is_empty() {
            return Err(Error::BadRequest("label required".into()));
        }
        if label.len() > 64 {
            return Err(Error::BadRequest("label too long".into()));
        }
        validate_scopes(scopes)?;

        let (secret, hash) = generate_token();
        let token = sqlx::query_as::<_, ApiToken>(
            "INSERT INTO api_tokens (user_id, label, token_hash, expires_at, scopes) \
             VALUES ($1, $2, $3, $4, $5) \
             RETURNING id, user_id, label, expires_at, created_at, \
                       revoked_at, last_used_at, scopes",
        )
        .bind(user_id)
        .bind(label)
        .bind(&hash)
        .bind(expires_at)
        .bind(scopes)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db)?;

        self.write_audit(token.id, origin.audit_action(), ip, user_agent)
            .await?;

        Ok(CreatedToken { token, secret })
    }

    pub async fn list(&self, user_id: Uuid) -> Result<Vec<ApiToken>> {
        let rows = sqlx::query_as::<_, ApiToken>(
            "SELECT id, user_id, label, expires_at, created_at, \
                    revoked_at, last_used_at, scopes \
             FROM api_tokens \
             WHERE user_id = $1 \
             ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(rows)
    }

    /// Soft-revoke: keeps the row for audit history, but
    /// `validate()` skips revoked tokens.
    pub async fn revoke(
        &self,
        user_id: Uuid,
        token_id: Uuid,
        ip: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE api_tokens SET revoked_at = now() \
             WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL",
        )
        .bind(token_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_db)?;
        if res.rows_affected() == 0 {
            return Ok(false);
        }
        self.write_audit(token_id, ACTION_REVOKED, ip, user_agent)
            .await?;
        Ok(true)
    }

    pub async fn audit(
        &self,
        user_id: Uuid,
        token_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AuditEntry>> {
        // Ownership check first so we don't leak audit rows for
        // someone else's token id.
        let owned = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM api_tokens \
             WHERE id = $1 AND user_id = $2)",
        )
        .bind(token_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db)?;
        if !owned {
            return Err(Error::NotFound);
        }
        let rows = sqlx::query_as::<_, AuditEntry>(
            "SELECT id, ts, action, ip, user_agent \
             FROM api_token_audit \
             WHERE token_id = $1 \
             ORDER BY ts DESC \
             LIMIT $2",
        )
        .bind(token_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(rows)
    }

    /// Look up by token, enforce active + non-expired, bump
    /// `last_used_at` and write an audit row. Single statement so
    /// concurrent validates can't both write conflicting timestamps.
    pub async fn validate(
        &self,
        token: &str,
        ip: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<ValidatedToken> {
        if !token.starts_with(TOKEN_PREFIX) {
            return Err(Error::Unauthorized);
        }
        let hash = hash_token(token);

        // Join to users to get the username on the same round trip.
        // Blocked accounts (`u.blocked_at IS NOT NULL`) are filtered
        // here so a token issued before the block stops working
        // immediately, even if the admin block step somehow missed
        // its auto-revoke pass.
        let row = sqlx::query_as::<_, (Uuid, Uuid, String, Option<DateTime<Utc>>, Vec<String>)>(
            "UPDATE api_tokens t \
             SET last_used_at = now() \
             FROM users u \
             WHERE t.token_hash = $1 \
               AND t.user_id = u.id \
               AND t.revoked_at IS NULL \
               AND (t.expires_at IS NULL OR t.expires_at > now()) \
               AND u.blocked_at IS NULL \
             RETURNING t.id, u.id, u.username, t.expires_at, t.scopes",
        )
        .bind(&hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db)?
        .ok_or(Error::Unauthorized)?;

        self.write_audit(row.0, ACTION_USED, ip, user_agent).await?;

        Ok(ValidatedToken {
            token_id: row.0,
            user_id: row.1,
            username: row.2,
            expires_at: row.3,
            scopes: row.4,
        })
    }

    async fn write_audit(
        &self,
        token_id: Uuid,
        action: &str,
        ip: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO api_token_audit (token_id, action, ip, user_agent) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(token_id)
        .bind(action)
        .bind(ip)
        .bind(user_agent)
        .execute(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(())
    }
}

/// Hard cap on the per-token scope list to bound row width and
/// keep validate-time copies tiny. The current set is two values
/// (`tunnel`, `tunnel.public`); the cap leaves headroom for future
/// scopes without admitting unbounded lists.
const MAX_SCOPES_PER_TOKEN: usize = 16;
/// Hard cap on the length of any single scope. Scope names are
/// short identifiers (`tunnel`, `tunnel.public`); the cap guards
/// against pathological inputs in the create body.
const MAX_SCOPE_LEN: usize = 64;

/// Reject scope lists that are empty, oversized, contain
/// blank entries, or carry duplicate names. The validator at the
/// other end (`chan_tunnel_server::Validated`) does a linear search
/// for the scopes it cares about, so duplicates would waste space
/// without changing behaviour; rejecting them keeps the row clean.
fn validate_scopes(scopes: &[String]) -> Result<()> {
    if scopes.is_empty() {
        return Err(Error::BadRequest("scopes must be non-empty".into()));
    }
    if scopes.len() > MAX_SCOPES_PER_TOKEN {
        return Err(Error::BadRequest(format!(
            "scopes list exceeds the {MAX_SCOPES_PER_TOKEN}-entry cap"
        )));
    }
    let mut seen: Vec<&str> = Vec::with_capacity(scopes.len());
    for s in scopes {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(Error::BadRequest("scope must not be blank".into()));
        }
        if trimmed.len() > MAX_SCOPE_LEN {
            return Err(Error::BadRequest(format!(
                "scope {trimmed:?} exceeds the {MAX_SCOPE_LEN}-char cap"
            )));
        }
        if trimmed != s {
            return Err(Error::BadRequest(format!(
                "scope {s:?} must not contain leading/trailing whitespace"
            )));
        }
        if seen.contains(&trimmed) {
            return Err(Error::BadRequest(format!("duplicate scope {trimmed:?}")));
        }
        seen.push(trimmed);
    }
    Ok(())
}

fn generate_token() -> (String, String) {
    // OsRng directly: never falls back to a userspace PRNG even on
    // exotic platforms where ThreadRng's reseed window matters.
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let secret = format!(
        "{TOKEN_PREFIX}{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    );
    let hash = hash_token(&secret);
    (secret, hash)
}

fn hash_token(token: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(token.as_bytes()))
}

fn map_db(e: sqlx::Error) -> Error {
    tracing::error!(error = ?e, "api_tokens db error");
    Error::Anyhow(anyhow::anyhow!(e))
}
