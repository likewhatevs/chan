-- Per-workspace sharing grants.
--
-- A row reads "owner_user_id shares workspace_name with whoever signs in
-- on a verified OAuth email matching grantee_email, with the given
-- role". The grant is keyed on (owner, workspace, email) so an owner can
-- pre-seed access before `chan serve --tunnel-workspace=<name>` is ever
-- run; workspace-proxy's in-memory registry decides whether the workspace is
-- currently live, this table decides who is allowed in once it is.
--
-- grantee_user_id is resolved lazily: NULL while the invite is
-- pending, set to the matching users.id once a sign-in with a
-- verified email matching grantee_email is observed (claim sweep at
-- OAuth callback time, plus best-effort resolution on grant create).
-- accepted_at tracks the moment of resolution; the dashboard lists
-- "Shared with me" for rows where grantee_user_id = me.
--
-- Normalization: workspace_name and grantee_email are stored as the
-- handler wrote them; uniqueness is enforced case-insensitively on
-- both via a lower() functional index. workspace_name is also rejected
-- at the handler if it contains anything other than lowercase ascii
-- alnum / [._-] / 1..=64 chars, so the stored form is always the
-- canonical path segment workspace-proxy serves.

CREATE TABLE workspace_grants (
    id               uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_user_id    uuid        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    workspace_name       text        NOT NULL,
    grantee_email    text        NOT NULL,
    grantee_user_id  uuid        REFERENCES users(id) ON DELETE CASCADE,
    role             text        NOT NULL CHECK (role IN ('viewer', 'editor')),
    created_at       timestamptz NOT NULL DEFAULT now(),
    accepted_at      timestamptz
);

CREATE UNIQUE INDEX workspace_grants_owner_workspace_email_idx
    ON workspace_grants (owner_user_id, workspace_name, lower(grantee_email));

-- Hot path: "what is shared with me right now". Partial because the
-- column is NULL until the grant is claimed; the unclaimed half is
-- only swept by email and never by user.
CREATE INDEX workspace_grants_grantee_user_idx
    ON workspace_grants (grantee_user_id)
    WHERE grantee_user_id IS NOT NULL;

-- Claim sweep: at OAuth callback time, identity hands profile a list
-- of the user's verified emails; profile updates every matching row.
-- Partial so the index stays small after most grants are claimed.
CREATE INDEX workspace_grants_pending_email_idx
    ON workspace_grants (lower(grantee_email))
    WHERE grantee_user_id IS NULL;
