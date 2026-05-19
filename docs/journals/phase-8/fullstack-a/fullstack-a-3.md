# fullstack-a-3: Cmd+K cluster (label, flashing H, immediate commit for 1/2/3)

Owner: @@FullStackA
Date: 2026-05-19

## Goal

Three related Hybrid-NAV / Cmd+K behaviours:

1. **Status-bar label on Cmd+K** today prints `pane mode`. It
   should print: `Hybrid ☯ Enter commit, Esc discard, H help`.
2. **Remove the flashing "H" in the middle of the screen** when
   Hybrid NAV opens. The PaneModeHelp cheat-sheet already covers
   the H affordance; the mid-screen flash is noise.
3. **Cmd+K → 1 / 2 / 3 commits immediately.** Slots 1-3 are the
   "common" Hybrid actions; pressing one of those numbers should
   exit Hybrid mode and execute, without waiting for Enter.

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md) under the
`cmd+k` and `commands 1, 2, 3` items.

Phase-7 reference: `fullstack-61` flashed the H, `fullstack-62`
renamed "Pane Mode" → "Hybrid NAV", `fullstack-76` bumped the
flash duration. This task removes the mid-screen flash entirely
and amends the status-bar copy to match.

## Acceptance criteria

* Pressing Cmd+K shows the exact status-bar label
  `Hybrid ☯ Enter commit, Esc discard, H help`.
* No flashing H glyph anywhere on screen on Hybrid entry.
* Cmd+K followed by `1`, `2`, or `3` commits immediately
  (matches the existing Enter-to-commit path, just skipped).
* `H` still opens the help overlay (PaneModeHelp.svelte
  unchanged).

## How to start

Status-bar copy: likely `web/src/components/PaneModeStatus.svelte`
or wherever the Hybrid status string is rendered. Flash-H glyph:
look for the `fullstack-61` / `fullstack-76` work to find the
component. Immediate-commit: the Hybrid keymap dispatch table.
