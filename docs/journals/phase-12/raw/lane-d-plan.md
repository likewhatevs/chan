# Lane D plan: CI + release (phase 12)

Added mid-phase 2026-05-27 by @@Alex - the release/build lane the opening
`bootstrap.md` left room for. @@LaneD = the CI + release lane. @@Lead (the
@@Architect orchestrator seat) serializes merges + re-gates; @@Alex launches
this session and rules on scope / infra / version calls.

Recover from: this plan + `bootstrap.md` (shared protocol: worktrees, the
merge/gate gate, the coordination bus) + `coordination/README.md`. Your
channels: `coordination/event-architect-lane-d.md` (in), `event-lane-d-architect.md`
(reports out), `event-lane-d-alex.md` (escalation). main baseline: `fe6e126`
(phase-11 close, version `0.15.5`, NOTHING pushed to origin yet).

## Mission (two phases)

1. NOW: fix the current CI issues. INVESTIGATE them yourself - do not take a
   diagnosis from @@Lead. The CI/release machinery exists but the GitHub Actions
   side is UNPROVEN: per the phase-11 carryover, the first origin push was meant
   to be the shakedown of `make ci-linux` / `ci-macos` + `release.yml`, and it
   has not happened. Find what is broken or latent and make CI green.
2. ALIGN for the next release: a PATCH on top of `0.15.5`, cut AFTER @@LaneA +
   @@LaneB + @@LaneC land. Get the release machinery ready so the cut is a clean,
   repeatable step once those merge.

## Where CI/release lives (orientation, NOT a diagnosis)

- `.github/workflows/`: `ci.yml`, `pages.yml`, `release-desktop.yml`,
  `release.yml`.
- `Makefile`: `ci-linux` (pre-push + Linux validation), `ci-macos` (focused
  macOS validation), `ci-release` (local release validation), `models`
  (pre-fetch the embedded search model).
- Recent commits: `9163404 ci: align release workflows with the metadata
  contract`, `fe6e126 chore(release): 0.15.5`.
- Phase-11 release carryover (ASSESS + propose in/out; do NOT assume scope):
  - Slice 5: Tauri desktop updater UX (Check-for-Updates menu, prompt, signed
    payloads) -> `/dl/desktop/latest.json` + a Tauri dep bump (Cargo.lock).
  - Slice 6: graph manual/site copy - BLOCKED on @@LaneA GI-10 + loading-state.
- This box has lima + sdme for local Linux validation (aarch64); CI still owns
  x86_64. Reproduce CI LOCALLY (`make ci-macos` natively; `make ci-linux` via
  `limactl shell default sudo sdme ...`). Do NOT rely on an origin push to
  discover failures - see "Pushing" below.

## How @@LaneB's rename affects the release (coordinate with @@LaneB)

@@LaneB is renaming the `chan-drive` crate -> `chan-workspace` with a FULL CLEAN
BREAK on the on-disk registry + HTTP routes (existing registries/bookmarks break,
no migration, accepted pre-release; the tunnel domain `drive.chan.app` is the one
preserved "drive" string). For the release that means: Cargo package names +
Cargo.lock churn, possible release-artifact / `install.sh` / `/dl` naming, and a
CHANGELOG entry calling out the breaking clean break (still a PATCH per @@Alex -
0.x semver permits it). So the release CUT lands AFTER @@LaneB. Declare any
release-config touch that overlaps @@LaneB's crate rename on
`event-lane-d-lane-b.md` (created on first use). You do NOT touch `web/src` -
zero overlap with @@LaneA / @@LaneC source.

## Boundaries + authorization

- @@Alex AUTHORIZES @@LaneD to edit shared infra: `.github/workflows/`, the
  Makefile CI/release targets, signing/release config, `scripts/`, `/dl`
  tooling, `CHANGELOG`, and the workspace version in `Cargo.toml`. State this
  authorization inline in the relevant commit message / journal entry so the
  change is grounded for the auto-classifier.
- SECRETS: signing/credential secret VALUES NEVER appear in journals, chat, or
  commits. Reference secret NAMES only, consumed via GitHub Actions Secrets.
  Surface any new secret @@Alex must provision on `event-lane-d-alex.md`.
- PUSHING: the first origin push is a LOAD-BEARING coordinated event (it fires
  CI over all of phase 11). Do NOT push to origin or cut/tag a release
  unilaterally. Investigate + fix locally; when CI is green locally and you are
  ready, report ready-to-merge to @@Lead - the push / release cut happens on
  @@Alex + @@Lead's go.

## Worktree + merge/gate

- Create worktree `../chan-lane-d` on branch `phase-12-lane-d` from main
  (`git worktree add`). Source/config edits in the worktree ONLY; channels +
  this plan + your journal are edited in the MAIN checkout by absolute path.
- Keep `docs/journals/phase-12/lane-d/journal.md` self-documenting +
  append-only.
- Full gate before any ready-to-merge: `cargo fmt --check`; `cargo clippy
  --all-targets -- -D warnings`; `cargo test`; `cargo build
  --no-default-features`; in `web/`: `npm run check` + `npm run build` when web
  is touched. Report ready-to-merge as `phase-12-lane-d@<sha>` on
  `event-lane-d-architect.md`; @@Lead serializes + re-gates before main.
- Need an unblock? Cut a TASK to @@Lead (the Task tool). @@Lead auto-resolves the
  routine; contention / high-stakes goes to @@Alex.

## First step

Identify as @@LaneD, create the worktree, write your kickoff entry +
investigation plan to your journal, and START INVESTIGATING the current CI state
(local `make ci-*` runs, read the workflow YAML, find the failures). Post
findings + a fix/sequencing plan on `event-lane-d-architect.md`. @@Lead reviews
before you execute fixes that touch shared infra - same green-light flow the
other lanes got.