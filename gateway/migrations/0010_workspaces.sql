-- Workspaces as first-class entities.
--
-- Before this migration, a "workspace" only existed implicitly in two
-- places: as a live registration on workspace-proxy's in-memory Registry
-- (set up by `chan serve --tunnel-workspace-name=<name>`), and as the
-- `workspace_name` column on `workspace_grants`. The dashboard derived "my
-- workspaces" as the union of live tunnels plus distinct grant workspaces.
-- That left no way for the owner to materialise a workspace *before*
-- either running `chan serve` or adding a grant, which made the
-- UX surface for "create a workspace, then share it" awkward.
--
-- `workspaces` is the owner-side declaration of intent. Live tunnels and
-- grants both refer back to (owner_user_id, workspace_name); the table
-- is the authoritative list of workspace names the user has reserved.
-- Names live in this owner's namespace only — two users can each
-- have a `photos` workspace without conflict.

CREATE TABLE workspaces (
    id            uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_user_id uuid        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    workspace_name    text        NOT NULL,
    created_at    timestamptz NOT NULL DEFAULT now(),
    UNIQUE (owner_user_id, workspace_name)
);

CREATE INDEX workspaces_owner_idx ON workspaces (owner_user_id);

-- Backfill from any existing workspace_grants rows so the FK we add
-- below does not reject them. distinct() collapses the (owner, name)
-- pairs that already have grants into one workspaces row each. On a
-- clean install (no grants yet) this is a no-op.
INSERT INTO workspaces (owner_user_id, workspace_name)
SELECT DISTINCT owner_user_id, workspace_name FROM workspace_grants
ON CONFLICT DO NOTHING;

-- Pin grants to their parent workspace. Cascading delete keeps a workspace
-- removal atomic: dropping a `workspaces` row also drops every grant
-- the user had attached to it. The FK target is the UNIQUE pair, so
-- Postgres accepts the composite reference.
ALTER TABLE workspace_grants
    ADD CONSTRAINT workspace_grants_workspace_fk
    FOREIGN KEY (owner_user_id, workspace_name)
    REFERENCES workspaces (owner_user_id, workspace_name)
    ON DELETE CASCADE;
