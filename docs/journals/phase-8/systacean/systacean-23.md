# systacean-23 — chan-drive macOS indexer test flakiness: writes_to_disk_get_indexed_after_debounce

Owner: @@Systacean
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Diagnose and fix (or quarantine) the macOS-only
flakiness in
`crates/chan-drive/src/indexer.rs:385`'s test
`writes_to_disk_get_indexed_after_debounce`. The
test panics with `expected watched.md in search hits;
got []` on `macos-latest` CI runners.

Empirical history: green on `ci-13-smoke-v2`
(`26254608202`, 2026-05-21); red on `ci-14-smoke`
(`26274161414`, 2026-05-22). No code change to the
indexer/debounce path between those runs — the only
chan-server-side commits since `ci-13-smoke-v2` are
`fullstack-b-25` (chan-desktop) + `systacean-21`
(event_watcher schema + dispatch_agent_event
template) + `fullstack-a-58` (graph parent-edge
rendering; SPA-only) + `systacean-22` (graph route
contact filter; chan-server but routes side, not
indexer).

## Reference

* CI verdict: `26274161414` macos-latest cargo test
  step.
* Cross-lane finding from @@CI's `-14` audit (commit
  `ce3a269`); flagged in
  [`../alex/event-ci-architect.md`](../alex/event-ci-architect.md)
  tail "macOS indexer test failure".

## Diagnosis hooks

Two hypothesis classes:

### H1 — Recent regression masked by macOS event delivery

The debounce-then-search probe relies on chan-drive's
indexer running on FSEvent-delivered notifications.
macOS FSEvent ordering CAN differ from Linux inotify;
a recent commit may have introduced a latent
dependency on inotify-style ordering.

Hooks:
* Spot-check `crates/chan-drive/src/indexer.rs:385`'s
  test setup. What's the debounce window? Is it
  long enough to absorb macOS FSEvent jitter?
* Compare what passes on Linux (`ubuntu-latest`
  green on the same run) vs fails on macOS. Single
  test or multiple?

### H2 — macOS-specific timing flakiness

The test may have been flaky on macOS all along but
the macOS matrix was only just-added (per `ci-13`).
Test debounce was tuned against Linux; macOS just
needs longer/different timing.

Hooks:
* Re-run the smoke against current HEAD on macOS to
  see if it reproduces consistently OR is genuinely
  flaky.
* Check the debounce constants in `chan_drive::watcher`
  + the test's poll-loop deadlines.

## Fix options

Three reads, picker's call after audit:

* **(A) Fix the underlying timing**: tune debounce
  constants OR add a longer wait_for in the test on
  macOS. Cleanest; preserves test value.
* **(B) `#[cfg(not(target_os = "macos"))]` gate**:
  same shape as the existing `-20` lock-contract
  + helpers gates (per
  `feedback_chan_invariants`). Skips the test on
  macOS until a proper fix lands. Documents the
  gap in `phase-8-bugs.md`. Lower-cost; preserves
  CI green.
* **(C) Quarantine via `#[ignore]` + tracking
  issue**: same shape as the pre-`-19` BGE gates.
  Leaves the test in place but skipped; can be
  re-enabled when fixed.

Recommend **(A)** if the audit reveals a clear timing
target. Fall back to **(B)** if the root cause is
ambiguous + the test's value vs CI-stability tradeoff
favors gating. **(C)** as a last resort if (B)'s
`#[cfg]` shape doesn't fit the test layout.

## Acceptance

### Diagnosis verdict

1. Audit tail in task body identifies whether it's a
   regression (H1) or pre-existing flakiness (H2).
2. Reference to the specific debounce/timing primitive
   responsible if H1.

### Fix lands

3. The `ci.yml` macos-latest cargo test step passes
   reliably (rerun smoke + green).
4. Per the picked option:
   * (A): test stays + debounce/timing fixed.
   * (B): `#[cfg(not(target_os = "macos"))]` gate +
     bug-list entry under "Round-2 platform parity"
     section.
   * (C): `#[ignore]` + bug-list tracking entry.

### Tests

* The fix itself shouldn't need new tests; this is a
  fix-existing-test task.
* If (A) and the timing fix is non-trivial, consider
  adding a separate "macOS-specific debounce sanity"
  test pin to lock the new threshold.

### Gate

* `cargo fmt --check`, `cargo clippy --all-targets --
  -D warnings`, `cargo test -p chan-drive` (both
  Linux + macOS green via smoke branch).
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`
  green.
* CI smoke via `gh workflow run ci.yml --ref
  systacean-23-smoke` on a fresh smoke branch +
  verify macos-latest green.

## Coordination

* @@Systacean lane (chan-drive indexer ownership).
* Atomic-audit-commit discipline.
* `feedback_destructive_cleanups_coordinate_with_docs`:
  keep the `systacean-23-smoke` branch on origin
  until @@Alex sequences cleanup.

## Authorization

**Yes** for:

* `crates/chan-drive/src/indexer.rs` (test fix /
  gate).
* `crates/chan-drive/src/watcher.rs` (if debounce
  constants need tuning).
* `docs/journals/phase-8/phase-8-bugs.md` (Round-2
  platform-parity entry if (B) or (C)).
* `docs/journals/phase-8/systacean/systacean-23.md`
  (task tail).
* `docs/journals/phase-8/alex/event-systacean-architect.md`
  (outbound).

## Numbering

Highest committed `systacean-N` is `-22` (contact
filter + bucket emit). This is `-23`.

## Out of scope

* Refactoring chan-drive's debounce architecture
  beyond what's needed to fix the test.
* Re-enabling Windows on the CI matrix (separate
  scope decision per `ci-13`).
* Cross-platform watcher abstraction beyond the
  existing notify-rs usage.
