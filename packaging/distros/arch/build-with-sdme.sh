#!/usr/bin/env bash
# Build the AUR packages from a committed local revision in a disposable sdme
# Arch container. The rootfs architecture determines the package architecture.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$SCRIPT_DIR/../../.." && pwd)"
REV="${REV:-HEAD}"
SDME="${SDME:-sudo sdme}"
AUR_ROOTFS="${AUR_ROOTFS:-archlinux}"
OUT="${OUT:-$REPO/target/aur-out}"
SOURCE_DIR="$REPO/target/aur-source"
CONTAINER="chan-aur-build-$(uname -m)-$$"

git -C "$REPO" rev-parse --verify --quiet "$REV^{commit}" >/dev/null || {
    echo "error: $REV does not name a commit" >&2
    exit 1
}
version="$(git -C "$REPO" show "$REV:Cargo.toml" | sed -n 's/^version = "\(.*\)"/\1/p' | head -1)"
[ -n "$version" ] || { echo "error: cannot derive version from $REV" >&2; exit 1; }

mkdir -p "$SOURCE_DIR" "$OUT"
source_archive="$SOURCE_DIR/chan-$version.tar.gz"
git -C "$REPO" archive --format=tar.gz --prefix="chan-$version/" -o "$source_archive" "$REV"

cleanup() {
    $SDME rm -f "$CONTAINER" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

echo ">> running AUR checks in sdme rootfs '$AUR_ROOTFS'" >&2
# Pass the build environment to the joined command itself. sdme's `--env`
# configures the container service, but the auto-join command does not inherit
# those values.
$SDME new "$CONTAINER" -r "$AUR_ROOTFS" -t 120 \
    -b "$REPO:/src:ro" -b "$SOURCE_DIR:/local:ro" -b "$OUT:/out" \
    -- env SRC=/src OUT=/out VERSION="$version" \
    AUR_LOCAL_SOURCE="/local/chan-$version.tar.gz" \
    bash /src/packaging/distros/arch/build-in-container.sh
