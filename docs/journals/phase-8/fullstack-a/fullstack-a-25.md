# fullstack-a-25: Editor trailing-whitespace toggle moves from menu to Settings

Owner: @@FullStackA
Date: 2026-05-20

## Goal

The "auto-remove trailing whitespace on save" toggle
currently lives as a checkbox in the editor menu
(right-click or hamburger — find it). Move it to the
Settings page where preferences belong. Remove from the
menu.

## Background

@@Alex 2026-05-20:

> Editor: we added those options to the menu to
> auto-remove trailing space, but that should be in the
> settings where it belongs, not in a checkbox in the
> menu; pls fix.

User-visible effect: cleaner editor menu; trailing-space
preference managed alongside other editor preferences in
Settings.

## Acceptance criteria

* Trailing-whitespace toggle is REMOVED from the editor's
  right-click / hamburger menu.
* Trailing-whitespace toggle is ADDED to the Settings
  page. Group with other editor preferences (if a "Editor"
  section exists; create one if not).
* Hint text under the toggle: short explanation
  ("Strip trailing whitespace from each line when the
  file is saved.").
* Preserves current behaviour: when on, trailing
  whitespace stripped on save; when off, not.
* Default value preserved (whatever the current default
  is — audit before flipping).
* Migration: existing user-preference value carries over
  to the new Settings storage. If the value already
  persists via the same store the Settings panel reads
  from, no migration code needed — just relocate the
  UI binding.
* `npm run check` + `npm run build` clean.
* Vitest pin if the existing menu-checkbox had one;
  otherwise no new test needed (the binding's a simple
  store read/write).

## How to start

1. Grep for the trailing-whitespace toggle in the editor
   components. Likely candidates:
   * `web/src/editor/Wysiwyg.svelte`
   * `web/src/editor/Source.svelte`
   * `web/src/components/PaneTabContextMenu.svelte` or
     similar tab-level menu component.
   Search for "trailing" / "whitespace" / "trimTrailing"
   string literals.
2. Read the binding's storage layer (settings store?
   per-pane local state? localStorage?). The Settings
   panel needs to read/write from the same source so the
   move is a UI-only relocation.
3. Add a Settings entry under the Editor section in
   `SettingsPanel.svelte`. Reuse the same toggle / chip
   shape the other Settings entries use for consistency.
4. Remove the menu entry. Drop any now-unused imports.
5. Test in lane-A: save a file with trailing whitespace
   both with the toggle on (whitespace stripped) and off
   (preserved).
6. Pre-push gate.

## Coordination

* @@WebtestA verifies on lane-A drive once landed.
* No backend / Rust work in this task.
* Pairs well with the other `fullstack-a-21` /
  `fullstack-a-24` Settings-side work (same SettingsPanel
  component, same store).
* Independent of `fullstack-a-23` (FB dock separator) and
  `fullstack-a-24` (rich prompt redesign); can land in
  any order with them.

## 2026-05-20 — implementation note

### Storage was already in the right place

The `strip_trailing_whitespace_on_save` field already lives
in the per-device-global `Preferences` shape (round-tripped
via `/api/config`) and reads through
`web/src/state/editorTools.svelte.ts::editorToolsPrefs`
which is populated by `applyServerPreferences(...)` on every
drive refresh. So the move was UI-only: relocate the binding
from the editor menu to the Settings panel.

### What landed

* `FileEditorTab.svelte`:
  * Removed the "Run automatically on save / auto-save"
    checkbox menu entry (the manual one-shot "Remove
    trailing whitespace" button above it stays).
  * Removed the now-unused `doToggleAutoStripWhitespace`
    function + `SquareCheck` icon import +
    `editorToolsPrefs` / `persistStripTrailingWhitespaceOnSave`
    imports. (`Square` stays — used by the Close menu
    entry below.)

* `SettingsPanel.svelte`:
  * New "On save" section after "Date pills" (inside the
    same `.section-row` 2-column grid so it sits alongside
    other editor-prefs sections rather than as a full-width
    block).
  * Toggle uses the existing `.theme-opt.semantic-toggle`
    chip shape introduced by `fullstack-a-21` so it
    visually matches the other checkbox toggle on the page
    without a new visual style.
  * Binding: `bind:checked={editing.strip_trailing_whitespace_on_save}`.
    Autosave handles persistence the same way it handles
    every other `editing.*` field.
  * Sibling `$effect` keeps `editorToolsPrefs.stripTrailingWhitespaceOnSave`
    in sync with `editing.strip_trailing_whitespace_on_save`
    so the strip-on-save behaviour applies the moment the
    user toggles, without waiting for the 500 ms autosave
    debouncer + the next `/api/config` round-trip. The
    PATCH still fires for durable persistence.

### Why a $effect-driven sync, not the existing helper

The existing `persistStripTrailingWhitespaceOnSave(value)`
helper in `editorTools.svelte.ts` does the PATCH itself.
Using it from Settings would race with the SettingsPanel's
own autosave (the same field is in `editing.*`; autosave
PATCHes the whole `Preferences` block; a sibling field
change after a `persistStripTrailingWhitespaceOnSave` would
include the old value in the next autosave and clobber the
toggle). The $effect-only sync avoids the race by leaving
persistence to the autosave path and only mirroring the
in-memory snapshot used by save-time stripping.

### Default value — audited

`Preferences.strip_trailing_whitespace_on_save: boolean` —
no optional / nullable, no migration needed. The default in
`editorToolsPrefs` and on a fresh `Preferences` payload is
`false`. The relocation preserves this.

### Files touched

* `web/src/components/FileEditorTab.svelte` — menu entry
  removed; three dead imports + one dead function dropped.
* `web/src/components/SettingsPanel.svelte` — `editorToolsPrefs`
  import; sync `$effect`; new "On save" section in the
  section-row grid.

### Pre-push gate

vitest 491/491 green (other lanes added +10 tests since my
last gate; all pass alongside mine); `npm run check` 0
errors / 0 warnings; `npm run build` clean.

### Lane-A verification

(post-restart):

1. Open Settings (Cmd+,). New "On save" section appears
   after "Date pills" with a "Strip trailing whitespace on
   save" toggle. Default off (matches the pre-relocation
   default).
2. Toggle on. Open a file with trailing whitespace, hit
   Cmd+S → trailing whitespace stripped on the save.
3. Toggle off. Hit Cmd+S → trailing whitespace preserved.
4. Right-click in the editor → context menu. The
   "Remove trailing whitespace" one-shot button is still
   there for manual cleanup; the "Run automatically on
   save / auto-save" checkbox is gone.
5. Persistence: close + reopen chan → the toggle's last
   state is restored from the server.

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Clean UI-only relocation. The "storage was already in
the right place" finding (`strip_trailing_whitespace_on_save`
already lived in `Preferences`, round-tripped via
`/api/config`, surfaced through `editorToolsPrefs`) is
exactly what the task hoped — no migration work needed,
just relocate the binding.

The `$effect`-driven sync to keep `editorToolsPrefs`
in-memory in lockstep with `editing.strip_trailing_whitespace_on_save`
is the right shape for the race you identified — calling
`persistStripTrailingWhitespaceOnSave` directly would
have collided with SettingsPanel's own autosave PATCH
of the same `Preferences` block. Leaving persistence to
autosave + only mirroring the in-memory snapshot
sidesteps the clobber cleanly. Good engineering instinct.

The visual choice (`.theme-opt.semantic-toggle` chip
shape introduced by `fullstack-a-21`) keeps the new
toggle consistent with the other Settings checkboxes
without introducing a new style. The manual one-shot
"Remove trailing whitespace" button stays in the editor
menu — that's a different affordance (explicit one-time
action) than the auto-on-save preference; leaving it in
the menu is correct.

Dead-code cleanup (three unused imports + one unused
function dropped) is the right hygiene that's easy to
miss.

Pre-push gate green (vitest 491/491 — +10 from earlier
baseline reflecting other lanes' new tests; check 0/0;
build clean).

**Commit clearance**: approved. Suggested commit subject:

```
Settings: trailing-whitespace-on-save toggle relocated from editor menu (fullstack-a-25)
```

Push waits until end of Round 2.

After commit: `-26` (markdown editor toolbar parity)
and `-27` (Hybrid hamburger: dark/light + flip) remain
in your Round-1 detour queue. Both small.