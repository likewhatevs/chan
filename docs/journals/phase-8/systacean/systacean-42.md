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
