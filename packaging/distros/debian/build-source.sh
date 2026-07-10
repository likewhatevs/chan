#!/usr/bin/env bash
#
# Build the signed per-series Ubuntu source packages for Launchpad from the
# vendored orig tarball (packaging/distros/mkdist). Launchpad PPAs take
# source-only uploads and build in offline chroots, so everything the build
# needs is inside the orig; each series gets its own version suffix
# (X.Y.Z-1~<series>1) while the orig stays byte-identical across series, as
# Launchpad requires for one upstream version.
#
# Usage: build-source.sh [chan|chan-desktop ...]
#
#   no package args   build both source packages
#
# Env:
#   PPA_SERIES   space-separated series (default: "noble <host series>")
#   SERIESREV    per-series revision suffix number (default: 1; bump for a
#                packaging-only re-upload of the same upstream version)
#   DEBSIGN_KEY  GPG key id for debsign (default: debsign's default key)
#   PPA_NOSIGN   set to 1 to skip signing (-us -uc), e.g. for container
#                build tests
#
# The host needs devscripts + dpkg-dev, not the rust toolchain: -d skips
# local build-dep checks (the series chroot on Launchpad resolves them).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$SCRIPT_DIR/../../.." && pwd)"

HOST_SERIES="$(. /etc/os-release && echo "${VERSION_CODENAME:-}")"
PPA_SERIES="${PPA_SERIES:-noble $HOST_SERIES}"
SERIESREV="${SERIESREV:-1}"

PKGS=("$@")
[ ${#PKGS[@]} -gt 0 ] || PKGS=(chan chan-desktop)

# Same derivation as mkdist: version of what HEAD would package.
VERSION="$(git -C "$REPO" show HEAD:Cargo.toml \
    | sed -n 's/^version = "\(.*\)"/\1/p' | head -1)"
DEBVER="$(printf '%s' "$VERSION" | tr -- '-' '~')"
DATE_RFC="$(date -R)"

SIGN_ARGS=()
if [ "${PPA_NOSIGN:-0}" = 1 ]; then
    SIGN_ARGS+=(-us -uc)
elif [ -n "${DEBSIGN_KEY:-}" ]; then
    SIGN_ARGS+=("-k$DEBSIGN_KEY")
fi

for pkg in "${PKGS[@]}"; do
    ORIG="$REPO/target/distros/${pkg}_$DEBVER.orig.tar.xz"
    if [ ! -f "$ORIG" ]; then
        echo "error: $ORIG not found; run 'make distros-tarball' first" >&2
        exit 1
    fi
    # dedupe series (host series may already be noble)
    for series in $(printf '%s\n' $PPA_SERIES | awk '!seen[$0]++'); do
        work="$REPO/target/distros/ppa/$pkg/$series"
        echo "==> source package: $pkg $DEBVER $series"
        rm -rf "$work"
        mkdir -p "$work"
        ln "$ORIG" "$work/" 2>/dev/null || cp "$ORIG" "$work/"
        tar -C "$work" -xf "$ORIG"
        src="$work/chan-$VERSION"
        # The debian dir comes from the repo checkout driving this build
        # (not from inside the orig), rendered per series; check out the
        # release tag to build exactly what it shipped.
        cp -a "$REPO/packaging/distros/debian/$pkg/debian" "$src/debian"
        sed -e "s/@VERSION@/$DEBVER/g" \
            -e "s/@SERIES@/$series/g" \
            -e "s/@SERIESREV@/$SERIESREV/g" \
            -e "s/@DATE@/$DATE_RFC/g" \
            "$src/debian/changelog.in" > "$src/debian/changelog"
        rm "$src/debian/changelog.in"
        (cd "$src" && debuild -S -sa -d "${SIGN_ARGS[@]}")
    done
done

echo "==> built source packages:"
ls -1 "$REPO"/target/distros/ppa/*/*/*_source.changes
