-- Refresh tokens for non-web clients (CLI, desktop, mobile).
--
-- Rows are written by identity-service's `mint()` and updated by
-- /v1/token/refresh; revoked by /v1/token/revoke (and by cascade
-- when a user is deleted).
--
-- We store SHA-256(token) so a database leak doesn't hand out
-- live refresh tokens. The token itself only ever lives in the
-- response body and on the client (OS keychain).
--
-- One row corresponds to one device. The `label` and `platform`
-- columns surface in /v1/devices so a user can tell which sign-in
-- belongs to which laptop / phone.

CREATE TABLE refresh_tokens (
    id           uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      uuid        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash   text        NOT NULL UNIQUE,
    label        text,
    platform     text,
    created_at   timestamptz NOT NULL DEFAULT now(),
    last_used_at timestamptz NOT NULL DEFAULT now(),
    expires_at   timestamptz NOT NULL
);

CREATE INDEX refresh_tokens_user_id_idx ON refresh_tokens (user_id);
CREATE INDEX refresh_tokens_expires_at_idx ON refresh_tokens (expires_at);
