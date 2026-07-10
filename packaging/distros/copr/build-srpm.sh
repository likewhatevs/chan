#!/usr/bin/env bash
#
# Build the chan / chan-desktop SRPMs locally, mirroring what COPR's
# .copr/Makefile does in its SRPM chroot, and optionally submit them with
# copr-cli. The host only needs docker (rpmbuild and the Fedora toolchain
# run inside a fedora:latest container); artifacts land in
# target/distros/srpm/ on the host through the bind mount.
#
# Usage: build-srpm.sh [chan|chan-desktop ...] [--submit]
#
#   no package args   build both packages
#   --submit          after building, `copr-cli build <project> <srpm>`
#                     (needs ~/.config/copr; see packaging/distros/README.md)
#
# Env: COPR_PROJECT (default fiorix/chan), FEDORA_IMAGE
#      (default registry.fedoraproject.org/fedora:latest).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$SCRIPT_DIR/../../.." && pwd)"
COPR_PROJECT="${COPR_PROJECT:-fiorix/chan}"
FEDORA_IMAGE="${FEDORA_IMAGE:-registry.fedoraproject.org/fedora:latest}"

PKGS=()
SUBMIT=0
while [ $# -gt 0 ]; do
    case "$1" in
        --submit) SUBMIT=1; shift ;;
        chan|chan-desktop) PKGS+=("$1"); shift ;;
        *) echo "error: unknown argument: $1" >&2; exit 1 ;;
    esac
done
[ ${#PKGS[@]} -gt 0 ] || PKGS=(chan chan-desktop)

OUTDIR="$REPO/target/distros/srpm"
mkdir -p "$OUTDIR"

for pkg in "${PKGS[@]}"; do
    echo "==> building SRPM: $pkg"
    # The container runs as root: mark the bind-mounted repo safe for git,
    # and chown the outputs back to the invoking user at the end.
    docker run --rm -v "$REPO:/src" "$FEDORA_IMAGE" bash -ec "
        dnf -y -q install git-core make tar xz rust cargo nodejs npm rpm-build
        dnf -y -q install cargo-vendor-filterer || true
        git config --global --add safe.directory /src
        /src/packaging/distros/copr/make-srpm.sh --repo /src \
            --spec /src/packaging/distros/fedora/$pkg.spec \
            --outdir /src/target/distros/srpm
        chown -R $(id -u):$(id -g) /src/target/distros
    "
done

ls -1 "$OUTDIR"/*.src.rpm

if [ "$SUBMIT" = 1 ]; then
    command -v copr-cli >/dev/null || {
        echo "error: copr-cli not installed (pip install copr-cli)" >&2
        exit 1
    }
    for pkg in "${PKGS[@]}"; do
        srpm="$(ls -t "$OUTDIR/$pkg"-[0-9]*.src.rpm 2>/dev/null | head -1)"
        [ -n "$srpm" ] || { echo "error: no SRPM for $pkg" >&2; exit 1; }
        echo "==> submitting $srpm to $COPR_PROJECT"
        copr-cli build "$COPR_PROJECT" "$srpm"
    done
fi
