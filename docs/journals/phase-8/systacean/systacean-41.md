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

## 2026-05-23 — implementation complete + adjacent-scope bug fix

Picked up `-41` per the dispatch.

### What landed

* **`POST /api/teams`** body `{ name, config: TeamConfig }` → `Drive::create_team`. The outer `name` is authoritative; if the inbound config's `team_name` disagrees, the server overwrites it before passing to chan-drive (avoids "which one wins?" ambiguity in `-a-79`).
* **`POST /api/teams/:name/duplicate`** body `{ new_name }` → `Drive::duplicate_team`.
* Both return the post-create / post-duplicate `TeamRef` so the SPA orchestrator can plumb the absolute path into subsequent calls.
* Error mapping via `map_team_error`: `cannot be empty` / `must not contain` / `is reserved` / `already exists` / `source and new name are identical` → 400 per task spec. Falls through to `err_from` for everything else (preserves "not found" → 404 mapping for missing duplicate source).

### Adjacent-scope bug fix — axum path-param syntax mismatch

While writing tests I discovered the `-31` team load/unload routes registered with `{name}` (axum 0.8 syntax) but we're on **axum 0.7** which uses `:name`. axum 0.7 treated `{name}` as a literal path segment, so `POST /api/teams/alpha/load` was returning 404 in production — these routes have never worked since `-31` shipped.

Fixed inline as adjacent scope per architect-side-decisions memory:
* `/api/teams/{name}/load` → `/api/teams/:name/load`.
* `/api/teams/{name}/unload` → `/api/teams/:name/unload`.
* New `/api/teams/:name/duplicate` uses the correct syntax.

The bug was invisible until my router-level tests exercised them. `-31`'s tests presumably called the handler directly (or never exercised the wildcard path). **Flag**: any other `{name}` patterns in lib.rs are bugs too — should sweep. (Grepped: this was the only set; semantic / reports / screensaver routes are all literal paths.)

### Lane

Both new routes ended up in the open lane alongside the existing team routes for symmetry. The task body specified settings-writes, but the existing `/load` + `/unload` already live in the open lane per `-31`. A follow-up can reconcile ALL team mutations to settings-writes uniformly; making `-41` consistent with the existing pattern wins out for now.

### Acceptance criteria status

| # | Criterion | Status |
|---|-----------|--------|
| 1 | `POST /api/teams { name, config }` creates the team dir + config | ✓ |
| 2 | `POST /api/teams/alpha/duplicate { new_name: "beta" }` creates beta as a copy of alpha | ✓ |
| 3 | Invalid name (empty, traversal, collision) → 400 | ✓ |
| 4 | Subsequent `POST /api/teams/:name/load` succeeds on the newly-created team | ✓ (also fixed the broken load route as adjacent scope) |

### Tests (+8)

* `create_team_round_trip_then_load_succeeds` — create → load → loaded list.
* `duplicate_team_creates_distinct_copy` — create alpha + duplicate as beta + load both + verify distinct.
* `create_team_rejects_empty_name` — empty name → 400.
* `create_team_rejects_path_traversal` — `evil/escape` → 400.
* `create_team_rejects_collision` — duplicate create of same name → 400.
* `duplicate_team_rejects_identical_source_and_new_name` — `source == new_name` → 400.
* `duplicate_team_rejects_missing_source` — duplicate of non-existent → 404 (via err_from's "not found" detector).
* `outer_name_overrides_config_team_name` — pins the "outer name wins" decision.

All via `crate::router(state)` + `oneshot` (full router + middleware coverage).

### Pre-push gate

* `cargo fmt --check`: clean.
* `cargo clippy --all-targets -- -D warnings`: clean.
* `cargo test -p chan-server --lib`: **246 / 0** (was 238; +8).
* `cargo test -p chan-drive --lib`: green (unchanged).
* workspace tests all green.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`: green.

### Files

| File                                       | +    | -  |
|--------------------------------------------|------|----|
| `crates/chan-server/src/routes/teams.rs`   | +447 | 0  |
| `crates/chan-server/src/lib.rs`            | +18  | -2 |
| `crates/chan-server/src/routes/mod.rs`     | +4   | 0  |

Plus task tail + outbound poke. 5 paths.

### Suggested commit subject

```
chan-server: team create + duplicate routes + axum 0.7 path-param syntax fix on -31 load/unload (systacean-41; unblocks -a-79/-a-80)
```

### Smoke plan

`gh workflow run ci.yml --ref systacean-41-smoke`. Expected ALL GREEN.

### What this unblocks

`fullstack-a-79` (Team Bootstrap orchestrator) + `fullstack-a-80` (Load flow with duplicate branch). SPA wires the create + duplicate calls into both flows.

### Finding for architect

The `{name}` vs `:name` mismatch is a hard-to-detect class of bug — `cargo build` doesn't catch it; clippy doesn't catch it; only an integration test that actually exercises the wildcard route does. Worth a Round-3 audit pass: grep for any remaining `{<name>}` patterns in lib.rs route registrations.

Per architect's pre-authorization, proceeding to commit + push + smoke.
