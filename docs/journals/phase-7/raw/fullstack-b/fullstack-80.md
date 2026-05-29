# fullstack-80: right-click menu trims across tabs + auto-open FB inspector on click

Owner: @@FullStackB
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex flagged four coupled UX changes:

1. **Terminal right-click menu**: drop
   `Search`, `Settings`.
2. **File Browser right-click menu** (post
   `-67` / `-71`): drop `Search this`,
   `Settings`, `Show/Hide Details`.
3. **Graph right-click menu** (post `-68` /
   `-75` when it lands): drop `Settings`,
   `Show/Hide Details`.
4. **File Browser click behaviour**: clicking
   a file/dir in **tab and overlay** variants
   auto-opens the DETAILS inspector. Dock
   variants (left + right) do NOT auto-open —
   clicking just selects.

Rationale: Search + Settings are global
commands (Cmd+K f for search post-`-74`,
Cmd+, for Settings) — duplicating them in
every per-tab right-click menu is noise.
`Show/Hide Details` becomes redundant once
clicking auto-opens the inspector in
tab/overlay (the only variants where it
matters).

## Coordination notes

* `-75` (Graph right-click consistency +
  vertical filters) is queued on Lane A and
  will land before this. By the time you
  start `-80`, the Graph bubble has the
  standard menu-row shape — find the
  `Settings` + `Show/Hide Details` rows
  there and drop them. If `-75` slips past
  this task, do the trim against whatever
  shape the Graph bubble has and flag the
  coordination in your impl note.
* This task does NOT amend `-75` — it
  layers on top per the locked
  no-amendment rule.

## Relevant code

### Terminal right-click

* `web/src/components/TerminalTab.svelte` —
  the kebab / right-click menu (post
  `-52`'s "New Terminal" drop). Find the
  `Search` row + `Settings` row.

### File Browser right-click

* `web/src/components/FileBrowserSurface.svelte`
  — post `-67` the menu lives in the
  triggerless HamburgerMenu reachable via
  tab right-click (or dock body right-click
  per `-71`). Items to drop: `Search this`,
  `Settings`, `Show/Hide Details` (toggle
  inspector).

### Graph right-click

* `web/src/components/GraphPanel.svelte` —
  the `tab-menu-bubble` (from `-68`,
  restyled by `-75`). Find `Settings` row
  + `Show/Hide Details` row.

### FB click-to-inspector

* `web/src/components/FileBrowserSurface.svelte`
  and `FileTree.svelte` — the click handler
  that sets selection. Currently selection
  drives the inspector when `inspectorOpen`
  is true.
  * **Tab variant + Overlay variant**:
    clicking a file or dir should ALSO set
    `inspectorOpen = true` (or the
    equivalent per-tab/per-overlay state).
  * **Dock variant** (left + right):
    clicking just selects; do NOT auto-open
    inspector.
  * Behaviour is per-variant: read
    `variant` (already prop'd) and gate
    accordingly.

## Acceptance criteria

### Menu trims

* Terminal right-click menu: no `Search`
  row, no `Settings` row. Other entries
  (Restart, Copy Scrollback, etc.) keep.
* File Browser right-click menu (any
  variant that hosts it): no `Search this`,
  no `Settings`, no `Show/Hide Details`.
  Surviving entries: new file here, new dir
  here, reload, Stick/Unstick (dock
  variant), etc.
* Graph right-click menu: no `Settings`,
  no `Show/Hide Details`. Surviving:
  `Depth` slider, `Reload`, plus the
  vertical filter rows from `-75`.

### FB click-to-inspector behaviour

* Tab variant: clicking a file row →
  `inspectorOpen = true` for that tab AND
  selection is set to that file. The
  DETAILS panel appears (if it wasn't
  already open).
* Tab variant: clicking a directory row →
  same (inspector opens with directory
  info).
* Overlay variant: same as tab —
  auto-open on click.
* **Left dock**: clicking only sets
  selection; inspector state unchanged.
* **Right dock**: same as left.
* Per-tab `inspectorOpen` state from
  `-58`'s schema extension still
  round-trips via hash.
* If the user has explicitly closed the
  inspector (via the inspector's own
  close button) on a tab variant, the
  next click on a row should still
  auto-open it. The user opted out for
  THAT click; the next click is a new
  interaction.

### Tests

* Component test: terminal-tab right-click
  menu DOM has no `Search` / `Settings`
  labels.
* Component test: FB right-click menu DOM
  has no `Search this` / `Settings` /
  `Show/Hide Details` labels.
* Component test: Graph right-click menu
  DOM has no `Settings` / `Show/Hide
  Details` labels.
* Component test: FB tab variant click on
  a file row → `inspectorOpen = true`.
* Component test: FB dock variant click on
  a file row → `inspectorOpen` unchanged
  (selection still updates).
* Negative grep: the dropped string
  literals don't appear in the rendered
  menu surfaces of the affected components.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* v0.11.0-blocking-soft for the menu trims
  (UX polish), v0.11.0-blocking-soft for
  the FB click-to-inspector (behavior
  change improves the marquee multi-FB
  surface from `-58`).
* Re-walk: light. Lane A's 8801 can
  spot-check all four changes in a single
  pass after this lands.
* Hygiene sweep: any handlers / imports
  only used by the dropped menu rows can
  go too (mirror the `-52` / `-60`
  pattern).
* Queue position: end of Lane B queue.
  Updated queue: `-78` → `-79` → `-80`.
* Standing topic-level commit clearance.

## 2026-05-19 23:25 BST — implementation

**Terminal trims.**
* Dropped `Search` row at `TerminalTab.svelte:1024-1030`
  (the row + the `openSearch` helper + its only
  consumer the `searchPanel` import).
* Dropped `Settings` row at `:1031-1037`
  (the row + the `openSettingsFromMenu` helper +
  the `openSettings` import + the `Settings`
  lucide icon).
* Find row kept — different chord, different
  behaviour.

**File Browser trims** (shared menu reaches tab +
dock variants by `-71`'s impl):
* Dropped `Show/Hide Details` row + the
  `toggleInspector` helper (only consumer was
  the dropped row).
* Dropped `Search this` row + the `searchDrive`
  helper + the `searchPanel` import.
* Dropped `Settings` row + the `doOpenSettings`
  helper + the `openSettings` import.
* Lucide icons no longer used after the trim:
  `Search`, `Settings`.

**Graph trims** (both the bubble at the bottom
AND the `menuItems` snippet for the hamburger
overlay variant):
* Dropped Show/Hide Details row + `toggleInspector`
  helper (no remaining consumer).
* Dropped Settings row + `doOpenSettings` helper
  + the `openSettings` import.
* Lucide icons no longer used: `ArrowLeft`,
  `ArrowRight`, `Settings` (the bubble's
  inspector toggle was their last consumer).
* Updated the existing `revealBrowserActions.test.ts`
  bubble-shape assertion to flip from the
  toggleInspector check to the depth-row check
  (depth slider is now the bubble's canonical
  first row).

**FB click-to-inspector wiring.**
* `FileTree.svelte` gains an `onClickRow?: (path:
  string) => void` prop. `selectPath` no longer
  pokes `browserOverlay.inspectorOpen = true`
  directly — it just sets the selection +
  invokes `onClickRow?.(path)`.
* `FileBrowserSurface.svelte` provides
  `onRowClicked(path)`: if `isTab || isOverlay`,
  sets `browserState.inspectorOpen = true`. Dock
  variants ignore (and they have no inspector
  pane anyway via `isWideSurface`).
* This preserves the existing overlay
  auto-open behaviour AND extends it to tab
  variant (using the per-tab `browserState
  .inspectorOpen` slot from `-58`).

**Keyboard navigation unchanged.** `selectPath`
runs only on `onclick` handlers in FileTree;
arrow-key navigation writes
`browserSelection.path` directly without firing
the click hook. Click vs keyboard remains
distinguishable per the spec.

**Edits:**

* `web/src/components/TerminalTab.svelte`:
  Search + Settings rows dropped, helpers
  dropped, imports cleaned.
* `web/src/components/FileBrowserSurface.svelte`:
  Search this + Settings + Show/Hide Details
  rows dropped, helpers dropped, imports
  cleaned. `onRowClicked` helper added.
* `web/src/components/FileTree.svelte`:
  `onClickRow` prop added, `selectPath`
  rewired to use the hook.
* `web/src/components/GraphPanel.svelte`:
  Bubble + menuItems snippets dropped Show
  Details + Settings rows; handlers dropped;
  imports cleaned.
* `web/src/components/revealBrowserActions.test.ts`:
  flipped the GraphPanel bubble-shape
  assertion to match the post-trim shape
  (depth-row first, no toggleInspector /
  doOpenSettings).
* `web/src/components/menuTrims.test.ts`
  (new): 15+ assertions across Terminal, FB,
  Graph trims + FB click-to-inspector wiring.

**Gate.** `npm run check` 0/0; `npm run test`
42 files / 433 tests (was 41 / 417; +13 from
new sentinels + the bundled `-82` work +
parallel-lane carryover); `npm run build`
clean; `scripts/pre-push` green.

**Re-walk.** Lane A's 8801 can spot-check
all four changes (Terminal, FB, Graph trims +
FB click) in a single walkthrough after this
lands per the task note.

**Visual eyeball.** Skipped — string-grep
sentinels pin all the menu drops, and the
click-to-inspector wiring is a single function
that gates on `isTab || isOverlay`. If @@Alex
flags edge cases (e.g. inspector flicker, or
keyboard nav unexpectedly triggering
auto-open) on the walkthrough, follow-up.

**Coupled with `-82`**: bundled in the same
commit since they're tightly related (both
trim the FB shared menu; the audit pass after
the FB trims is what catches the surviving
dock-variant Open overlay entry).

**Out of scope:**
* Inspector close-button-to-stay-closed
  behaviour: the user can still close the
  inspector via its own × button; the next
  row click will reopen it. This matches the
  acceptance criterion ("user opted out for
  THAT click; the next click is a new
  interaction").

**Commit readiness:** see -82 task file for the
bundled commit description.
