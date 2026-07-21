-- Durable second-cut settlement for grant, block, PAT, and account deletion.
-- API handlers write this outbox in the same transaction as profile-owned
-- denial state; the profile worker resumes it after process restart.
CREATE TABLE control_revocation_jobs (
    job_key                text        PRIMARY KEY,
    kind                   text        NOT NULL CHECK (kind IN ('exact', 'subject', 'account_delete')),
    subject_user_id        uuid        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    owner_user_id          uuid,
    devserver_id           text,
    phase                  text        NOT NULL DEFAULT 'pending_first_cut'
                                      CHECK (phase IN ('pending_first_cut', 'settling')),
    first_cut_confirmed_at timestamptz,
    settle_not_before      timestamptz,
    deadline               timestamptz,
    next_attempt_at        timestamptz NOT NULL DEFAULT now(),
    attempts               integer     NOT NULL DEFAULT 0,
    generation             bigint      NOT NULL DEFAULT 0,
    created_at             timestamptz NOT NULL DEFAULT now(),
    updated_at             timestamptz NOT NULL DEFAULT now(),
    CHECK (
        (kind = 'exact' AND owner_user_id IS NOT NULL AND devserver_id IS NOT NULL)
        OR
        (kind <> 'exact' AND owner_user_id IS NULL AND devserver_id IS NULL)
    ),
    CHECK (
        (phase = 'pending_first_cut' AND first_cut_confirmed_at IS NULL AND settle_not_before IS NULL)
        OR
        (phase = 'settling' AND first_cut_confirmed_at IS NOT NULL AND settle_not_before IS NOT NULL)
    )
);

CREATE INDEX control_revocation_jobs_due_idx
    ON control_revocation_jobs (next_attempt_at);

CREATE INDEX control_revocation_jobs_subject_idx
    ON control_revocation_jobs (subject_user_id);
