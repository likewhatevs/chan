#!/usr/bin/env bash
# Drive the shared clean-container AUR build for one package on a CI runner.
# The caller names the Arch-family image; the package architecture comes from
# the runner the job landed on, never QEMU.
#
# Usage: build-in-ci.sh <image>
#
# Reads RELEASE_TAG (GA vX.Y.Z), AUR_PKGREL, and PKGBASE from the environment.

set -euo pipefail

image="${1:?usage: build-in-ci.sh <image>}"
release_tag="${RELEASE_TAG:?RELEASE_TAG must name the GA tag}"
pkgrel="${AUR_PKGREL:-1}"
pkgbase="${PKGBASE:?PKGBASE must be set}"
repo="${GITHUB_WORKSPACE:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)}"

case "$pkgbase" in
    chan|chan-desktop) ;;
    *) echo "::error::PKGBASE must be chan or chan-desktop, got $pkgbase"; exit 1 ;;
esac
if [[ ! "$release_tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "::error::the AUR jobs need a GA vX.Y.Z tag, got $release_tag"
    exit 1
fi
if [[ ! "$pkgrel" =~ ^[1-9][0-9]*$ ]]; then
    echo "::error::aur_pkgrel must be a positive integer, got $pkgrel"
    exit 1
fi

out="$repo/target/aur-ci-out"
mkdir -p "$out"
# HOST_UID/HOST_GID hand the bind-mounted output back to the runner user, so
# the workspace stays cleanable after the container's builder writes into it.
docker run --rm \
    -e SRC=/src -e OUT=/out -e VERSION="${release_tag#v}" \
    -e PKGREL="$pkgrel" -e PKGBASE="$pkgbase" \
    -e HOST_UID="$(id -u)" -e HOST_GID="$(id -g)" \
    -v "$repo:/src:ro" \
    -v "$out:/out" \
    "$image" bash /src/packaging/distros/arch/build-in-container.sh
