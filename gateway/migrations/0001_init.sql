-- Initial schema for the profile service.
--
-- One canonical user keyed by an internal uuid. Multiple linked
-- OAuth identities resolve to the same user when the verified
-- emails match: signing in with a second provider attaches a row
-- to `identities` instead of creating a duplicate user.
--
-- Email is verified-by-provider; we do not run our own verification.
-- Email on `users` is the primary contact email; email on
-- `identities` is whatever the provider returned for that link.

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE users (
    id           uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    email        text        NOT NULL,
    display_name text,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX users_email_lower_idx ON users (lower(email));

CREATE TABLE identities (
    id               uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id          uuid        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider         text        NOT NULL,
    provider_subject text        NOT NULL,
    email            text,
    created_at       timestamptz NOT NULL DEFAULT now(),
    UNIQUE (provider, provider_subject)
);

CREATE INDEX identities_user_id_idx ON identities (user_id);

CREATE TABLE drives (
    id         uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    uuid        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    label      text        NOT NULL,
    url        text        NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX drives_user_id_idx ON drives (user_id);
