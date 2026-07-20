# Arch Linux AUR Support

> Status: shipped in [v0.72.0](../../release/release-v0.72.0.md).

Status: implementation complete and validated on x86_64 at `41a87d8d`; release validation pending. Grounded against `525dfa75` (`v0.71.0`) and the `sdme` AUR implementation through `20f4296` on 2026-07-19.

## Summary

Publish two source-built AUR packages beginning with v0.72.0:

- `chan`: standalone CLI and server.
- `chan-desktop`: native Tauri desktop plus `chan` and `cs` command aliases.

Both packages declare `arch=('x86_64' 'aarch64')`. Do not add `-bin` variants. Only x86_64 is proven: the aarch64 leg (Arch Linux ARM rootfs import, keyring bootstrap, native ARM Tauri build) has never executed on any host or runner, so every claim about it is a declaration, not a validation.

The same packages work on CachyOS x86_64 because CachyOS is Arch-based and supports AUR packages. AUR availability does not automatically place them in CachyOS's precompiled repositories; that requires a separate CachyOS package request and is out of scope. CachyOS currently targets x86_64, so the aarch64 contract applies to Arch Linux ARM rather than CachyOS.

AUR stores `PKGBUILD` recipes and does not build or host our resulting binaries. Users or their AUR helper build locally. Source-building `chan-desktop` against the host WebKitGTK/Mesa stack also avoids the known Ubuntu-built AppImage compatibility failure on rolling Arch/CachyOS systems.

## Package Contract

- Keep versionless templates under `packaging/distros/arch/aur/{chan,chan-desktop}/PKGBUILD.in`. Generated AUR repositories contain only `PKGBUILD` and `.SRCINFO`.
- Render `pkgver`, `pkgrel`, and the checksum of the release tag input. Automatic GA publication uses `pkgrel=1`; manual packaging-only repairs accept an explicit higher `pkgrel`.
- Source both packages from the GitHub GA tag archive. Reject prerelease versions and unresolved placeholders.
- Run `npm ci` and `cargo fetch --locked` in `prepare()`, then build the existing embedded web assets and the selected Rust package with offline/frozen dependency resolution.
- Build with `CHAN_PACKAGED=aur`, disabling self-upgrade and directing the user to their AUR helper.
- Disable makepkg's injected LTO because the Cargo release profile already owns thin LTO and the workspace includes native C/C++ dependencies.

`chan` installs `/usr/bin/{chan,cs}`, the devserver systemd user unit, license, and documentation.

`chan-desktop` installs `/usr/bin/chan-desktop`, `chan` and `cs` symlinks to it, the desktop entry, hicolor icons, the same user unit, license, and documentation. It provides and conflicts with `chan` but does not replace it.

Keep `make linux-archpkg` as the existing working-tree binary QA path, distinct from AUR publication. Bring its payload into parity with the new `chan` package. It stamps `CHAN_PACKAGED=pacman`, not `aur`: a hand-built package hands update ownership to the package manager without naming an AUR helper the user never used.

## Build and Publication Tooling

- Add a renderer modeled on `sdme` that accepts an allowlisted package base, GA version, optional `pkgrel`, output directory, and either a release-tag source or local source archive. It generates and validates `PKGBUILD` and `.SRCINFO` and never pushes.
- Add one container-internal build script shared by CI and sdme. It creates an unprivileged builder, installs dependencies through `makepkg --syncdeps`, builds, installs, and smokes the result.
- Add `make aur-check`, which archives a requested committed revision and runs both packages in a disposable sdme container. This permits validation before the release tag exists.
- Use a clean upstream Arch rootfs on native x86_64 and the official Arch Linux ARM rootfs on native aarch64. Do not use the pre-provisioned desktop build rootfs because its baked dependencies would hide missing `PKGBUILD` declarations.
- Extend `publish-downstream.yml` with native x86_64 validation for both packages followed by AUR publication. The aarch64 job stays outside `aur-publish`'s `needs` for v0.73.0 so an unproven ARM cell cannot block shipping the validated x86_64 recipes; it runs automatically at GA for evidence and remains opt-in on manual dispatch. Publication remains post-release and non-gating for the GitHub release.
- Prove the AUR credential with an `ssh -T aur@aur.archlinux.org` probe before any push. A key whose public half is not registered on the AUR account otherwise surfaces only as a failed clone at publication time.
- Give the workflow the release.yml dry-run shape: `publish` defaults to false on manual dispatch, and `targets` scopes a retry to one distro so a transient Launchpad ftp 550 does not also rebuild the Arch packages. A dry run reports only what it observed: it verifies the signature on the source packages it built, separates a failure before the signing key from one after it, and fails on the canonical repository when a credential the run exists to prove is absent.
- Filter a prerelease version out of every job, COPR and Launchpad included, so a tag that is not a GA release cannot reach a distro that has no prerelease channel.
- Pin the AUR SSH host key. Use the existing AUR account key through the chan repository secret `AUR_SSH_PRIVATE_KEY`; a missing secret validates and skips publication, except on a dry run against the canonical repository, whose only product is the credential proof.
- Push only generated metadata to `HEAD:master`, treat unchanged output as success, and verify the published version through the AUR RPC.
- Announce the capability where users look for it: the marketing install page carries an Arch/AUR section next to the PPA and COPR ones, asserted by `web/packages/marketing/scripts/smoke-dist.mjs`.

## Validation and Acceptance

- Renderer failures cover unknown packages, invalid versions and releases, missing source archives, unresolved placeholders, and mismatched `.SRCINFO`.
- `make aur-check` succeeds for both packages on a native x86_64 sdme host; it exits `0` there as of 2026-07-20. The same command on a native aarch64 host is the open ARM item, not a shipped claim.
- Each package builds through `makepkg`, passes its package-scoped Rust tests, installs with pacman, and passes `namcap` review. The container build enforces that review: namcap's error class fails the build, warnings are printed, and waivers live in `build-in-container.sh` with the reason each one cannot be fixed in the recipe. That gate runs inside `aur-validate`, a hard `needs` of `aur-publish`, so an error-class finding in either package holds back both AUR pushes and nothing else.
- The installed `chan` and `cs` commands dispatch correctly and the systemd user unit verifies. The standalone `chan` package's `chan upgrade` exits unsuccessfully and names the AUR update path. The desktop personality routes `chan upgrade` to a running GUI, so that path is not a valid headless-container smoke; `chan-desktop` instead proves the `CHAN_PACKAGED=aur` stamp reached the build, through both the rendered recipe's export and the refusal hint in the installed binary.
- `chan-desktop` has no unresolved shared libraries; its desktop entry and icons validate; and its package conflict/provide relationship with `chan` is correct.
- An actual CachyOS x86_64 hand-smoke opens a workspace and embedded terminal without the AppImage `EGL_BAD_PARAMETER` failure.
- The v0.72.0 post-release workflow reports the two x86_64 package builds green, the AUR credential probe green, and both AUR repositories at `0.72.0-1`. The run schedules no aarch64 cell, so every job in it is expected green.

## Assumptions

- The user provisions only `AUR_SSH_PRIVATE_KEY`; the existing AUR account and registered public key remain unchanged.
- A CachyOS precompiled-package request is a later distribution step.
- aarch64 support means native Arch Linux ARM builds and tests, and remains unverified until such a build runs green. No claim is made that CachyOS itself supports aarch64.
- Binary AUR variants, official Arch repository inclusion, and AppImage changes are out of scope.

## Implementation Evidence

Implemented on 2026-07-19. The acceptance run is dated 2026-07-20 and measures the merged tip, `41a87d8d`, which carries the packaging, terminal, dump-skill, and upgrade-refusal work together. It ran from a detached worktree on this x86_64 host against the local sdme rootfs `archlinux`:

```
make aur-check SDME='sudo -n sdme' AUR_ROOTFS=archlinux
```

`make` exited `0`. Both packages rendered, built, installed, smoked, and were removed in one disposable container:

```
>> AUR validation: version=0.71.0 pkgrel=1 arch=x86_64
>> building chan
>> rendered chan 0.71.0-1 in /out/chan
>> built and smoked chan-0.71.0-1-x86_64.pkg.tar.zst
>> building chan-desktop
>> rendered chan-desktop 0.71.0-1 in /out/chan-desktop
>> built and smoked chan-desktop-0.71.0-1-x86_64.pkg.tar.zst
```

Each recipe's `check()` ran the release-mode test suite in the container with `CHAN_PACKAGED=aur` exported by the recipe, and both passed. That includes the 258-test `chan` lib suite, `close_then_reopen_under_pressure`, and the packaged-upgrade refusals `upgrade_route_refuses_a_packaged_build_in_every_personality` and `test_packaged_upgrade_refusal_only_fires_for_packaged_builds`. The `chan` post-install smoke additionally required a failing `chan upgrade` naming the AUR helper path, and the `chan-desktop` smoke required both the `export CHAN_PACKAGED=aur` line in the rendered recipe and the AUR-helper refusal hint in the installed binary.

namcap emitted no error-class (`E:`) line for either package on the resulting artifacts, so the gate passed with its waiver list genuinely empty rather than by exemption. `chan` drew four warnings: an unused `/usr/lib64/ld-linux-x86-64.so.2` (the dynamic loader), `libgcc` detected and implicitly satisfied, and `gcc-libs` and `systemd` included but possibly unneeded. `chan-desktop` drew eleven: the same unused dynamic loader; `libgcc`, `cairo`, `gdk-pixbuf2`, `dbus`, and `glib2` detected and implicitly satisfied; and `gcc-libs`, `libayatana-appindicator`, `librsvg`, `systemd`, and `xdg-utils` included but possibly unneeded.

Every one of those fifteen findings is a dependency-declaration observation from namcap's soname-only analysis, and none is acted on: `gcc-libs` is the package that provides the `libgcc_s.so.1` the same output says the binary needs and there is no `libgcc` package to declare; `systemd` is a runtime dependency for the packaged user unit and `chan devserver --service=systemd`; and `xdg-utils` and `libayatana-appindicator` carry the `chan://` scheme handler and the tray. No ELF header shows any of those.

The renderer's unknown-package, prerelease, and missing-source failure paths, and a `chan-desktop` render carrying both architectures with `pkgrel=2`, were exercised by hand on 2026-07-19 against `4a9199ad`. `make-aur-package.sh` is byte-identical between that revision and `41a87d8d`, so those observations still describe the shipped renderer.

Four things the run does not establish. It proves x86_64 only: this host has no binfmt or QEMU, so the declared aarch64 architecture has never been built anywhere. It proves nothing about AUR publication: nothing was pushed, and the AUR RPC returned no existing result for either `chan` or `chan-desktop` on 2026-07-19, so both package bases were unclaimed. It proves nothing about CachyOS: the rootfs was upstream `archlinux`. It proves no GUI behavior: the container is headless, so the `chan-desktop` leg checks the build stamp and the installed artifacts, not a window.

The run also used the local sdme rootfs rather than the `docker.io/archlinux/archlinux:base-devel` image `aur-validate` builds in, so the CI job's own rootfs is unmeasured. The findings still carry across: both paths run the same `build-in-container.sh`, and namcap derives its output from each recipe's `depends` and the built binary's sonames.

The remaining acceptance evidence is deliberately release-bound: the CachyOS desktop hand-smoke and AUR RPC confirmation of both `0.72.0-1` repositories.

The aarch64 leg is the one item that no local gate can close before it runs somewhere: this host is x86_64, and no runner has executed the ALARM rootfs import or the native ARM Tauri build. v0.73.0 runs it automatically at GA as observed evidence without making it a publication dependency; a manual diagnostic uses `targets=aur` and `aur_validate_arm=true`. After the first green run, v0.74.0 adds it to `aur-publish`'s `needs`.
