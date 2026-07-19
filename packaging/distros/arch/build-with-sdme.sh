#!/usr/bin/env bash
# Build the AUR packages from a committed local revision in a disposable sdme
# Arch container. The rootfs architecture determines the package architecture.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$SCRIPT_DIR/../../.." && pwd)"
REV="${REV:-HEAD}"
SDME="${SDME:-sudo sdme}"
# A plain upstream Arch base rootfs, named like the other base imports the
# packaging paths use (ubuntu, centos-stream-*). The pre-provisioned desktop
# build rootfs is deliberately not reused: its baked dependencies would hide
# missing PKGBUILD declarations.
AUR_ROOTFS="${AUR_ROOTFS:-archlinux}"
OUT="${OUT:-$REPO/target/aur-out}"
SOURCE_DIR="$REPO/target/aur-source"
CONTAINER="chan-aur-build-$(uname -m)-$$"

# SDME carries the transport too (a lima VM on macOS, sudo on a Linux host),
# so parse it into an array once instead of relying on word splitting.
read -r -a SDME_CMD <<<"$SDME"
[ ${#SDME_CMD[@]} -gt 0 ] || {
    echo "error: SDME must name the sdme command" >&2
    exit 1
}

git -C "$REPO" rev-parse --verify --quiet "$REV^{commit}" >/dev/null || {
    echo "error: $REV does not name a commit" >&2
    exit 1
}
version="$(git -C "$REPO" show "$REV:Cargo.toml" | sed -n 's/^version = "\(.*\)"/\1/p' | head -1)"
[ -n "$version" ] || { echo "error: cannot derive version from $REV" >&2; exit 1; }

if ! "${SDME_CMD[@]}" fs ls | awk -v name="$AUR_ROOTFS" \
    '$1 == name { found = 1 } END { exit !found }'; then
    echo "error: sdme rootfs '$AUR_ROOTFS' is not imported" >&2
    echo "hint: sudo sdme fs import $AUR_ROOTFS docker.io/archlinux/archlinux:base" >&2
    exit 1
fi

mkdir -p "$SOURCE_DIR" "$OUT"
source_archive="$SOURCE_DIR/chan-$version.tar.gz"
git -C "$REPO" archive --format=tar.gz --prefix="chan-$version/" -o "$source_archive" "$REV"

cleanup() {
    "${SDME_CMD[@]}" rm -f "$CONTAINER" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

echo ">> running AUR checks in sdme rootfs '$AUR_ROOTFS'" >&2
# Pass the build environment to the joined command itself. sdme's `--env`
# configures the container service, but the auto-join command does not inherit
# those values.
"${SDME_CMD[@]}" new "$CONTAINER" -r "$AUR_ROOTFS" -t 120 \
    -b "$REPO:/src:ro" -b "$SOURCE_DIR:/local:ro" -b "$OUT:/out" \
    -- env SRC=/src OUT=/out VERSION="$version" \
    HOST_UID="$(id -u)" HOST_GID="$(id -g)" \
    AUR_LOCAL_SOURCE="/local/chan-$version.tar.gz" \
    bash /src/packaging/distros/arch/build-in-container.sh
