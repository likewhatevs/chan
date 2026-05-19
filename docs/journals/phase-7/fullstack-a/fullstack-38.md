# fullstack-38: right-docked file browser mirrors text alignment

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Goal

The left-docked file browser pane (from `fullstack-1`)
aligns its tree text to the left edge. The right-docked
version should mirror that — text aligned to the RIGHT
edge, so the tree visually anchors against the closest
viewport edge. @@Alex flagged 2026-05-19 06:45 BST.

## Relevant links

* Predecessor: [./fullstack-1.md](../fullstack-a/fullstack-1.md)
  (docked side-pane File Browser).

## Acceptance criteria

* When the file-browser pane is docked on the **right**:
  * Text labels (file / dir names) render with
    right-aligned text.
  * Tree indent guides invert: indentation grows from
    the right toward the left, matching how the
    left-docked pane grows from left toward the right.
  * Icons / chevrons sit on the right side of the row,
    mirroring the left-dock layout.
* Left-docked behavior unchanged.
* Overlay (non-docked) mode behavior unchanged (it
  doesn't have a "side" — uses the default left
  alignment).
* If the user toggles between left/right dock mid-
  session, the alignment updates immediately.

## Out of scope

* The first-class File Browser tab (`fullstack-14`) —
  that's a tab, not a docked pane, alignment stays
  default.
* Drag-to-resize behavior.
* The Graph pane (no docked-graph version exists).

## How to start

1. The docked file browser component lives in
   `web/src/components/` (likely `FileTree.svelte`
   or a sibling). Locate the dock-side prop /
   detection.
2. Branch the row layout CSS on dock side. A flex-
   direction flip + `text-align: right` is probably
   the smallest change.
3. Indent guides: invert the offset direction when
   right-docked.

## Hand-off

Standard. Pre-push gate green. Coordinate with
@@WebtestA for the dock-flip walkthrough. Ping via
`alex/event-fullstack-a-architect.md`.
