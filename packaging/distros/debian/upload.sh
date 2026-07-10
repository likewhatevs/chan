#!/usr/bin/env bash
#
# dput the source packages built by build-source.sh to the Launchpad PPA.
# Requires the GPG key used by debsign to be registered with the Launchpad
# account that owns the PPA (see packaging/distros/README.md).
#
# Usage: upload.sh [changes files ...]
#
#   no args   upload every *_source.changes under target/distros/ppa/
#
# Env: PPA (default ppa:fiorix/chan)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$SCRIPT_DIR/../../.." && pwd)"
PPA="${PPA:-ppa:fiorix/chan}"

CHANGES=("$@")
if [ ${#CHANGES[@]} -eq 0 ]; then
    mapfile -t CHANGES < <(ls "$REPO"/target/distros/ppa/*/*/*_source.changes 2>/dev/null)
fi
if [ ${#CHANGES[@]} -eq 0 ]; then
    echo "error: no *_source.changes under target/distros/ppa/; run 'make ppa-source' first" >&2
    exit 1
fi

for changes in "${CHANGES[@]}"; do
    echo "==> dput $PPA $changes"
    dput "$PPA" "$changes"
done
