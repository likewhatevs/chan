# fullstack-47: allow multiple File Browser + Graph tabs + verify tab DnD

Owner: @@FullStackB
Cut by: @@Architect
Date: 2026-05-19

## Goal

Two related cleanups on tab UX from @@Alex's
2026-05-19 12:45 BST chat note:

1. **Multiple File Browser + Graph tabs** — today (per
   `fullstack-14`'s migration), the SPA may
   deduplicate file-browser tabs / graph tabs to one
   each. Lift the restriction so users can have as
   many of each as they want, same as terminal +
   editor tabs.
2. **Tab drag-and-drop** — confirm the existing
   tab DnD machinery from `fullstack-5` (reorder)
   and `fullstack-15` (detach to pane via edge drop)
   works end-to-end on desktop. Fix anything
   missing. Mobile / tablet click-based reordering
   is out of scope for this task.

## Relevant links

* @@Alex's chat note 2026-05-19 12:45 BST.
* Predecessors:
  * `fullstack-5` — tab D&D reorder within a tab strip.
  * `fullstack-14` — File Browser + Graph as first-
    class tab types.
  * `fullstack-15` — detach tab to new pane via edge
    drop.

## Acceptance criteria

### Multiple File Browser tabs

* Pressing `Cmd+K 2` (or any spawn affordance for File
  Browser) creates a NEW File Browser tab each time,
  even if one is already open in the same pane / a
  sibling pane.
* Each File Browser tab maintains its own state
  (current dir, inspector state, expansion state).
* No silent dedup logic. If there's a "focus existing
  if duplicate" path, drop it for File Browser /
  Graph.

### Multiple Graph tabs

* Same as File Browser. Pressing `Cmd+K 3` always
  creates a new Graph tab. Each carries its own
  scope, filter chips, selected node, inspector
  state.

### Tab DnD verification

* Drag a tab within its own tab strip → reorders
  (per `fullstack-5`). Confirm works.
* Drag a tab to another pane's tab strip → moves to
  that pane's list (per `fullstack-15`). Confirm
  works.
* Drag a tab to another pane's BODY edge → splits
  that pane in the drop-edge direction (per
  `fullstack-15`). Confirm works.
* If any of these is broken, fix.

Add a small regression test asserting the spawn paths
do NOT dedup for File Browser / Graph.

## Out of scope

* Touch / mobile drag-and-drop.
* Reorder via keyboard (already covered indirectly
  by Cmd+K pane swap for moving panes; tab reorder
  via keyboard is separate, not asked for).

## How to start

1. Locate the spawn handlers for File Browser +
   Graph (probably in `tabs.svelte.ts` or sibling).
   Grep for any "focus existing" / "dedup" branch
   on those tab types and remove.
2. Verify each tab type carries independent state
   when multiple are open.
3. Tab DnD: walk through the three drag flows above
   manually + add the regression assertion.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-b-architect.md`.
