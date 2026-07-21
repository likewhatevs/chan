#!/usr/bin/env bash
# Check verify-copr-publication.sh's control flow against recorded COPR API
# fixtures, one per outcome, so every verdict branch is proven reachable
# without a live publication (which lands only at a GA tag push).
#
# The probe reaches the COPR API only through `curl`, so a stub curl on PATH
# stands in for the unauthenticated endpoints. It routes by URL: a
# `build/list` request returns a per-call fixture (so a build can be observed
# running and then succeeded across polls), and a `build-chroot/list` request
# returns the chroot fixture. The probe under test is the real file; only curl
# and the clock knobs (COPR_POLL_INTERVAL, COPR_POLL_BUDGET) are stubbed.
#
# The fixtures mirror the real API shape observed on 2026-07-21 from
# https://copr.fedorainfracloud.org/api_3 (build/list carries id, state,
# source_package.version, submitted_on, chroots; build-chroot/list carries
# per-chroot name and state).
#
# Run: packaging/distros/copr/test-verify-copr-publication.sh

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROBE="$SCRIPT_DIR/verify-copr-publication.sh"
WORK="$(mktemp -d)"
FAILURES=0

# shellcheck disable=SC2329  # runs from the EXIT trap
cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT

mkdir -p "$WORK/bin"

# Stub curl: emits the fixture the routed request maps to. The build/list call
# index advances a per-fixture counter so a scenario can hand back a build that
# is running on early polls and succeeded later.
cat >"$WORK/bin/curl" <<'STUB'
#!/usr/bin/env bash
set -uo pipefail
fix="${COPR_FIXTURE_DIR:?stub curl: COPR_FIXTURE_DIR unset}"
url=""
for a in "$@"; do
    case "$a" in http*) url="$a" ;; esac
done
[ -n "$url" ] || { echo "stub curl: no URL in args" >&2; exit 2; }
case "$url" in
    *build-chroot/list*)
        cat "$fix/build-chroot.json"
        ;;
    *build/list*)
        n=$(( $(cat "$fix/.calls" 2>/dev/null || echo 0) + 1 ))
        echo "$n" >"$fix/.calls"
        if [ -f "$fix/build-list.$n.json" ]; then
            cat "$fix/build-list.$n.json"
        else
            cat "$fix/build-list.json"
        fi
        ;;
    *)
        echo "stub curl: unrouted URL $url" >&2
        exit 2
        ;;
esac
STUB
chmod +x "$WORK/bin/curl"

ok() { echo "ok   $1"; }
bad() {
    echo "FAIL $1"
    FAILURES=$((FAILURES + 1))
}
assert_status() {
    if [ "$1" = "$2" ]; then
        ok "$3 (exit $2)"
    else
        bad "$3: expected exit $1, got $2"
    fi
}
assert_grep() {
    if grep -qF -- "$1" "$2"; then
        ok "$3"
    else
        bad "$3: '$1' missing from $2"
        sed 's/^/     | /' "$2"
    fi
}

# new_fixture <name> -> prints a fresh fixture dir path
new_fixture() {
    local d="$WORK/fix-$1"
    rm -rf "$d"
    mkdir -p "$d"
    printf '%s' "$d"
}

build_json() { # <id> <state> <version> <submitted>
    cat <<JSON
{"items":[
  {"id":$1,"state":"$2","source_package":{"version":"$3"},
   "submitted_on":$4,"started_on":$(($4 + 100)),"ended_on":$(($4 + 1000)),
   "chroots":["fedora-44-x86_64","centos-stream-10-x86_64","centos-stream-10-aarch64"]}
]}
JSON
}

chroots_all_ok() {
    cat <<'JSON'
{"items":[
  {"name":"fedora-44-x86_64","state":"succeeded"},
  {"name":"centos-stream-10-x86_64","state":"succeeded"},
  {"name":"centos-stream-10-aarch64","state":"succeeded"}
]}
JSON
}

chroots_one_failed() {
    cat <<'JSON'
{"items":[
  {"name":"fedora-44-x86_64","state":"succeeded"},
  {"name":"centos-stream-10-x86_64","state":"succeeded"},
  {"name":"centos-stream-10-aarch64","state":"failed"}
]}
JSON
}

# run_probe <fixture dir> <log> [ENV=VAL ...]
# Later ENV=VAL operands override the defaults (env applies them left to right),
# and env parses assignments from "$@" that the shell would treat as commands.
run_probe() {
    local fix="$1" log="$2"
    shift 2
    env PATH="$WORK/bin:$PATH" COPR_FIXTURE_DIR="$fix" \
        PACKAGE=chan RELEASE_TAG=v0.74.0 WEBHOOK_PRESENT=1 CANONICAL=true \
        POSTED_AT=1000 COPR_POLL_INTERVAL=1 COPR_POLL_BUDGET=30 \
        "$@" "$PROBE" >"$log" 2>&1
    return $?
}

echo "== every chroot succeeded at the expected version -> green"
fix="$(new_fixture green)"
build_json 10800001 succeeded 0.74.0-1 1000 >"$fix/build-list.json"
chroots_all_ok >"$fix/build-chroot.json"
run_probe "$fix" "$WORK/green.log"
assert_status 0 $? "a fully succeeded build at the tag is green"
assert_grep "succeeded at 0.74.0-1 on every chroot" "$WORK/green.log" "the green line names the version and chroot set"

echo "== a chroot failed -> red naming the chroot and build id"
fix="$(new_fixture chrootfail)"
build_json 10800002 failed 0.74.0-1 1000 >"$fix/build-list.json"
chroots_one_failed >"$fix/build-chroot.json"
run_probe "$fix" "$WORK/chrootfail.log"
assert_status 1 $? "a failed chroot fails the probe"
assert_grep "build 10800002 for chan ended 'failed'" "$WORK/chrootfail.log" "the red names the build id"
assert_grep "centos-stream-10-aarch64 failed" "$WORK/chrootfail.log" "the red names the failing chroot"

echo "== built version is not the released tag -> red (freeze broken)"
fix="$(new_fixture mismatch)"
build_json 10800003 succeeded 0.73.0-1 1000 >"$fix/build-list.json"
chroots_all_ok >"$fix/build-chroot.json"
run_probe "$fix" "$WORK/mismatch.log"
assert_status 1 $? "a version mismatch fails the probe"
assert_grep "COPR built chan 0.73.0-1 (build 10800003), not the released 0.74.0" "$WORK/mismatch.log" "the red states the provenance mismatch"
assert_grep "main was not frozen" "$WORK/mismatch.log" "the red names the frozen-main cause"

echo "== build still running past the budget -> red, unconfirmed not failed"
fix="$(new_fixture running)"
build_json 10800004 running 0.74.0-1 1000 >"$fix/build-list.json"
chroots_all_ok >"$fix/build-chroot.json"
run_probe "$fix" "$WORK/running.log" COPR_POLL_BUDGET=1
assert_status 1 $? "an unfinished build past budget fails the probe"
assert_grep "still 'running' after" "$WORK/running.log" "the red names the non-terminal state"
assert_grep "UNCONFIRMED (not failed)" "$WORK/running.log" "the budget red says unconfirmed, not failed"

echo "== no build for the package appeared -> red, unconfirmed not failed"
fix="$(new_fixture nobuild)"
# The only build predates the POST, so POSTED_AT excludes it: no build is ours.
build_json 10799000 succeeded 0.74.0-1 500 >"$fix/build-list.json"
chroots_all_ok >"$fix/build-chroot.json"
run_probe "$fix" "$WORK/nobuild.log" COPR_POLL_BUDGET=1
assert_status 1 $? "no matching build past budget fails the probe"
assert_grep "no COPR build for chan appeared" "$WORK/nobuild.log" "the red says no build appeared"
assert_grep "UNCONFIRMED (not failed)" "$WORK/nobuild.log" "the absent-build red says unconfirmed, not failed"

echo "== a build observed running then succeeding -> green (the poll actually waits)"
fix="$(new_fixture wait)"
build_json 10800005 running 0.74.0-1 1000 >"$fix/build-list.1.json"
build_json 10800005 running 0.74.0-1 1000 >"$fix/build-list.2.json"
build_json 10800005 succeeded 0.74.0-1 1000 >"$fix/build-list.json"
chroots_all_ok >"$fix/build-chroot.json"
run_probe "$fix" "$WORK/wait.log"
assert_status 0 $? "a build that finishes within budget greens after polling"
assert_grep "build 10800005 succeeded at 0.74.0-1" "$WORK/wait.log" "the green arrives after the running polls"

echo "== COPR_WEBHOOK absent on the canonical repository -> red"
fix="$(new_fixture canonabsent)"
PATH="$WORK/bin:$PATH" COPR_FIXTURE_DIR="$fix" \
    PACKAGE=chan RELEASE_TAG=v0.74.0 WEBHOOK_PRESENT=0 CANONICAL=true \
    "$PROBE" >"$WORK/canonabsent.log" 2>&1
assert_status 1 $? "an absent webhook on the canonical repo is red"
assert_grep "COPR_WEBHOOK is absent on the canonical repository" "$WORK/canonabsent.log" "the canonical red names the absent secret"

echo "== COPR_WEBHOOK absent on a fork -> green no-op"
fix="$(new_fixture forkabsent)"
PATH="$WORK/bin:$PATH" COPR_FIXTURE_DIR="$fix" \
    PACKAGE=chan RELEASE_TAG=v0.74.0 WEBHOOK_PRESENT=0 CANONICAL=false \
    "$PROBE" >"$WORK/forkabsent.log" 2>&1
assert_status 0 $? "an absent webhook on a fork is a green no-op"
assert_grep "absent on a fork" "$WORK/forkabsent.log" "the fork no-op says so"

echo
if [ "$FAILURES" -eq 0 ]; then
    echo "all checks passed"
    exit 0
fi
echo "$FAILURES check(s) failed"
exit 1
