-- Liveness stamp for the devserver registry sweeper.
--
-- profile-service marks rows from devserver-proxy's live-tunnel
-- snapshot each sweep tick and deletes rows offline longer than the
-- configured retention. Nullable, no backfill: a row that predates
-- this column has never been marked, so its offline age falls back to
-- registration time -- age = now() - COALESCE(last_seen_at,
-- created_at). Rows swept while merely powered off are recreated on
-- the next dial (identity mints rows per PAT on sight); their grants
-- and label do not survive the sweep.

ALTER TABLE devservers ADD COLUMN last_seen_at timestamptz;
