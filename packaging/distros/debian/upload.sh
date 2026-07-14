#!/usr/bin/env bash
#
# dput the source packages built by build-source.sh to the Launchpad
# PPA, retry-idempotently: each *_source.changes is skipped when
# Launchpad already accepted that (package, version) for its series
# (lp-accepted.sh), otherwise dput'ed with bounded retries and a
# fresh acceptance check between attempts -- a client-side failure
# (an FTP 550, a dropped connection) may still have been accepted
# server-side, where a blind retry would be rejected as a duplicate.
# One failing series does not abort the rest: the run uploads
# everything it can and exits nonzero listing every series that still
# failed. Re-running the whole workflow after a transient failure is
# therefore safe: accepted series skip, failed series retry.
#
# Requires the GPG key used by debsign to be registered with the
# Launchpad account that owns the PPA (see packaging/distros/README.md).
#
# Usage: upload.sh [changes files ...]
#
#   no args   upload every *_source.changes under target/distros/ppa/
#
# Env: PPA (default ppa:fiorix/chan)
#      DPUT_ATTEMPTS      per-series dput attempts (default 3)
#      DPUT_BACKOFF_SECS  sleep between attempts (default 30)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$SCRIPT_DIR/../../.." && pwd)"
PPA="${PPA:-ppa:fiorix/chan}"
DPUT_ATTEMPTS="${DPUT_ATTEMPTS:-3}"
DPUT_BACKOFF_SECS="${DPUT_BACKOFF_SECS:-30}"

CHANGES=("$@")
if [ ${#CHANGES[@]} -eq 0 ]; then
    mapfile -t CHANGES < <(ls "$REPO"/target/distros/ppa/*/*/*_source.changes 2>/dev/null)
fi
if [ ${#CHANGES[@]} -eq 0 ]; then
    echo "error: no *_source.changes under target/distros/ppa/; run 'make ppa-source' first" >&2
    exit 1
fi

# (pkg, series, version) from the changes path and name:
# .../ppa/<pkg>/<series>/<pkg>_<version>_source.changes, where
# <version> is the full source version (e.g. 0.67.3-1~noble1).
identify() { # identify <changes-path> -> "pkg series version"
    local base series pkg version
    base="$(basename "$1")"
    series="$(basename "$(dirname "$1")")"
    pkg="${base%%_*}"
    version="${base#"${pkg}"_}"
    version="${version%_source.changes}"
    echo "$pkg $series $version"
}

FAILED=()
for changes in "${CHANGES[@]}"; do
    read -r pkg series version <<< "$(identify "$changes")"
    if "$SCRIPT_DIR/lp-accepted.sh" "$pkg" "$version" "$series"; then
        echo "SKIP $pkg/$series: already accepted ($version)"
        continue
    fi
    uploaded=""
    for attempt in $(seq 1 "$DPUT_ATTEMPTS"); do
        echo "==> dput $PPA $changes (attempt $attempt/$DPUT_ATTEMPTS)"
        # A retry forces past dput's own already-uploaded log: whether
        # Launchpad really has the upload is decided by the acceptance
        # check, not by a local marker a failed attempt may have left.
        force_arg=()
        [ "$attempt" -gt 1 ] && force_arg=(-f)
        if dput "${force_arg[@]}" "$PPA" "$changes"; then
            uploaded=1
            break
        fi
        if "$SCRIPT_DIR/lp-accepted.sh" "$pkg" "$version" "$series"; then
            echo "==> $pkg/$series: accepted server-side despite the client error"
            uploaded=1
            break
        fi
        [ "$attempt" -lt "$DPUT_ATTEMPTS" ] && sleep "$DPUT_BACKOFF_SECS"
    done
    if [ -z "$uploaded" ]; then
        FAILED+=("$pkg/$series")
    fi
done

if [ ${#FAILED[@]} -gt 0 ]; then
    echo "error: upload failed for: ${FAILED[*]}" >&2
    exit 1
fi
echo "==> all series uploaded or already accepted"
