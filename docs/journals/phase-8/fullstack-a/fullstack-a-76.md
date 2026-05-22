# fullstack-a-76 — SPA Settings surface for pre-flight feature toggles (BGE + reports)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched
Round: 2 wave-3
Dependency: `systacean-27`

## Goal

Add SPA Settings UI for the per-drive BGE + chan-
reports feature toggles. Mirrors the pre-flight
screen from `fullstack-b-28` so users can flip the
toggles after initial drive setup.

## Reference

[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Pre-flight feature toggles" — "Enable later (via
Settings or CLI)".

## Scope

* Add a "Features" section to the existing Settings
  overlay surfacing BGE + reports toggles.
* Read state from chan-drive config via existing
  drive-config IPC.
* On toggle ON: persist + trigger incremental
  indexing (chan-drive handles the indexing pass
  per `-27`).
* On toggle OFF: persist; chan-drive stops the
  indexing pass.
* Inline help text per toggle.

## Acceptance

1. Settings shows Features section with two
   toggles.
2. Toggle state reflects current drive config.
3. Flipping persists + triggers indexing as
   appropriate.
4. Web build + chan-desktop both work.

### Tests

Vitest pins for the Settings rendering + toggle
handler.

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA lane. SPA-only.
* Consumes `systacean-27` API.

## Authorization

Yes for Settings SPA + tests + task tail + outbound.

## Numbering

This is `-a-76`.

## 2026-05-22 — audit findings + scope-poke (chan-server gap on reports toggle)

Audit-only round. Same shape as `-a-70`'s
initial audit.

### Audit summary

**Goal**: Surface BGE + chan-reports toggles
in the SPA Settings overlay.

**Pre-flight precedent** (`fullstack-b-28`):
chan-desktop's launcher expand-panel pre-flight
modal exposes both toggles via Tauri IPC
(`get_drive_features` / `set_drive_features`
at `desktop/src-tauri/src/main.rs:490`).
The Tauri commands call out to the chan CLI:
`chan index status --json --path <path>` to
read; `chan reports enable <path>` /
`chan reports disable <path>` to flip
reports; `chan semantic enable` /
`semantic disable` for BGE.

### BGE (semantic) toggle: chan-server endpoint exists

`/api/index/semantic/state` (GET) +
`/api/index/semantic/enable` (POST) +
`/api/index/semantic/disable` (POST) +
`/api/index/semantic/download` (POST) all
wired in `chan-server/src/lib.rs:784-790`
(gated by `#[cfg(feature = "embeddings")]`).

SPA already consumes them via
`api.semanticState()` /
`semanticDownload()` / `semanticEnable()` /
`semanticDisable()` in
`web/src/api/client.ts:504-507`.

A Features section in
`SettingsPanel.svelte` for BGE is a UI-only
add — no new endpoint needed.

### Reports toggle: chan-server endpoint gap

`chan-drive` exposes
`Drive::reports_enabled()` +
`Drive::set_reports_enabled(bool)` at
`crates/chan-drive/src/drive.rs:2030-2040`.

But chan-server has NO HTTP endpoint
exposing these. The CLI path
(`chan reports enable/disable <path>`)
mutates state through chan-drive directly,
bypassing chan-server.

**The SPA in browser mode has no way to
read or flip the reports toggle today.**
(In chan-desktop, the Tauri IPC chain
calls the chan CLI; that's how the
pre-flight modal works.)

### Routing decision

Audit-only this round. Two ways to close
the chan-server gap:

1. **New `/api/drive/features` route** that
   exposes both reports + semantic state
   in one shape. Mirrors `/api/drive` (the
   existing drive-metadata GET).
2. **Per-feature routes** mirroring the
   semantic shape: `/api/index/reports/state`
   (GET) + `/api/index/reports/enable`
   (POST) + `/api/index/reports/disable`
   (POST).

**Routing #2** matches the existing
semantic shape — consistent for the SPA
client.

### Scope-poke to @@Systacean (via architect)

`crates/chan-server/src/routes/`:
* New `routes/reports_toggle.rs` (or
  extension to existing `report.rs`):
  * `api_get_reports_state(state)` → JSON
    `{ enabled: bool }`. Reads
    `state.drive().reports_enabled()`.
  * `api_reports_enable(state)` → calls
    `state.drive().set_reports_enabled(true)`.
  * `api_reports_disable(state)` → calls
    `state.drive().set_reports_enabled(false)`.
* Wire `/api/index/reports/state` (GET) +
  `/api/index/reports/enable` (POST) +
  `/api/index/reports/disable` (POST) in
  `lib.rs`. Pin alongside the semantic
  routes for symmetry.
* Re-export from `routes/mod.rs`.
* Rust pins covering each handler against
  a fixture drive.

Per-call cost: `Drive::reports_enabled()`
reads `IndexConfig::reports_enabled` (a
small SQL config row); cheap enough for a
per-request call. `set_reports_enabled`
mutates the same row + triggers an
incremental indexing pass for the "true"
transition.

### Follow-up SPA side (after the endpoint lands)

Small. Mirrors the existing semantic
client + UI shape:
* `api.reportsState()` /
  `api.reportsEnable()` /
  `api.reportsDisable()` in client.ts.
* New "Features" section in
  `SettingsPanel.svelte` with two
  toggles (BGE + reports). Each follows
  the existing semantic-toggle shape
  from `HybridFileBrowserConfig.svelte`:
  read state on mount + on focus;
  optimistic toggle with rollback on
  error.
* Inline help text per toggle per the
  task body.

### No commit this round

Audit-only. Deliverable:
* This impl note documenting the gap +
  routing decision.
* Outbound poke to architect for
  @@Systacean routing of the reports
  endpoints.

### Acceptance (pending chan-server piece)

1. Settings shows Features section with
   two toggles ✓ (UI work post-endpoint).
2. Toggle state reflects current drive
   config ✓.
3. Flipping persists + triggers indexing
   as appropriate ✓ — both BGE +
   reports server-side handle the
   indexing pass.
4. Web build + chan-desktop both work ✓.

### Suggested commit subject (when shipping)

```
docs(fullstack-a-76): audit + scope-poke for chan-server reports endpoints
```

### Files for `git add` (per-path discipline)

* `docs/journals/phase-8/fullstack-a/fullstack-a-76.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for the chan-server
endpoint landing + the SPA-side follow-up.
