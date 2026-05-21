# fullstack-a-48 — Search / Indexing / Reports settings migration to Hybrid FB back (Task F)

Owner: @@FullStackA
Cut: 2026-05-21 by @@Architect
Status: queued (sequenced AFTER fullstack-a-43 lands in HEAD)

## Goal

Migrate drive-level search + indexing + reports settings
out of `SettingsPanel.svelte` into the new
`HybridFileBrowserConfig.svelte` mount point introduced
by `-a-43` (Task A). The FB back-side stops being a
"reserved for future use" placeholder and becomes the
**Search / Indexing / Reports** configuration surface.

Three toggles in v1:

1. **Semantic search** (from `-a-21`; existing).
2. **Multi-model picker** (Round-3 Track 2 future;
   placeholder slot for now).
3. **chan-reports** (RESTORE — toggle was specced in
   the pre-flight feature toggles plan but never landed
   in v1, surfaced as a regression by @@Alex
   2026-05-21).

## Background

Locked design:
[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Hybrid back-side revisited" — Task F (expanded
2026-05-21 to absorb the chan-reports toggle).

Also referenced in
[`../architect/graph-overhaul-plan.md`](../architect/graph-overhaul-plan.md)
as a prereq for graph-overhaul G3 (directory inspector
with aggregated reports stats can't render until the
reports toggle is restored + ON).

UX rationale: FB is where users see their indexed
content + where search results land them; chan-reports'
aggregated stats also surface in the FB-adjacent
directory inspector. Config-lives-next-to-the-affected-
surface holds for all three toggles.

The Search OVERLAY (`Cmd+K F` global spawn) stays
out-of-Hybrid; only the search + reports SETTINGS move.

## Acceptance criteria

### Settings migration (existing two toggles)

* Semantic search toggle moves out of
  `SettingsPanel.svelte` into
  `HybridFileBrowserConfig.svelte`.
* Multi-model picker placeholder slot present (future
  slot; render disabled if no models registered).

### chan-reports toggle restoration (G1)

* New chan-reports toggle in
  `HybridFileBrowserConfig.svelte`.
* Storage shape: extend `Preferences` with a `reports`
  section if one doesn't already exist; reuse if a
  prior shape is found in the codebase.
* When the toggle is ON:
  * chan-report's per-drive `.chan/report.jsonl`
    indexing runs (or continues running) for the drive.
  * Aggregated stats are available for downstream
    consumers (graph G3 directory inspector,
    Infographics tab if applicable).
* When the toggle is OFF:
  * Indexing stops; per-drive report data left in
    place (do NOT drop it on toggle-off; that's a
    destructive action and Task F doesn't ship the
    confirmation modal). The "disable drops per-drive
    data" semantic from the round-2-plan pre-flight
    spec stays parked for a follow-up task.
* Default state: OFF (matches the existing default
  for semantic-search; opt-in shape).
* Investigate WHY the toggle disappeared (audit
  history). Document the regression cause in the
  task tail.

### Tests

* Settings migration test pins.
* chan-reports toggle persist + restore-state test.
* Pre-push gate green.

## How to start

1. Audit current Settings surfaces — find any vestigial
   chan-reports references (commented-out code, dead
   route, stale Preferences field). Document what you
   find in the task tail.
2. Audit chan-server's chan-report integration to
   confirm the backend gating is still present. If
   the backend is missing too, fire a scope question
   (would need @@Systacean cross-pollination to restore
   the chan-server endpoint).
3. Move semantic-search + multi-model placeholder into
   `HybridFileBrowserConfig.svelte`.
4. Add the chan-reports toggle as the third toggle.
5. Wire storage shape + tests.

## Coordination

* SPA-primary; possible @@Systacean cross-pollination
  if chan-server gating for chan-reports also went
  missing. Surface a scope question if you hit that.
* Append "Commit readiness" + poke @@Architect when
  ready.

### Sequencing constraint — HARD prereq

Depends on
[`fullstack-a-43`](fullstack-a-43.md) landing in HEAD.

## Numbering

This is `-a-48`. See `-a-45` for the broader wave
numbering note. The relocated G (About + QR) at
`-a-42` gates on A+B+C+F, so it sequences AFTER this
task lands.

## 2026-05-21 — scope question for @@Architect

Audit complete. The chan-reports toggle situation
is bigger than the task body's "restore" framing
suggests; firing a scope question per the task's
"surface a scope question if you hit that" rule.

### What the audit found

* **SPA**: no chan-reports toggle anywhere
  (`grep -rn 'reports.*enabled\|reports.*toggle\|
  reports_enabled\|chan-reports\|chan_reports\|
  reportEnabled' --include="*.ts" --include="*.svelte"
  web/src/` returns nothing). `Preferences` type in
  `web/src/api/types.ts` does NOT have a `reports`
  field.
* **chan-server**: `PreferencesView` in
  `crates/chan-server/src/routes/preferences.rs:26`
  does NOT have a `reports` field. The chan-report
  integration runs **unconditionally** via
  `drive.report()` / `drive.report_for_*` calls in
  `chan-server/src/routes/{inspector, graph, report,
  storage}.rs`. There's no toggle gate on the
  read or write paths.
* **chan-drive**: chan-report is imported + re-
  exported (`crates/chan-drive/src/lib.rs:43`); the
  per-drive `.chan/report.jsonl` is unconditionally
  populated via the indexer pass + `Drive::report_jsonl_path`.
* **Git history**: no commit ever introduced a
  chan-reports toggle. The
  `round-2-plan.md` §"Pre-flight feature toggles
  (added 2026-05-20)" pre-flight scope DID spec the
  toggle (lines 193-220, ON/OFF semantics +
  destructive-on-disable parking), but the spec
  never landed as code in v1.

So "RESTORE" is technically a misnomer — the
toggle was specced but never built. There IS no
regression to revert; the work is implement-from-
scratch.

### Why it matters for scope

The task body covers SPA wiring +
`Preferences` field extension cleanly. But the
ON/OFF semantics ("indexing stops when OFF;
aggregated stats unavailable to downstream
consumers") REQUIRES chan-server backend gating —
otherwise the toggle is a no-op user-visible lie:
flip OFF, chan-report still runs, file inspector +
graph G3 directory inspector still render reports.

Backend gating spans 4+ chan-server routes
(`inspector`, `graph`, `report`, `storage`) +
likely a chan-drive `index_with_report: bool`
flag on the indexer pass entry. Plus the
`Preferences.reports` shape in
`PreferencesView` server-side + serialization
round-trip.

### Three options for routing

**(A) Full -a-48 as specced — SPA + chan-server gating**

I take the whole thing: extend `Preferences` (TS +
Rust), wire SPA toggle, route gating in
chan-server, indexer pass gating, default OFF
(lean drive per the round-2-plan spec). Heavy
lift; spans my lane end-to-end. Estimate:
substantially bigger than `-a-45`/`-a-46`/`-a-47`
combined. Probably right call given chan-server
is fullstack lane.

**(B) SPA wiring + default ON; backend gating
deferred to a follow-up task** (my lean recommendation)

This commit lands:
- SPA toggle UI in HybridFileBrowserConfig.
- `Preferences.reports.enabled` field (TS + Rust
  PreferencesView).
- Persistence + round-trip wiring.
- Default ON (matches existing UX; no behavior
  surprise — user toggles aren't lies because
  chan-report keeps running).

Follow-up task `-a-N` (or `-b-N`) lands:
- chan-server route gating.
- chan-drive indexer pass gating.
- Default flip to OFF (lean drive spec from
  round-2-plan).
- The destructive-on-disable confirmation modal
  (round-2-plan §"Pre-flight feature toggles"
  lines 211-220).

Smaller commit per task; clean separation of
concerns; user-visible toggle lands NOW so
downstream graph G3 directory inspector work can
read `preferences.reports.enabled` as the gating
signal. Behavior gating lands next beat.

**(C) Defer chan-reports entirely; ship Tasks F
minus chan-reports**

`-a-48` becomes Semantic search migration +
multi-model picker placeholder only. chan-reports
cut as a separate task (`-a-N`) with full scope
(SPA + backend) bundled. Smallest current
commit; respects scope discipline; defers the
"is this big" decision.

### My lean

(B) feels right. Lands user-visible progress
this beat without dragging in 4+ chan-server
routes + the chan-drive indexer pass surgery.
Default ON keeps the existing UX honest (no
"toggle says OFF but reports still render"
confusion). Backend gating + default flip can
land as a clean focused task with its own gate.

Either way, I'd appreciate a steer because the
"SPA-primary" framing in the task body assumed
the backend gating WAS present, and it isn't.

### Standing by

Holding `-a-48` here until you route. Cleared
through `-a-47` (`dd586fc`) committed; queue
behind `-a-48` is `-a-42` (gates on A+B+C+F
landing).

## 2026-05-21 — ready for review (option B landed)

Six-file change. SPA + chan-server wiring; no
chan-drive touched.

### Architecture

Option (B) as routed by @@Architect at
[`event-architect-fullstack-a.md`](../alex/event-architect-fullstack-a.md)
"2026-05-21 — @@Architect: routing on -a-48
scope question". The SPA toggle UI lands now;
backend gating + default-flip-to-OFF +
destructive-on-disable modal land in a
follow-up task.

### chan-reports wire-up (option B)

* `crates/chan-server/src/config.rs`: new
  `ReportsConfig { enabled: bool }` struct;
  `ServerConfig.reports: ReportsConfig` field
  with `#[serde(default)]` so older
  `server.toml` files round-trip cleanly.
  Default `true` (matches today's
  unconditional chan-report behaviour).
* `crates/chan-server/src/routes/preferences.rs`:
  `PreferencesView.reports: ReportsConfig`
  field; `preferences_view()` reads from
  `server.reports.clone()`; `apply_preferences()`
  writes `server.reports = view.reports`.
* `web/src/api/types.ts`: new
  `ReportsPreferences { enabled: boolean }`
  type; `Preferences.reports?:
  ReportsPreferences` optional field (so older
  servers that don't ship the field don't trip
  the type contract).

### HybridFileBrowserConfig — three toggles

* **Semantic search**: migrated verbatim from
  SettingsPanel `-a-21`. Full state machine
  (`semanticState` + `semanticDownloading` +
  `semanticEnabling` + `semanticError` + 3-second
  polling timer), `semanticToggle` /
  `loadSemanticState` / `stopSemanticPoll`
  helpers, `formatModelSize` formatter,
  `BuildInfo` feature-flag guard. Stateful POSTs
  against the chan-server (`api.semanticEnable` /
  `Download` / `Disable` / `State`) — not a
  Preferences-stored value, so the dirty/save
  pipeline doesn't touch it.
* **Multi-model picker**: placeholder
  `<select disabled>` slot with the default
  `BAAI/bge-small-en-v1.5` option. Round-3
  Track 2 lands the model registry + the
  picker functionality on top.
* **chan-reports**: NEW toggle. Writes
  `editing.reports.enabled`; persists via the
  merge-against-current-server PATCH pattern
  (re-fetches GlobalConfig before overlaying
  just the `reports` field). Default ON
  matches the option (B) wire default. Help
  text flags that backend gating + the
  destructive-on-disable modal land in a
  follow-up task.

### SettingsPanel trim

After `-a-48` SettingsPanel is reduced to the
About section + the GlobalConfig autosave
plumbing. Removed:

* Semantic-search markup (~65 lines).
* `let semanticState/Downloading/Enabling/Error/
  PollTimer` state.
* `loadSemanticState` / `stopSemanticPoll` /
  `semanticToggle` / `formatModelSize` helpers.
* `SemanticState` type import.
* `onDestroy` import (no longer needed; nothing
  to clean up).
* CSS sweep: `.hint`, `.hint code`, `.hint.err`,
  `.theme-opt` (chip + variants),
  `.semantic-toggle` (chip + disabled
  variants), `.semantic-info`, `.spinner` +
  `@keyframes spin`, `label` + `label > span`,
  `input` + `input:focus`. All went unused
  after the section migrated.

### Tests

`HybridFileBrowserConfig.test.ts` (new) — 11
wiring pins + 4 negative pins against
SettingsPanel:

* Wiring: warning copy, semantic state machine
  carries over, feature-flag guard, toggle
  disable-during-progress, formatModelSize,
  multi-model picker placeholder shape,
  chan-reports binding, normalizeReports
  default-ON backfill, save merges only
  `reports` field, dirty scoped to the
  `reports.enabled` field, polling cleanup on
  destroy.
* Negative: SemanticState type import gone,
  semantic helpers gone, semantic state
  variables gone, section header gone.

### Gate

* vitest **637 / 637** (+15 net from `-a-47`'s
  622).
* svelte-check 0 errors / 0 warnings across
  3989 files. CSS sweep cleared 14 warnings
  that surfaced after the markup removal.
* npm build clean.
* cargo fmt --check clean.
* cargo clippy --all-targets -- -D warnings
  clean.
* cargo test -p chan-server: 205 / 205 pass.

### Decisions

* **Default ON for `reports.enabled`** — option
  (B) routing's call; matches today's
  unconditional behaviour. The default flips
  to OFF when the backend gating ships.
* **`reports` field optional on the TS side**
  (`Preferences.reports?`) — old servers that
  don't yet emit the field don't trip the
  type contract. Backfill in
  `normalizeReports` handles the load path.
* **Help text under chan-reports toggle**
  explicitly calls out that the toggle is
  wire-only today + backend gating + modal
  ships in the follow-up. Users see "Enable
  chan-reports indexing" with a sub-hint
  that flips to "OFF default once gating
  lands". Avoids the toggle-lie surface
  area the architect's option (B) call
  carefully sidesteps.

### Follow-up needed (per @@Architect's routing)

When this lands, the next task should cover:

* Backend gating in 4 chan-server route files
  (`inspector`, `graph`, `report`, `storage`)
  — return 404 / empty when `reports.enabled
  == false`.
* chan-drive indexer pass flag — skip the
  per-drive chan-report indexing pass when
  the toggle is OFF.
* Destructive-on-disable confirmation modal —
  per round-2-plan §"Pre-flight feature
  toggles" lines 211-220; "OFF drops the
  per-drive report data" requires a UI/CLI
  confirmation gate.
* Default flip ON → OFF once gating lands —
  matches the "opt-in feature" framing from
  the round-2-plan.

Probably crosses lanes to @@Systacean for the
chan-drive indexer pass flag piece.

### Suggested commit subject

```
Migrate Search/Indexing/Reports settings to Hybrid FB back-side (fullstack-a-48 option B)
```

Single commit. SPA toggles + Rust ServerConfig
field + PreferencesView round-trip + tests are
all part of the same option (B) landing.

### Files for `git add` (per-path discipline)

* `crates/chan-server/src/config.rs`
* `crates/chan-server/src/routes/preferences.rs`
* `web/src/api/types.ts`
* `web/src/components/HybridFileBrowserConfig.svelte`
* `web/src/components/HybridFileBrowserConfig.test.ts`
* `web/src/components/SettingsPanel.svelte`
* `docs/journals/phase-8/fullstack-a/fullstack-a-48.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

Push held — multi-agent tree commit
discipline. Standing by for clearance.

## 2026-05-21 — committed as 0391eae

Cleared by @@Architect; committed as `0391eae
Migrate Search/Indexing/Reports settings to
Hybrid FB back-side (fullstack-a-48 option B)`.
Pre-commit `git diff --staged --stat` matched
the cleared path list exactly (9 files);
post-commit `git show --stat HEAD` confirmed
no stowaways from other lanes
(chan-drive/tests changes from systacean-18
follow-ups, ci-12 docs, plus various event
channels all stayed unstaged). Push held per
protocol.
