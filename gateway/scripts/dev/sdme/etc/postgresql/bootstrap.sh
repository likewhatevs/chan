#!/usr/bin/env bash
# Dev first-boot setup for chan-psql (sdme): create a throwaway `chan`
# SUPERUSER (password `chan`) and the chan_gateway + chan_gateway_test
# databases it owns. Idempotent; re-runs are no-ops.
#
# Driven by chan-pg-bootstrap.service (oneshot, after postgresql).
#
# DEV ONLY. The prod bootstrap reads POSTGRES_PASSWORD from
# /run/secrets/postgres.env and creates a locked-down, non-superuser
# `chan` role plus only chan_gateway. This one hardcodes the password
# (so the container is zero-config) and grants SUPERUSER so the
# integration-test harness can create a per-test schema and the
# pg_reaper can terminate leaked peer sessions without extra grants.
# Also seeds chan_gateway_test, which the tests connect to via
# TEST_DATABASE_URL.

set -euo pipefail

DEV_PASSWORD="chan"

# Wait until postgres actually accepts connections. After=postgresql
# is not enough on first boot: the unit reports up before the socket
# is listening.
for _ in $(seq 1 30); do
    if runuser -u postgres -- pg_isready -q; then
        break
    fi
    sleep 1
done

psql_as_pg() { runuser -u postgres -- psql -v ON_ERROR_STOP=1 "$@"; }

# Password on stdin via heredoc, not -c, so it never lands on argv
# (process-private). Mirrors the prod bootstrap's hygiene even though
# this password is not secret.
role_exists=$(psql_as_pg -tAc "SELECT 1 FROM pg_roles WHERE rolname='chan'" | tr -d '[:space:]')
if [[ "$role_exists" == "1" ]]; then
    psql_as_pg <<SQL
ALTER ROLE chan WITH LOGIN SUPERUSER PASSWORD '${DEV_PASSWORD}';
SQL
else
    psql_as_pg <<SQL
CREATE ROLE chan WITH LOGIN SUPERUSER PASSWORD '${DEV_PASSWORD}';
SQL
fi

for db in chan_gateway chan_gateway_test; do
    exists=$(psql_as_pg -tAc "SELECT 1 FROM pg_database WHERE datname='${db}'" | tr -d '[:space:]')
    if [[ "$exists" != "1" ]]; then
        psql_as_pg -c "CREATE DATABASE ${db} OWNER chan;"
    fi
done

echo "chan-pg-bootstrap: dev role + databases ready"
