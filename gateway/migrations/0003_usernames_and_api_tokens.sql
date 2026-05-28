-- Usernames + personal access tokens.
--
-- Each user gets a public-facing handle, surfaced under
-- chan.app/{username}. Renames are gated by a per-user counter
-- (cap enforced in app code) so handles don't churn.
--
-- Existing rows are backfilled with a deterministic, ugly-but-unique
-- handle derived from the user's uuid. Users will rename to whatever
-- they want on their first sign-in.
--
-- api_tokens are personal access tokens issued from the identity
-- service for the chan CLI / chan-tunnel. Stored as SHA-256 hash
-- so a DB leak doesn't hand out live secrets. Token-level audit
-- lives in api_token_audit; one row per create / use / revoke.

ALTER TABLE users ADD COLUMN username      text;
ALTER TABLE users ADD COLUMN username_edits integer NOT NULL DEFAULT 0;

-- 'u' + first 12 hex chars of the uuid: 13-char handle, fits the
-- ^[a-z0-9][a-z0-9-]{1,30}[a-z0-9]$ rule, collision-free in
-- practice. Users rename later; the counter starts at 0 so they
-- get the full edit budget.
UPDATE users
SET username = 'u' || substr(replace(id::text, '-', ''), 1, 12)
WHERE username IS NULL;

ALTER TABLE users ALTER COLUMN username SET NOT NULL;

CREATE UNIQUE INDEX users_username_lower_idx ON users (lower(username));

CREATE TABLE api_tokens (
    id           uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      uuid        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    label        text        NOT NULL,
    token_hash   text        NOT NULL UNIQUE,
    expires_at   timestamptz,
    created_at   timestamptz NOT NULL DEFAULT now(),
    revoked_at   timestamptz,
    last_used_at timestamptz
);

CREATE INDEX api_tokens_user_id_idx ON api_tokens (user_id);

-- Per-token audit log. Three actions in v0: 'created', 'used',
-- 'revoked'. Keep the schema minimal; surface the latest entries
-- on the token row in the UI.
CREATE TABLE api_token_audit (
    id         bigserial   PRIMARY KEY,
    token_id   uuid        NOT NULL REFERENCES api_tokens(id) ON DELETE CASCADE,
    ts         timestamptz NOT NULL DEFAULT now(),
    action     text        NOT NULL,
    -- ip stored as text rather than inet so sqlx doesn't need the
    -- ipnetwork feature flag. We never query by ip, only display it
    -- in the audit log.
    ip         text,
    user_agent text
);

CREATE INDEX api_token_audit_token_idx ON api_token_audit (token_id, ts DESC);
