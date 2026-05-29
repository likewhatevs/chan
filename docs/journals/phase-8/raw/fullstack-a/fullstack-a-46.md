# fullstack-a-46 — Editor Settings migration to Hybrid Editor back (Task C)

Owner: @@FullStackA
Cut: 2026-05-21 by @@Architect
Status: queued (sequenced AFTER fullstack-a-43 lands in HEAD)

## Goal

Migrate the Editor section out of `SettingsPanel.svelte`
into the new `HybridEditorConfig.svelte` mount point
introduced by `-a-43` (Task A).

## Background

Locked design:
[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Hybrid back-side revisited".

Scope of the migration (settings that move):

* Theme (per-Hybrid, surviving the per-Hybrid override
  from `-b-5`).
* Layout settings.
* Date Pills toggle.
* On Save toggle (from `-a-25`).

Settings storage shape is unchanged. Only the
mounting point moves.

## Acceptance criteria

* Editor section in `SettingsPanel.svelte` is removed;
  the same settings render inside
  `HybridEditorConfig.svelte` instead.
* Settings persist across reload via the existing
  `Preferences` fields.
* Tests cover the new mount point + persistence.
* Pre-push gate green.

## How to start

1. Audit current Editor section in
   `SettingsPanel.svelte`.
2. Move into `HybridEditorConfig.svelte`.
3. Remove from `SettingsPanel.svelte`.
4. Wire tests.

## Coordination

* SPA-only.
* Append "Commit readiness" + poke @@Architect when
  ready.

### Sequencing constraint — HARD prereq

Depends on
[`fullstack-a-43`](fullstack-a-43.md) landing in HEAD.

## Numbering

This is `-a-46`. See `-a-45` for the broader wave
numbering note.

## 2026-05-21 — ready for review

Three-file change. SPA-only; no Rust touched.

### Architecture

Mirrors `-a-45`'s shape: self-contained component
with merge-against-current-server save. Five sections
moved into `HybridEditorConfig.svelte`:

* Editor theme (`editing.editor_theme`).
* Appearance (`ui.themeChoice` + `editing.theme`).
* Layout / line spacing (`editing.line_spacing`).
* Date pills / date format (`editing.date_format`).
* On save / strip trailing whitespace
  (`editing.strip_trailing_whitespace_on_save`).

Two side-effects carried over from SettingsPanel:

* Live-apply `data-editor-theme` on the document root
  so the editor re-skins instantly without waiting
  for the autosave round-trip.
* Sync `editorToolsPrefs.stripTrailingWhitespaceOnSave`
  from `editing.strip_trailing_whitespace_on_save` so
  the editor's save() reads the new value before the
  autosave PATCH lands.

Dirty comparator is scoped to the five editor-related
fields, NOT the whole Preferences object — so a
SettingsPanel autosave (semantic-search edit, etc.)
doesn't trigger a spurious editor PATCH, and the
editor's PATCH can't clobber non-editor fields.

The Appearance section was included in the migration
per the task body's "Theme (per-Hybrid, surviving
the per-Hybrid override from `-b-5`)" — read as
"the global Appearance theme setting moves to
the Hybrid Editor back; the per-Hybrid override at
`pane.theme` survives unchanged." Flag if a different
read was intended.

### Files

`HybridEditorConfig.svelte` populated from the
-a-43 stub:

* Imports: `EditorTheme`, `GlobalConfig`,
  `LineSpacing`, `Preferences` types;
  `drive`, `setThemeChoice`, `ThemeChoice`, `ui`
  from store; `DATE_FORMATS`; `editorToolsPrefs`;
  `api`.
* Local `editing: Preferences | null` synced from
  `drive.info` via $effect when no local edit
  pending.
* `normalizeEditor(p)` — handles `line_spacing`
  migration ("tight" → "compact"), default
  fallback ("compact" / "standard"), and the
  catalog-default backstop for retired
  `date_format` ids.
* Side-effects: live-apply data-editor-theme;
  sync editorToolsPrefs.
* Dirty / autosave: `editorDirty()`,
  `scheduleSave()`, `save()` (merge-against-
  server pattern from -a-45).
* Save status surfaced in the header band.
* Five `<section>` blocks: Editor theme,
  Appearance, Layout, Date pills, On save.
  Single-column stack (the two-column
  `.section-row` layout from SettingsPanel
  doesn't fit a back-side surface).
* Control `name` attributes namespaced
  `hybrid-editor-theme` / `hybrid-appearance`
  / `hybrid-line-spacing` so the radios don't
  collide with any potential SettingsPanel
  controls (defensive — SettingsPanel no
  longer has these, but the namespacing is
  good hygiene).
* CSS: `.theme-row` / `.theme-opt` carried
  over locally; `.strip-toggle` replaces
  the legacy `.semantic-toggle` name that
  no longer fits its content; `.font-row`
  carried over for the Date pills select.

`SettingsPanel.svelte` trimmed:

* Imports: `EditorTheme`, `LineSpacing`,
  `setThemeChoice`, `ThemeChoice`,
  `DATE_FORMATS`, `editorToolsPrefs` all
  removed. The two side-effects ($effect for
  data-editor-theme + editorToolsPrefs) also
  removed.
* `normalizePrefs(p)` reduced to a pass-through
  (no longer normalizes line_spacing or
  date_format — `HybridEditorConfig` owns
  that now).
* Markup: 5 sections (~140 lines) + the two
  `<div class="section-row">` wrappers
  removed.
* CSS: `.section-row` + `.section-row > section`
  + `.theme-row` + `.theme-opt input[type="radio"]`
  + the 760 px `.section-row` @media query
  removed. The generic `input, select` rule
  collapsed to `input` (no select element
  remains in the overlay). `.theme-opt` stays
  because semantic-search still uses it via
  `.semantic-toggle`.

`HybridEditorConfig.test.ts` (new):

* 11 wiring pins covering warning copy, the
  three Editor theme radios, the three
  Appearance radios + setThemeChoice + sync,
  Layout radios, Date pills select +
  DATE_FORMATS iteration, On save checkbox
  bind, data-editor-theme attribute side-
  effect, editorToolsPrefs sync side-effect,
  save merge-against-server pattern, dirty
  scope, normalizeEditor defaults.
* 4 negative pins on SettingsPanel.svelte
  (regression guard: section headers gone,
  editor-only imports gone, editor side-
  effects gone, editor preference accesses
  gone).

### Gate

* vitest **621 / 621** (+15 net from -a-45's
  606: 15 new pins all in
  HybridEditorConfig.test.ts).
* svelte-check 0 errors / 0 warnings across
  3988 files (after dropping the now-unused
  `.theme-row`, `.section-row`, `select`
  combined rule, and `.theme-opt
  input[type="radio"]` reset).
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions flagged

* **Appearance included in the migration**
  per the task body's "Theme (per-Hybrid,
  surviving the per-Hybrid override from
  `-b-5`)". Alternative read: Appearance
  stays in SettingsPanel because it's a
  global UI theme that affects every
  surface. If wrong, the Appearance section
  + `setThemeChoice` import + Appearance
  tests can move back via a small follow-up.
* **`hybrid-editor-*` / `hybrid-appearance` /
  `hybrid-line-spacing` name attributes**
  on the radios. Defensive against duplicate-
  name collisions; SettingsPanel no longer
  has matching radios so this is theoretical.
* **`.strip-toggle` rename** from
  SettingsPanel's `.semantic-toggle`. The
  legacy name was content-mismatched (the
  class was applied to the On save checkbox,
  not to semantic-search). Local to the new
  component; SettingsPanel keeps its own
  `.semantic-toggle` for its actual
  semantic-search toggle.

### Suggested commit subject

```
Migrate Editor Settings to Hybrid Editor back-side (fullstack-a-46)
```

Single commit. State imports + side-effects +
markup + CSS + tests are tightly coupled
around the same move.

### Files for `git add` (per-path discipline)

* `web/src/components/HybridEditorConfig.svelte`
* `web/src/components/HybridEditorConfig.test.ts`
* `web/src/components/SettingsPanel.svelte`
* `docs/journals/phase-8/fullstack-a/fullstack-a-46.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`
  (commit-readiness poke)

Push held — multi-agent tree commit discipline.
Standing by for clearance.

## 2026-05-21 — committed as 5166223

Cleared by @@Architect with all 3 deviations
accepted. Committed as `5166223 Migrate Editor
Settings to Hybrid Editor back-side
(fullstack-a-46)`. Pre-commit
`git diff --staged --stat` matched the cleared
path list (7 files; one small deviation: I
bundled my dangling `fullstack-a-45.md`
"committed as 1f80d09" trailing append per
shared-worktree discipline to avoid leaving it
uncommitted across sessions). Post-commit
`git show --stat HEAD` confirmed no stowaways.
Push held per protocol.
