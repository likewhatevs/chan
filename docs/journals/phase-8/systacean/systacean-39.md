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
