# backend-teardown-1: Original backend slot teardown

Owner: @@Backend.

Status: REVIEW.

Related:

- [journal.md](./journal.md)
- [backend-rustacean-1.md](./backend-rustacean-1.md)
- [backend-1.md](./backend-1.md)
- [backend-2.md](./backend-2.md)
- [backend-3.md](./backend-3.md)

## Goal

Tear down the original backend slot now that backend/Rust ownership has moved to
the Rustacean+Backend slot.

## Context

The active backend/Rust role is now tracked in
[backend-rustacean-1.md](./backend-rustacean-1.md). Backend tasks
[backend-1.md](./backend-1.md), [backend-2.md](./backend-2.md), and
[backend-3.md](./backend-3.md) are all in REVIEW.

## Teardown checklist

- Stop any dev servers, test servers, watchers, or background processes started
  by the original backend slot.
- Remove backend-owned temporary files, logs, scratch outputs, or build
  artifacts that are safe to remove.
- Do not remove shared webtest services from [webtest-1.md](./webtest-1.md).
- Do not remove source changes or files owned by other agents.
- Record anything intentionally left behind and why.
- Confirm the original backend slot is closed and future backend/Rust work
  should go through the Rustacean+Backend slot.

## Progress notes

### 2026-05-16 @@Backend: teardown.

Original backend slot is closed.

- **Background processes**: none owned by the backend slot. The
  `chan serve` on `:8787` (pid 81899, fixture drive
  `/tmp/chan-phase3-drive`) was started by @@Webtest under
  [webtest-1.md](./webtest-1.md) and is explicitly out of scope per
  the teardown checklist; left running.
- **Temporary files / logs / scratch**: none created by the backend
  slot. `cargo target/` is the shared workspace build cache (not
  backend-owned).
- **Branches**: none created. Working branch remains `main`.
- **Source changes left in place** (REVIEW artifacts, not to be
  removed until commit coordination):
  - `crates/chan/src/main.rs` — Agent rename of error-context
    strings (backend-1) + `parse_line_spacing` / `line_spacing_label`
    rewrite for `standard | compact` with legacy `tight` alias
    (backend-3) + 4 new CLI tests (backend-3).
  - `crates/chan-server/src/preferences.rs` — `LineSpacing` enum
    renamed `Tight` → `Compact`, default flipped to `Standard`,
    `#[serde(alias = "tight")]` shim, 5 new tests (backend-3).
  - Note: `crates/chan-llm/src/session.rs::LlmSession::backend()`
    was touched by @@Frontend (active-backend gate, partial CODEx-
    on-CLAUDE belt-and-braces); not part of this slot's teardown.

### Confirmation

- Original backend slot has no active implementation work.
- All three backend tasks ([backend-1](./backend-1.md),
  [backend-2](./backend-2.md), [backend-3](./backend-3.md)) are in
  REVIEW.
- Future backend / Rust work for phase 3 routes through
  [backend-rustacean-1.md](./backend-rustacean-1.md) (@@Backend+Rustacean).

## Completion notes

- Teardown complete. No state needed to be removed; the slot's
  outputs are the REVIEW-status source edits in
  `crates/chan/src/main.rs` and `crates/chan-server/src/preferences.rs`
  awaiting commit coordination.
- Verified: `cargo fmt --check` clean, `cargo clippy --all-targets
  -- -D warnings` clean, `cargo test -p chan-server -p chan` green
  (after the most recent backend-3 changes; re-confirmed earlier in
  this session).
- No follow-ups left for the original backend slot.
