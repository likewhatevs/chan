#!/usr/bin/env bash
#
# Finder-less Chan.app installer DMG builder.
#
# tauri-bundler's DMG step lays out the Finder window by mounting the volume
# and driving Finder over AppleScript (osascript). That needs a live GUI
# session, so a headless CI runner silently no-ops it and ships a flat,
# default-layout DMG, while a local build looks right. This script instead
# drives `dmgbuild`, which writes the .DS_Store layout PROGRAMMATICALLY (pure
# Python via the ds_store lib, then `hdiutil` to make the image) with no Finder
# at all, so local == CI byte-for-byte on layout.
#
# dmgbuild is a BUILD-time-only dependency (like the Node web build): installed
# into a throwaway venv, never shipped, so the single-binary runtime principle
# is untouched. The layout itself is pinned by packaging/desktop/dmg_settings.py, so any
# 1.x dmgbuild produces the same layout.
#
# Usage: build-dmg.sh <Chan.app> <out.dmg> [volume-name]
# Env:   DMGBUILD_SPEC (pip spec, default "dmgbuild>=1.6,<2")
#        DMG_VENV      (venv dir, default <repo>/target/dmg-venv)

set -euo pipefail

APP="${1:?usage: build-dmg.sh <Chan.app> <out.dmg> [volume-name]}"
OUT="${2:?usage: build-dmg.sh <Chan.app> <out.dmg> [volume-name]}"
VOLNAME="${3:-Chan}"

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
settings="$here/dmg_settings.py"
spec="${DMGBUILD_SPEC:-dmgbuild>=1.6,<2}"
# Default venv under the repo's target/ (gitignored, reused across runs).
venv="${DMG_VENV:-$(cd "$here/../.." && pwd)/target/dmg-venv}"

if [ ! -d "$APP" ]; then
    echo "error: app bundle not found: $APP" >&2
    exit 1
fi

# Hermetic, version-pinned dmgbuild in a venv: sidesteps PEP 668 on a
# system/Homebrew python3 and keeps the tool out of the global environment.
# Pure-Python wheels (dmgbuild + ds_store + mac_alias), no native compilation.
if [ ! -x "$venv/bin/dmgbuild" ]; then
    python3 -m venv "$venv"
    "$venv/bin/pip" install --quiet --disable-pip-version-check "$spec"
fi

mkdir -p "$(dirname "$OUT")"
rm -f "$OUT"
"$venv/bin/dmgbuild" -s "$settings" -D app="$APP" "$VOLNAME" "$OUT"
echo "built DMG (Finder-less): $OUT"
