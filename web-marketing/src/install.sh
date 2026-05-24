#!/bin/sh
# chan CLI installer for macOS and Linux.
#
#   curl -fsSL https://chan.app/install.sh | sh
#
# Downloads the matching standalone CLI tarball from GitHub Releases
# and installs `chan` into PREFIX/bin. Defaults:
#
#   PREFIX=$HOME/.local
#   BASE=https://github.com/fiorix/chan/releases/latest/download

set -eu

BASE="${BASE:-https://github.com/fiorix/chan/releases/latest/download}"
BASE="${BASE%/}"
PREFIX="${PREFIX:-$HOME/.local}"

err() { printf 'install: %s\n' "$1" >&2; exit 1; }

os=$(uname -s)
arch=$(uname -m)

case "$os" in
    Darwin)
        case "$arch" in
            arm64|aarch64) asset="chan-aarch64-apple-darwin.tar.gz" ;;
            *) err "macOS on $arch is not published. arm64 only for now." ;;
        esac
        ;;
    Linux)
        case "$arch" in
            x86_64|amd64)  asset="chan-x86_64-unknown-linux-gnu.tar.gz" ;;
            aarch64|arm64) asset="chan-aarch64-unknown-linux-gnu.tar.gz" ;;
            *) err "Linux on $arch is not published." ;;
        esac
        ;;
    *) err "Unsupported OS: $os." ;;
esac

url="$BASE/$asset"
bindir="$PREFIX/bin"
mkdir -p "$bindir"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

printf 'install: downloading %s\n' "$url"
if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "$tmp/chan.tar.gz"
elif command -v wget >/dev/null 2>&1; then
    wget -qO "$tmp/chan.tar.gz" "$url"
else
    err "need curl or wget on PATH."
fi

tar -xzf "$tmp/chan.tar.gz" -C "$tmp"

bin=$(find "$tmp" -type f -name chan -perm -u+x | head -n1 || true)
[ -n "$bin" ] || err "binary 'chan' not found inside $asset"

install -m 0755 "$bin" "$bindir/chan"
printf 'install: %s\n' "$bindir/chan"

case ":$PATH:" in
    *":$bindir:"*) ;;
    *)
        printf 'install: note: %s is not on your PATH.\n' "$bindir" >&2
        printf '  add this to your shell rc:  export PATH="%s:$PATH"\n' "$bindir" >&2
        ;;
esac
