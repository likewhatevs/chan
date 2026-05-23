# systacean-42 — chan-server GET /api/teams/{name}/config endpoint + teamCreate idempotency check (unblocks -a-80 slice 2)

Owner: @@Systacean
Cut: 2026-05-23 by @@Architect
Status: dispatched

## Goal

Add `GET /api/teams/:name/config` endpoint returning
the persisted `TeamConfig`. Plus verify (and fix if
needed) that `Drive::create_team` is idempotent for
already-existing teams. Unblocks @@FullStackA's
`-a-80 slice 2` (Load Team dialog populated from
persisted config).

## Reference

@@FullStackA's scope-poke (`c9b8489`) on `-a-80
slice 1` shipped:

* SPA needs the persisted team config to populate
  the Load Team dialog (per addendum-b §"Loading
  team").
* Once user clicks Bootstrap on the populated
  dialog, `-a-79`'s orchestrator calls `teamCreate`
  — needs to be a no-op for an already-existing
  team.

## Scope

### 1. GET /api/teams/:name/config

* Reads `Drafts/team-{name}/config.toml` via
  `chan_drive::teams::load(drafts_dir, team_name)
  → TeamConfig`.
* Returns same `TeamConfig` JSON shape that the
  existing `POST /api/teams` body's `config` field
  uses.
* 404 on missing team.
* 500 on parse error.
* Uses `:name` (axum 0.7 syntax — same as `-41`).
* Lands in open lane (matches other team routes;
  read-only).

### 2. teamCreate idempotency

* Verify `Drive::create_team` returns Ok(_) when
  the team dir already exists with matching
  config OR returns a structured "already exists"
  error that chan-server can map to a no-op
  success.
* Document the semantics inline.
* If the current behavior is "error on existing,"
  consider:
  * (A) Adding `create_team_idempotent` variant.
  * (B) Adding a flag param `overwrite: bool` /
    `if_not_exists: bool`.
  * (C) Letting the SPA detect "already exists"
    + treat as no-op.
* @@FullStackA's flag suggests (C) may already
  be the right call (SPA can check before
  calling); confirm + document.

## Acceptance

1. `GET /api/teams/alpha/config` returns persisted
   `TeamConfig` JSON for an existing team.
2. `GET /api/teams/nope/config` returns 404.
3. Round-trip: `POST /api/teams { name, config }`
   then `GET /api/teams/{name}/config` returns the
   same config.
4. Idempotency: documented behavior on
   `create_team` for existing team — either no-op
   success OR clear error code that SPA can detect.

### Tests

* Round-trip test: POST + GET returns same shape.
* 404 on missing team.
* If idempotency fix lands: pin behavior.

### Gate

`cargo fmt / clippy / test`; smoke green.

## Coordination

* @@Systacean lane.
* @@FullStackA wires `api.teamGetConfig(name)`
  + slice 2 dialog flow after this lands.

## Authorization

Yes for `crates/chan-server/src/routes/teams.rs`
+ `crates/chan-drive/src/drive.rs` if idempotency
needs adjustment + tests + task tail + outbound.

## Numbering

This is `-42`.

## Out of scope

* SPA-side dialog wire-up (`-a-80 slice 2` lane).
* Team-config schema changes — use `TeamConfig`
  as-is.

## 2026-05-23 — implementation complete + idempotency contract documented

Picked up `-42` per the dispatch.

### 1. GET /api/teams/:name/config

* Reads via `Drive::load_team` (→ `chan_drive::teams::load`).
* Returns `TeamConfig` JSON matching the `POST /api/teams` body's `config` field shape — symmetric for SPA round-trip pipelines.
* Errors:
  * Missing team → 404 (via `err_from`'s "not found" detector on `teams::load`'s `"team \`{name}\` not found at {path}"` message).
  * Malformed `config.toml` → 500 via the `ChanError::ConfigDecode` fallback.
  * Invalid name (empty / traversal / reserved) → 400 via `map_team_error` (already in place from `-41`).
* Lives in the open lane alongside the rest of `/api/teams/*` for consistency.
* axum 0.7 `:name` syntax (also consistent with the `-41` fix).

### 2. teamCreate idempotency — option (C) chosen

**The task body offered 3 options; (C) is the right call**: the SPA detects the structured "already exists" response + treats it as a no-op success for bootstrap-on-existing flows.

Reasoning (now documented inline on `api_team_create`):

* A silent no-op on existing would mask a real user mistake (typo on team name colliding with an unrelated team).
* An overwrite-on-existing would corrupt the existing config.
* A structured 400 with `already exists` in the body preserves both safety + lets the SPA detect the case.

**Pinned by a new test** (`create_team_returns_400_on_existing_team_for_spa_idempotency`) so future refactors can't break the SPA contract silently.

No chan-drive changes needed; the existing error message ("team `{name}` already exists at {path}") already carries the marker `map_team_error` picks up.

### Acceptance criteria status

| # | Criterion | Status |
|---|-----------|--------|
| 1 | `GET /api/teams/alpha/config` returns persisted `TeamConfig` JSON | ✓ |
| 2 | `GET /api/teams/nope/config` returns 404 | ✓ |
| 3 | Round-trip: `POST /api/teams { name, config }` then GET returns same config | ✓ (pinned in `get_team_config_round_trips_with_post`) |
| 4 | Idempotency: documented behavior on `create_team` for existing team — clear error code SPA can detect | ✓ option (C): 400 + `already exists` marker in body |

### Tests (+3)

* `get_team_config_round_trips_with_post` — POST then GET returns matching JSON (with the outer-name-wins rule preserving `team_name = "alpha"` in the response).
* `get_team_config_returns_404_when_missing` — GET on non-existent team → 404.
* `create_team_returns_400_on_existing_team_for_spa_idempotency` — pins the documented contract; asserts `already exists` appears in the response body.

All via `crate::router(state)` + `oneshot` (full router + middleware coverage, also exercising the new `:name` path-param).

### Pre-push gate

* `cargo fmt --check`: clean.
* `cargo clippy --all-targets -- -D warnings`: clean.
* `cargo test -p chan-server --lib`: **249 / 0** (was 246; +3).
* `cargo test -p chan-drive --lib`: green (unchanged).
* workspace tests all green.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`: green.

### Files

| File                                       | +    | -  |
|--------------------------------------------|------|----|
| `crates/chan-server/src/routes/teams.rs`   | +125 | 0  |
| `crates/chan-server/src/lib.rs`            | +7   | -2 |
| `crates/chan-server/src/routes/mod.rs`     | +3   | -1 |

Plus task tail + outbound poke. 5 paths.

### Suggested commit subject

```
chan-server: GET /api/teams/:name/config + documented idempotency contract for POST (systacean-42; unblocks -a-80 slice 2)
```

### Smoke plan

`gh workflow run ci.yml --ref systacean-42-smoke`. Expected ALL GREEN.

### What this unblocks

@@FullStackA's `-a-80 slice 2` Load Team dialog. SPA wires:
* `api.teamGetConfig(name)` to populate the dialog from persisted state.
* `api.teamCreate(...)` for Bootstrap, with the "already exists → no-op" branch handled per the documented idempotency contract.

Per architect's pre-authorization, proceeding to commit + push + smoke.
