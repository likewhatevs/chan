#!/usr/bin/env bash
# packaging/gateway/scripts/dev/run.sh
#
# Foreground runner for the local dev stack. Starts profile,
# identity, and devserver-proxy concurrently with their generated env
# files sourced; multiplexes stdout/stderr to this terminal with a
# per-service prefix; Ctrl-C cleanly stops all three.
#
# Builds in cargo `dev` profile. First run can take a couple of
# minutes; subsequent runs are incremental.

set -euo pipefail

cd "$(dirname "$0")"
SCRIPT_DIR="$(pwd -P)"
SECRETS_DIR="$SCRIPT_DIR/secrets"
ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)/gateway"

for f in profile identity devserver-proxy; do
    if [[ ! -f "$SECRETS_DIR/$f.env" ]]; then
        echo "error: $SECRETS_DIR/$f.env missing; run packaging/gateway/scripts/dev/setup.sh first" >&2
        exit 1
    fi
done

# Build everything up front so the three services don't race to
# compile the same dependency graph from cold.
echo "==> cargo build (workspace)"
(cd "$ROOT" && cargo build --quiet \
    --bin profile-service \
    --bin identity-service \
    --bin devserver-proxy-service)

pids=()
cleanup() {
    echo
    echo "==> stopping services"
    for pid in "${pids[@]:-}"; do
        kill -INT "$pid" 2>/dev/null || true
    done
    # Give them a beat to drain; then SIGTERM stragglers.
    sleep 1
    for pid in "${pids[@]:-}"; do
        kill -TERM "$pid" 2>/dev/null || true
    done
    wait 2>/dev/null || true
    rm -f "$SCRIPT_DIR/.run.pid"
}
trap cleanup EXIT INT TERM

# Publish our pgid so an external caller (or another shell) can
# `kill -INT -- -<pgid>` if Ctrl-C ever fails to reach us.
echo "$$" > "$SCRIPT_DIR/.run.pid"

start_service() {
    local name=$1 bin=$2 env=$3 color=$4
    (
        set -a
        # shellcheck disable=SC1090
        . "$env"
        set +a
        cd "$ROOT"
        # Prefix every line so multiplexed output stays readable.
        cargo run --quiet --bin "$bin" 2>&1 \
            | awk -v name="$name" -v c="$color" '
                BEGIN { reset = "\033[0m" }
                { printf "%s%-12s%s | %s\n", c, "[" name "]", reset, $0; fflush() }
              '
    ) &
    pids+=($!)
}

# Order: profile first so migrations are done before identity
# tries to look up users; identity second; devserver-proxy last so
# its tunnel handshakes go to a live identity.
start_service profile      profile-service     "$SECRETS_DIR/profile.env"     $'\033[36m'
sleep 1
start_service identity     identity-service    "$SECRETS_DIR/identity.env"    $'\033[33m'
sleep 1
start_service devserver-proxy  devserver-proxy-service "$SECRETS_DIR/devserver-proxy.env" $'\033[35m'

echo
echo "==> services starting"
echo "    profile     127.0.0.1:17001"
echo "    identity    http://id.localtest.me:17000"
echo "    devserver-proxy http://devserver.localtest.me:17002 (apex)"
echo "                http://*.devserver.localtest.me:17002 (wildcard)"
echo "                127.0.0.1:17100 (h2c tunnel)"
echo
echo "Open the dashboard: http://id.localtest.me:17000"
echo "Ctrl-C to stop."
echo

# Poll the pid list every second; bail when any of the three dies.
# `wait -n` would be cleaner but macOS's bash 3.2 doesn't have it.
exit_code=0
while :; do
    for pid in "${pids[@]}"; do
        if ! kill -0 "$pid" 2>/dev/null; then
            echo "==> pid $pid exited; shutting down"
            wait "$pid" 2>/dev/null
            exit_code=$?
            exit "$exit_code"
        fi
    done
    sleep 1
done
