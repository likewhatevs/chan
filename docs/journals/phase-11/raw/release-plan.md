# Phase 11 Release Plan

## Summary

Treat release infrastructure as greenfield pre-release work.

This phase resets the public release surface, switches tags to the common
`vX.Y.Z` form, makes release metadata the source of truth for downloads and
updates, and turns the Makefile/CI setup into one consistent local and remote
workflow.

The first public release version will be chosen later. Expected shape:
`v0.X.Y`.

## Release Reset

- Delete all existing upstream GitHub Releases and release tags from
  `fiorix/chan` before cutting the first new release.
- Keep source history intact. Only remote release objects and release tags are
  reset.
- Do not publish metadata that references deleted pre-release versions.
- Do not support migration from existing pre-release installs. Users who have
  old pre-release binaries or desktop apps reinstall manually.

## Release Contract

- Tags use `vX.Y.Z`.
- User-facing versions remain bare `X.Y.Z`.
- Asset names keep the product prefix where useful:
  - `chan-x86_64-unknown-linux-gnu.tar.gz`
  - `chan-aarch64-unknown-linux-gnu.tar.gz`
  - `chan-aarch64-apple-darwin.tar.gz`
  - desktop package names continue to be explicit per platform/package type.
- Linux CLI tarballs remain `gnu` for this phase.
- Do not describe Linux CLI tarballs as static until a musl/static lane is
  validated.

Release order:

1. Linux preflight.
2. Linux CLI and package builds.
3. macOS preflight.
4. macOS CLI and desktop builds, signing, and notarisation.
5. Upload all GitHub Release assets.
6. Generate release metadata from uploaded assets.
7. Deploy GitHub Pages with `/dl/**` metadata.

## Download Metadata

GitHub Releases store artifacts. GitHub Pages stores release indexes and
updater metadata.

Generate static metadata under `https://chan.app/dl/` after all required
assets exist:

- `/dl/releases.json`
- `/dl/cli/latest.json`
- `/dl/cli/vX.Y.Z.json`
- `/dl/desktop/latest.json`
- `/dl/desktop/vX.Y.Z.json`

CLI metadata includes:

- version
- tag
- publish time
- supported targets
- asset URLs
- SHA256 values

Desktop metadata must be compatible with `tauri-plugin-updater` and include:

- version
- notes
- publication date
- signed platform entries

The website reads metadata at runtime and links to existing asset URLs from the
metadata. It must not infer downloads from tags, Cargo versions, or GitHub
`/releases/latest/download` URLs.

If metadata fetch fails, the website falls back to the GitHub Releases page
instead of crafting guessed asset URLs.

## Installer

`install.sh` supports:

- macOS aarch64
- Linux x86_64
- Linux aarch64

Defaults:

- metadata: `https://chan.app/dl/cli/latest.json`
- install prefix: `$HOME/.local`
- binary path: `$HOME/.local/bin/chan`

Keep `PREFIX` for alternate install roots. Keep an override for local
testing/mirrors, but the normal path should resolve through complete-release
metadata, not GitHub latest-download URLs.

## CLI Upgrade

`chan upgrade` default behavior:

- read `/dl/cli/latest.json`
- pick the current OS/arch target
- download the matching asset URL
- verify SHA256 from metadata
- replace the running executable by the existing atomic rename flow

`chan upgrade --version X.Y.Z`:

- read `/dl/cli/vX.Y.Z.json`
- allow both upgrade and downgrade
- keep downgrade confirmation defaulting to no

`chan serve` update probe:

- keep the best-effort cached banner behavior
- point it at complete-release CLI metadata
- continue to honor `CHAN_UPDATE_CHECK=0`

Tag parsing tests should accept only the new public tag shape `vX.Y.Z`,
normalize to bare `X.Y.Z`, and reject malformed tags or versions. Do not add
special tests for old private tag shapes.

## Desktop Updater

Desktop uses `tauri-plugin-updater`.

Change the endpoint from the current versioned path to complete-release
metadata:

- new endpoint: `https://chan.app/dl/desktop/latest.json`

Desktop updater behavior:

- check once per process launch
- add an explicit `Check for Updates...` menu item
- prompt before install
- install signed updater payloads only
- relaunch after successful install

Desktop downgrade is manual reinstall only. Tauri updater handles forward
updates.

Do not rely on the existing pre-release updater bridge. Existing pre-release
desktop installs can manually reinstall.

## Makefile Shape

The root `Makefile` is the public command surface and delegation layer.
Implementation details live in platform/package subdirectories.

Root targets:

- `make`: show help
- `make chan`
- `make chan-desktop`
- `make desktop-dev`
- `make linux-chan-tarball`
- `make linux-deb`
- `make linux-rpm`
- `make linux-archpkg`
- `make linux-packages`
- `make macos-chan-app`
- `make macos-chan-dmg`
- `make macos-chan-dmg-notarised`
- `make macos-chan-dmg-notarized`: hidden alias
- `make pre-push`
- `make ci-linux`
- `make ci-macos`
- `make ci-release`

Keep old high-use targets as temporary aliases where useful, but the new
surface is the contract.

## Linux Packaging With sdme

Linux package builds run inside `sdme` root filesystems for supported
distros.

On Linux:

- host must provide `sdme` and required systemd/nspawn host tooling
- build deps live inside sdme rootfs/container environments

On macOS:

- use Lima only as the Linux host for `sdme`
- install only `sdme` and required host tooling in the Lima VM
- do not install Chan build deps, Rust, Node, or Tauri deps on the VM host
- build Chan inside sdme containers

macOS does not support x86_64 macOS artifacts. For Linux cross-arch package
builds, require a user-provided VM of the matching architecture unless a later
phase explicitly adds emulation.

## CI

CI should call the same Make targets developers call.

PR CI:

1. Run Linux first.
2. Run macOS only after Linux passes.
3. Keep macOS focused because it is expensive.

Release CI:

1. Run Linux validation and Linux artifacts first.
2. Run macOS validation and notarised artifacts after Linux passes.
3. Publish assets.
4. Verify assets.
5. Generate metadata.
6. Publish Pages.

`scripts/pre-push` should delegate to `make pre-push`.

`make pre-push` should be a local, useful gate, not the full release matrix.
It should include Rust formatting/lint/test/no-default checks and web checks.

## Contribution Guidelines

Issue templates become problem-first:

- problem or use case
- environment
- reproduction steps or workflow
- optional proposed solution, explicitly non-binding

Do not discard useful proposed solutions just because they are included.
Instead, require the issue to lead with the problem and reproduction/use case.

PR template should point contributors to `make pre-push`.

Non-trivial features should start from an issue or discussion. Small fixes may
go straight to PR.

## Test Plan

- Tag parsing accepts `vX.Y.Z`, normalizes to `X.Y.Z`, and rejects malformed
  input.
- CLI metadata parsing selects the correct target asset and SHA256.
- CLI upgrade tests cover latest upgrade, exact-version downgrade, unsupported
  target errors, and checksum mismatch.
- `install.sh` smoke tests cover OS/arch selection, `PREFIX`, and metadata
  failure fallback.
- Web-marketing checks verify generated pages do not contain baked
  `/releases/latest/download` asset links.
- Release verifier checks that:
  - all required GitHub assets exist
  - metadata points only at existing assets
  - SHA256 values match uploaded assets
  - Tauri updater manifests include required signatures
- CI dry-run/manual release validates Linux first, then macOS, then metadata
  publication.
- One-time release reset checklist verifies no old GitHub Releases or release
  tags remain before publishing the first new `v0.X.Y`.

## Assumptions

- The first reset version will be chosen later.
- Deleting upstream release history is intentional.
- No compatibility migration is required for pre-release installs.
- Linux CLI tarballs remain `gnu` in this phase.
- Desktop auto-update is enabled only for platforms with signed updater
  artifacts.
- Other desktop packages remain manual/package-manager installs until their
  updater path is proven.
