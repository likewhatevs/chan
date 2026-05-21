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

## 2026-05-21 — implementation + commit readiness

Implemented shape (a1) `#[ignore = "..."]` per the
empirical test list surfaced by `systacean-17-smoke` run
[`26235956637`](https://github.com/fiorix/chan/actions/runs/26235956637).
14 tests gated; 14 lines added across 2 files.

### Why (a1) over (a2)

Architect's preferred shape was (a2) `#[cfg(feature =
"embed-model")]`. Audited the chan-drive `Cargo.toml`
features: `default = ["embeddings"]`, plus `metal` /
`cuda` GPU backends. **chan-drive does NOT declare
`embed-model`**; that feature lives in chan-server
(controls whether the BGE-small bytes are rust-embedded
into the binary).

To use (a2) I would have had to:
1. Add a dummy `embed-model` feature to
   `crates/chan-drive/Cargo.toml` with no deps.
2. Gate 14 tests on that feature.

The dummy feature would carry no actual code (no deps,
no `#[cfg]` branches outside tests), making it a
test-only flag that re-uses the `embed-model` name for a
semantically different purpose. The architect's task body
explicitly anticipated this case: "If (a2) introduces
awkward `#[cfg]` boilerplate at module scope (helper
functions used by both gated and ungated tests), fall
back to (a1) `#[ignore]`."

(a1) `#[ignore]` is:
* No `Cargo.toml` changes.
* Tests stay discoverable via the standard `cargo test`
  output ("16 ignored").
* Opt-in via the standard `cargo test -- --ignored` flag.
* Skip reason carried in the attribute string so the
  user reads "requires BGE-small embedding model" in the
  test output.

### Gating

All 14 tests gated with the same skip reason:

```rust
#[test]
#[ignore = "requires BGE-small embedding model on disk; run with `cargo test -- --ignored` on a workstation with the model cached (see systacean-18)"]
fn <name>() { ... }
```

* `crates/chan-drive/src/drive.rs` (12 tests, +12 lines):
  `link_targets_finds_file_after_index`,
  `index_file_stamps_pre_read_stat_so_concurrent_writes_stay_visible`,
  `pending_writes_journal_handles_forget_op`,
  `pending_writes_journal_is_empty_on_a_clean_path`,
  `pending_writes_journal_replay_converges_after_simulated_crash`,
  `pending_writes_replay_degrades_index_op_to_forget_when_file_is_gone`,
  `reconcile_catches_same_mtime_different_size_rewrite`,
  `reconcile_on_empty_graph_indexes_everything_like_a_fresh_reindex`,
  `reconcile_picks_up_files_added_offline`,
  `reconcile_picks_up_modified_files`,
  `resolve_link_returns_contact_kind_for_contact_node`,
  `resolve_link_returns_file_kind_for_plain_note`.
* `crates/chan-drive/src/indexer.rs` (2 tests, +2 lines):
  `debounce_coalesces_rapid_writes_into_one_index`,
  `writes_to_disk_get_indexed_after_debounce`.

Empirical list above is what the Ubuntu runner panicked
on in `26235956637` — not the architect's line-number
list (which was a close-but-not-exact approximation).
Some architect-listed tests (`reindex_consumes_pending_rename_log_after_reopen`,
`stat_uses_lstat_for_symlinks`, `resolve_link_path_escape_rejected`)
were NOT in the empirical panic list, so they don't
need gating. Three other tests
(`link_targets_finds_file_after_index`,
`resolve_link_returns_file_kind_for_plain_note`,
`pending_writes_journal_is_empty_on_a_clean_path`) WERE
in the empirical list but weren't in the architect's
line-number callout; gating them too per the empirical
evidence.

### Local verification

Default `cargo test -p chan-drive`:

```
test result: ok. 411 passed; 0 failed; 16 ignored; 0 measured; 0 filtered out
```

Was previously `425 passed; 2 ignored` (425 - 14 = 411;
2 + 14 = 16). Two pre-existing ignored tests carry
forward; my 14 are the delta.

`cargo test -p chan-drive -- --ignored` on this
workstation (with the BGE-small model cached at
`~/.cache/chan/models/models--BAAI--bge-small-en-v1.5/`):

```
test drive::tests::pending_writes_journal_replay_converges_after_simulated_crash ... ok
test drive::tests::pending_writes_replay_degrades_index_op_to_forget_when_file_is_gone ... ok
test drive::tests::link_targets_finds_file_after_index ... ok
test drive::tests::pending_writes_journal_is_empty_on_a_clean_path ... ok
test drive::tests::pending_writes_journal_handles_forget_op ... ok
test drive::tests::resolve_link_returns_contact_kind_for_contact_node ... ok
test drive::tests::reconcile_picks_up_files_added_offline ... ok
test drive::tests::resolve_link_returns_file_kind_for_plain_note ... ok
test drive::tests::reconcile_on_empty_graph_indexes_everything_like_a_fresh_reindex ... ok
test drive::tests::reconcile_catches_same_mtime_different_size_rewrite ... ok
test drive::tests::reconcile_picks_up_modified_files ... ok
test indexer::tests::debounce_coalesces_rapid_writes_into_one_index ... ok
test indexer::tests::writes_to_disk_get_indexed_after_debounce ... ok
test drive::tests::index_file_stamps_pre_read_stat_so_concurrent_writes_stay_visible ... ok
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 411 filtered out; finished in 18.27s
```

All 14 gated tests run + pass with `--ignored` on a
workstation with the model cached. Total = 411 + 16 =
427; matches pre-gating's `425 + 2 = 427`. **No coverage
loss** — power users / local dev keep the full suite via
`--ignored`.

### Pre-push gate

All green at HEAD `663ab26` (post-`-17` commit):

* `cargo fmt --check` — clean.
* `cargo clippy --all-targets -- -D warnings` — clean.
* `cargo test` (workspace) — 411+16 chan-drive; 205
  chan-server; 20 chan-report; 29 chan-llm; all other
  crates green; no drift.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`
  — green.
* `cd web && npm run check` — 0e / 0w / 3987 files.
* `cd web && npm test -- --run` — 600 / 600 passed.

### Suggested commit subject

```
chan-drive: gate 14 model-dependent tests behind #[ignore] (systacean-18)
```

### Files

| File                                    | +     | -    |
|-----------------------------------------|-------|------|
| crates/chan-drive/src/drive.rs          | +12   | 0    |
| crates/chan-drive/src/indexer.rs        | +2    | 0    |
| **Total**                               | +14   | 0    |

Plus this task tail append. Single-purpose commit.

### Shared-worktree discipline

Working tree is dirty with several foreign in-flight
files (event channels from concurrent agents, ci/webtest
task tails, the architect's journal). Pre-stage explicit
`git add` of exactly:

```
crates/chan-drive/src/drive.rs
crates/chan-drive/src/indexer.rs
docs/journals/phase-8/systacean/systacean-18.md
docs/journals/phase-8/alex/event-systacean-architect.md
```

Pre-commit `git diff --staged --stat` audit + post-commit
`git show --stat HEAD` audit will confirm exactly 4 paths
staged.

### Round-3 (b) follow-up

Per the task's "Optional follow-up note": shape (b)
deterministic-fixture / mock-embedder for the affected
tests is Round-3 cleanup territory. With (b) in place,
the tests could exercise chunking + embedding-orchestration
logic against a tiny in-tree mock without needing the
real BGE model on disk. Out of scope here.

### CI smoke after commit

Same shape as `-17`: push the commit (plus the prior
`f4a197d` + `663ab26` queue) to a
`systacean-18-smoke` branch + `gh workflow run ci.yml`.
Expected outcome: `clippy + test (ubuntu-latest)`
passes (14 BGE tests skipped instead of panicking).
Windows still reds on the chan-desktop dead_code lints
(pre-existing, out of scope here per `-17`'s smoke
report).

Holding for @@Architect commit clearance + smoke-dispatch
decision (push to branch OR fold into main push).
