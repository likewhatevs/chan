# @@LaneC journal - phase 11 continuation

Append-only journal for the CI / release lane.

## 2026-05-27 03:09 Bootstrap

Identity: @@LaneC. Repo root verified at
`/Users/fiorix/dev/github.com/fiorix/chan`; HEAD is
`85e6f1541e8a45f5da618078afd5a66f8ee1b09d` on main.

Read:
- `docs/journals/phase-11/lane-c-kickoff.md`
- `docs/journals/phase-11/release-plan.md`
- `docs/journals/phase-11/next-round-backlog.md`
- `docs/journals/phase-11/architect/journal.md`
- `docs/journals/phase-11/coordination/README.md`
- `docs/journals/phase-11/coordination/event-architect-lane-c.md`
- `docs/journals/phase-11/coordination/event-lane-a-lane-c.md`
- `crates/chan/src/update.rs`
- root `Makefile`, `desktop/Makefile`, `scripts/pre-push`
- `web-marketing/src/install.sh`, web-marketing install page and release
  verifier
- `.github/workflows/{ci,pages,release,release-desktop}.yml`
- `desktop/src-tauri/tauri.conf.json`, `desktop/src-tauri/Cargo.toml`

Current state:
- Release tags and workflows still use `chan-v*`; release plan requires
  public `vX.Y.Z` tags with bare user-facing versions.
- CLI updater reads GitHub latest-release JSON, downloads release assets
  directly, and verifies against `SHA256SUMS`; release plan requires
  `/dl/cli/latest.json` and `/dl/cli/vX.Y.Z.json` metadata as source of truth.
- Installer defaults to GitHub latest-download URLs via `BASE`; release plan
  requires metadata default `https://chan.app/dl/cli/latest.json`, with a local
  testing/mirror override.
- Site install page bakes latest-download asset URLs today; release plan
  requires runtime metadata and GitHub Releases fallback.
- Root Makefile is old dev-wrapper shape; release plan names a new public
  target surface and wants CI to call the same targets.
- Desktop updater endpoint is versioned by target/current version; release plan
  wants `https://chan.app/dl/desktop/latest.json`, one launch check, manual menu
  item, prompt-before-install, signed payloads only, relaunch after install.
- CI is split across ci/pages/release/release-desktop and publishes directly to
  GitHub Releases; release plan wants Linux-first, macOS-second, asset verify,
  metadata generation, then Pages publication.

Plan posted to `event-lane-c-architect.md`; awaiting @@Architect ratification
before source edits.

## 2026-05-27 03:14 Slice 1 start

Read @@Architect ratification on `event-architect-lane-c.md`. Created
`/Users/fiorix/dev/github.com/fiorix/chan-lane-c` on branch
`phase-11-lane-c` from main at `85e6f15`.

Slice 1 boundary:
- Update `crates/chan/src/update.rs` to use `/dl/cli/latest.json` and
  `/dl/cli/vX.Y.Z.json` metadata, not GitHub latest-release JSON or
  `SHA256SUMS`.
- Switch release tag parsing/formatting to `vX.Y.Z`.
- Update `web-marketing/src/install.sh` to resolve assets through CLI metadata,
  keeping a local/mirror override.
- Add focused tests for tag parsing, metadata parsing/target selection, and
  installer smoke where practical.
- Do not touch workflows, Cargo.lock, Tauri, graph surfaces, or phase-11 docs
  outside this journal/channel write.

## 2026-05-27 07:10 Slice 1 ready

Committed `feat(release): use CLI metadata for upgrades` and rebased onto
current main `64225b9` (LaneA GI-9 merge). Final branch tip:
`c0bdf015ee92e81218bdc0ea26655f971379e341`.

Implementation:
- `crates/chan/src/update.rs`
  - Replaced GitHub latest-release JSON and `SHA256SUMS` lookup with
    `https://chan.app/dl/cli/latest.json` and
    `https://chan.app/dl/cli/vX.Y.Z.json`.
  - Strict tag parser now accepts public `vX.Y.Z`, normalizes to bare
    `X.Y.Z`, and rejects malformed tags/versions.
  - CLI metadata schema used by updater:
    `version`, `tag`, `published_at`, `targets[]` with `target`, `asset`,
    `url`, `sha256`.
  - Upgrade target selection validates the active target's expected asset name
    and SHA256 before download/install.
- `web-marketing/src/install.sh`
  - Defaults to `/dl/cli/latest.json`.
  - Supports `METADATA_URL`, `BASE`, and `VERSION` for local/mirror tests.
  - Verifies SHA256 using `sha256sum` or `shasum`.
  - Handles missing metadata with a manual GitHub Releases fallback message.
- `web-marketing/scripts/smoke-install-sh.mjs`
  - Uses local metadata/tarballs and fake `uname` to smoke Linux x86_64 and
    macOS arm64 selection, `PREFIX`, and metadata failure fallback.
- Updated docs/manual install + upgrade pages and the install page helper copy.

Verification after rebase:
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `cargo build --no-default-features`
- `npm run check` in `web-marketing/`
- `npm run check` in `web/`
- `npm run build` in `web/`

No `.github/workflows/`, Cargo.toml/Cargo.lock, Tauri files, or graph surfaces
touched.

## 2026-05-27 07:11 Slice 2 start

Starting Makefile/pre-push target surface on top of slice 1 commit
`c0bdf01`. Scope:
- Reshape root `Makefile` into the release-plan public target surface.
- Keep useful old targets as temporary aliases where they do not conflict.
- Make `scripts/pre-push` delegate to `make pre-push`.
- Do not touch workflows, Cargo.lock, Tauri files, or graph surfaces.

## 2026-05-27 07:24 Slice 2 ready

Committed `build: add phase 11 release make targets`. Final branch tip:
`24c8d438b6a71bc7615f30e5f6685a72035d484e`.

Implementation:
- Root `Makefile`
  - `make` now shows help by default.
  - Added the release-plan public targets:
    `chan`, `chan-desktop`, `desktop-dev`, `linux-chan-tarball`,
    `linux-deb`, `linux-rpm`, `linux-archpkg`, `linux-packages`,
    `macos-chan-app`, `macos-chan-dmg`,
    `macos-chan-dmg-notarised`, hidden alias
    `macos-chan-dmg-notarized`, `pre-push`, `ci-linux`, `ci-macos`,
    and `ci-release`.
  - Kept useful old aliases: `all`, `build`, `rpm`, `dev`, `install`,
    `uninstall`, `models`, `build-release`, `test`, `lint`, `hooks`,
    and `clean`.
  - `make pre-push` runs Rust fmt, clippy with warnings denied, all-target
    tests, no-default-features build, web check/build, and marketing-site
    checks.
- `packaging/linux/Makefile`
  - Added Linux packaging detail targets behind the root delegates.
  - CLI tarballs use the release-plan asset name
    `chan-<target>.tar.gz` and keep files at the archive root, matching the
    existing updater/installer-compatible shape.
  - `.deb` and `.rpm` targets wrap the existing Cargo package metadata.
  - `linux-archpkg` creates an Arch package from a checked-in PKGBUILD and
    expects `makepkg` inside an Arch sdme rootfs.
- `scripts/pre-push`
  - Now delegates to `make pre-push`.

Verification:
- `make`
- `make -n chan`
- `make -n linux-chan-tarball`
- `make -n linux-deb`
- `make -n linux-rpm`
- `make -n linux-archpkg`
- `make -n linux-packages`
- `make -n chan-desktop`
- `make -n desktop-dev`
- `make -n macos-chan-app`
- `make -n macos-chan-dmg`
- `make -n macos-chan-dmg-notarised`
- `make -n macos-chan-dmg-notarized`
- `bash -n scripts/pre-push`
- `git diff --check`
- `make pre-push`

No `.github/workflows/`, Cargo.toml/Cargo.lock, Tauri source/config, or graph
surfaces touched.

## 2026-05-27 08:30 Slice 4 start

Read @@Architect update: slice 3 merged to main at
`96c9c17` and re-gated green. Rebasing LaneC worktree onto that merge commit.

Scope:
- Update CI workflow wiring so PR CI calls the Phase 11 Make targets and runs
  Linux before macOS.
- Update release workflow tag shape from old `chan-v*` to public `vX.Y.Z`.
- Structure release publishing so upload, metadata generation, and Pages
  publication only run on tag or explicit manual release-cut inputs, never on a
  normal PR/push.
- Keep signing-secret values out of YAML, journals, and commits. Reference
  existing GitHub Actions secret names only.
- Do not touch Cargo.toml/Cargo.lock, Tauri updater behavior, or graph
  surfaces in this slice.

## 2026-05-27 07:36 Slice 3 start

Read @@Architect update: slices 1-2 merged to main at
`bd979bcc8abc95b5af9d90dae5902c769867c0f5` and re-gated green. Fast-forwarded
the LaneC worktree to that merge commit.

Slice 3 scope:
- Add tooling/dry-run generation for `/dl/**` release metadata from local
  verified asset fixtures.
- Make the marketing install/download surfaces consume metadata with GitHub
  Releases fallback, instead of inferring asset URLs from versions or tags.
- Update non-graph manual/site copy for the release metadata contract.
- Do not publish metadata, do not touch release-cut surfaces, workflows,
  Cargo.lock, Tauri update behavior, or graph manual copy.

## 2026-05-27 08:26 Slice 3 ready

Committed `feat(release): generate metadata for site downloads`, then rebased
onto current main `e61b8c467b50901b7d83b701c8ce78fc31108305`. Final branch
tip: `a75bbb3ba6ce42976357a8e87a782c384a45461b`.

Implementation:
- `web-marketing/scripts/generate-release-metadata.mjs`
  - Reads a verified asset manifest and writes the static `/dl/**` metadata
    files that the release plan makes authoritative.
  - Validates bare semantic versions, public `vX.Y.Z` tags, ISO publish time,
    concrete HTTPS release asset URLs, 64-character lowercase SHA256 values,
    and signed desktop updater platform entries.
  - Writes `releases.json`, `cli/latest.json`, `cli/vX.Y.Z.json`,
    `desktop/latest.json`, and `desktop/vX.Y.Z.json`.
- `web-marketing/scripts/smoke-release-metadata.mjs`
  - Runs the generator against the checked-in `v0.15.4` fixture and asserts
    output shape for site downloads, CLI metadata, and desktop updater
    metadata.
- `web-marketing/src/site.js`
  - Fetches `/dl/releases.json` at runtime and maps download ids to metadata
    URLs.
  - Keeps GitHub Releases as the fallback when metadata is absent or invalid.
  - Rejects unsafe download URLs, including latest-download indirection.
- `web-marketing/scripts/build.mjs`
  - Removed build-time latest-download URL construction.
  - Added generated-site checks for download metadata hooks, runtime metadata
    loading, GitHub Releases fallback, and no inferred latest asset URLs.
- `web-marketing/scripts/verify-release-assets.mjs`
  - Uses public `vX.Y.Z` tags and concrete release asset URLs.
  - Verifies `VERSION` and `SHA256SUMS` when they exist, without requiring
    them for the greenfield reset.
- Updated `web-marketing/README.md`, install page copy, home page download
  hooks, and non-graph manual install/upgrade copy.

Verification:
- `npm run smoke:metadata` in `web-marketing/`
- `npm run check` in `web-marketing/`
- `node web-marketing/scripts/generate-release-metadata.mjs --manifest web-marketing/fixtures/release-assets/v0.15.4.json --out /tmp/chan-dl-smoke`
- inspected `/tmp/chan-dl-smoke`
- `git diff --check`
- `make pre-push` after rebase onto `e61b8c4`

No `.github/workflows/`, Cargo.toml/Cargo.lock, Tauri source/config, or graph
surfaces touched.
