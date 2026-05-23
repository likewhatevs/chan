# systacean-41 — chan-server team create + duplicate endpoints (unblocks -a-79/-a-80)

Owner: @@Systacean
Cut: 2026-05-23 by @@Architect
Status: dispatched

## Goal

Add `POST /api/teams` (create) + `POST
/api/teams/{name}/duplicate` chan-server routes
exposing the existing `Drive::create_team` +
`Drive::duplicate_team` primitives (per
`systacean-30`). Unblocks @@FullStackA's `-a-79`
(Team Bootstrap orchestrator) + `-a-80` (Load
flow with duplicate branch).

## Reference

@@FullStackA's scope-poke (`93b9ea8`):

* `Drive::create_team(team_name, config)` exists
  per `-30` but not surfaced via HTTP.
* `Drive::duplicate_team` also exists per `-30`
  but not surfaced.
* `chan-server/src/routes/teams.rs` today carries
  ONLY watcher endpoints: `POST
  /api/teams/{name}/load`, `POST
  /api/teams/{name}/unload`, `GET
  /api/teams/loaded`.

Without these routes the SPA orchestrator can't
drive the bootstrap chain end-to-end; the watcher
load step fails because the team dir doesn't
exist yet.

## Scope

Extend `crates/chan-server/src/routes/teams.rs`:

* `POST /api/teams` body `{ name: string, config:
  TeamConfig }` → creates the team via
  `Drive::create_team`. Returns post-create state.
* `POST /api/teams/{name}/duplicate` body
  `{ new_name: string }` → duplicates the existing
  team via `Drive::duplicate_team`. Returns
  post-duplicate state.

Both lands in the **settings-writes lane** (create
/ duplicate are mutating operations).

## Acceptance

1. `POST /api/teams { name: "alpha", config: {...} }`
   creates the team dir + persists config.
2. `POST /api/teams/alpha/duplicate { new_name: "beta" }`
   creates `beta` as a copy of `alpha`.
3. Invalid name (empty, traversal, collision) →
   400.
4. Subsequent `POST /api/teams/{name}/load`
   succeeds on the newly-created team.

### Tests

* Round-trip: create → load → unload.
* Duplicate → load duplicate → distinct from
  original.
* Name validation: empty / traversal / collision
  → 400.

### Gate

`cargo fmt / clippy / test`; smoke green.

## Coordination

* @@Systacean lane.
* After this lands @@FullStackA wires the SPA
  orchestrator's create + duplicate calls into
  `-a-79` / `-a-80`.

## Authorization

Yes for `crates/chan-server/src/routes/teams.rs`
+ lib.rs route registration (if needed) + tests
+ task tail + outbound.

## Numbering

This is `-41`.

## Out of scope

* SPA-side orchestrator (`-a-79`/`-a-80` lane).
* Team-config schema changes — use `TeamConfig` as
  defined in chan-drive per `-30`.
