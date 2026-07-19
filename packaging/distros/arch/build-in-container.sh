#!/usr/bin/env bash
# Render, build, install, and smoke AUR packages inside a clean Arch-family
# container. CI and build-with-sdme.sh deliberately share this implementation.

set -euo pipefail

SRC="${SRC:-/src}"
OUT="${OUT:-/out}"
VERSION="${VERSION:-}"
PKGREL="${PKGREL:-1}"
PKGBASE="${PKGBASE:-all}"

case "$PKGBASE" in
    all) packages=(chan chan-desktop) ;;
    chan|chan-desktop) packages=("$PKGBASE") ;;
    *) echo "error: PKGBASE must be all, chan, or chan-desktop" >&2; exit 1 ;;
esac

if [ "$(id -u)" -eq 0 ]; then
    echo ">> refreshing the package database and base build tools" >&2
    if pacman -Q archlinuxarm-keyring >/dev/null 2>&1; then
        pacman-key --init
        pacman-key --populate archlinuxarm
        pacman -Syu --needed --noconfirm base-devel curl namcap sudo
    else
        pacman -Sy --needed --noconfirm archlinux-keyring
        pacman -Su --needed --noconfirm base-devel curl namcap sudo
    fi

    id builder >/dev/null 2>&1 || useradd -m builder
    install -d -m 0755 /etc/sudoers.d
    printf 'builder ALL=(ALL:ALL) NOPASSWD: /usr/bin/pacman\n' > /etc/sudoers.d/aur-builder
    chmod 0440 /etc/sudoers.d/aur-builder
    mkdir -p "$OUT"
    chown -R builder:builder "$OUT"

    exec runuser -u builder -- env -u SUDO_USER -u SUDO_UID -u SUDO_GID -u SUDO_COMMAND \
        HOME=/home/builder USER=builder LOGNAME=builder SRC="$SRC" OUT="$OUT" \
        VERSION="$VERSION" PKGREL="$PKGREL" PKGBASE="$PKGBASE" \
        AUR_LOCAL_SOURCE="${AUR_LOCAL_SOURCE:-}" bash "$0"
fi

export HOME="${HOME:-/home/builder}"
if [ -z "$VERSION" ]; then
    VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$SRC/Cargo.toml" | head -1)"
fi

echo ">> AUR validation: version=$VERSION pkgrel=$PKGREL arch=$(uname -m)" >&2
for pkgbase in "${packages[@]}"; do
    echo ">> building $pkgbase" >&2
    AUR_LOCAL_SOURCE="${AUR_LOCAL_SOURCE:-}" \
        bash "$SRC/packaging/distros/arch/make-aur-package.sh" \
        "$pkgbase" "$VERSION" "$PKGREL" "$OUT"

    pkgdir="$OUT/$pkgbase"
    (cd "$pkgdir" && makepkg --cleanbuild --force --syncdeps --noconfirm)
    # makepkg may also emit a $pkgbase-debug package. Select the main package
    # by its exact versioned prefix instead of relying on find's ordering.
    pkg="$(find "$pkgdir" -maxdepth 1 \
        -name "$pkgbase-$VERSION-$PKGREL-*.pkg.tar.zst" -type f | head -1)"
    [ -n "$pkg" ] || { echo "error: $pkgbase produced no package" >&2; exit 1; }

    namcap "$pkgdir/PKGBUILD" "$pkg" || true
    sudo pacman -U --noconfirm "$pkg"
    chan --version
    cs --help >/dev/null
    if chan upgrade >"$pkgdir/upgrade.out" 2>&1; then
        echo "error: packaged chan upgrade unexpectedly succeeded" >&2
        exit 1
    fi
    grep -q 'AUR helper' "$pkgdir/upgrade.out"
    systemd-analyze verify /usr/lib/systemd/user/chan-devserver.service

    if [ "$pkgbase" = chan-desktop ]; then
        desktop-file-validate /usr/share/applications/chan-desktop.desktop
        for size in 32x32 64x64 128x128 256x256 512x512; do
            test -f "/usr/share/icons/hicolor/$size/apps/chan-desktop.png"
        done
        ldd /usr/bin/chan-desktop > "$pkgdir/ldd.out"
        ! grep -q 'not found' "$pkgdir/ldd.out"
    fi

    sudo pacman -Rdd --noconfirm "$pkgbase"
    echo ">> built and smoked $(basename "$pkg")" >&2
done
