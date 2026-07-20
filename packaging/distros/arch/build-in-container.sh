#!/usr/bin/env bash
# Render, build, install, and smoke AUR packages inside a clean Arch-family
# container. CI and build-with-sdme.sh deliberately share this implementation.

set -euo pipefail

SRC="${SRC:-/src}"
OUT="${OUT:-/out}"
VERSION="${VERSION:-}"
PKGREL="${PKGREL:-1}"
PKGBASE="${PKGBASE:-all}"
HOST_UID="${HOST_UID:-0}"
HOST_GID="${HOST_GID:-0}"

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

    status=0
    runuser -u builder -- env -u SUDO_USER -u SUDO_UID -u SUDO_GID -u SUDO_COMMAND \
        HOME=/home/builder USER=builder LOGNAME=builder SRC="$SRC" OUT="$OUT" \
        VERSION="$VERSION" PKGREL="$PKGREL" PKGBASE="$PKGBASE" \
        AUR_LOCAL_SOURCE="${AUR_LOCAL_SOURCE:-}" bash "$0" || status=$?

    # $OUT is bind-mounted from the host, so hand the artifacts back to the
    # invoking user, on failure too: left owned by the in-container builder
    # they would block a later non-root `rm -rf target` or `cargo clean`.
    chown -R "$HOST_UID:$HOST_GID" "$OUT"
    exit "$status"
fi

export HOME="${HOME:-/home/builder}"
if [ -z "$VERSION" ]; then
    VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$SRC/Cargo.toml" | head -1)"
fi

# namcap findings gate the build. The recipes carry a hand-written runtime
# dependency set, so a library that namcap detects but the PKGBUILD does not
# declare (namcap's `E:` class) would otherwise ship silently to every AUR
# user. Warnings stay advisory and are printed in full.
#
# A waiver is one line here: an extended regex matched against a whole namcap
# line, plus the reason the finding cannot be fixed in the recipe. Shape:
#     'E: Dependency detected and not included .libfoo.' # dlopened at runtime
# Both packages' measured findings are all warnings, so neither needs one.
namcap_waivers=()

# Print the namcap error lines that no waiver covers.
unwaived_namcap_errors() {
    local log="$1"
    local line pattern waived
    while IFS= read -r line; do
        waived=0
        for pattern in ${namcap_waivers[@]+"${namcap_waivers[@]}"}; do
            if [[ "$line" =~ $pattern ]]; then
                waived=1
                break
            fi
        done
        [ "$waived" -eq 1 ] || printf '%s\n' "$line"
    done < <(grep -E ' E: ' "$log" || true)
}

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

    namcap_log="$pkgdir/namcap.out"
    set +e
    namcap "$pkgdir/PKGBUILD" "$pkg" > "$namcap_log" 2>&1
    namcap_status=$?
    set -e
    cat "$namcap_log"
    # namcap reports findings on stdout and its exit status is not a
    # documented contract, so a nonzero exit with nothing reported is namcap
    # itself failing to run.
    if [ "$namcap_status" -ne 0 ] && ! grep -qE ' (E|W): ' "$namcap_log"; then
        echo "error: namcap failed to run for $pkgbase (exit $namcap_status)" >&2
        exit 1
    fi
    namcap_errors="$(unwaived_namcap_errors "$namcap_log")"
    if [ -n "$namcap_errors" ]; then
        echo "error: namcap rejected $pkgbase:" >&2
        printf '%s\n' "$namcap_errors" >&2
        exit 1
    fi

    sudo pacman -U --noconfirm "$pkg"
    chan --version
    cs --help >/dev/null
    systemd-analyze verify /usr/lib/systemd/user/chan-devserver.service

    if [ "$pkgbase" = chan-desktop ]; then
        # The desktop personality routes `chan upgrade` to a running GUI, so a
        # headless container cannot exercise the packaged refusal. Prove the
        # CHAN_PACKAGED=aur stamp reached the build instead, at both ends: the
        # rendered recipe must export it, and the refusal hint must survive
        # into the binary. The hint is only reachable through
        # `option_env!("CHAN_PACKAGED")` being `Some`, so an unstamped release
        # build drops the literal along with the dead branch. The `chan`
        # package below covers the executable refusal path.
        grep -q 'export CHAN_PACKAGED=aur' "$pkgdir/PKGBUILD"
        grep -aFq 'your AUR helper (for example, paru -Syu or yay -Syu)' \
            /usr/bin/chan-desktop
        desktop-file-validate /usr/share/applications/chan-desktop.desktop
        for size in 32x32 64x64 128x128 256x256 512x512; do
            test -f "/usr/share/icons/hicolor/$size/apps/chan-desktop.png"
        done
        ldd_output="$(ldd /usr/bin/chan-desktop)"
        if grep -q 'not found' <<<"$ldd_output"; then
            echo "error: chan-desktop has unresolved shared libraries" >&2
            echo "$ldd_output" >&2
            exit 1
        fi
    else
        if chan upgrade >"$pkgdir/upgrade.out" 2>&1; then
            echo "error: packaged chan upgrade unexpectedly succeeded" >&2
            exit 1
        fi
        grep -q 'AUR helper' "$pkgdir/upgrade.out"
    fi

    sudo pacman -Rdd --noconfirm "$pkgbase"
    echo ">> built and smoked $(basename "$pkg")" >&2
done
