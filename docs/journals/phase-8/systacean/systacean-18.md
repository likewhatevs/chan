# systacean-18 — chan-drive tests skip / feature-gate on missing BGE-small model

Owner: @@Systacean
Cut: 2026-05-21 by @@Architect
Status: dispatched

## Goal

Unblock `ci.yml::test (ubuntu-latest)` cargo test step
which currently panics with 14 test failures because the
BGE-small embedding model isn't cached on the CI runner.

Make the tests skip cleanly (via `#[ignore]` or
feature-gating on `embed-model`) so the default-build CI
shape — which intentionally does NOT include the embedded
model — runs the test suite green.

## Background

Surfaced 2026-05-21 by @@CI during `ci-12` smoke validation.
See [`../alex/event-ci-architect.md`](../alex/event-ci-architect.md)
"Out-of-lane finding: model-dependent tests" + ci-12's
post-mortem note for the empirical trace.

Failure mode (from `ci-12-smoke` workflow_dispatch):

```
called `Result::unwrap()` on an `Err` value:
  Search("embedding model 'BAAI/bge-small-en-v1.5' not
  downloaded; expected at \"/home/runner/.cache/chan/models/
  models--BAAI--bge-small-en-v1.5\". Run
  `chan index download-model` or rebuild with
  `--features embed-model`.")
```

Origin: `systacean-6` + `-7` made the BGE-small bundle
opt-in (default builds drop the ~140 MB embed). The
chan-drive tests panic when they hit the embedding path
without the model present. Affected tests (from @@CI's
audit):

* `crates/chan-drive/src/drive.rs:{3365, 3442, 3478,
  3522, 3589, 3670, 3735, 3782, 3845, 4659, 4806, 4818}`
  — 12 tests.
* `crates/chan-drive/src/indexer.rs:{378, 444}` — 2 tests.

Total: 14 tests.

Hidden behind the GTK gap until ci-12 unmasked them. Was
red on CI for the last ~15 commits' worth of unverified
main; the local pre-push hook didn't catch it because dev
workstations have the model cached.

## Decision: fix shape (a) — `#[ignore]` or feature-gate

Per @@CI's recommendation + @@Architect routing: shape (a)
for the immediate unblock. The deterministic-fixture shape
(b) is Round-3 cleanup territory; not pursuing now.

Two sub-options for shape (a):

* **(a1) `#[ignore]` with descriptive reason**: tests
  remain in the suite, skip by default, runnable via
  `cargo test -- --ignored` on a workstation with the
  model. Smallest diff; preserves test discoverability.
* **(a2) Feature-gate on `embed-model`**: `#[cfg(feature
  = "embed-model")]` on the affected tests. They compile +
  run only when the model is bundled. Cleaner semantic
  (the tests legitimately depend on the model being
  present); makes the gating explicit at the type level.

**Recommend (a2)**: the tests genuinely need the model;
feature-gating expresses that as a build-level invariant
rather than an opt-in runtime skip. Power users running
`cargo test --features embed-model` get the full coverage;
default CI runs skip cleanly.

If (a2) introduces awkward `#[cfg]` boilerplate at module
scope (helper functions used by both gated and ungated
tests), fall back to (a1) `#[ignore]`. Pick the cleaner
shape per the actual code layout.

## Acceptance criteria

### Audit + gate the affected tests

1. Read each of the 14 test sites from the file:line list
   above. Confirm which tests genuinely call into the
   embedding path vs. tests that incidentally trip on it
   via a shared setup helper.
2. For tests that DIRECTLY exercise the embedding path:
   apply `#[cfg(feature = "embed-model")]` (preferred) or
   `#[ignore = "requires BGE-small embedding model"]`
   (fallback).
3. For tests that incidentally trip via a shared helper:
   either move the embedding call out of the shared helper
   so unrelated tests don't depend on the model, OR gate
   the helper itself and accept the broader gating.

### Local verification

1. `cargo test -p chan-drive` — should pass with the 14
   tests skipped / not compiled. Confirm via the count
   delta vs. the baseline test count.
2. `cargo test -p chan-drive --features embed-model` —
   should run all tests INCLUDING the 14 gated ones.
   Confirm by re-running with `--features embed-model`
   and verifying the count matches the original
   pre-gating count.
3. `cargo fmt --all`; `cargo clippy --all-targets -- -D
   warnings`. All green.

### CI verification

1. After the patch lands on a branch, `gh workflow run
   ci.yml --ref <branch>`. Confirm:
   * `test (ubuntu-latest)` reaches the cargo test step
     and passes (no more 14-test panic).
   * Test count delta matches the skipped / not-compiled
     14.
   * No regression on other matrix entries.

### Optional follow-up note (Round-3 cleanup)

Append a note to the task tail flagging the deterministic-
fixture shape (b) for Round-3 cleanup. Shape (b) would
introduce a tiny in-tree fixture (or a deterministic mock
embedder) so the affected tests can exercise the chunking
+ embedding-orchestration logic without requiring the real
model. NOT in scope for `-18`; cleanup-track candidate.

## How to start

1. Read `crates/chan-drive/src/drive.rs` + `indexer.rs` at
   the named line ranges; identify the actual embedding
   call sites + test scope.
2. Pick `#[cfg(feature = "embed-model")]` vs `#[ignore]`
   per the per-site readability.
3. Apply the gating patch.
4. Local pre-push gate green workspace-wide.
5. CI smoke via `gh workflow run ci.yml` on the patch
   branch.
6. Append "Commit readiness" + fire poke to @@Architect.

## Coordination

* @@Systacean lane (chan-drive primary scope).
* No interaction with @@CI's ci-12 fix needed; layers on
  top once ci-12 lands.
* Pre-push gate green workspace-wide before commit
  clearance.

### Shared-infra authorization

This task touches `crates/chan-drive/src/{drive,indexer}.rs`
Rust source. No `.github/workflows/` edits expected. If
the gating needs a workspace-level Cargo feature
declaration update (e.g., `embed-model` feature
propagation through the workspace), that's a one-line
addition to `crates/chan-drive/Cargo.toml` + acceptable
within this task.

## Numbering

Highest committed `systacean-N` is `-14`; `-15` is
in-flight (cleared but uncommitted); `-16` is queued;
`-17` is queued (Windows lint, gate-unblocker); this is
`-18`.

### Queue order (revised 2026-05-21)

`-15` (committable now) → `-17` (Windows lint;
gate-unblocker) → `-18` (this task; gate-unblocker) →
`-16` (file-class buckets; feature work). After `-17` +
`-18` both land, the per-PR CI gate goes fully green for
the first time since ~2026-05-19.

## Out of scope

* Deterministic-fixture / mock-embedder shape (b) —
  Round-3 cleanup candidate.
* Restructuring chan-drive's embedding integration. Stay
  narrow.
* Pre-fetching the model in CI (shape (c) from @@CI's
  analysis) — ~30-60s + 140 MB per run; doesn't match the
  default-build shape we want CI to validate.
* Auditing OTHER test failures on CI beyond the 14
  model-dependent ones. If you spot adjacent issues,
  surface as separate tasks or bug-list entries.
