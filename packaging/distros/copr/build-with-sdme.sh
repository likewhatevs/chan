#!/usr/bin/env bash
# Build the supported CentOS COPR matrix in disposable sdme containers.
#
# Env: SDME (how sdme is reached), DOCKER, PKG, COPR_RELEASE, REUSE_SRPM,
#      COPR_EL9_ROOTFS, COPR_EL10_ROOTFS, KEEP_CONTAINER, OUT.
#
# Every target runs even when an earlier one fails; the exit status is
# non-zero if any target failed.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$SCRIPT_DIR/../../.." && pwd)"
SDME="${SDME:-sudo sdme}"
COPR_EL9_ROOTFS="${COPR_EL9_ROOTFS:-centos-stream-9}"
COPR_EL10_ROOTFS="${COPR_EL10_ROOTFS:-centos-stream-10}"
COPR_RELEASE="${COPR_RELEASE:-all}"
PKG="${PKG:-all}"
OUT="${OUT:-$REPO/target/distros/copr-check}"
KEEP_CONTAINER="${KEEP_CONTAINER:-0}"
REUSE_SRPM="${REUSE_SRPM:-0}"
HOST_ARCH="$(uname -m)"
CONTAINER_ARCH="${HOST_ARCH//_/-}"
HOST_UID="$(id -u)"
HOST_GID="$(id -g)"
CONTAINERS=()
RESULTS=()
FAILED=0

read -r -a SDME_CMD <<<"$SDME"
[ ${#SDME_CMD[@]} -gt 0 ] || {
    echo "error: SDME must name the sdme command" >&2
    exit 1
}
command -v "${SDME_CMD[0]}" >/dev/null || {
    echo "error: '${SDME_CMD[0]}' not found on this host" >&2
    echo "hint: SDME names how sdme is reached: SDME='sudo sdme' on a Linux host, SDME='limactl shell default sudo sdme' on macOS" >&2
    exit 1
}

case "$COPR_RELEASE" in
    all) releases=(9 10) ;;
    9|10) releases=("$COPR_RELEASE") ;;
    *) echo "error: COPR_RELEASE must be all, 9, or 10" >&2; exit 1 ;;
esac
case "$PKG" in
    all|chan|chan-desktop) ;;
    *) echo "error: PKG must be all, chan, or chan-desktop" >&2; exit 1 ;;
esac
if [ "$COPR_RELEASE" = 9 ] && [ "$PKG" = chan-desktop ]; then
    echo "error: chan-desktop is unsupported on EPEL Next 9 (WebKitGTK 4.1 and libsoup3 are unavailable)" >&2
    exit 1
fi

# Bash before 4.4 treats an empty array expansion as unset under `set -u`, so
# guard the length before expanding.
cleanup() {
    [ "$KEEP_CONTAINER" = 1 ] && return
    [ ${#CONTAINERS[@]} -gt 0 ] || return
    for container in "${CONTAINERS[@]}"; do
        "${SDME_CMD[@]}" rm -f "$container" >/dev/null 2>&1 || true
    done
}
trap cleanup EXIT INT TERM

MATRIX=()
need_desktop=0
for release in "${releases[@]}"; do
    case "$PKG" in
        all)
            MATRIX+=("$release:chan")
            if [ "$release" = 10 ]; then
                MATRIX+=("$release:chan-desktop")
                need_desktop=1
            fi
            ;;
        chan) MATRIX+=("$release:chan") ;;
        chan-desktop)
            if [ "$release" = 10 ]; then
                MATRIX+=("$release:chan-desktop")
                need_desktop=1
            fi
            ;;
    esac
done

FS_LIST="$("${SDME_CMD[@]}" fs ls)"
require_rootfs() {
    local rootfs="$1"
    local release="$2"
    if ! awk -v name="$rootfs" '$1 == name { found = 1 } END { exit !found }' <<<"$FS_LIST"; then
        echo "error: sdme rootfs '$rootfs' is not imported" >&2
        echo "hint: import it as ${SDME_CMD[*]} fs import $rootfs quay.io/centos/centos:stream${release} --install-packages=yes -v" >&2
        echo "hint: or set COPR_EL${release}_ROOTFS to one of the entries this host already has:" >&2
        echo "$FS_LIST" >&2
        exit 1
    fi
}

for target in "${MATRIX[@]}"; do
    release="${target%%:*}"
    package="${target#*:}"
    if [ "$release" = 9 ]; then
        rootfs="$COPR_EL9_ROOTFS"
    else
        rootfs="$COPR_EL10_ROOTFS"
    fi
    require_rootfs "$rootfs" "$release"
done

srpm_packages=(chan)
[ "$need_desktop" = 1 ] && srpm_packages+=(chan-desktop)
if [ "$PKG" = chan-desktop ]; then
    srpm_packages=(chan-desktop)
fi

echo ">> preparing vendored SRPMs: ${srpm_packages[*]}" >&2
if [ "$REUSE_SRPM" = 1 ]; then
    for package in "${srpm_packages[@]}"; do
        find "$REPO/target/distros/srpm" -maxdepth 1 -type f \
            -name "$package-[0-9]*.src.rpm" -print -quit | grep -q . || {
            echo "error: REUSE_SRPM=1 but no $package SRPM exists" >&2
            exit 1
        }
    done
    echo ">> reusing existing SRPMs" >&2
else
    "$SCRIPT_DIR/build-srpm.sh" "${srpm_packages[@]}"
fi

# `sdme new` deletes the container when its guest command exits non-zero, which
# would destroy exactly the container an operator wants to inspect. The guest
# wrapper always exits 0 and carries the real status out on the writable /out
# bind, so a failed target leaves its container alive for KEEP_CONTAINER=1.
GUEST_RUN='status=0
/bin/bash /src/packaging/distros/copr/build-in-container.sh || status=$?
printf "%s\n" "$status" >/out/status
chown "$HOST_UID:$HOST_GID" /out/status 2>/dev/null || true
exit 0'

for target in "${MATRIX[@]}"; do
    release="${target%%:*}"
    package="${target#*:}"
    if [ "$release" = 9 ]; then
        rootfs="$COPR_EL9_ROOTFS"
    else
        rootfs="$COPR_EL10_ROOTFS"
    fi
    result_dir="$OUT/el${release}/${HOST_ARCH}/${package}"
    mkdir -p "$result_dir"
    status_file="$result_dir/status"
    rm -f "$status_file"
    container="chan-copr-el${release}-${package}-${CONTAINER_ARCH}-$$"
    CONTAINERS+=("$container")

    echo ">> COPR validation: el${release} package=${package} rootfs=${rootfs} arch=${HOST_ARCH}" >&2
    "${SDME_CMD[@]}" rm -f "$container" >/dev/null 2>&1 || true
    sdme_status=0
    "${SDME_CMD[@]}" new "$container" -r "$rootfs" -t 180 \
        -b "$REPO:/src:ro" \
        -b "$REPO/target/distros/srpm:/srpm:ro" \
        -b "$result_dir:/out" \
        -- /usr/bin/env PKG="$package" EL_RELEASE="$release" \
        HOST_UID="$HOST_UID" HOST_GID="$HOST_GID" \
        /bin/bash -c "$GUEST_RUN" \
        2>&1 | tee "$result_dir/build.log" || sdme_status=$?

    if [ "$sdme_status" -ne 0 ]; then
        # The container never reached the guest wrapper.
        status="$sdme_status"
    elif [ -r "$status_file" ]; then
        status="$(cat "$status_file")"
    else
        status=1
    fi

    if [ "$status" = 0 ]; then
        RESULTS+=("PASS el${release} ${package} ${HOST_ARCH}")
    else
        RESULTS+=("FAIL el${release} ${package} ${HOST_ARCH} (status $status, log $result_dir/build.log)")
        FAILED=1
    fi

    # Each container overlay holds a full offline Rust release build, so drop it
    # before the next target starts rather than at the end of the matrix.
    if [ "$KEEP_CONTAINER" = 1 ]; then
        echo ">> keeping container $container for diagnosis" >&2
    else
        "${SDME_CMD[@]}" rm -f "$container" >/dev/null 2>&1 || true
    fi
done

echo ">> COPR sdme validation results:" >&2
for result in "${RESULTS[@]}"; do
    echo "   $result" >&2
done
echo ">> artifacts: $OUT" >&2
exit "$FAILED"
