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

The recipes declare x86_64 and aarch64. Only the x86_64 path is validated: it is what the container gate and the release workflow build, install, and smoke. The aarch64 leg (Arch Linux ARM rootfs import, keyring bootstrap, native ARM Tauri build) has never completed anywhere, so treat aarch64 as unverified until a green `aur-validate-arm` run says otherwise. CachyOS can consume the AUR recipes directly, but inclusion in CachyOS's precompiled package repositories is a separate request to that project.

## Maintainer flow

The AUR is a collection of git repositories containing build recipes; it does not build or validate packages. Pushing `PKGBUILD` and `.SRCINFO` publishes immediately. The checked-in files here are versionless templates so the repository cannot strand an old release number.

- `aur/{chan,chan-desktop}/PKGBUILD.in` are the source package templates.
- `make-aur-package.sh` resolves a GA tag or a local test archive, renders `PKGBUILD`, generates `.SRCINFO`, and validates the metadata. It never pushes.
- `build-in-container.sh` is the shared clean-Arch build/install/smoke path used by CI and sdme.
- `build-with-sdme.sh` archives a committed local revision and runs both packages in a disposable sdme container.
- `build-in-ci.sh` is the workflow's entry point: it validates the tag and `pkgrel`, then runs the same container build against the image the job provides, so the x86_64 and aarch64 jobs cannot drift apart.

Import an Arch rootfs once, then run the local gate:

```bash
sudo sdme fs import archlinux docker.io/archlinux/archlinux:base
make aur-check SDME='sudo sdme' AUR_ROOTFS=archlinux
```

`archlinux` is a plain upstream base rootfs, named like the other base imports the packaging paths use (`ubuntu`, `centos-stream-9`); only purpose-built rootfs images with a `.sdme` template carry a `chan-`/`gateway-` prefix. `AUR_ROOTFS` selects a different import. The pre-provisioned desktop build rootfs is deliberately not reused because its baked dependencies would hide a missing `PKGBUILD` declaration.

`AUR_REV` defaults to `HEAD` and may name another committed revision. The wrapper archives that revision rather than the working tree, so commit the packaging changes in a worktree before using this as the final local gate.

The package architecture comes from the host and rootfs; the build never uses QEMU. An aarch64 host runs the same target against an Arch Linux ARM import under the same name, which is the unverified leg described above.

The automatic path lives in `.github/workflows/distros-publish.yml`. After a successful GA Release run it builds, installs, and smokes `chan` and `chan-desktop` on a clean upstream Arch x86_64 container, probes the AUR credential, and pushes both AUR repositories. It needs `AUR_SSH_PRIVATE_KEY`, the same private key already registered with the maintainer's AUR account for `sdme`. Without the secret, validation still runs and publication is skipped.

Manual dispatch takes the existing GA tag plus:

| Input | Effect |
|---|---|
| `targets` | Runs one distro (`copr`, `launchpad`, `aur`) instead of all three, so a Launchpad retry costs one job. |
| `publish` | Defaults to false: renders, validates, and probes the credentials, then pushes nothing. A retry that must actually publish needs `publish=true`. |
| `aur_pkgrel` | Keep `1` for a normal release; raise it when repairing packaging for an already-published upstream version. |
| `aur_validate_arm` | Runs the unverified aarch64 build. It never gates publication. |

An rc tag skips the AUR jobs entirely, the way COPR and Launchpad no-op on one.

`aur-auth` is the only pre-release proof that the private key in the secret has its public half registered on the AUR account: it runs `ssh -T aur@aur.archlinux.org`, which a registered key answers with a greeting and a refused interactive shell. Without that probe a wrong key surfaces as a failed clone at publication time, after the release is already out.

`aur-validate-arm` builds the same recipes on a native aarch64 runner against the imported Arch Linux ARM rootfs. It is not in `aur-publish`'s `needs`: an unproven ARM cell must not block shipping the validated x86_64 recipes, and a red cell there is the visible signal that the ARM path is still broken.

Only `PKGBUILD` and `.SRCINFO` belong in the AUR repositories. The generated source archive and `.pkg.tar.zst` files under `target/aur-*` are local artifacts.

## Package behavior

- Both recipes build from the GA tag and use locked npm and Cargo dependencies. `RUSTUP_TOOLCHAIN=stable` keeps the tree's `rust-toolchain.toml` pin from making a rustup-provided cargo download a second toolchain mid-build.
- `CHAN_PACKAGED=aur` disables `chan` self-upgrade so package ownership remains with the AUR helper. The local `make linux-archpkg` QA package is stamped `pacman` instead, because it never came from the AUR.
- `namcap` gates the container build: its error class covers a library the binary needs and the recipe does not declare, which would otherwise ship silently to every AUR user. Warnings stay advisory and are printed. `build-in-container.sh` holds the waiver list.
- `chan-desktop` links to the host WebKitGTK/Mesa stack. This is the correct Arch/CachyOS path and avoids the rolling-distro incompatibility of the Ubuntu-built AppImage.
- The AUR Ed25519 host key is pinned to the fingerprint published by the AUR. Do not replace it with `ssh-keyscan`.
