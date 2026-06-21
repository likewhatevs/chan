-- Per-devserver sharing model.
--
-- ADR-0001 makes the DEVSERVER the unit of registration, gate, and
-- sharing: one gateway-exposed `chan devserver` per user carries the
-- whole library behind one tunnel registration. A grant gives a
-- collaborator the WHOLE devserver; the path `{workspace}` segment is
-- tenant routing only and never gates. This replaces the per-workspace
-- `workspaces` / `workspace_grants` tables wholesale.
--
-- Pre-release, fresh-state cutover (no rows to migrate): the old tables
-- are dropped, not transformed. workspace_grants FKs onto workspaces,
-- so drop the child first.

DROP TABLE IF EXISTS workspace_grants;
DROP TABLE IF EXISTS workspaces;

-- A devserver row is the owner-side declaration of a shareable
-- devserver. `devserver_id` is the lowercase hex SHA-256 of the owner's
-- PAT, produced by identity-service (the only holder of the raw token);
-- 1 token : 1 devserver, so rotating the PAT yields a new id and grants
-- do not survive rotation. The id is a public handle (a hash, not the
-- secret); the SPA passes it around to grant on a devserver.
--
-- `label` mirrors the PAT label so the owner's devserver list renders
-- without a second hop to api_tokens; identity writes it at row-create
-- time. PAT labels are not renamed today, so drift against
-- api_tokens.label is not a live concern. The surrogate uuid `id` is for
-- FK joins only; `(owner_user_id, devserver_id)` is the canonical key.
CREATE TABLE devservers (
    id            uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_user_id uuid        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    devserver_id  text        NOT NULL,
    label         text        NOT NULL DEFAULT '',
    created_at    timestamptz NOT NULL DEFAULT now(),
    UNIQUE (owner_user_id, devserver_id)
);

CREATE INDEX devservers_owner_idx ON devservers (owner_user_id);

-- A grant row reads "owner_user_id shares devserver_id (the whole
-- library) with whoever signs in on a verified OAuth email matching
-- grantee_email, with the given role". Keyed on
-- (owner, devserver_id, email) so an owner can pre-seed access before
-- the devserver ever registers; the devserver-proxy registry decides
-- whether the devserver is currently live, this table decides who is
-- allowed in once it is.
--
-- grantee_user_id resolves lazily: NULL while the invite is pending, set
-- to users.id once a sign-in with a verified email matching
-- grantee_email is observed (claim sweep at OAuth callback, plus
-- best-effort at grant create). accepted_at tracks the moment of
-- resolution; the dashboard lists "Shared with me" for rows where
-- grantee_user_id = me.
--
-- Normalization: grantee_email is stored as the handler wrote it;
-- uniqueness is case-insensitive via a lower() functional index.
-- devserver_id is the canonical hex hash the handler validated.
CREATE TABLE devserver_grants (
    id               uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_user_id    uuid        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    devserver_id     text        NOT NULL,
    grantee_email    text        NOT NULL,
    grantee_user_id  uuid        REFERENCES users(id) ON DELETE CASCADE,
    role             text        NOT NULL CHECK (role IN ('viewer', 'editor')),
    created_at       timestamptz NOT NULL DEFAULT now(),
    accepted_at      timestamptz,
    CONSTRAINT devserver_grants_devserver_fk
        FOREIGN KEY (owner_user_id, devserver_id)
        REFERENCES devservers (owner_user_id, devserver_id)
        ON DELETE CASCADE
);

CREATE UNIQUE INDEX devserver_grants_owner_devserver_email_idx
    ON devserver_grants (owner_user_id, devserver_id, lower(grantee_email));

-- Hot path: "what is shared with me right now". Partial because the
-- column is NULL until the grant is claimed; the unclaimed half is only
-- swept by email and never by user.
CREATE INDEX devserver_grants_grantee_user_idx
    ON devserver_grants (grantee_user_id)
    WHERE grantee_user_id IS NOT NULL;

-- Claim sweep: at OAuth callback time, identity hands profile a list of
-- the user's verified emails; profile updates every matching row.
-- Partial so the index stays small after most grants are claimed.
CREATE INDEX devserver_grants_pending_email_idx
    ON devserver_grants (lower(grantee_email))
    WHERE grantee_user_id IS NULL;
