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

## 2026-05-23 — SPA client wiring slice 1 ready for review (Settings UI in slice 2)

Two-file change. SPA-only.

### What landed

`web/src/api/client.ts`:
* New `api.reportsState()` →
  `GET /api/index/reports/state` →
  `{ enabled: boolean }`.
* New `api.reportsEnable()` →
  `POST /api/index/reports/enable`.
* New `api.reportsDisable()` →
  `POST /api/index/reports/disable`.
* All three return the post-flip
  `{ enabled }` shape so callers update
  cache from the response body.
* Doc-comment cross-references
  `fullstack-a-76` + `systacean-39` +
  the `-27` incremental-indexing-pass
  contract.
* Sits next to the semantic client
  methods so a future audit reads the
  parallel.

`web/src/api/reportsToggleClient.test.ts`
(new): 5 raw-source pins covering the 3
method shapes, the doc-comment cross-
references, and the semantic-parallel
audit pin.

### Slice 2 deferred — dual-toggle decision needed

The pre-existing
`Preferences.reports?.enabled` field at
`web/src/api/types.ts:164` is the GLOBAL
config flag (round-tripped via
`/api/config`); UI in
`HybridFileBrowserConfig.svelte`.

The new `Drive::reports_enabled` flag
(this slice) is PER-DRIVE metadata via
the chan-server's `/api/index/reports/*`
routes.

These are TWO different control surfaces
for what reads as conceptually-similar
state. Three resolutions possible:

1. **Hierarchical**: global = "feature
   available"; per-drive = "is it on
   for THIS drive". Both must be ON
   for indexer to run reports.
2. **Migrate**: deprecate the global
   `Preferences.reports` field;
   per-drive is the source of truth.
   HybridFileBrowserConfig's existing
   UI gets re-wired.
3. **Coexist**: keep both surfaces;
   document the distinction; let the
   user understand they're different.

Architect's call. Slice 2 (Settings UI)
ships once the resolution is settled.

### Acceptance (slice 1 — client methods only)

1. **3 client methods exposed** ✓ —
   shape mirrors semantic.
2. **No regression on the existing
   preferences.reports flow** ✓ —
   nothing touched.
3. **No new UI yet** — slice 2 awaits
   the dual-toggle decision.

### Gate

* vitest **1052 / 1052** (+5 net from
  `-a-77` audit's 1047).
* svelte-check 0 errors / 0 warnings
  across 4040 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions

* **Ship client methods alone** — they're
  harmless (no caller yet) + unblock
  slice 2 the moment the dual-toggle
  decision lands.
* **Test-pin the semantic-parallel** —
  future audits should read both
  toggles as siblings.
* **Defer Settings UI** rather than ship
  a third surface alongside the
  existing two. Three surfaces would
  amplify the confusion.

### Suggested commit subject

```
api.reports{State,Enable,Disable}: client methods for systacean-39 endpoints (fullstack-a-76 slice 1)
```

Single commit. Client methods + 5 test
pins.

### Files for `git add` (per-path discipline)

* `web/src/api/client.ts`
* `web/src/api/reportsToggleClient.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-76.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for the
dual-toggle architectural decision +
then slice 2 ships the Settings UI.

## 2026-05-23 — SPA slice 2 (Settings Features section) ready for review

Three-file change. SPA-only.

Architect's note from `0eae028` ack: "Slice 2
(Settings UI Features section pairing reports
+ BGE) is the next pick." Treating that as
the dual-toggle resolution → COEXIST: Settings
gets a single-screen quick-toggle Features
section; `HybridFileBrowserConfig.svelte`
keeps its richer per-feature controls (model
download flow + global preferences mirror).

### What landed

`web/src/components/SettingsPanel.svelte`:
* `SemanticState` type re-imported (was
  removed in `-a-48`).
* New state variables:
  `reportsEnabled` / `reportsBusy` /
  `reportsError`; `semanticState` /
  `semanticBusy` / `semanticError`.
* New `loadFeaturesState()` async helper —
  fans `api.reportsState()` +
  `api.semanticState()` in parallel; semantic
  fetch failure (build without
  `embeddings` feature) is caught + leaves
  `semanticState` null so the UI renders an
  "n/a" affordance.
* New `toggleReports()` — flips per-drive
  reports via the right endpoint;
  optimistic state update from the
  response body.
* New `toggleSemantic()` —
  enabled→disable, disabled+model_present→enable,
  disabled+no_model→error message pointing
  to FB config (the download flow lives
  there).
* `onMount` invokes `loadFeaturesState`.
* New `<section class="features">` markup
  with two `.feature-row` blocks (each =
  meta block + switch). Each row has a
  title, sub-description, optional inline
  error, and a checkbox.
* CSS rules for `.features`,
  `.feature-row`, `.feature-meta`,
  `.feature-title`, `.feature-sub`,
  `.feature-meta .err`,
  `.feature-switch`. Bordered card per
  row matches the Appearance section's
  visual density.

`web/src/components/HybridFileBrowserConfig.test.ts`:
* `fullstack-a-48: Semantic search removed
  from SettingsPanel` test block updated to
  reflect slice 2's re-introduction. The
  RICH model-download state machine
  (semanticDownloading, semanticEnabling,
  semanticPollTimer, semanticToggle,
  loadSemanticState, formatModelSize,
  api.semanticDownload) STAYS forbidden in
  SettingsPanel — those belong to the FB
  config. Simple toggle helpers introduced
  in slice 2 ARE allowed.
* Old `<h3>Semantic search</h3>` section
  header check stays — the new
  `<h3>Features</h3>` is a different
  header.

`web/src/components/settingsFeaturesSection.test.ts`
(new): 12 raw-source pins covering:
* State variable declarations.
* `SemanticState` type import.
* `loadFeaturesState` parallel-fetch.
* `onMount` invocation.
* `toggleReports` direction-aware
  endpoint pick.
* `toggleSemantic` model-present guard +
  error message for model-absent case.
* Features section markup +
  title-per-row.
* Onchange handler wiring per row.
* "Model not downloaded" copy.
* Rationale comment cross-references.

### Acceptance (slice 2)

1. **Settings shows Features section
   with two toggles** ✓ — chan-reports +
   BGE.
2. **Toggle state reflects current drive
   config** ✓ — both load on mount;
   refresh-on-toggle from the response
   body.
3. **Flipping persists + triggers
   indexing as appropriate** ✓ — per
   `-39`'s `set_reports_enabled` contract
   + the existing semantic enable flow.
4. **Web build + chan-desktop both
   work** ✓ — same endpoints; same
   shape.

### Gate

* vitest **1064 / 1064** (+12 net from
  `-a-76` slice 1's 1052: +12 new pins
  in settingsFeaturesSection.test.ts;
  the existing HybridFileBrowserConfig.test.ts
  block was REWRITTEN, not added/removed).
* svelte-check 0 errors / 0 warnings
  across 4041 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions

* **Coexist** (option 3 from slice 1's
  dual-toggle framing). Per architect's
  "Settings UI Features section pairing
  reports + BGE is the next pick" — the
  Settings is the quick-toggle surface;
  HybridFileBrowserConfig is the
  rich-controls surface.
* **Defer model download** for BGE. If
  the user toggles BGE ON with the model
  absent, the UI surfaces an error
  pointing to FB config rather than
  re-implementing the download flow. The
  download is non-trivial (polling +
  progress); duplicating it would be
  bloat.
* **Semantic feature-gated graceful**.
  When chan-server is built without
  `embeddings`, `/api/index/semantic/state`
  returns 404. The fetch is caught +
  `semanticState` stays null; the row
  renders "n/a" + disabled checkbox.
  Visible but inert.
* **Updated the `-a-48` test block**
  rather than deleting it. The
  "semantic state machine helpers are
  gone" intent (don't re-introduce the
  download-flow complexity into
  Settings) is still valid; just the
  specific allowlist changed.

### Suggested commit subject

```
Settings: Features section pairs chan-reports + BGE toggles (fullstack-a-76 slice 2)
```

Single commit. Markup + state + CSS + 12
new pins + 1 updated test block.

### Files for `git add` (per-path discipline)

* `web/src/components/SettingsPanel.svelte`
* `web/src/components/HybridFileBrowserConfig.test.ts`
* `web/src/components/settingsFeaturesSection.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-76.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance +
the @@WebtestA empirical walk.
