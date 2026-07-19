# Arch Linux AUR Support

Status: implementation complete; release validation pending. Grounded against `525dfa75` (`v0.71.0`) and the `sdme` AUR implementation through `20f4296` on 2026-07-19.

## Summary

Publish two source-built AUR packages beginning with v0.72.0:

- `chan`: standalone CLI and server.
- `chan-desktop`: native Tauri desktop plus `chan` and `cs` command aliases.

Both packages declare `arch=('x86_64' 'aarch64')`. Do not add `-bin` variants.

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

Keep `make linux-archpkg` as the existing working-tree binary QA path, distinct from AUR publication. Bring its payload and `CHAN_PACKAGED=aur` behavior into parity with the new `chan` package.

## Build and Publication Tooling

- Add a renderer modeled on `sdme` that accepts an allowlisted package base, GA version, optional `pkgrel`, output directory, and either a release-tag source or local source archive. It generates and validates `PKGBUILD` and `.SRCINFO` and never pushes.
- Add one container-internal build script shared by CI and sdme. It creates an unprivileged builder, installs dependencies through `makepkg --syncdeps`, builds, installs, and smokes the result.
- Add `make aur-check`, which archives a requested committed revision and runs both packages in a disposable sdme container. This permits validation before the release tag exists.
- Use a clean upstream Arch rootfs on native x86_64 and the official Arch Linux ARM rootfs on native aarch64. Do not use the pre-provisioned desktop build rootfs because its baked dependencies would hide missing `PKGBUILD` declarations.
- Extend `distros-publish.yml` with native x86_64/aarch64 validation for both packages followed by AUR publication. Publication remains post-release and non-gating for the GitHub release.
- Pin the AUR SSH host key. Use the existing AUR account key through the chan repository secret `AUR_SSH_PRIVATE_KEY`; missing secrets validate but skip publication.
- Push only generated metadata to `HEAD:master`, treat unchanged output as success, and verify the published version through the AUR RPC.

## Validation and Acceptance

- Renderer failures cover unknown packages, invalid versions and releases, missing source archives, unresolved placeholders, and mismatched `.SRCINFO`.
- `make aur-check` succeeds for both packages on native x86_64 and native aarch64 sdme hosts.
- Each package builds through `makepkg`, passes its package-scoped Rust tests, installs with pacman, and passes `namcap` review.
- The installed `chan` and `cs` commands dispatch correctly, `chan upgrade` names the AUR update path, and the systemd user unit verifies.
- `chan-desktop` has no unresolved shared libraries; its desktop entry and icons validate; and its package conflict/provide relationship with `chan` is correct.
- An actual CachyOS x86_64 hand-smoke opens a workspace and embedded terminal without the AppImage `EGL_BAD_PARAMETER` failure.
- The v0.72.0 post-release workflow reports all four architecture/package builds green and both AUR repositories at `0.72.0-1`.

## Assumptions

- The user provisions only `AUR_SSH_PRIVATE_KEY`; the existing AUR account and registered public key remain unchanged.
- A CachyOS precompiled-package request is a later distribution step.
- aarch64 support means native Arch Linux ARM builds and tests. No claim is made that CachyOS itself supports aarch64.
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

The remaining acceptance evidence is deliberately release-bound: native
aarch64 and full `chan-desktop` Arch builds in the post-release matrix, the
CachyOS desktop hand-smoke, and AUR RPC confirmation of both `0.72.0-1`
repositories.
