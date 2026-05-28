-- Feature flags.
--
-- Two-table layout: `feature_flags` is the registry (key, description,
-- default), and `feature_flag_overrides` carries per-user explicit
-- enable/disable rows. The effective value for (flag, user) is the
-- override row when present, otherwise the registry default; unknown
-- flag resolves to false.
--
-- The default-off-with-allowlist shape is what we want for the
-- "gradual rollout" use case: ship a flag closed, then grant a
-- short list of internal users while a feature stabilises.
-- default-on-with-denylist also works (override.enabled = false) for
-- when we want to land a feature broadly with a back-out for
-- specific accounts.

CREATE TABLE feature_flags (
    key             text        PRIMARY KEY,
    description     text        NOT NULL DEFAULT '',
    default_enabled boolean     NOT NULL DEFAULT false,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE feature_flag_overrides (
    flag_key text        NOT NULL REFERENCES feature_flags(key) ON DELETE CASCADE,
    user_id  uuid        NOT NULL REFERENCES users(id)          ON DELETE CASCADE,
    enabled  boolean     NOT NULL,
    set_at   timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (flag_key, user_id)
);

CREATE INDEX feature_flag_overrides_user_idx
    ON feature_flag_overrides (user_id);

-- Seed the two flags identity-service already reads. Defaults are
-- closed: a fresh deploy refuses OAuth and hides the sharing UI
-- until an operator grants the relevant users. Idempotent so this
-- migration is safe to re-run on a database that already has the
-- rows (manual `chan-admin flag create` between migrations and
-- this file ever being merged).
INSERT INTO feature_flags (key, description, default_enabled) VALUES
    ('oauth_login',
     'Allow this user to complete OAuth sign-in. Gates access to the dashboard and PAT minting.',
     false),
    ('share_workspaces',
     'Surface the per-workspace sharing UI (Workspaces tab + grant management + share links).',
     false)
ON CONFLICT (key) DO NOTHING;
