#!/usr/bin/env bash
#
# Assemble the SRPM for one package (chan or chan-desktop) from the current
# repo: run mkdist to produce the vendored source tarball, sync the spec's
# %upstream_version to the tarball version, and rpmbuild -bs. Expects a
# Fedora-ish host with rpmbuild, cargo/rust, node/npm, git, tar, xz -- the
# COPR SRPM chroot after .copr/Makefile's dnf install, or the container
# copr/build-srpm.sh spins up locally.
#
# Usage: make-srpm.sh [--spec <path>] [--outdir <dir>] [--repo <path>]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$SCRIPT_DIR/../../.." && pwd)"
SPEC=""
OUTDIR=""

while [ $# -gt 0 ]; do
    case "$1" in
        --spec) SPEC="$2"; shift 2 ;;
        --outdir) OUTDIR="$2"; shift 2 ;;
        --repo) REPO="$(cd "$2" && pwd)"; shift 2 ;;
        *) echo "error: unknown argument: $1" >&2; exit 1 ;;
    esac
done

SPEC="${SPEC:-$REPO/packaging/distros/fedora/chan.spec}"
OUTDIR="${OUTDIR:-$REPO/target/distros/srpm}"
SOURCEDIR="$REPO/target/distros"

TARBALL="$("$REPO/packaging/distros/mkdist" --repo "$REPO" --outdir "$SOURCEDIR" \
    | head -1)"
# chan-vendored-<version>.tar.xz -> <version>
VERSION="$(basename "$TARBALL" .tar.xz)"
VERSION="${VERSION#chan-vendored-}"

# The committed spec's %upstream_version is a fallback; the tarball just
# built is the truth. Rewrite a copy next to the sources.
SPEC_COPY="$SOURCEDIR/$(basename "$SPEC")"
sed "s/^%global upstream_version .*/%global upstream_version $VERSION/" \
    "$SPEC" > "$SPEC_COPY"

mkdir -p "$OUTDIR"
rpmbuild -bs "$SPEC_COPY" \
    --define "_sourcedir $SOURCEDIR" \
    --define "_srcrpmdir $OUTDIR"
