# Arch Linux AUR Support

Status: implementation complete; release validation pending. Grounded against `525dfa75` (`v0.71.0`) and the `sdme` AUR implementation through `20f4296` on 2026-07-19.

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
- Extend `distros-publish.yml` with native x86_64 validation for both packages followed by AUR publication. The aarch64 job stays outside `aur-publish`'s `needs` so an unproven ARM cell cannot block shipping the validated x86_64 recipes, and it is opt-in on manual dispatch so a leg that has never completed anywhere does not make two red cells the normal end state of every GA run. Publication remains post-release and non-gating for the GitHub release.
- Prove the AUR credential with an `ssh -T aur@aur.archlinux.org` probe before any push. A key whose public half is not registered on the AUR account otherwise surfaces only as a failed clone at publication time.
- Give the workflow the release.yml dry-run shape: `publish` defaults to false on manual dispatch, and `targets` scopes a retry to one distro so a transient Launchpad ftp 550 does not also rebuild the Arch packages. A dry run reports only what it observed: it verifies the signature on the source packages it built, separates a failure before the signing key from one after it, and fails on the canonical repository when a credential the run exists to prove is absent.
- Filter a prerelease version out of every job, COPR and Launchpad included, so a tag that is not a GA release cannot reach a distro that has no prerelease channel.
- Pin the AUR SSH host key. Use the existing AUR account key through the chan repository secret `AUR_SSH_PRIVATE_KEY`; a missing secret validates and skips publication, except on a dry run against the canonical repository, whose only product is the credential proof.
- Push only generated metadata to `HEAD:master`, treat unchanged output as success, and verify the published version through the AUR RPC.
- Announce the capability where users look for it: the marketing install page carries an Arch/AUR section next to the PPA and COPR ones, asserted by `web/packages/marketing/scripts/smoke-dist.mjs`.

## Validation and Acceptance

- Renderer failures cover unknown packages, invalid versions and releases, missing source archives, unresolved placeholders, and mismatched `.SRCINFO`.
- `make aur-check` succeeds for both packages on a native x86_64 sdme host. The same command on a native aarch64 host is the open ARM item, not a shipped claim.
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

Implemented on 2026-07-19. The local pre-push gate passed, including formatting,
clippy, all Rust targets, the no-default-features build, gateway builds, and all
frontend and marketing checks. A clean x86_64 Arch sdme container rendered,
built, tested, installed, smoked, and removed the local-source `chan` package;
the installed binary reported the AUR-helper update path and the systemd unit
verified. A second clean Arch pass rendered `chan-desktop` metadata with both
architectures and `pkgrel=2`, and exercised the renderer's unknown-package,
prerelease, and missing-source failures. The official AUR RPC returned no
existing results for either `chan` or `chan-desktop`, so both package bases were
available for their first push on the implementation date.

Both packages were built, packaged, and installed on a local x86_64 sdme Arch container on 2026-07-19, from a separate probe worktree pinned at `4a9199ad`. That revision predates this item's namcap-gate and validator fixes, so the run measures the recipes as authored: namcap ran advisory rather than enforcing, and the packaged-upgrade check was still the pre-fix one. A re-run against this branch's tip is still owed.

`chan` built in 23m47s, passed its release-mode test suite inside the container, packaged, installed, and smoked. Its packaged `chan upgrade` exited unsuccessfully with `this build of chan is managed by the system package manager (aur); self-upgrade is disabled. Update with your AUR helper (for example, paru -Syu or yay -Syu).` namcap reported four warnings and no error-class line: an unused `/usr/lib64/ld-linux-x86-64.so.2` (the dynamic loader), `libgcc` detected and implicitly satisfied, and `gcc-libs` and `systemd` flagged as possibly unneeded.

`chan-desktop` built, produced a 21 MB package, and installed cleanly. namcap reported eleven warnings and no error-class line: the same unused dynamic loader; `dbus`, `gdk-pixbuf2`, `libgcc`, `cairo`, and `glib2` detected and implicitly satisfied; and `gcc-libs`, `libayatana-appindicator`, `librsvg`, `systemd`, and `xdg-utils` included but possibly unneeded.

Every one of those fifteen findings is a dependency-declaration observation from namcap's soname-only analysis, and none is acted on: `gcc-libs` is the package that provides the `libgcc_s.so.1` the same output says the binary needs and there is no `libgcc` package to declare; `systemd` is a runtime dependency for the packaged user unit and `chan devserver --service=systemd`; and `xdg-utils` and `libayatana-appindicator` carry the `chan://` scheme handler and the tray. No ELF header shows any of those. So the error-class gate passes for both packages with an empty waiver list, on measured output rather than on expectation.

The run also exposed a validator defect rather than a package defect. The post-install smoke ran `chan upgrade` for both packages and required the AUR-helper refusal in its output. That holds for `chan`, but the desktop personality routes `chan upgrade` to a running GUI: in a headless container it launched nothing, waited twenty seconds, and left `timed out waiting for chan-desktop to start` in `upgrade.out`, failing the run over a package that was otherwise good. The corrected validator forks by package. `chan` keeps the executable refusal. `chan-desktop` asserts the `CHAN_PACKAGED=aur` stamp directly, requiring both the export in the rendered recipe and the refusal hint in the installed binary; that hint is reachable only through `option_env!("CHAN_PACKAGED")` being `Some`, so an unstamped release build drops the literal along with the dead branch. Both assertions were checked against the artifacts this run left on disk and against negative controls.

That probe ran in the local sdme Arch rootfs, not in the `docker.io/archlinux/archlinux:base-devel` image `aur-validate` uses, so the gate's own rootfs is unmeasured. The findings still carry: namcap derives them from each recipe's `depends` and the built binary's sonames, and neither `depends` list has changed since the probe.

The remaining acceptance evidence is deliberately release-bound: the CachyOS desktop hand-smoke and AUR RPC confirmation of both `0.72.0-1` repositories.

The aarch64 leg is the one item that no gate can close before it runs somewhere: this host is x86_64, and no runner has executed the ALARM rootfs import or the native ARM Tauri build. The release path keeps it out of GA entirely rather than shipping a red cell every release that nobody is expected to act on. Prove it with a `distros-publish` dispatch carrying `targets=aur` and `aur_validate_arm=true`, then fold the result back into this item and give the job the GA trigger and a place in `aur-publish`'s `needs`.
