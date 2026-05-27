# Channel: @@LaneC -> @@Architect

Append-only. @@LaneC writes progress reports here; @@Architect reads. Never
edit prior entries. Curated highlights/lowlights/contention; link your
journal for detail.

## 2026-05-27 03:09 @@LaneC -> @@Architect
Bootstrap plan, awaiting ratification.

Identity confirmed as @@LaneC on main at 85e6f1541e8a45f5da618078afd5a66f8ee1b09d.
Read kickoff, release plan, backlog, architect journal, coordination bus,
event-architect-lane-c, current updater, Makefiles, installer/site, desktop
config, and workflows.

Proposed slice order:
1. Release contract foundation: switch public tag shape to vX.Y.Z, move CLI
   updater and installer to /dl/cli metadata, add focused tests.
2. Makefile and local gates: reshape root targets around the release plan and
   make scripts/pre-push delegate to make pre-push.
3. Metadata and site: generate /dl metadata from verified release assets, make
   site downloads consume metadata/fallback to GitHub Releases, update
   non-graph manual copy.
4. CI release workflows: make release CI call the same Make targets, publish
   assets, verify them, generate metadata, then publish Pages. Shared infra.
5. Tauri updater workflow: retarget desktop updater to /dl/desktop/latest.json,
   add launch/manual update UX, and bump Tauri deps if needed. Shared infra,
   Cargo.lock contention.
6. Graph manual/site copy only after @@LaneA settles GI-8/9/10/11 and loading
   state behavior.

Release-cut gate: all implementation and dry-runs stay in lane scope. Any tag
push, GitHub Release publish, release reset, or Pages metadata publish for an
actual release goes to @@Alex first via event-lane-c-alex.md.

Questions for ratification:
- OK to split the shared-infra slices so Cargo.lock/Tauri dep churn lands last?
- Should release reset tooling be scripted now, or remain a documented
  one-time operator checklist behind the @@Alex release-cut gate?

## 2026-05-27 03:14 @@LaneC -> @@Architect
Slice 1 started.

Ratification read on event-architect-lane-c. Created source worktree
`../chan-lane-c` on branch `phase-11-lane-c` at 85e6f15.

Slice 1 scope: CLI release contract foundation only. I will change `chan
upgrade` and the shell installer from GitHub latest/`chan-v*`/SHA256SUMS to
`vX.Y.Z` plus `/dl/cli/*.json` metadata, with focused parser/selection tests.
No `.github/workflows/` or Cargo.lock churn in this slice.

## 2026-05-27 07:10 @@LaneC -> @@Architect
ready to merge: phase-11-lane-c@c0bdf015ee92e81218bdc0ea26655f971379e341

Slice 1 is complete and rebased onto current main `64225b9`.

What changed:
- `chan upgrade` now reads complete-release CLI metadata from
  `/dl/cli/latest.json` or `/dl/cli/vX.Y.Z.json`, validates strict public
  `vX.Y.Z` tags, selects the current target asset, and verifies SHA256 from
  metadata instead of `SHA256SUMS`.
- `web-marketing/src/install.sh` now defaults to
  `https://chan.app/dl/cli/latest.json`, supports `METADATA_URL`, `BASE`, and
  `VERSION` for local/mirror testing, selects the OS/arch target from metadata,
  verifies SHA256, and falls back with a manual GitHub Releases pointer when
  metadata is unavailable.
- Added `web-marketing/scripts/smoke-install-sh.mjs` for file-backed installer
  smoke tests with fake `uname`, target selection, `PREFIX`, and metadata
  failure fallback.
- Updated non-graph install/upgrade manual copy and the install page helper
  text for the metadata contract.

Touched no graph surfaces, no `.github/workflows/`, no Cargo.toml/Cargo.lock.

Gate on rebased branch:
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `cargo build --no-default-features`
- `npm run check` in `web-marketing/`
- `npm run check` in `web/`
- `npm run build` in `web/`

Notes:
- `main` advanced beyond the last LaneC bus note to include LaneA GI-9
  (`64225b9`); rebase was clean and the gate above is after that rebase.
- `web/` needed `npm install` in the fresh lane worktree before the web gate;
  no package files changed.

## 2026-05-27 07:11 @@LaneC -> @@Architect
Slice 2 started.

Starting Makefile/pre-push target surface on top of slice 1 commit
`c0bdf01`. This slice touches the root Makefile and `scripts/pre-push` only
unless local inspection shows a small doc/test helper is needed. No workflows,
Cargo.lock, Tauri, or graph surfaces.

## 2026-05-27 07:24 @@LaneC -> @@Architect
ready to merge: phase-11-lane-c@24c8d438b6a71bc7615f30e5f6685a72035d484e

Slice 2 is complete on current main `64225b9` plus Slice 1.

What changed:
- Root `Makefile` is now the Phase 11 public command surface. `make` shows
  help by default; added release-plan targets for CLI, desktop, Linux package,
  macOS bundle, local pre-push, and CI entry points.
- Linux package implementation details moved behind root delegates into
  `packaging/linux/Makefile`, with a checked-in Arch `PKGBUILD` helper. CLI
  tarballs keep the existing archive-root binary layout and use
  `chan-<target>.tar.gz` asset names.
- `scripts/pre-push` now delegates to `make pre-push`.
- Kept old useful aliases where they do not conflict: `all`, `build`, `rpm`,
  `dev`, `install`, `uninstall`, `models`, `build-release`, `test`, `lint`,
  `hooks`, and `clean`.

Verification:
- `make pre-push` passed. This exercised Rust fmt, clippy with warnings
  denied, all-target tests, no-default-features build, web check/build, and
  marketing-site checks.
- Dry-ran target wiring with `make -n` for CLI, Linux package, desktop, and
  macOS signed/notarised aliases, including hidden `macos-chan-dmg-notarized`.
- `bash -n scripts/pre-push` and `git diff --check` passed.

Touched no graph surfaces, no `.github/workflows/`, no Cargo.toml/Cargo.lock,
and no Tauri source/config in this slice.

## 2026-05-27 07:36 @@LaneC -> @@Architect
Slice 3 started.

Read the 07:31 merge/re-gate note. Fast-forwarded the LaneC worktree to merged
main `bd979bc`.

Slice 3 scope: metadata generator + verifier tooling for `/dl/**` using local
fixtures, marketing/site downloads consuming metadata with GitHub Releases
fallback, and non-graph manual copy. This stays dry-run/tooling only: no
release reset, no tag push, no GitHub Release publish, no Pages publish. No
`.github/workflows/`, Cargo.toml/Cargo.lock, Tauri updater work, or graph
manual copy in this slice.

## 2026-05-27 08:26 @@LaneC -> @@Architect
ready to merge: phase-11-lane-c@a75bbb3ba6ce42976357a8e87a782c384a45461b

Slice 3 is complete and rebased onto current main `e61b8c4`.

What changed:
- Added deterministic `/dl/**` metadata generation from a verified release
  asset manifest. The generator writes `releases.json`, CLI latest/version
  metadata, and Tauri desktop updater latest/version metadata.
- Added fixture-backed smoke coverage for release metadata generation. This is
  tooling/dry-run only; no release reset, tag push, GitHub Release publish, or
  Pages publish.
- Marketing download links now consume `/dl/releases.json` at runtime and fall
  back to the GitHub Releases page. Generated pages no longer infer latest
  asset URLs.
- `verify-release-assets` now validates `vX.Y.Z` release assets and concrete
  asset URLs. `VERSION` and `SHA256SUMS` are checked when present, not required.
- Updated non-graph manual/site copy for the metadata contract.

Verification:
- `make pre-push` passed after rebase. This exercised Rust fmt, clippy with
  warnings denied, all-target tests, no-default-features build, web
  check/build, and marketing-site checks.
- `npm run smoke:metadata` and `npm run check` passed in `web-marketing/`
  before the final rebase and are covered again by `make pre-push`.
- Metadata generator dry-run to `/tmp/chan-dl-smoke` passed and output was
  inspected.
- `git diff --check` passed.

Touched no `.github/workflows/`, Cargo.toml/Cargo.lock, Tauri source/config,
or graph surfaces.

## 2026-05-27 08:30 @@LaneC -> @@Architect
Slice 4 started.

Read the 08:28 merge/re-gate note. Rebasing LaneC worktree onto merged main
`96c9c17`.

Slice 4 scope is `.github/workflows/` CI/release wiring. This is the
shared-infra slice authorized by the Phase 11 release plan and @@Architect's
08:28 directive. I will make PR CI call the Make targets, keep Linux before
macOS, retarget release tags to `vX.Y.Z`, wire release asset verification and
metadata generation, and ensure all publish paths are gated on tag or explicit
manual release-cut inputs. No secret values will be written; signing secrets
are referenced by existing GitHub Actions secret names only.

## 2026-05-27 09:22 @@LaneC -> @@Architect
ready to merge: phase-11-lane-c@9163404f347669d2f5bce7ca3786b4ffaf4c32ad

Slice 4 is complete on current main `96c9c17`.

Shared-infra authorization: this workflow-YAML change is the Phase 11
release-plan CI slice, explicitly cleared by @@Architect at 08:28.

Gate-definition change, flagged clearly:
- PR CI now calls `make ci-linux` first. That target is the full local
  pre-push gate.
- PR CI then calls `make ci-macos` only after Linux passes. `make ci-macos`
  is now focused on Rust clippy/tests for macOS-specific coverage.

Release publish structure:
- Public release tags are `vX.Y.Z`.
- `workflow_dispatch` defaults to dry-run. It publishes only with
  `publish=true` plus an explicit `release_tag`.
- GitHub Release asset upload, asset verification, fresh `/dl/**` metadata
  generation, and Pages deploy are all behind tag push or explicit manual
  publish. No PR path or normal branch push path generates or publishes fresh
  `/dl/**` metadata.
- Normal `pages.yml` deploys preserve already published `/dl/**` metadata
  only; they do not generate new release metadata.

Signing-secret discipline:
- No secret values are in YAML, docs, journals, or commit text.
- Workflows reference secret NAMES only:
  `APPLE_CERTIFICATE_BASE64`, `APPLE_CERTIFICATE_PASSWORD`,
  `APPLE_SIGNING_IDENTITY`, `APPLE_TEAM_ID`, `APPLE_ID`, `APPLE_PASSWORD`,
  `TAURI_SIGNING_PRIVATE_KEY`, and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

What changed:
- `release.yml` now owns ordered release CI: Linux validation, Linux CLI and
  desktop artifacts, macOS validation, macOS CLI and signed/notarised desktop
  artifacts, upload, verify, metadata generation, Pages deploy.
- `release-desktop.yml` is manual package dry-run only, so it cannot race the
  unified release publisher on tag pushes.
- Added release asset collection tooling that hashes uploaded GitHub Release
  assets and carries detached desktop updater signatures into the metadata
  manifest.
- `verify-release-assets` now requires the macOS updater payload and detached
  signature asset.
- PR/issue templates now match the release plan: `make pre-push` is the PR
  gate and feature issues lead with the problem, with the proposed solution
  explicitly optional.

Verification:
- `make pre-push` passed.
- `npm run check` passed in `web-marketing/`.
- Ruby parsed all workflow YAML files.
- `bash -n docs/release/populate-apple-secrets.sh` passed.
- `git diff --check` passed.
- `make -n` dry-runs passed for `ci-linux`, `ci-macos`,
  `chan CHAN_TARGET=aarch64-apple-darwin`, `linux-chan-tarball`, and
  `macos-chan-dmg-notarised`.

Touched no Cargo.toml/Cargo.lock, Tauri config/source, or graph surfaces.
