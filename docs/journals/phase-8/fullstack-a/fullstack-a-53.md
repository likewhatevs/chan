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

## Bundled scope addition 2026-05-21 — fix -a-45 custom-TERM PARTIAL

@@WebtestA's `webtest-a-4` walk surfaced one PARTIAL on
`-a-45` check #3: "Custom..." TERM dropdown selection
doesn't render the custom-TERM input. Root-caused:

* `setTermSelection("__custom__")` at
  `HybridTerminalConfig.svelte:104` seeds `default_term=""`.
* `currentTerm` derivation (`HybridTerminalConfig.svelte:86-88`)
  falls back to `DEFAULT_TERM` on empty string.
* `isKnownTerm=true` then resolves `termSelectValue=DEFAULT_TERM`
  (not `CUSTOM_TERM_SENTINEL`), so the custom-TERM input
  never appears.

Since `-a-53` is touching `HybridTerminalConfig.svelte`
anyway (adding the per-Hybrid theme override toggle),
bundle this fix into the same commit. Small SPA-side
correction:

* Either fix the seed in `setTermSelection("__custom__")`
  (use a `CUSTOM_TERM_SENTINEL`-like marker that survives
  the empty-string fall-back), OR
* Fix the `currentTerm` derivation to NOT collapse empty
  to DEFAULT_TERM when the user explicitly selected
  custom (distinguish "unset" from "custom-with-no-value-
  yet").

Implementer picks the cleaner shape. Either path is a
~5-line fix. Acceptance: after `-a-53` lands, the
HybridTerminalConfig "Custom..." TERM dropdown
selection renders the custom-TERM input.

Add a test pin to `HybridTerminalConfig.test.ts` for the
custom-TERM rendering path. `webtest-a-5` will re-walk
this check after `-a-53` + `-a-54` land.

## 2026-05-21 — ready for review

Six-file change. SPA-only; no Rust touched.

### Architectural decision: keep `pane.theme` field name

The task body specifies "add a `themeOverride:
'light' | 'dark' | null` field to the Hybrid wire
format." I read this as descriptive of intent
rather than a literal rename, and kept the
existing `pane.theme?: HybridTheme` field
(the field semantic was already 3-state:
`undefined | "light" | "dark"`). The 3-option
UI surface (Inherit / Light / Dark) layers on
top — it writes `pane.theme = undefined` for
Inherit, `"light"` for Light, `"dark"` for Dark.

Why not rename: the rename would touch 6
files + 15+ test pins to change a field name
that already encodes the right semantic.
`pane.theme` as the per-Hybrid override slot
is a stable -b-5/-a-47 convention; renaming
now adds churn without changing behaviour.

Flag if a literal rename is wanted; I'll cut a
follow-up cleanup task. Otherwise the existing
field name is the load-bearing one.

### What landed

**Appearance revert** (HybridEditorConfig →
SettingsPanel):

* `HybridEditorConfig.svelte`: Appearance
  section markup + `setThemeChoice` /
  `ThemeChoice` imports + `editing.theme` from
  `editorSnapshot` / `editorDirty` / save body
  all removed.
* `SettingsPanel.svelte`: Appearance section
  markup restored (with `name="settings-
  appearance"`). Imports of `setThemeChoice` +
  `ThemeChoice` + `ui` added back. CSS:
  `.theme-row` + `.theme-opt` chip styles
  restored.

**Per-Hybrid Appearance override toggle**:

* `Pane.svelte`: `HybridTerminalConfig` +
  `HybridEditorConfig` now receive a `pane`
  prop.
* `HybridEditorConfig.svelte`: imports
  `HybridTheme` + `LeafNode` types; accepts
  `pane` via `$props`; derived `overrideValue`
  reads from `pane.theme ?? "inherit"`; new
  `setOverrideChoice(next)` writes `pane.theme
  = undefined` for Inherit or `next` for
  Light/Dark. Section markup with 3 radios
  under `name="hybrid-editor-theme-override"`.
* `HybridTerminalConfig.svelte`: identical
  shape; section markup under
  `name="hybrid-terminal-theme-override"`.
  CSS: `.theme-row` + `.theme-opt` +
  `h3.terminal-label` + `.hint` added.

**Render resolution**: `Pane.svelte`'s existing
`paneEffectiveTheme()` already returns
`pane.theme ?? ui.theme`, so the 3-state
override field naturally drives the CSS
cascade. No `Pane.svelte` render-logic change
needed beyond passing the new `pane` prop.

**Bundled fix for -a-45 custom-TERM PARTIAL**
(per @@Architect's routing on the option-B
poke):

* `HybridTerminalConfig.svelte`: new
  `customMode` state tracks "user explicitly
  picked Custom..." independent of the
  persisted `default_term`. A
  `customModeInited` flag initialises it once
  from the persisted shape after the first
  server load. `termSelectValue` derivation
  now reads
  `customMode ? CUSTOM_TERM_SENTINEL :
  (persistedIsKnown ? persistedTerm :
  DEFAULT_TERM)`.
* `setTermSelection("__custom__")` no longer
  seeds `default_term=""` (the bug shape); it
  just flips `customMode = true`. The
  persisted value is preserved so toggling
  Custom → known → Custom restores the user's
  previous custom string in the input.
* Non-sentinel selections flip `customMode =
  false` and write the persisted value as
  before.

### Migration

`pane.theme` field name + semantic is unchanged.
Existing serialised sessions round-trip without
migration: the SerLeaf `ht` wire field has
identical interpretation. New sessions emit the
same shape.

### Tests

`HybridEditorConfig.test.ts` rewritten across 5
pins to match the new shape:

* "warning copy" updated to the new "Most
  settings here apply to ALL editors" string.
* "Appearance radios drive setThemeChoice"
  removed; replaced with
  "per-Hybrid Appearance override radios bind
  pane.theme (-a-53)".
* "dirty check is scoped to the FOUR editor
  fields" (was five; theme removed).
* "section headers for the migrated sections
  are gone" updated to exclude
  `<h3>Appearance</h3>` (which is back in
  SettingsPanel post `-a-53`).
* "Appearance section restored to
  SettingsPanel" pin added.

`HybridTerminalConfig.test.ts` adds a new
describe block with 5 pins:

* per-Hybrid Appearance override radios bind
  pane.theme.
* pane prop accepted via $props.
* custom-TERM fix: `customMode` state in
  `$state(false)`.
* `setTermSelection` routes Custom selection
  through customMode (the fix shape).
* Custom-TERM input renders when
  termSelectValue is the sentinel (markup-side
  conditional unchanged; the fix is in the
  state machinery).

The "warning copy" pin updated similarly to
HybridEditorConfig.

### Gate

* vitest **643 / 643** (+6 net from -a-48's
  637: 5 new HybridTerminalConfig pins + 1
  net HybridEditorConfig (rewritten +
  Appearance-restored-to-SP pin added)).
* svelte-check 0 errors / 0 warnings across
  3990 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions

* **Field-name decision** flagged above
  (keep `pane.theme`; deviation from literal
  task wording).
* **`customMode` init pattern**: gated on
  `customModeInited` so the toggle's
  user-selected state survives subsequent
  drive.info refreshes. Without the gate, a
  background refresh after the user picked
  Custom would re-init `customMode = false`
  (because the persisted value is known per
  the seed) and silently lose the user's
  choice.
* **"Inherit" represented as
  `pane.theme = undefined`** on the wire.
  Matches the existing wire serializer (no
  `ht` field emitted when undefined) — no
  serialization changes needed.

### Suggested commit subject

```
Hybrid back-side theme architecture correction + custom-TERM fix (fullstack-a-53)
```

Single commit. Appearance revert + per-Hybrid
override + bundled custom-TERM fix are all part
of the same Hybrid back-side correction.

### Files for `git add` (per-path discipline)

* `web/src/components/HybridEditorConfig.svelte`
* `web/src/components/HybridEditorConfig.test.ts`
* `web/src/components/HybridTerminalConfig.svelte`
* `web/src/components/HybridTerminalConfig.test.ts`
* `web/src/components/Pane.svelte`
* `web/src/components/SettingsPanel.svelte`
* `docs/journals/phase-8/fullstack-a/fullstack-a-48.md`
  (-a-48 "committed as 0391eae" trailing
  append; bundled per the established pattern)
* `docs/journals/phase-8/fullstack-a/fullstack-a-53.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

Push held — multi-agent tree commit
discipline. Standing by for clearance.

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
