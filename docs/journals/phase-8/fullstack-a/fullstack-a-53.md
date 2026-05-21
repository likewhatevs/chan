# fullstack-a-53 — theme architecture correction: Appearance stays global; per-Hybrid override toggles

Owner: @@FullStackA
Cut: 2026-05-21 by @@Architect
Status: dispatched

## Goal

Revert `-a-46`'s "Appearance moved to Hybrid Editor back"
piece (the load-bearing PART of `-a-46` — Layout, Date
Pills, On Save migrations stay). Replace with a **per-
Hybrid theme override toggle** in BOTH Hybrid Editor +
Hybrid Terminal back-sides.

End state:

* **Settings overlay** (`Cmd+,`): keeps the Appearance
  section (system / light / dark — the global default).
* **Hybrid Editor back-side** (`HybridEditorConfig.svelte`):
  Layout, Date Pills, On Save (no change from `-a-46`).
  ADDS a "theme" toggle: `inherit | light | dark`.
* **Hybrid Terminal back-side** (`HybridTerminalConfig.svelte`):
  scrollback MB, default TERM (no change from `-a-45`).
  ADDS the same `inherit | light | dark` toggle.

Resolution order at render time:

1. If per-Hybrid override is `light` or `dark`: render
   THAT theme on this Hybrid (front + back per `-a-47`'s
   collapse).
2. Else (`inherit`, the default): use the Settings
   Appearance value (which resolves system/dark/light as
   before).

## Background

Surfaced 2026-05-21 by @@Alex as a Hybrid back-side
design correction. See
[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Theme architecture correction 2026-05-21" + the
architect journal entry of the same date.

`-a-46` (Editor Settings migration; HEAD `5166223`) moved
the entire Appearance section into `HybridEditorConfig`.
The flagged-deviation acceptance was correct GIVEN the
spec as-written at the time (Editor back-side scope
explicitly included Theme). @@Alex's correction post-walk
clarifies the intent: Appearance is a GLOBAL default with
per-Hybrid OVERRIDES, not a per-Hybrid-only setting.

Use-case @@Alex named: "i want dark mode from the settings
but all my editors are light mode" — global = dark;
per-Hybrid override on every Editor pane = light. The
override is the user's way to say "this surface
specifically renders different." Inherit (default) means
"track the global."

## Decision: fix shape

Two coupled changes:

* **Revert the Appearance section migration** in
  `HybridEditorConfig.svelte`. Move Appearance markup +
  `setThemeChoice` import + the 3 Appearance test pins
  back into `SettingsPanel.svelte`. Per @@FullStackA's
  `-a-46` clearance flagged-deviation note ("If so, the
  section + `setThemeChoice` import + 3 Appearance tests
  can revert via a small follow-up") — this is that
  follow-up.

* **Add per-Hybrid theme override toggle** to BOTH
  `HybridEditorConfig.svelte` AND `HybridTerminalConfig.svelte`.
  Three-option toggle (inherit / light / dark). Persistence:
  per-Hybrid field on the existing Hybrid wire format (likely
  a `themeOverride: 'light' | 'dark' | null` field where
  `null` = inherit). Default: `null`/inherit.

* **Render-time resolution** in `Pane.svelte` (or wherever
  the per-Hybrid theme is resolved post-`-a-47`): check
  `themeOverride` first; if set, use it; else fall through
  to the global Settings Appearance value.

* **Migration** for existing Hybrid panes: existing
  per-Hybrid theme values (from `-b-5` originally) get
  migrated to the new `themeOverride` field. Empirically
  most users haven't explicitly set per-Hybrid theme, so
  the migration is mostly `null` (inherit). For Hybrid
  panes that DID have a per-Hybrid theme set, preserve
  that value as the override.

## Acceptance criteria

### Revert Appearance from HybridEditorConfig

1. Read `web/src/components/HybridEditorConfig.svelte`
   tail; identify the Appearance section's exact code
   span.
2. Move the Appearance markup + `setThemeChoice` import +
   the 3 Appearance test pins back into
   `SettingsPanel.svelte` (and `SettingsPanel.appearance.test.ts`
   if that's the layout; otherwise revert into whatever
   test file held the original Appearance pins).
3. `HybridEditorConfig.svelte` no longer carries
   Appearance. Layout, Date Pills, On Save stay.

### Add per-Hybrid theme override

1. Add a `themeOverride: 'light' | 'dark' | null` field
   to the Hybrid wire format (likely `tabs.svelte.ts`
   pane state).
2. UI: 3-option toggle (Inherit / Light / Dark) in BOTH
   `HybridEditorConfig.svelte` and `HybridTerminalConfig.svelte`.
   Default selected: Inherit. Persistence round-trips.
3. Render-time resolution: post-`-a-47` collapse the
   per-Hybrid theme value uses `themeOverride` if set;
   else inherits global Settings Appearance.
4. Migration: existing Hybrid panes with a per-Hybrid
   theme value get migrated to `themeOverride =
   '<that value>'`. Panes without a per-Hybrid theme
   value stay at `themeOverride = null` (inherit).

### Tests

1. Vitest pins for the Inherit/Light/Dark toggle on
   both `HybridEditorConfig.test.ts` and
   `HybridTerminalConfig.test.ts`.
2. Vitest pins for the resolution order (override > global).
3. Vitest pins for the migration (per-Hybrid theme value
   → `themeOverride`).
4. Vitest pin verifying Appearance section is BACK in
   `SettingsPanel.svelte` (regression guard).

### Gate

* `web/npm test -- --run` green.
* `web/npm run check` 0e/0w.
* `web/npm run build` clean.

## How to start

1. Read `HybridEditorConfig.svelte` post-`-a-46`; identify
   Appearance scope.
2. Read `Pane.svelte` post-`-a-47` (or `-a-47` head) to
   understand the per-Hybrid theme resolution path; that
   resolution gets extended with the override semantic.
3. Read `SettingsPanel.svelte` post-`-a-46`; find the
   shape it expected before Appearance moved (git log or
   git show of the `-a-46` diff to find the revert
   target).
4. Apply changes.
5. Local gate green workspace-wide.
6. Append "Commit readiness" + fire poke to @@Architect.

## Coordination

* @@FullStackA lane.
* SEQUENCING: pick up AFTER `-a-47` commits (front/back
  theme collapse is the right baseline for adding
  override semantic on top). `-a-48` (FB-back
  Search/Indexing/Reports migration) can land before or
  after this one; they don't conflict.
* No interaction with other lanes.

## Numbering

Highest dispatched `-a-N` is `-a-52` (graph overhaul
queue); this is `-a-53`. `-a-54` is the flip UX redesign
(separate task).

### Queue (revised 2026-05-21)

`-a-47` (committable) → `-a-48` (FB-back migration)
→ `-a-53` (this task; theme architecture correction)
→ `-a-54` (flip UX redesign) → `-a-49..52` (graph
overhaul) → `-a-42` (About).

`-a-53` + `-a-54` insert ahead of `-a-49..52` because
they correct the Hybrid back-side wave that's currently
mid-flight; graph overhaul work doesn't depend on the
correction landing, but it's cleaner to finish the
Hybrid back-side semantic before moving to the next
major surface.

## Out of scope

* Re-naming `themeOverride` to something cleaner if a
  better name surfaces. Stay narrow.
* Adding theme override to Hybrid Graph / Hybrid File
  Browser back-sides. The Graph back hosts the node-
  colour legend (per round-2-plan); FB back hosts
  Search/Indexing/Reports settings (per `-a-48`).
  Neither is a target user surface where "render in
  light" vs "render in dark" makes sense beyond the
  pane-chrome theme; the chrome inherits from the pane,
  which inherits from the resolved theme. Don't add
  override toggles where they don't apply.
* The flip UX redesign (tab strip preserved + mirrored
  tabs + hamburger position swap + title in tab area)
  — that's `-a-54`.

## What this task is NOT

* Reverting `-a-46` wholesale. Layout, Date Pills, On
  Save migrations are CORRECT + stay in HybridEditorConfig.
  Only Appearance reverts.
* Touching `-a-45`'s Terminal Settings migration.
  scrollback MB + default TERM stay in HybridTerminalConfig
  — only the theme override ADDS to it.
* Reverting `-a-47`. The front/back theme collapse is
  still correct; this task adds the OVERRIDE layer on
  top of the collapsed per-Hybrid value.
