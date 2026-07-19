#!/usr/bin/env bash
# Check build-with-sdme.sh's host-side control flow against a stub sdme.
#
# The real matrix needs imported CentOS rootfs images and hours of offline RPM
# rebuilds, so the parts that are easy to get wrong and expensive to reach
# (per-target status capture, result ownership, a re-run after a failed target,
# interrupt handling, knob and preflight validation) are exercised here. A stub
# sdme reproduces the two container behaviours the driver is built around,
# confirmed by a live sdme probe on 2026-07-19:
#
#   - `sdme new` propagates the guest exit status and deletes the container
#     when it is non-zero, while the writable host bind survives.
#   - `-- /usr/bin/env VAR=... /bin/bash -c '<multiline>'` delivers the script
#     as one argv element.
#
# The driver under test is symlinked into a throwaway repo skeleton, so it is
# the real file; only its guest side and sdme are stubs.
#
# Run: packaging/distros/copr/test-build-with-sdme.sh

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DRIVER="$SCRIPT_DIR/build-with-sdme.sh"
WORK="$(mktemp -d)"
REPO="$WORK/repo"
COPR_DIR="$REPO/packaging/distros/copr"
export STUB_STATE="$WORK/state"
FAILURES=0

cleanup() {
    chmod -R u+rwX "$WORK" 2>/dev/null || true
    rm -rf "$WORK"
}
trap cleanup EXIT

mkdir -p "$COPR_DIR" "$WORK/bin" "$STUB_STATE/containers"
ln -s "$DRIVER" "$COPR_DIR/build-with-sdme.sh"
printf '%s\n' centos-stream-9 centos-stream-10 >"$STUB_STATE/rootfs"

cat >"$WORK/bin/sdme" <<'STUB'
#!/usr/bin/env bash
# Stub sdme: enough of `fs ls`, `rm -f`, and `new` to drive the host side.
set -uo pipefail
state="${STUB_STATE:?}"
cmd="${1:?stub sdme: no subcommand}"
shift
fail() {
    echo "stub sdme: $1" >&2
    exit 64
}
case "$cmd" in
    fs)
        [ "${1:-}" = ls ] || fail "unsupported fs subcommand ${1:-}"
        cat "$state/rootfs"
        ;;
    rm)
        [ "${1:-}" = -f ] || fail "unsupported rm form"
        [ -e "$state/containers/$2" ] || exit 1
        rm -f "$state/containers/$2"
        ;;
    new)
        name="${1:?stub sdme: no container name}"
        shift
        binds=()
        while [ $# -gt 0 ]; do
            case "$1" in
                -r|-t) shift 2 ;;
                -b) binds+=("$2"); shift 2 ;;
                --) shift; break ;;
                *) fail "unexpected argument $1" ;;
            esac
        done
        [ "${1:-}" = /usr/bin/env ] || fail "guest argv does not start with /usr/bin/env"
        shift
        envs=()
        while [ $# -gt 0 ] && [ "$1" != /bin/bash ]; do
            envs+=("$1")
            shift
        done
        [ "${1:-}" = /bin/bash ] || fail "guest argv has no /bin/bash"
        shift
        [ "${1:-}" = -c ] || fail "guest argv has no -c"
        shift
        script="${1:?stub sdme: no guest script}"
        shift
        [ $# -eq 0 ] || fail "guest script did not arrive as one argv element"
        case "$script" in
            *$'\n'*) ;;
            *) fail "guest script lost its line structure" ;;
        esac

        # Retarget the guest paths at their host side, longest first so /srpm
        # survives the /src substitution.
        out=""
        while read -r _ bind; do
            host="${bind%%:*}"
            guest="${bind#*:}"
            guest="${guest%%:*}"
            script="${script//$guest/$host}"
            [ "$guest" = /out ] && out="$host"
        done < <(printf '%s\n' "${binds[@]}" | awk -F: '{ print length($2), $0 }' | sort -rn)
        [ -n "$out" ] || fail "no /out bind"

        printf '%s\n' "$name" >>"$state/started"
        : >"$state/containers/$name"
        [ -n "${STUB_SLEEP:-}" ] && sleep "$STUB_SLEEP"
        if [ -n "${STUB_EXIT:-}" ]; then
            rm -f "$state/containers/$name"
            exit "$STUB_EXIT"
        fi
        env "${envs[@]}" STUB_OUT="$out" /bin/bash -c "$script"
        status=$?
        if [ -n "${STUB_EAT_STATUS:-}" ]; then
            rm -f "$out/status"
            status=0
        fi
        # A non-zero guest status takes the container with it.
        [ "$status" -eq 0 ] || rm -f "$state/containers/$name"
        exit "$status"
        ;;
    *)
        fail "unsupported subcommand $cmd"
        ;;
esac
STUB

cat >"$WORK/bin/sudo" <<'STUB'
#!/usr/bin/env bash
# Stub sudo: runs the command, so a missing sdme surfaces the way it does on a
# host where sudo itself always exists.
exec "$@"
STUB

cat >"$COPR_DIR/build-srpm.sh" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)/target/distros/srpm"
mkdir -p "$dir"
for pkg in "$@"; do
    : >"$dir/$pkg-0.0.0-1.src.rpm"
done
STUB

cat >"$COPR_DIR/build-in-container.sh" <<'STUB'
#!/usr/bin/env bash
# Stub guest: writes the artifacts a real run leaves behind, then exits with
# the status the test asked for. $STUB_OUT stands in for the /out bind.
set -uo pipefail
out="${STUB_OUT:?}"
state="${STUB_STATE:?}"
: >"$out/$PKG-0.0.0-1.el$EL_RELEASE.$(uname -m).rpm"
printf 'use dnf upgrade\n' >"$out/upgrade.out"
status_file="$state/guest-status-$EL_RELEASE-$PKG"
[ -r "$status_file" ] && exit "$(cat "$status_file")"
exit "${STUB_GUEST_STATUS:-0}"
STUB

chmod +x "$WORK/bin/sdme" "$WORK/bin/sudo" "$COPR_DIR/build-srpm.sh" \
    "$COPR_DIR/build-in-container.sh"

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
    fi
}
assert_present() {
    if [ -e "$1" ]; then
        ok "$2"
    else
        bad "$2: $1 is missing"
    fi
}

run_driver() {
    # run_driver <log name> <results name> [VAR=VALUE ...]
    local name="$1" out="$2"
    shift 2
    rm -f "$STUB_STATE/started"
    LOG="$WORK/$name.log"
    OUT_DIR="$WORK/out-$out"
    env SDME="$WORK/bin/sdme" OUT="$OUT_DIR" "$@" \
        "$COPR_DIR/build-with-sdme.sh" >"$LOG" 2>&1
    return $?
}

echo "== all targets pass"
run_driver pass pass PKG=all COPR_RELEASE=all
assert_status 0 $? "clean matrix succeeds"
assert_grep "PASS el9 chan" "$LOG" "el9 chan reported PASS"
assert_grep "PASS el10 chan-desktop" "$LOG" "el10 chan-desktop reported PASS"
assert_status 3 "$(wc -l <"$STUB_STATE/started")" "three targets ran"
assert_status 0 "$(ls "$STUB_STATE/containers" | wc -l)" "no container survives a clean run"

echo "== one target fails"
printf '7\n' >"$STUB_STATE/guest-status-10-chan"
run_driver fail1 fail PKG=chan COPR_RELEASE=all KEEP_CONTAINER=1
assert_status 1 $? "a failed target fails the matrix"
assert_grep "PASS el9 chan" "$LOG" "the earlier target still passes"
assert_grep "FAIL el10 chan $(uname -m) (status 7" "$LOG" "the failed target reports the guest status"
if ls "$STUB_STATE/containers" | grep -q -- "-el10-chan-"; then
    ok "the failed target's container survives KEEP_CONTAINER=1"
else
    bad "the failed target's container was deleted under KEEP_CONTAINER=1"
fi
FAIL_DIR="$OUT_DIR/el10/$(uname -m)/chan"
assert_present "$FAIL_DIR/build.log" "the failed target leaves a build log"
assert_present "$FAIL_DIR/upgrade.out" "the failed target leaves its guest artifacts"
if [ -O "$FAIL_DIR/upgrade.out" ] && [ -w "$FAIL_DIR" ]; then
    ok "the failed target's results stay writable by the host user"
else
    bad "the failed target's results are not host-owned"
fi
rm -f "$STUB_STATE/containers"/*

echo "== re-run over a failed run's results"
run_driver fail2 fail PKG=chan COPR_RELEASE=all REUSE_SRPM=1
assert_status 1 $? "the failure reproduces on the second run"
assert_grep "reusing existing SRPMs" "$LOG" "REUSE_SRPM=1 skips the SRPM build"
rm -f "$STUB_STATE/guest-status-10-chan"
run_driver fixed fail PKG=chan COPR_RELEASE=all REUSE_SRPM=1
assert_status 0 $? "a fixed target passes on re-run over the same results"
assert_grep "PASS el10 chan" "$LOG" "the stale FAIL status is not reused"

echo "== the guest wrapper never ran"
run_driver nostatus nostatus PKG=chan COPR_RELEASE=9 STUB_EAT_STATUS=1
assert_status 1 $? "a missing status file with sdme exit 0 is a failure"
assert_grep "wrote no" "$LOG" "the missing status file is reported"

echo "== interrupt"
# The operator's Ctrl-C reaches the whole process group, so cover both halves:
# the driver taking the signal itself, and sdme dying on it first. Job control
# gives the driver its own process group and stops bash from making a
# background job ignore SIGINT, which is what a terminal run looks like.
rm -f "$STUB_STATE/started"
set -m
env SDME="$WORK/bin/sdme" OUT="$WORK/out-int" PKG=chan COPR_RELEASE=all \
    STUB_SLEEP=2 "$COPR_DIR/build-with-sdme.sh" >"$WORK/int.log" 2>&1 &
int_pid=$!
set +m
for _ in $(seq 1 100); do
    [ -s "$STUB_STATE/started" ] && break
    sleep 0.1
done
kill -INT -- -"$int_pid"
wait "$int_pid"
assert_status 130 $? "SIGINT to the driver aborts with 130"
assert_status 1 "$(wc -l <"$STUB_STATE/started")" "the matrix stops at the interrupted target"
assert_grep "aborting the matrix" "$WORK/int.log" "the abort is announced"

run_driver int2 int2 PKG=chan COPR_RELEASE=all STUB_EXIT=130
assert_status 130 $? "an interrupted sdme aborts with 130"
assert_status 1 "$(wc -l <"$STUB_STATE/started")" "no later target is started"
assert_grep "was interrupted" "$LOG" "the interrupted sdme is named"

echo "== knob and preflight validation"
run_driver knob1 knob1 KEEP_CONTAINER=yes
assert_status 1 $? "KEEP_CONTAINER=yes is rejected"
assert_grep "KEEP_CONTAINER must be 0 or 1" "$LOG" "the rejection names the knob"
run_driver knob2 knob2 REUSE_SRPM=true
assert_status 1 $? "REUSE_SRPM=true is rejected"
assert_grep "REUSE_SRPM must be 0 or 1" "$LOG" "the rejection names the knob"

PATH="$WORK/bin:$PATH" run_driver nosdme nosdme SDME="sudo definitely-not-sdme"
assert_status 1 $? "a missing sdme behind sudo fails the preflight"
assert_grep "fs ls' failed" "$LOG" "the preflight probes through the sudo form"

run_driver norootfs norootfs PKG=chan COPR_RELEASE=9 COPR_EL9_ROOTFS=not-imported
assert_status 1 $? "an unimported rootfs stops the run"
assert_grep "centos-stream-10" "$LOG" "the error lists the host's rootfs entries"

echo "== unwritable results"
STALE="$WORK/out-stale/el9/$(uname -m)/chan"
mkdir -p "$STALE"
: >"$STALE/status"
chmod 500 "$STALE"
env SDME="$WORK/bin/sdme" OUT="$WORK/out-stale" PKG=chan COPR_RELEASE=9 \
    "$COPR_DIR/build-with-sdme.sh" >"$WORK/stale.log" 2>&1
assert_status 1 $? "an unclearable result directory stops the run"
assert_grep "cannot write results into" "$WORK/stale.log" "the error names the directory"
assert_grep "sudo rm -rf" "$WORK/stale.log" "the error says how to clear it"
chmod 700 "$STALE"

echo
if [ "$FAILURES" -eq 0 ]; then
    echo "all checks passed"
    exit 0
fi
echo "$FAILURES check(s) failed"
exit 1
