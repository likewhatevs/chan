# ci-1: GitHub Actions scaffold — lint + test + build matrix

Owner: @@CI
Date: 2026-05-19

## Goal

Land the first GitHub Actions workflow set so the chan repo
runs the same gate CI would, on every push to `main` and on
every PR.

Target gate (matches `scripts/pre-push` locally):

* `cargo fmt --check`
* `cargo clippy --all-targets -- -D warnings`
* `cargo test`
* `cargo build --no-default-features`
* `web/`: `npm ci`, `npm run check`, `npm run test`,
  `npm run build`

Matrix: Linux + macOS (Windows can land later if it doubles the
runtime; default is to include from day one and let the runtime
pressure decide).

## Background

Source in [`../request.md`](../request.md) and the phase-8
backlog item 8 in
[`../../phase-7/next-phase-backlog.md`](../../phase-7/next-phase-backlog.md).

The repo is currently private; CI runs against private-repo
minutes. Open-sourcing is a Round-2 ask and not a blocker for
this task.

## Acceptance criteria

* Workflow file(s) under `.github/workflows/` that run the gate
  above on push + PR.
* Rust toolchain pinned from `rust-toolchain.toml` (rustup auto-
  installs).
* Caching configured for `cargo` and `npm` so the gate runs in
  reasonable time.
* Green on the current `main` (v0.11.0) without modification —
  i.e. the workflow reflects the *actual* gate that exists
  today.
* No secrets used in this scaffold; signing / release flows
  land in `ci-2`.

## How to start

* `.github/workflows/ci.yml` is the natural home.
* Reference `scripts/pre-push` for the exact command sequence.
* Reference `crates/fetch-models/` and `make models` — release
  builds bundle the BGE-small model; ci-1 does not need to run
  `make models` because it's a build helper, but flag in this
  task file if any test depends on the bundled model being
  present.

## 2026-05-19 — landed (ready for review)

Owner: @@CI.

### What was already there

`.github/workflows/ci.yml` already covers the Rust half of the
gate the task asked for:

| Job                  | Runner(s)              | Commands                                                  |
|----------------------|------------------------|-----------------------------------------------------------|
| fmt                  | ubuntu-latest          | cargo fmt --check                                         |
| test                 | ubuntu + windows       | cargo clippy --all-targets -- -D warnings, cargo test     |
| no-default-features  | ubuntu-latest          | cargo build --no-default-features                         |

Toolchain pinned via `actions-rust-lang/setup-rust-toolchain@v1`
reading `rust-toolchain.toml`. `Swatinem/rust-cache@v2` covers
cargo caching. RUSTFLAGS=-D warnings env-wide.

Existing windows-latest job also smoke-builds `cargo build
--release -p chan` with `-C target-feature=+crt-static` to catch
release-flag link regressions per-PR (commit dbc6ec6).

macOS clippy+test was scoped to `release.yml` per commit e45654a
("ci: scope macos to release-time only") — an explicit cost call,
not an oversight. macOS Actions minutes bill at 10x. The ci-1
spec lists "Linux + macOS" as the default; I am preserving the
existing Linux + Windows shape and flagging the trade-off below
rather than reverting @@Systacean's earlier call without
@@Architect input.

No test depends on the bundled BGE-small model being present;
release.yml runs `cargo run --release -p fetch-models` at build
time, and the per-PR jobs do not need it.

### What I added

New `web` job in `.github/workflows/ci.yml` (Linux only):

| Step               | Command                                              |
|--------------------|------------------------------------------------------|
| Node setup         | actions/setup-node@v4, node 20, npm cache            |
| Install            | npm ci (working-directory chan/web)                  |
| Type check         | npm run check (svelte-check)                         |
| Unit tests         | npm run test (vitest run)                            |
| Bundle build       | npm run build (vite build)                           |

Mirrors the existing subdir-checkout pattern (`path: chan`) so
the npm cache key reads `chan/web/package-lock.json` like
release.yml does. Linux-only — npm toolchain is portable;
OS-specific frontend regressions are rare and not worth the
matrix cost.

Top-of-file comment updated to drop the stale "web bundle is NOT
built in CI today" note.

### Local sanity check on main (v0.11.0)

| Step             | Result                                                 |
|------------------|--------------------------------------------------------|
| npm run check    | 3964 files, 0 errors, 0 warnings                       |
| npm run test     | 43 test files, 446 tests, all pass (13s)               |
| npm run build    | built in 2.28s, vite chunk-size warnings only          |

Green on current main without source modification, matching the
ci-1 acceptance criterion.

### Open question for @@Architect

Per-PR macOS coverage. The ci-1 spec lists Linux + macOS as the
default matrix; current ci.yml uses Linux + Windows with macOS
deferred to release.yml. Three options:

1. Keep current shape (Linux + Windows per-PR, macOS at tag
   time). Lowest minutes spend; matches commit e45654a's explicit
   cost call.
2. Add macOS to the per-PR `test` job alongside Linux + Windows.
   Adds ~10x minutes on every PR; catches macOS-specific breakage
   earlier.
3. Add a lighter macOS lane (e.g. fmt + clippy only, skip the
   full test run). Middle ground.

Recommendation: option 1 (no change) unless a macOS-only
regression has bitten us. Flagging for @@Architect to decide
rather than overriding silently.

### Files changed

* `.github/workflows/ci.yml` — added `web` job, updated header
  comment.

No `crates/` or `web/` source touched. Lane boundary respected.

### Tests / verification

* Local: `npm run check`, `npm run test`, `npm run build` all
  pass under `web/` on v0.11.0.
* Workflow YAML structurally validated (jobs present: fmt, test,
  no-default-features, web).
* The actual job will run on the next push / PR; cannot exercise
  GitHub Actions from the working tree.

### Commit readiness

Not committing per the rule "do not commit unless @@Architect or
@@Alex tells you to." Files in working tree:

* `.github/workflows/ci.yml`

Proposed commit message:

```
ci: gate web/ on every PR (svelte-check + vitest + vite build)

Existing ci.yml ran fmt + clippy + test + no-default-features but
left the frontend bundle uncovered on push/PR; only release.yml
exercised it at tag time. Add a Linux-only `web` job that runs
npm ci + npm run check + npm run test + npm run build so a
broken bundle fails per-PR.
```

## 2026-05-19 — @@Architect: option 1 + commit clearance

Reviewer: @@Architect.

**Open question — macOS per-PR coverage**: going with **option 1**.
Keep the current Linux + Windows shape on per-PR; macOS stays
deferred to `release.yml` per commit e45654a's explicit cost
call. We have not been bitten by macOS-only regressions yet, and
the 10x minutes ratio is real; if a macOS-only break ships, we
escalate to option 3 (lighter macOS lane: fmt + clippy only) as
a follow-up.

**Commit clearance**: approved for the `.github/workflows/ci.yml`
edit as proposed. Commit message stands. Push waits for the
Round-1 close commit-grouping plan; do NOT push yet.

Carry on with `ci-2` (release CI scaffold) once committed.
