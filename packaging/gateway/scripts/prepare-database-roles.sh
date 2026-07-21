#!/bin/sh
# Prepare non-DDL gateway roles and invalidate the rollout-ready marker.

set -eu

: "${DATABASE_URL:?DATABASE_URL is required}"
: "${IDENTITY_DATABASE_PASSWORD:?IDENTITY_DATABASE_PASSWORD is required}"
: "${PROFILE_DATABASE_PASSWORD:?PROFILE_DATABASE_PASSWORD is required}"

psql "$DATABASE_URL" --no-password --set=ON_ERROR_STOP=1 \
    --set=identity_password="$IDENTITY_DATABASE_PASSWORD" \
    --set=profile_password="$PROFILE_DATABASE_PASSWORD" <<'SQL'
SELECT format(
    'CREATE ROLE chan_gateway_identity LOGIN PASSWORD %L',
    :'identity_password'
)
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'chan_gateway_identity')
\gexec

SELECT format(
    'CREATE ROLE chan_gateway_profile LOGIN PASSWORD %L',
    :'profile_password'
)
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'chan_gateway_profile')
\gexec

SELECT format('ALTER ROLE chan_gateway_identity PASSWORD %L', :'identity_password')
\gexec
SELECT format('ALTER ROLE chan_gateway_profile PASSWORD %L', :'profile_password')
\gexec

-- NOINHERIT is not enough: a member can still SET ROLE. Remove every role
-- membership before applying the fixed administrative-attribute baseline.
SELECT format('REVOKE %I FROM chan_gateway_identity', parent.rolname)
FROM pg_auth_members membership
JOIN pg_roles parent ON parent.oid = membership.roleid
JOIN pg_roles member ON member.oid = membership.member
WHERE member.rolname = 'chan_gateway_identity'
\gexec
SELECT format('REVOKE %I FROM chan_gateway_profile', parent.rolname)
FROM pg_auth_members membership
JOIN pg_roles parent ON parent.oid = membership.roleid
JOIN pg_roles member ON member.oid = membership.member
WHERE member.rolname = 'chan_gateway_profile'
\gexec

ALTER ROLE chan_gateway_identity NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;
ALTER ROLE chan_gateway_profile NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;

SELECT format(
    'REVOKE ALL PRIVILEGES ON DATABASE %I FROM chan_gateway_identity, chan_gateway_profile',
    current_database()
)
\gexec
SELECT format(
    'GRANT CONNECT ON DATABASE %I TO chan_gateway_identity, chan_gateway_profile',
    current_database()
)
\gexec

CREATE SCHEMA IF NOT EXISTS tower_sessions AUTHORIZATION CURRENT_USER;
ALTER SCHEMA tower_sessions OWNER TO CURRENT_USER;

-- Remove both PostgreSQL defaults and any grants left by an earlier package.
-- Positive privileges are restored only by the post-migration reconcile step.
REVOKE ALL ON SCHEMA public, tower_sessions
    FROM PUBLIC, chan_gateway_identity, chan_gateway_profile;
REVOKE ALL ON ALL TABLES IN SCHEMA public, tower_sessions
    FROM PUBLIC, chan_gateway_identity, chan_gateway_profile;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA public, tower_sessions
    FROM PUBLIC, chan_gateway_identity, chan_gateway_profile;

-- Clean up the broad default grants shipped by the initial hardening draft.
-- No deployment path adds a positive default privilege.
ALTER DEFAULT PRIVILEGES FOR ROLE CURRENT_USER IN SCHEMA public, tower_sessions
    REVOKE ALL ON TABLES FROM chan_gateway_identity, chan_gateway_profile;
ALTER DEFAULT PRIVILEGES FOR ROLE CURRENT_USER IN SCHEMA public, tower_sessions
    REVOKE ALL ON SEQUENCES FROM chan_gateway_identity, chan_gateway_profile;

CREATE TABLE IF NOT EXISTS public.chan_gateway_deployment_state (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    migration_version bigint NOT NULL,
    role_policy_version bigint NOT NULL,
    reconciled_at timestamptz
);
ALTER TABLE public.chan_gateway_deployment_state OWNER TO CURRENT_USER;
REVOKE ALL ON public.chan_gateway_deployment_state
    FROM PUBLIC, chan_gateway_identity, chan_gateway_profile;
GRANT USAGE ON SCHEMA public TO chan_gateway_identity, chan_gateway_profile;
GRANT SELECT ON public.chan_gateway_deployment_state
    TO chan_gateway_identity, chan_gateway_profile;
INSERT INTO public.chan_gateway_deployment_state (
    singleton, migration_version, role_policy_version, reconciled_at
) VALUES (true, -1, -1, NULL)
ON CONFLICT (singleton) DO UPDATE SET
    migration_version = -1,
    role_policy_version = -1,
    reconciled_at = NULL;

-- REVOKE cannot constrain an object owner. Refuse poisoned pre-existing roles
-- rather than claiming a least-privilege matrix that PostgreSQL would bypass.
DO $ownership$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_database database
        JOIN pg_roles owner ON owner.oid = database.datdba
        WHERE database.datname = current_database()
          AND owner.rolname IN ('chan_gateway_identity', 'chan_gateway_profile')
    ) OR EXISTS (
        SELECT 1 FROM pg_namespace namespace
        JOIN pg_roles owner ON owner.oid = namespace.nspowner
        WHERE owner.rolname IN ('chan_gateway_identity', 'chan_gateway_profile')
    ) OR EXISTS (
        SELECT 1 FROM pg_class relation
        JOIN pg_roles owner ON owner.oid = relation.relowner
        WHERE owner.rolname IN ('chan_gateway_identity', 'chan_gateway_profile')
    ) OR EXISTS (
        SELECT 1 FROM pg_proc function
        JOIN pg_roles owner ON owner.oid = function.proowner
        WHERE owner.rolname IN ('chan_gateway_identity', 'chan_gateway_profile')
    ) OR EXISTS (
        SELECT 1 FROM pg_type type
        JOIN pg_roles owner ON owner.oid = type.typowner
        WHERE owner.rolname IN ('chan_gateway_identity', 'chan_gateway_profile')
    ) OR EXISTS (
        SELECT 1 FROM pg_extension extension
        JOIN pg_roles owner ON owner.oid = extension.extowner
        WHERE owner.rolname IN ('chan_gateway_identity', 'chan_gateway_profile')
    ) THEN
        RAISE EXCEPTION 'application database role owns an object';
    END IF;
END
$ownership$;
SQL
