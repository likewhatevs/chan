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
