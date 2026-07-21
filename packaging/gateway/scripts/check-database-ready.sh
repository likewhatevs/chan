#!/bin/sh
# Require the exact post-migration privilege-reconcile marker using an app URL.

set -eu

: "${DATABASE_URL:?DATABASE_URL is required}"
: "${EXPECTED_SQLX_MIGRATION:?EXPECTED_SQLX_MIGRATION is required}"
: "${DATABASE_ROLE_POLICY_VERSION:?DATABASE_ROLE_POLICY_VERSION is required}"

case "$EXPECTED_SQLX_MIGRATION" in
    ''|*[!0-9]*) echo "EXPECTED_SQLX_MIGRATION must be numeric" >&2; exit 1 ;;
esac
case "$DATABASE_ROLE_POLICY_VERSION" in
    ''|*[!0-9]*) echo "DATABASE_ROLE_POLICY_VERSION must be numeric" >&2; exit 1 ;;
esac
[ "$EXPECTED_SQLX_MIGRATION" -gt 0 ] \
    || { echo "EXPECTED_SQLX_MIGRATION must be positive" >&2; exit 1; }
[ "$DATABASE_ROLE_POLICY_VERSION" -gt 0 ] \
    || { echo "DATABASE_ROLE_POLICY_VERSION must be positive" >&2; exit 1; }

ready=$(psql "$DATABASE_URL" --no-password --set=ON_ERROR_STOP=1 -Atqc \
    "SELECT migration_version = $EXPECTED_SQLX_MIGRATION
        AND role_policy_version = $DATABASE_ROLE_POLICY_VERSION
     FROM public.chan_gateway_deployment_state
     WHERE singleton")
[ "$ready" = t ] \
    || { echo "database migration/role-policy marker is not ready" >&2; exit 1; }
