#!/usr/bin/env bash
#
# Ask Launchpad whether a source package version is already accepted
# into the PPA for a series: exit 0 iff a publication of
# <source-name> <version> exists for <series> in Pending or Published
# state; exit 1 when none does; exit 2 on usage, network, or API-shape
# errors (callers must treat 2 as "unknown", never as "accepted").
#
# upload.sh consults this before and between dput attempts so a
# retried publish run skips series Launchpad already accepted:
# Launchpad REJECTS a duplicate upload of the same source version, so
# a blind re-dput of an accepted series fails the whole run.
#
# Anonymous read-only API call; no credentials involved.
#
# Usage: lp-accepted.sh <source-name> <version> <series>
# Env:   LP_API_BASE  API origin (default https://api.launchpad.net);
#                     override points the query at a local mock
#                     (scripts/e2e/lp-skip-test.sh).
#        PPA          archive as ppa:owner/name (default ppa:fiorix/chan);
#                     owner and name derive from it.

set -uo pipefail

if [ $# -ne 3 ]; then
    echo "usage: lp-accepted.sh <source-name> <version> <series>" >&2
    exit 2
fi
PKG="$1"
VERSION="$2"
SERIES="$3"

LP_API_BASE="${LP_API_BASE:-https://api.launchpad.net}"
PPA="${PPA:-ppa:fiorix/chan}"
ppa_path="${PPA#ppa:}"
case "$ppa_path" in
*/*)
    OWNER="${ppa_path%%/*}"
    NAME="${ppa_path#*/}"
    ;;
*)
    echo "lp-accepted: PPA must look like ppa:owner/name, got $PPA" >&2
    exit 2
    ;;
esac
if [ -z "$OWNER" ] || [ -z "$NAME" ]; then
    echo "lp-accepted: PPA must look like ppa:owner/name, got $PPA" >&2
    exit 2
fi

resp="$(curl -fsS --max-time 30 -G \
    "$LP_API_BASE/1.0/~$OWNER/+archive/ubuntu/$NAME" \
    --data-urlencode "ws.op=getPublishedSources" \
    --data-urlencode "source_name=$PKG" \
    --data-urlencode "version=$VERSION" \
    --data-urlencode "exact_match=true")" || {
    echo "lp-accepted: Launchpad API request failed for $PKG $VERSION" >&2
    exit 2
}

# Defensive jq: a shape drift (entries missing, links renamed) parses
# to an empty list rather than a crash, and non-JSON exits 2 below.
matches="$(printf '%s' "$resp" | jq -r --arg series "$SERIES" '
    [.entries[]?
     | select((.status == "Pending" or .status == "Published")
              and ((.distro_series_link // "") | endswith("/" + $series)))]
    | length' 2>/dev/null)" || matches=""
if [ -z "$matches" ]; then
    echo "lp-accepted: unparseable Launchpad API response for $PKG $VERSION" >&2
    exit 2
fi
if [ "$matches" -gt 0 ] 2>/dev/null; then
    exit 0
fi
exit 1
