# fullstack-40: invert Cmd+K WASD ↔ arrow semantics

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Goal

Swap the two keybind families in Cmd+K transactional
pane mode. @@Alex's mental model: **arrows = move my
cursor (navigate)**, **WASD = move stuff (swap)**.
Today's binding is the reverse (from `fullstack-16`'s
original spec).

## Relevant links

* @@Alex's chat note 2026-05-19 12:15 BST.
* Predecessor: [./fullstack-16.md](../fullstack-a/fullstack-16.md)
  (Cmd+K transactional pane mode).

## Acceptance criteria

### New binding (post-inversion)

| Key                       | Action                                          |
|---------------------------|-------------------------------------------------|
| `↑` / `←` / `↓` / `→`     | **Move focus** up / left / down / right         |
| `W` / `A` / `S` / `D`     | **Swap** focused tile with neighbour direction  |

### What stays unchanged

* All other Cmd+K bindings (resize `[ ] - =`,
  equalize `0`, commit/discard Enter/Esc) stay as
  shipped.
* The 8 bindings from `fullstack-39` (1/2/3/4, /, \,
  x, k) stay.
* Visual chrome (tint, focused-pane border, status
  pill) stays.
* Transactional semantics (Cmd+K snapshots, Enter
  commits, Esc rolls back) stay.

### Update what users see

* If there's a help / cheatsheet rendered inside the
  Cmd+K mode (status bar pill, or a small inline
  hint), update it to reflect the new mapping.
* Update the spec section in
  `ui-exploration.md` if it locks the old mapping —
  this is a real spec change, not just code.

## Out of scope

* New keybind families.
* Customization / configurability.

## How to start

1. `web/src/App.svelte:handlePaneModeKey` — locate the
   WASD case + the arrow case. Swap which one calls
   `paneModeMoveFocus` vs `paneModeSwap`.
2. Existing tests that assert WASD moves focus / arrow
   swaps should be updated to the new mapping.
3. Update any inline hint text.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-a-architect.md`.

## 2026-05-19 13:08 BST — @@FullStackA specialist review

### Patch

* `web/src/App.svelte:handlePaneModeKey` — swapped the WASD and
  Arrow switch arms. Arrows now call `paneModeMoveFocus`; WASD
  calls `paneModeSwap`. Added a short comment citing @@Alex's
  mental model + this task as the reason so a future reader
  doesn't "fix" it back.
* `docs/journals/phase-7/ui-exploration.md` — keymap table
  inverted; appended a dated `(Updated 2026-05-19 per
  fullstack-40 ...)` line so the change is auditable.

### What stayed unchanged

* All other Cmd+K bindings (`[ ] - =`, `0`, Enter, Esc, the
  spawn / split / close set from `fullstack-39`) untouched.
* The status-bar pane-mode pill in `AppStatusBar.svelte` only
  prints "pane mode · Enter commit · Esc discard"; no inline
  keymap text to update.
* The pane-mode-preview block in `Pane.svelte` shows the tab
  title — no keymap reference, no edit needed.

### Tests

* `web/src/components/paneModeKeymap.test.ts` — new raw-source
  test asserting App.svelte's switch arms route ArrowUp/Left/
  Down/Right to `paneModeMoveFocus` and W/A/S/D to
  `paneModeSwap`. The dispatcher is inline in the App
  component and not easy to mount in isolation, so the
  raw-source check is the pragmatic guard. Catches an
  accidental revert to the `fullstack-16` defaults.
* The existing `paneModeMoveFocus` / `paneModeSwap` unit
  tests in `tabs.test.ts` continue to assert correct
  underlying behaviour; this task only changes which key
  triggers which.

### Gate

* `npm run test -- paneModeKeymap` — 2 passed.
* `npm run test` — 33 files / 291 tests, all pass.
* `npm run check` — 0 errors / 0 warnings.
* `npm run build` — clean.
* `bash -lc 'ulimit -n 4096; scripts/pre-push'` — green.

### Proposed commit message

> Invert Cmd+K WASD ↔ arrows in pane mode (fullstack-40)
>
> Arrows now move focus, WASD now swaps tiles — matching
> @@Alex's mental model (arrows navigate, WASD moves stuff).
> All other Cmd+K bindings (resize, equalize, spawn / split /
> close from fullstack-39) stay. Adds a raw-source test that
> guards the new mapping and updates ui-exploration.md.

Ready for commit + push under standing topic-level
clearance.
