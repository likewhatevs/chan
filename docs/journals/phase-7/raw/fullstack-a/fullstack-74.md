# fullstack-74: move Hybrid NAV search shortcut from `s` to `f`

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex caught a keybinding conflict in Hybrid
NAV:

* **Today**: `s` (lowercase) → Search overlay,
  `S` (uppercase / Shift) → swap-tile-down.
  Case-sensitivity is the only disambiguation
  — fragile and confusing.
* **WASD group**: should be free for the
  Hybrid swap-tile actions (move/swap with
  neighbour) regardless of case.

Fix: move Search to `Cmd+K + f`. After the
move:

* `Cmd+K + w` / `a` / `s` / `d` (any case)
  → swap-tile-with-neighbour in that
  direction.
* `Cmd+K + f` → Search overlay.
* `s` / `S` no longer treated separately.

## Relevant code

* `web/src/state/shortcuts.ts` — the
  `app.search.toggle` entry (or whichever
  shortcut ID owns Search). The chord
  string flips from `Mod+K s` (or however
  it's encoded) to `Mod+K f`.
* `web/src/App.svelte` —
  `handlePaneModeKey()` dispatch.
  * Remove the `case "s"` lowercase Search
    branch.
  * `case "s"` / `case "S"` collapse into
    the swap-tile-down dispatch (matching
    how `w` / `W`, `a` / `A`, `d` / `D`
    work today; mirror that pattern).
  * Add a `case "f"` / `case "F"` branch
    that opens Search.
* `web/src/components/PaneModeHelp.svelte` —
  cheatsheet:
  * The MOVE / SWAP rows already show
    `W A S D` — no change needed for the
    swap row's label.
  * The Search row (currently shows `s`)
    flips to `f`.
* `web/src/components/paneModeKeymap.test.ts`
  and `shortcuts.test.ts` — flip any
  assertions that referenced the old `s`
  binding for Search.

## Acceptance criteria

* `Cmd+K f` opens the Search overlay (in
  either case — `f` and `F` both work).
* `Cmd+K s` triggers swap-tile-down,
  consistent with W/A/D. Lowercase + uppercase
  both fire the swap (no more case-sensitive
  Search hijack).
* The PaneModeHelp cheatsheet shows the new
  `f` binding for Search; the WASD row
  description is unchanged (still
  "Swap tile with neighbour").
* No other keymap regressions: arrows
  focus-move, 1/2/3 spawn, Tab flip, p rich
  prompt, h help, `<` / `>` dock toggles,
  Q close, all keep their current bindings.
* `app.search.toggle` shortcut definition
  in `shortcuts.ts` reflects the new chord
  (so any `chordFor()` consumers in the
  UI display `Cmd+K f` not `Cmd+K s`).

### Tests

* Vitest:
  * `Cmd+K f` dispatches the Search-open
    action.
  * `Cmd+K F` dispatches the same (case-
    insensitive after the fix).
  * `Cmd+K s` and `Cmd+K S` both dispatch
    swap-tile-down.
* Update any tests that asserted the old
  `s` → Search mapping.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* Coordinate with `-63` (clickable help
  buttons): when this lands AFTER `-63`,
  the help overlay's `s` cap becomes a
  swap-tile button (per-direction) and the
  `f` cap (new) opens Search. The case
  collapse means the cap label can be
  lowercase `s` even though pressing
  Shift+S still works.
* If `Cmd+K + F` was hypothetically bound
  to something else (e.g. Find within
  file browser is `Cmd+F` standalone, not
  Pane Mode-prefixed) — audit during impl
  and flag if there's an unexpected
  collision.
* v0.11.0-blocking: keymap consistency is
  marquee Hybrid NAV polish. Should ship
  before tag.
* Queue position: end of Lane A queue.
  Updated queue: `-70` → `-72` → `-73` →
  `-74`.
* Standing topic-level commit clearance.

## 2026-05-19 18:42 BST — @@FullStackA implementation note

Implementation:

* `App.svelte:handlePaneModeKey`:
  * Collapsed the `case "S":` swap-down branch
    into `case "s": case "S":` so both cases
    fire `paneModeSwap("down")` like the other
    WASD pairs.
  * Old `case "s":` (Search overlay open) gone.
  * New `case "f": case "F":` opens the Search
    overlay using the same commit-then-open
    pattern as the original `s` handler.
* `PaneModeHelp.svelte`: Search row's cap flips
  from `s` to `f`. WASD swap row unchanged
  (already labelled `W A S D`).
* `paneModeKeymap.test.ts`:
  * WASD test now asserts the `s/S` collapsed
    case (instead of the old "uppercase-only").
  * Search assertion targets `case "f": case "F"`.
* `shortcuts.ts`: comment header trail updated
  to record the `s → f` move (the
  `app.search.toggle` entry was pruned in
  `fullstack-42`; this is documentation only,
  no behavioural change to the registry).

Acceptance:

* Cmd+K `f` / `F` both open Search overlay.
* Cmd+K `s` / `S` both fire swap-tile-down.
* No other keymap regressions; arrows /
  spawn / Tab / `p` / `h` / `<` / `>` / `Q`
  unchanged.
* Help cheatsheet shows `f` for Search.
* Comment in `shortcuts.ts` reflects the
  new mapping for any future reader.

Gate green:

* `npm run test` (404 passed),
* `npm run check` (0 errors / 0 warnings),
* `npm run build`,
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`
  (green).

Proposed commit message:

> Move Hybrid NAV Search shortcut from `s` to `f` (fullstack-74)
>
> Cmd+K + s (lowercase) was case-sensitive — opened
> Search overlay while uppercase `S` swapped tile
> down. Move Search to `Cmd+K + f` (case-insensitive)
> so the WASD swap-tile group can fully own w/a/s/d
> in either case. PaneModeHelp + the raw-source
> keymap test updated; shortcuts.ts comment trail
> documents the move.
