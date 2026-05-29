# systacean-5: round-1 closeout (patch bump + Chan.app + push)

Owner: @@Systacean
Cut by: @@Architect
Date: 2026-05-18

## Goal

Close out round 1: bump patch version, build the Chan.app
desktop bundle, push `main` to origin so other hosts (incl.
@@Alex's Linux box) can pull and rebuild. Wave-1.5 work
queues behind this and resumes after @@Alex recycles agent
sessions against the new version.

## Relevant links

* [../architect/journal.md](../architect/journal.md) for the
  wave-1 commit list + commit-order plan.
* [../request.md](../request.md) Round 2 preamble — @@Alex's
  closeout direction.
* Pinned toolchain in `rust-toolchain.toml` (1.95.0).

## Sequencing dependencies (do NOT start until all in)

* `6c53c2d` systacean-1 ✓
* `87a9a36` fullstack-1 ✓
* `c03d6f2` fullstack-5 + autosave ✓
* `1a937e8` systacean-2 ✓
* `fullstack-3` — @@Alex authorized 15:25 BST; @@FullStack to
  commit imminently.
* `fullstack-2` — revising for tunnel-aware Tauri
  `shell.open`; needs @@WebtestA's `webtest-a-3` walkthrough
  before commit clearance.

Watch the FullStack event file. When both `fullstack-3` and
`fullstack-2` commits land, start here.

## Acceptance criteria

### Patch version bump

* Current version: **0.10.0** (per `Cargo.toml` workspace
  + `CLAUDE.md` reference). Bump to **0.10.1**.
* Touch every version-string location coherently:
  * Workspace `Cargo.toml` `[workspace.package].version`.
  * Any member `Cargo.toml` that pins its own version
    (run `cargo metadata` / grep to enumerate).
  * `web/package.json` `version`.
  * `desktop/src-tauri/Cargo.toml` (the Tauri shell) and
    `desktop/src-tauri/tauri.conf.json` `version`.
  * Any embedded version string the chan binary uses for
    self-reporting (`chan --version`).
* `cargo update --workspace` after the bump if needed to
  keep the lockfile consistent.
* Single commit titled e.g. `chore: bump version to 0.10.1`
  matching the style of `f1f7c8c` (the 0.10.0 bump).

### Pre-push gate (local; macOS only, no CI keys yet)

Run, in order, all green before push:

* `cargo fmt --check`
* `cargo clippy --all-targets -- -D warnings`
* `cargo test`
* `cargo build --no-default-features` (the no-default-features
  variant per CLAUDE.md)
* `cd web && npm run check && npm run test && npm run build`

### Chan.app desktop build

Build the Tauri desktop bundle so @@Alex can install /
re-launch on a fresh version.

* Workflow lives under `desktop/`. Use whatever the project
  documents (`pnpm tauri build` / `npm run tauri build` /
  `cargo tauri build` — check the Makefile and desktop
  README for the canonical invocation).
* Target macOS (where @@Alex runs the desktop shell). Linux
  / Windows can wait.
* Confirm the bundle launches (open the produced `.app`
  bundle, verify it reaches a chan welcome screen).
* If signing / notarization is set up, run it. If not,
  produce an unsigned bundle and note that in your
  commit-readiness append.
* DO NOT commit the bundle artifact itself unless the repo
  already tracks release artifacts (it likely doesn't).

### Push to origin

* Push `main` to `origin/main`. Do NOT force-push.
* If a release tag is appropriate (look at history — past
  patch bumps may or may not be tagged), tag `v0.10.1` and
  push the tag.
* DO NOT skip hooks; pre-push hook should fire and pass.

## Out of scope

* Wave-1.5 tasks (`fullstack-6`, `fullstack-7`,
  `systacean-3`, `systacean-4`, `webtest-b-2`) — they sit
  in the working tree as task files for the
  post-recycle agents to pick up.
* CHANGELOG / release notes beyond the version bump commit
  message (do them if the repo has a CHANGELOG.md; check
  first).

## Hand-off

Standard commit-readiness shape. After push:

1. Append a section confirming what shipped (commit SHAs,
   version bump SHA, Chan.app bundle path).
2. Fire `alex/event-systacean-architect.md` (type `poke`)
   pointing at it.
3. @@Architect will broadcast "round 1 closed; safe to
   recycle agents" to @@Alex.

## 2026-05-18 16:55 BST - closeout shipped

Pushed `main` and tag `v0.10.1` to origin.

Commits landed after the wave-1 queue:

* `3ab0aac` - @@FullStack `fullstack-2`, external links via
  desktop shell.
* `f8014a9` - restore terminal prompt mode toggle. This was
  needed because `npm run test` caught that source-mode
  terminal prompts could no longer switch back to rendered
  mode after the shared toolbar unification.
* `f996f4c` - `systacean-4`, `chan open <dir>` enters the
  directory listing.
* `9e48367` - `systacean-5`, bump version to `0.10.1`.

Version bump touched:

* `Cargo.toml`
* `Cargo.lock`
* `web/package.json`
* `web/package-lock.json`
* `desktop/src-tauri/tauri.conf.json`

Gate:

* `cargo fmt --check`
* `cargo clippy --all-targets -- -D warnings`
* `cargo test`
* `cargo build --no-default-features`
* `cd web && npm run check`
* `cd web && npm run test`
* `cd web && npm run build`
* `scripts/pre-push` from a clean Rust target cache after
  deleting a 53G `target/` tree.

Desktop bundle:

* App bundle path:
  `target/release/bundle/macos/Chan.app`
* The DMG-producing `make build` path built and signed
  `Chan.app` first, then failed in `bundle_dmg.sh`.
* Per @@Alex, `/Applications/Chan.app` was not replaced
  because the running Chan.app could not be deleted safely.
* Rebuilt app-only with `cargo tauri build --bundles app`.
  That produced an ad-hoc signed app bundle; no notarization
  credentials were present.

Disk cleanup:

* Removed `target/` and any desktop nested target cache,
  reclaiming the old 53G Rust build tree.
* After the clean rebuild, `target/` is about 11G.
