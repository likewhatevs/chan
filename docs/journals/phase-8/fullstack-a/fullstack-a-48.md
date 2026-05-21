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
