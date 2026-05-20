# systacean-3: Round-1 close — patch version bump + tag + push

Owner: @@Systacean
Date: 2026-05-19

## Goal

Once Round 1's bug fixes have all landed on `main` (per
@@Architect's commit-grouping plan), cut the patch release.

* Bump `Cargo.toml` workspace version from `0.11.0` →
  `0.11.1` (or whatever @@Architect picks at close).
* Tag `chan-v0.11.1` on the release commit.
* Push branch + tag to the remote.
* If @@CI has the release workflow wired by close, the tag
  push triggers an artifact build; otherwise this is a local-
  only release for now and the artifact wiring carries to
  Round 2.

## Background

Mirrors phase-7's wave-1 closeout flow. Source in
[`../request.md`](../request.md) under "Round 1 — bug sweep +
new build".

## Acceptance criteria

* All wave-1 fixes landed on `main`.
* Pre-push gate green: `cargo fmt --check`, `cargo clippy
  --all-targets -- -D warnings`, `cargo test`, `web/npm run
  check`, `web/npm run build`, `scripts/pre-push`.
* Version bumped in every `Cargo.toml` that pins a workspace-
  member version.
* Tag created with the standard release-commit message shape.
* Push completed; commit + tag confirmed on the remote.
* @@Architect notified via poke event.

## How to start

Wait until @@Architect appends the commit-grouping plan to this
task file (or a sibling `architect-N.md`) listing the order in
which the fixes should land. Do not version-bump or tag until
@@Architect signals Round-1 close.
