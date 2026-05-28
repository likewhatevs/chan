-- Drives as first-class entities.
--
-- Before this migration, a "drive" only existed implicitly in two
-- places: as a live registration on drive-proxy's in-memory Registry
-- (set up by `chan serve --tunnel-drive=<name>`), and as the
-- `drive_name` column on `drive_grants`. The dashboard derived "my
-- drives" as the union of live tunnels plus distinct grant drives.
-- That left no way for the owner to materialise a drive *before*
-- either running `chan serve` or adding a grant, which made the
-- UX surface for "create a drive, then share it" awkward.
--
-- `drives` is the owner-side declaration of intent. Live tunnels and
-- grants both refer back to (owner_user_id, drive_name); the table
-- is the authoritative list of drive names the user has reserved.
-- Names live in this owner's namespace only — two users can each
-- have a `photos` drive without conflict.

CREATE TABLE drives (
    id            uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_user_id uuid        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    drive_name    text        NOT NULL,
    created_at    timestamptz NOT NULL DEFAULT now(),
    UNIQUE (owner_user_id, drive_name)
);

CREATE INDEX drives_owner_idx ON drives (owner_user_id);

-- Backfill from any existing drive_grants rows so the FK we add
-- below does not reject them. distinct() collapses the (owner, name)
-- pairs that already have grants into one drives row each. On a
-- clean install (no grants yet) this is a no-op.
INSERT INTO drives (owner_user_id, drive_name)
SELECT DISTINCT owner_user_id, drive_name FROM drive_grants
ON CONFLICT DO NOTHING;

-- Pin grants to their parent drive. Cascading delete keeps a drive
-- removal atomic: dropping a `drives` row also drops every grant
-- the user had attached to it. The FK target is the UNIQUE pair, so
-- Postgres accepts the composite reference.
ALTER TABLE drive_grants
    ADD CONSTRAINT drive_grants_drive_fk
    FOREIGN KEY (owner_user_id, drive_name)
    REFERENCES drives (owner_user_id, drive_name)
    ON DELETE CASCADE;
