# fullstack-32: Graph-from-here scope + inspector "Open" label

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-19

## Goal

Two Graph-tab fixes from @@Alex's click-around
session 2026-05-19 05:30 BST:

1. "Graph from here" today opens on **"Whole drive"**
   regardless of which file/folder triggered it. Fix
   the scope-resolution so the graph opens scoped to
   what the user actually picked.
2. The graph inspector's button label currently reads
   "Open in this pane" for editable files. Rename to
   just "Open" (we always open in the focused pane;
   the verbose label adds noise).

## Acceptance criteria

### Graph-from-here scope

* Trigger from a **folder** → graph opens scoped to that
  folder's tree (its files + subdirs).
* Trigger from a **file** → graph opens scoped to the
  file's parent directory, with the originating file
  auto-selected so the inspector pops with that
  file's metadata.
* The existing "Whole drive" option in the SCOPE
  selector stays as a choice; it's just no longer the
  default-on-spawn for scoped triggers.

### Sibling-file shading on file-from-dir selection

* When a file inside a directory is selected (whether
  via auto-select from a file-trigger or by user
  click), the OTHER files in the same directory render
  with a slightly darker / desaturated shade so the
  selected node visually pops.
* The inspector opens automatically (current behavior
  for selection).
* Existing rule preserved: labels render for the
  selected element + one depth of neighbors.

### Inspector label rename

* "Open in this pane" → "Open" for editable file
  selections in the graph inspector.
* No other label changes.

## Out of scope

* Carousel widget on empty panes (separate task,
  `fullstack-35`).
* Pane chrome refinements (separate task,
  `fullstack-34`).
* Graph node visual treatment beyond the sibling-
  shade-on-select.

## How to start

1. `web/src/components/GraphPanel.svelte` — locate the
   tab-creation handler / scope initialization. Branch
   on the trigger source (file vs folder) to pick the
   right scope + auto-selection.
2. Render path: add a `sibling-dim` (or similar) CSS
   class on file-kind nodes that share a parent with
   the currently-selected file but aren't the selected
   file. Class disappears when nothing's selected or
   when the selection moves to a different surface.
3. Inspector button text: grep for "Open in this pane",
   rename to "Open".

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-architect.md`.
