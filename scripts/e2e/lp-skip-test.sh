#!/usr/bin/env bash
#
# Host-only, no-network test for the PPA publish retry-idempotence:
#
#   1. lp-accepted.sh exit codes against a local mock of Launchpad's
#      getPublishedSources API (scripts/e2e/lp-mock.py): accepted and
#      pending are 0; missing, superseded, and wrong-series are 1;
#      malformed JSON and HTTP failures are 2 (unknown, never
#      "accepted").
#   2. upload.sh's per-changes loop with a stubbed dput on PATH:
#      accepted series SKIP without touching dput; a fresh series
#      uploads once; a series whose dput fails but whose acceptance
#      check flips true (client-side failure, server-side accept)
#      succeeds without exhausting retries; a series that keeps
#      failing retries DPUT_ATTEMPTS times, does not abort its
#      siblings, and is listed in the nonzero exit.
#
# Exit 0 iff every assertion passed. Plain-text PASS/FAIL lines on
# stdout.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$SCRIPT_DIR/../.." && pwd)"
DEBIAN_DIR="$REPO/packaging/distros/debian"

FAILURES=0
pass() { printf 'PASS %s\n' "$*"; }
fail() {
    printf 'FAIL %s\n' "$*"
    FAILURES=$((FAILURES + 1))
}

WORK="$(mktemp -d)"
MOCK_PID=""
cleanup() {
    [ -n "$MOCK_PID" ] && kill "$MOCK_PID" 2>/dev/null
    rm -rf "$WORK"
}
trap cleanup EXIT

# ---------------------------------------------------------------
# Mock up
# ---------------------------------------------------------------
python3 "$SCRIPT_DIR/lp-mock.py" > "$WORK/port" &
MOCK_PID=$!
for _ in $(seq 50); do
    [ -s "$WORK/port" ] && break
    sleep 0.1
done
PORT="$(cat "$WORK/port")"
if [ -z "$PORT" ]; then
    echo "mock did not start" >&2
    exit 2
fi
export LP_API_BASE="http://127.0.0.1:$PORT"
export PPA="ppa:fiorix/chan"

# ---------------------------------------------------------------
# 1. lp-accepted.sh exit codes
# ---------------------------------------------------------------
check_exit() { # check_exit <expected> <source-name> <label>
    local expected="$1" name="$2" label="$3" got
    "$DEBIAN_DIR/lp-accepted.sh" "$name" "1.0-1~noble1" noble >/dev/null 2>&1
    got=$?
    if [ "$got" = "$expected" ]; then
        pass "lp-accepted: $label exits $expected"
    else
        fail "lp-accepted: $label expected exit $expected, got $got"
    fi
}
check_exit 0 accepted "Published for the series"
check_exit 0 pending "Pending for the series"
check_exit 1 missing "no publication"
check_exit 1 superseded "Superseded is not an acceptance"
check_exit 1 otherseries "acceptance for another series"
check_exit 2 malformed "malformed API JSON"
check_exit 2 broken "HTTP 500 from the API"

# The API being unreachable is also 2, never 0/1.
LP_API_BASE="http://127.0.0.1:1" "$DEBIAN_DIR/lp-accepted.sh" chan 1.0 noble >/dev/null 2>&1
if [ $? = 2 ]; then
    pass "lp-accepted: unreachable API exits 2"
else
    fail "lp-accepted: unreachable API must exit 2"
fi

# ---------------------------------------------------------------
# 2. upload.sh loop behavior (stub dput, zero backoff)
# ---------------------------------------------------------------
mkdir -p "$WORK/bin"
DPUT_LOG="$WORK/dput.log"
: > "$DPUT_LOG"
cat > "$WORK/bin/dput" <<EOF
#!/usr/bin/env bash
# Test stub: log the call, then fail iff the changes file's package
# scenario says so (alwaysfail / flaky never "reach" Launchpad).
echo "\$*" >> "$DPUT_LOG"
case "\$*" in
*alwaysfail*|*flaky*) exit 1 ;;
*) exit 0 ;;
esac
EOF
chmod +x "$WORK/bin/dput"
export PATH="$WORK/bin:$PATH"
export DPUT_BACKOFF_SECS=0
export DPUT_ATTEMPTS=3

changes_for() { # changes_for <pkg> -> path (created)
    local dir="$WORK/ppa/$1/noble"
    mkdir -p "$dir"
    : > "$dir/${1}_1.0-1~noble1_source.changes"
    echo "$dir/${1}_1.0-1~noble1_source.changes"
}

c_accepted="$(changes_for accepted)"
c_fresh="$(changes_for fresh)"
c_flaky="$(changes_for flaky)"
c_alwaysfail="$(changes_for alwaysfail)"

out="$("$DEBIAN_DIR/upload.sh" "$c_accepted" "$c_fresh" "$c_flaky" "$c_alwaysfail" 2>&1)"
rc=$?

if printf '%s' "$out" | grep -q "SKIP accepted/noble: already accepted"; then
    pass "upload: accepted series logs SKIP"
else
    fail "upload: missing SKIP line, got: $out"
fi
if grep -q accepted "$DPUT_LOG"; then
    fail "upload: dput ran for an accepted series"
else
    pass "upload: accepted series never reaches dput"
fi
if [ "$(grep -c fresh "$DPUT_LOG")" = 1 ]; then
    pass "upload: fresh series dputs exactly once"
else
    fail "upload: fresh series dput count != 1: $(grep -c fresh "$DPUT_LOG")"
fi
# flaky: dput fails once, then the acceptance recheck (mock answers
# Published from its second request on) short-circuits the retries.
if [ "$(grep -c flaky "$DPUT_LOG")" = 1 ] &&
    printf '%s' "$out" | grep -q "flaky/noble: accepted server-side"; then
    pass "upload: client failure with server-side accept stops retrying"
else
    fail "upload: flaky handling wrong (dput count $(grep -c flaky "$DPUT_LOG")): $out"
fi
if [ "$(grep -c alwaysfail "$DPUT_LOG")" = "$DPUT_ATTEMPTS" ]; then
    pass "upload: persistent failure retries $DPUT_ATTEMPTS times"
else
    fail "upload: alwaysfail dput count != $DPUT_ATTEMPTS: $(grep -c alwaysfail "$DPUT_LOG")"
fi
if [ "$rc" != 0 ] && printf '%s' "$out" | grep -q "upload failed for: alwaysfail/noble"; then
    pass "upload: run exits nonzero listing only the failed series"
else
    fail "upload: expected nonzero exit naming alwaysfail/noble, got rc=$rc: $out"
fi
# The forced retries carry -f from attempt 2 on (dput's local
# already-uploaded marker must not block a real retry).
if [ "$(grep -c -- '-f ppa:fiorix/chan.*alwaysfail' "$DPUT_LOG")" = "$((DPUT_ATTEMPTS - 1))" ]; then
    pass "upload: retries force past dput's local upload log"
else
    fail "upload: expected $((DPUT_ATTEMPTS - 1)) forced retries: $(cat "$DPUT_LOG")"
fi

echo ""
if [ "$FAILURES" -gt 0 ]; then
    echo "RESULT: $FAILURES assertion(s) FAILED"
    exit 1
fi
echo "RESULT: all assertions passed"
