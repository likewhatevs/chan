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

The automatic path lives in `.github/workflows/distros-publish.yml`. After a successful GA Release run it builds, installs, and smokes `chan` and `chan-desktop` on a clean upstream Arch x86_64 container, probes the AUR credential, and pushes both AUR repositories. It needs `AUR_SSH_PRIVATE_KEY`, the same private key already registered with the maintainer's AUR account for `sdme`. Without the secret, validation still runs and publication is skipped; a `publish=false` dispatch on this repository fails instead, because proving the credential is that run's only product.

Manual dispatch takes the existing GA tag plus:

| Input | Effect |
|---|---|
| `targets` | Runs one distro (`copr`, `launchpad`, `aur`) instead of all three, so a Launchpad retry costs one job. |
| `publish` | Defaults to false: renders, validates, and probes the credentials, then pushes nothing. A retry that must actually publish needs `publish=true`. |
| `aur_pkgrel` | Keep `1` for a normal release; raise it when repairing packaging for an already-published upstream version. |
| `aur_validate_arm` | Runs the unverified aarch64 build. Opt-in here only: a GA run never schedules it, and it never gates publication. |

A prerelease version skips every job in the workflow: the AUR jobs, `copr`, and `launchpad` all filter a `-` out of the tag, on both the dispatch and the workflow_run path. Release candidates are validated on a branch and their tags are not pushed, so this is defense in depth rather than a path anything is expected to take.

`aur-auth` is the only pre-release proof that the private key in the secret has its public half registered on the AUR account: it runs `ssh -T aur@aur.archlinux.org`, which a registered key answers with a greeting and a refused interactive shell. Without that probe a wrong key surfaces as a failed clone at publication time, after the release is already out.

`aur-validate-arm` builds the same recipes on a native aarch64 runner against the imported Arch Linux ARM rootfs. It runs only on a manual dispatch carrying `aur_validate_arm=true`, and it is not in `aur-publish`'s `needs`. A leg that has never completed anywhere would otherwise make two red jobs the normal end state of every GA run, which trains the operator to ignore red; instead a GA run is expected all-green and the ARM work happens in a dispatch that is watched. Once a dispatch passes, give the job the same `workflow_run` trigger the other AUR jobs carry and add it to `aur-publish`'s `needs`.

`aur-validate` is a hard `needs` of `aur-publish`, so anything that fails inside it (the `makepkg` build, the package-scoped `cargo test`, a namcap error-class finding, the packaged-update check, the systemd unit check, or the `chan-desktop` desktop-entry, icon, and `ldd` checks) blocks the AUR push. The matrix covers both packages and `aur-publish` waits on the whole job, so a `chan-desktop` failure also holds back `chan`. That is deliberate: the AUR push is immediate and public, and a recipe that fails its own clean-container gate should not reach users. The blast radius stops there. COPR, the PPA, and the GitHub release are separate jobs and a separate workflow, so none of them is affected; the fix is a recipe change plus a `targets=aur` dispatch, with `aur_pkgrel` raised if that version already published.

Only `PKGBUILD` and `.SRCINFO` belong in the AUR repositories. The generated source archive and `.pkg.tar.zst` files under `target/aur-*` are local artifacts.

## Package behavior

- Both recipes build from the GA tag and use locked npm and Cargo dependencies. `RUSTUP_TOOLCHAIN=stable` keeps the tree's `rust-toolchain.toml` pin from making a rustup-provided cargo download a second toolchain mid-build.
- `CHAN_PACKAGED=aur` disables `chan` self-upgrade so package ownership remains with the AUR helper. The local `make linux-archpkg` QA package is stamped `pacman` instead, because it never came from the AUR.
- The container proves that stamp differently per package. Installed `chan` runs `chan upgrade`, which must exit unsuccessfully and name the AUR helper. The desktop personality routes `chan upgrade` to a running GUI instead, which a headless container cannot supply, so `chan-desktop` is checked at the stamp itself: the rendered recipe must export `CHAN_PACKAGED=aur`, and the installed binary must still carry the AUR-helper refusal hint. That hint is reachable only through `option_env!("CHAN_PACKAGED")` being `Some`, so an unstamped release build drops it with the dead branch and the check fails.
- `namcap` gates the container build: its error class covers a library the binary needs and the recipe does not declare, which would otherwise ship silently to every AUR user. Warnings stay advisory and are printed in full. Adding a waiver is one commented line in `build-in-container.sh`'s `namcap_waivers` array. The array is empty: every finding measured for either package is one of the dependency-declaration warnings below, all advisory.
- For `chan`, namcap reports `gcc-libs` and `systemd` as possibly unneeded, and reports `libgcc` as needed and implicitly satisfied. For `chan-desktop` it adds `libayatana-appindicator`, `librsvg`, and `xdg-utils` as possibly unneeded, and `dbus`, `gdk-pixbuf2`, `cairo`, and `glib2` as implicitly satisfied. All of them stay as they are. namcap derives dependencies from linked sonames alone: `systemd` is a runtime dependency for the packaged user unit and `chan devserver --service=systemd`, `xdg-utils` carries the `chan://` scheme handler, and `libayatana-appindicator` carries the tray, none of which an ELF header shows; `gcc-libs` is the package that provides the `libgcc_s.so.1` the same output says the binary needs (there is no `libgcc` package to declare). The remaining warning on both packages, an unused `ld-linux-x86-64.so.2`, is the dynamic loader itself.
- `chan-desktop` links to the host WebKitGTK/Mesa stack. This is the correct Arch/CachyOS path and avoids the rolling-distro incompatibility of the Ubuntu-built AppImage.
- The AUR Ed25519 host key is pinned to the fingerprint published by the AUR. Do not replace it with `ssh-keyscan`.
