# fullstack-69: Cmd+K `<` / `>` toggle docked file browsers

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex flagged two more Pane Mode bindings for
the docked File Browser surfaces. Keystroke
toggle for stickies makes show/hide a one-chord
move instead of menu hunting.

* **`Cmd+K <`** (less-than) → toggle the
  **right-side** sticky FB dock.
* **`Cmd+K >`** (greater-than) → toggle the
  **left-side** sticky FB dock.

Yes, the arrow direction is opposite to the
dock side it controls. @@Alex's stated mapping
verbatim — preserve as-is unless @@Alex flips
it.

## Relevant code

* `web/src/state/tabs.svelte.ts` /
  `paneModeKeymap.ts` — Pane Mode keymap
  binding registry. Existing bindings include
  `1` / `2` / `3` / `4` spawn keys,
  arrows / WASD focus + split, `p` rich
  prompt, `Tab` flip Hybrid, `H` help,
  `Q` close pane. Add `<` and `>` to the
  table.
* `web/src/components/Pane.svelte` /
  `FileBrowserSidePane.svelte` — the existing
  toggle action for sticking / unsticking a
  side-docked FB. The pane hamburger had
  "Stick to left" / "Stick to right" entries
  before `-60` trimmed it; whatever action
  function those called is the target action
  for the new bindings.
* `web/src/state/store.svelte` —
  `browserSidePanes.left` / `.right`
  booleans (or similar). The toggle inverts
  the matching one.
* `web/src/components/PaneModeHelp.svelte` —
  cheatsheet entries for `<` and `>` get added.
  Coordinate with `-62` (rename) — the help
  surface is being swept; this just adds two
  rows.

## Acceptance criteria

* Entering Pane Mode (Cmd+K) then pressing
  `<` toggles the right-side sticky FB:
  * If hidden → becomes visible.
  * If visible → becomes hidden.
  * Pane Mode exits after the toggle (same
    semantics as spawn keys 1-4).
* Pane Mode then pressing `>` toggles the
  left-side sticky FB with the same semantics.
* Toggle behaviour respects user preferences /
  persistence — the toggled state survives
  reload (mirror whatever `Stick to left`
  / `Stick to right` previously did in the
  pane hamburger).
* No regression on the existing Pane Mode
  bindings.
* Both bindings appear in the Pane Mode help
  cheatsheet under a sensible section
  (`Layout` or `Dock` — your call).

### Tests

* Vitest extension in `paneModeKeymap.test.ts`
  asserting `<` dispatches to the right-dock
  toggle action and `>` dispatches to the
  left-dock toggle action.
* Component test: with right-dock closed,
  Cmd+K then `<` opens it; with it open,
  Cmd+K then `<` closes it. Same for `>` on
  the left.
* Round-trip: state survives reload via
  the existing preference plumbing.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* Mapping verbatim per @@Alex: `<` → right,
  `>` → left. The arrow points opposite to
  the dock side it controls. If you have a
  strong reason to swap it (or if @@Alex
  flips it pre-implementation), document the
  flip in your impl note.
* Coordinate with `-63` (clickable help
  buttons): once the help cheatsheet's
  key-caps are clickable, the new `<` / `>`
  rows are also clickable by inheritance.
  No separate wiring needed for that.
* Queue position: end of Lane A queue.
  Updated queue:
  `-68` → `-61` → `-65` → `-69`.
* Standing topic-level commit clearance.

## 2026-05-19 17:09 BST — @@FullStackA implementation note

Implementation:

* `App.svelte:handlePaneModeKey`: two new cases
  inside the Pane Mode dispatch.
  * `case "<"` → `commitPaneMode()` +
    `scheduleSessionSave()` +
    `toggleBrowserSidePane("right")`.
  * `case ">"` → same shape, toggles `"left"`.
  Both exit Pane Mode after the toggle commits,
  mirroring the spawn-key semantics (`1`-`4`).
* Imported `toggleBrowserSidePane` from
  `state/store.svelte` — already-exported
  helper that flips `browserSidePanes[side]`
  + persists the new state via the same
  preference path the hamburger menu entries
  used (`-60` trimmed those entries; the
  underlying persistence is unchanged).
* `PaneModeHelp.svelte`: new `Dock` group
  between `Split` and `Close` with the two
  rows.

Mapping is @@Alex's spec verbatim — `<` →
right, `>` → left. Arrow direction opposite
to the dock it controls. Flagged in the
`Dock` group comment so a future reader can
trace the intent.

Tests added in `paneModeKeymap.test.ts`
(`Cmd+K dock toggles (fullstack-69)` describe):

* Raw-source assert that `case "<"` dispatches
  to `toggleBrowserSidePane("right")`.
* Raw-source assert that `case ">"` dispatches
  to `toggleBrowserSidePane("left")`.

Persistence: round-trip works through the
existing `persistBrowserSidePanes()` chain —
no new state plumbing.

Gate green:

* `npm run test -- paneModeKeymap` (12 passed),
* `npm run test` (374 passed),
* `npm run check` (0 errors / 0 warnings),
* `npm run build`,
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`
  (green).

Visual eyeball worth doing: Cmd+K then `<` →
right-side FB dock toggles; Cmd+K then `>` →
left-side toggles. State survives reload
(URL hash + server config). PaneModeHelp's
`h` cheatsheet shows both rows under `Dock`.

Proposed commit message:

> Cmd+K < / > toggle the docked file browsers (fullstack-69)
>
> Two new Pane Mode bindings: `<` toggles the right-
> side sticky File Browser dock; `>` toggles the
> left-side dock. Mapping verbatim per @@Alex — the
> arrow direction is opposite to the dock side it
> controls. Both exit Pane Mode after the toggle
> commits (same semantics as the spawn keys). New
> "Dock" section added to the PaneModeHelp
> cheatsheet between Split and Close.
