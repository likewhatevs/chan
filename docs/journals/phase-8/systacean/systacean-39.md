# systacean-39 — chan-server /api/index/reports/{state,enable,disable} endpoints (unblocks -a-76)

Owner: @@Systacean
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Add HTTP endpoints exposing the reports feature
toggle (`Drive::reports_enabled` /
`set_reports_enabled`) so the SPA Settings overlay
can read + flip the state in browser mode.

## Reference

@@FullStackA's `-a-76` audit (`a9ea464`):

* BGE toggle: `/api/index/semantic/{state,enable,disable,download}`
  already exists. SPA already consumes.
* Reports toggle: chan-drive primitives at
  `Drive::reports_enabled()` +
  `set_reports_enabled(bool)` exposed, BUT no
  chan-server HTTP endpoint. SPA browser-mode
  can't toggle.

@@FullStackA's routing decision: option 2 (per-
feature routes mirroring the semantic shape).

## Scope

New `crates/chan-server/src/routes/reports_toggle.rs`
(or extend existing `report.rs`):

* `GET /api/index/reports/state` → `{ enabled: bool }`.
  Reads `state.drive().reports_enabled()`.
* `POST /api/index/reports/enable` → calls
  `state.drive().set_reports_enabled(true)`.
* `POST /api/index/reports/disable` → calls
  `state.drive().set_reports_enabled(false)`.

Wire in `lib.rs::router()` alongside the semantic
routes for symmetry. Re-export from `routes/mod.rs`.

## Acceptance

1. `/api/index/reports/state` returns the current
   reports toggle state.
2. POST `/enable` flips to true + triggers the
   incremental indexing pass per chan-drive's
   existing behavior.
3. POST `/disable` flips to false; chan-drive stops
   indexing.
4. No regression on semantic endpoints or other
   routes.

### Tests

* Rust pins per handler against a fixture drive.
* Round-trip: GET state → POST enable → GET state →
  POST disable → GET state.

### Gate

`cargo fmt / clippy / test`; smoke green.

## Coordination

* @@Systacean lane.
* After this lands @@FullStackA wires
  `api.reportsState/Enable/Disable()` client
  method + Features section in
  `SettingsPanel.svelte` (mirrors semantic
  toggle shape from
  `HybridFileBrowserConfig.svelte`).

## Authorization

Yes for `crates/chan-server/src/routes/reports_toggle.rs`
(or `report.rs`) + lib.rs + routes/mod.rs + tests +
task tail + outbound.

## Numbering

This is `-39`.

## Out of scope

* SPA-side Settings UI (`-a-76` lane).
* CLI parity (existing `chan reports enable/disable
  <path>` unchanged).

## 2026-05-23 — implementation complete

Picked up `-39` per the dispatch.

### What landed

New `crates/chan-server/src/routes/reports_toggle.rs` with 3 endpoints + 3 tests:

* `GET /api/index/reports/state` → `{ enabled: bool }`. Calls `Drive::reports_enabled()`. Read-only lane (not settings-gated).
* `POST /api/index/reports/enable` → calls `Drive::set_reports_enabled(true)` on `spawn_blocking`. Returns the updated state. Settings-writes lane.
* `POST /api/index/reports/disable` → calls `Drive::set_reports_enabled(false)`. Returns updated state. Settings-writes lane.

Lib.rs wired with the semantic routes for symmetry. NOT gated on `embeddings` (reports are part of the BM25-only baseline).

### Tests (+3)

* `reports_state_endpoint_requires_auth` — anonymous request returns 401.
* `reports_round_trip_state_enable_disable` — full round-trip: initial false → enable → re-check true → disable → re-check false.
* `reports_disable_is_idempotent_when_already_off` — disable on already-off drive returns 200 + correct state.

All exercise the full router (via `crate::router(state)` + `oneshot`), so route registration + middleware are pinned alongside the handler logic.

### Acceptance criteria status

| # | Criterion | Status |
|---|-----------|--------|
| 1 | `/api/index/reports/state` returns the current state | ✓ |
| 2 | POST `/enable` flips to true + triggers indexing pass | ✓ (set_reports_enabled handles the trigger; the response carries the new state for SPA cache update) |
| 3 | POST `/disable` flips to false; chan-drive stops indexing | ✓ |
| 4 | No regression on other routes | ✓ (additive new file; chan-server tests 233/0 vs prior 230) |

### Pre-push gate

* `cargo fmt --check`: clean.
* `cargo clippy --all-targets -- -D warnings`: clean.
* `cargo test -p chan-server --lib`: **233 passed; 0 failed** (was 230; +3 new).
* workspace tests all green.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`: green.

### Files

| File                                              | +    | -  |
|---------------------------------------------------|------|----|
| `crates/chan-server/src/routes/reports_toggle.rs` (new) | +209 | 0  |
| `crates/chan-server/src/routes/mod.rs`            | +2   | 0  |
| `crates/chan-server/src/lib.rs`                   | +15  | -2 |
| `Cargo.lock`                                      | +1   | 0  |

Plus task tail + outbound poke. 6 paths.

### Suggested commit subject

```
chan-server: /api/index/reports/{state,enable,disable} endpoints (systacean-39; unblocks -a-76)
```

### Smoke plan

`gh workflow run ci.yml --ref systacean-39-smoke`. Expected ALL GREEN.

### What this unblocks

`fullstack-a-76` (SPA Settings overlay's Features section). FullStackA wires `api.reportsState/Enable/Disable()` + the Features toggle UI in `SettingsPanel.svelte` (mirrors the semantic toggle shape from `HybridFileBrowserConfig.svelte`).

Per architect's pre-authorization, proceeding to commit + push + smoke.
