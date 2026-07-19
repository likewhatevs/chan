# chan on Arch Linux (AUR)

Beginning with v0.72.0, two source-built packages are published to the Arch User Repository:

- [`chan`](https://aur.archlinux.org/packages/chan) installs the standalone CLI and devserver.
- [`chan-desktop`](https://aur.archlinux.org/packages/chan-desktop) installs the native desktop and provides the same `chan` and `cs` command surface.

They conflict because `chan-desktop` contains the full CLI in-process. Install one:

```bash
paru -S chan
paru -S chan-desktop
```

Without an AUR helper:

```bash
git clone https://aur.archlinux.org/chan.git
cd chan
makepkg -si
```

The recipes declare x86_64 and aarch64. Upstream Arch and CachyOS use the x86_64 path; aarch64 is built and tested on Arch Linux ARM. CachyOS can consume the AUR recipes directly, but inclusion in CachyOS's precompiled package repositories is a separate request to that project.

## Maintainer flow

The AUR is a collection of git repositories containing build recipes; it does not build or validate packages. Pushing `PKGBUILD` and `.SRCINFO` publishes immediately. The checked-in files here are versionless templates so the repository cannot strand an old release number.

- `aur/{chan,chan-desktop}/PKGBUILD.in` are the source package templates.
- `make-aur-package.sh` resolves a GA tag or a local test archive, renders `PKGBUILD`, generates `.SRCINFO`, and validates the metadata. It never pushes.
- `build-in-container.sh` is the shared clean-Arch build/install/smoke path used by CI and sdme.
- `build-with-sdme.sh` archives a committed local revision and runs both packages in a disposable sdme container.

Import an Arch rootfs once, then run the local gate:

```bash
sudo sdme fs import archlinux docker.io/archlinux/archlinux:base
make aur-check SDME='sudo sdme' AUR_ROOTFS=archlinux
```

`AUR_REV` defaults to `HEAD` and may name another committed revision. The
wrapper archives that revision rather than the working tree, so commit the
packaging changes in a worktree before using this as the final local gate.

On an aarch64 host, import the official Arch Linux ARM aarch64 rootfs as `archlinux` before running the same target. The current architecture comes from the host/rootfs; the build never uses QEMU.

The automatic path lives in `.github/workflows/distros-publish.yml`. After a successful GA Release run it builds `chan` and `chan-desktop` natively on x86_64 and aarch64, then pushes both AUR repositories. It needs `AUR_SSH_PRIVATE_KEY`, the same private key already registered with the maintainer's AUR account for `sdme`. Without the secret, validation still runs and publication is skipped.

The workflow's manual dispatch accepts the existing GA tag plus `aur_pkgrel`. Keep `pkgrel=1` for a normal release; raise it when repairing packaging for an already-published upstream version.

Only `PKGBUILD` and `.SRCINFO` belong in the AUR repositories. The generated source archive and `.pkg.tar.zst` files under `target/aur-*` are local artifacts.

## Package behavior

- Both recipes build from the GA tag and use locked npm and Cargo dependencies.
- `CHAN_PACKAGED=aur` disables `chan` self-upgrade so package ownership remains with the AUR helper.
- `chan-desktop` links to the host WebKitGTK/Mesa stack. This is the correct Arch/CachyOS path and avoids the rolling-distro incompatibility of the Ubuntu-built AppImage.
- The AUR Ed25519 host key is pinned to the fingerprint published by the AUR. Do not replace it with `ssh-keyscan`.
