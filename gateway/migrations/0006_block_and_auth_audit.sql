-- Account blocking + auth audit log.
--
-- A blocked user cannot complete OAuth sign-in (identity refuses the
-- callback) and any live PAT stops validating (validate query joins
-- users and filters on blocked_at IS NULL). Blocking also revokes
-- every active api_token in the same statement so a re-validate
-- after unblock requires a freshly issued token.
--
-- auth_audit captures user-level events that don't fit the per-token
-- audit (login, logout, blocked, unblocked, login_denied). Source IP
-- and user agent are stored as text for the same reason as
-- api_token_audit: avoid pulling in the sqlx ipnetwork feature for
-- a column we never query by.

ALTER TABLE users ADD COLUMN blocked_at   timestamptz;
ALTER TABLE users ADD COLUMN block_reason text;

CREATE INDEX users_blocked_at_idx ON users (blocked_at)
    WHERE blocked_at IS NOT NULL;

CREATE TABLE auth_audit (
    id         bigserial   PRIMARY KEY,
    user_id    uuid        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ts         timestamptz NOT NULL DEFAULT now(),
    action     text        NOT NULL,
    ip         text,
    user_agent text,
    note       text
);

CREATE INDEX auth_audit_user_idx ON auth_audit (user_id, ts DESC);
