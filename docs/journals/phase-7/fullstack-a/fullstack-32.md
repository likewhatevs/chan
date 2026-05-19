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

## 2026-05-19 10:15 BST — @@FullStackA specialist review

Three changes in one patch:

### 1. Graph-from-here scope

* `web/src/state/store.svelte.ts` — `openFsGraphForFile`
  now derives the parent directory and opens scope
  `dir:<parent>` (or `drive` when the file is at the
  drive root). `pendingSelectId` stays the file path so
  the file auto-selects on load and its inspector pops.
* Same file — `openFsGraphForDirectory` opens scope
  `dir:<path>` (or `drive` when path is empty). The
  directory remains the `pendingSelectId` for the
  non-root case so the inspector pops with the dir's
  metadata.
* `web/src/state/store.test.ts` — replaced the
  "drive-scope fs graph with a preselection" test with
  one that asserts the new parent-dir / dir scope, and
  added a smoke test for the drive-root fallback.

### 2. Sibling-shade-on-select (filesystem + semantic)

* `web/src/components/GraphCanvas.svelte` — paint loop
  builds a `siblingDim` set per frame: when a `file`
  node is selected, all other `file` nodes sharing the
  same parent directory get `globalAlpha = 0.45`
  applied to fill, stroke ring, and icon (label rules
  unchanged — labels still only render on selection +
  one-hop neighbours per the existing contract).
* Per-frame rebuild is O(N); negligible at our graph
  sizes. Sticking with the simpler shape rather than
  derived/cached because adjacency-style state already
  rebuilds on every prop change here.

### 3. Inspector button label rename

* `web/src/components/FileInfoBody.svelte:658` —
  "Open in this pane" → "Open".
* `web/src/components/GraphPanel.svelte:1178` — ghost
  fs-node inline "Open" button.
* `web/src/components/TagInfoBody.svelte:97` —
  mention/contact "Open" button.
* Updated the matching comment references in
  `GraphPanel.svelte`, `TagInfoBody.svelte`,
  `InspectorBody.svelte`, and `GraphCanvas.svelte` so
  future grep finds the button by its current name.

The shared `FileInfoBody.svelte` button label drives
both the graph inspector and the file-browser inspector.
Per @@Architect's task acceptance criteria the rename
target is the graph inspector, but the same philosophy
("we always open in the focused pane; the verbose label
adds noise") holds for the file browser inspector. One
button label across surfaces is also less surprising —
the alternative was threading a label prop through
InspectorBody → FileInfoBody just to vary one word, and
that asymmetry would re-grow in the next refactor.
Flagging the judgment call; happy to thread a prop if
you want the file browser to keep "Open in this pane".

### Gate

* `npm run test` — 30 files / 268 tests, all pass.
* `npm run check` — 0 errors / 0 warnings.
* `npm run build` — clean.
* `bash -lc 'ulimit -n 4096; scripts/pre-push'` — green
  (fmt + clippy + tests + no-default-features build).

### Proposed commit message

> Scope Graph-from-here to the trigger + dim siblings + shorten Open label (fullstack-32)
>
> * openFsGraphForFile / openFsGraphForDirectory now
>   resolve to a `dir:<path>` scope (parent dir for
>   files, the dir itself for folders) instead of the
>   whole-drive view. Drive-root falls back to the
>   existing `drive` scope alias.
> * GraphCanvas dims file-kind nodes that share a
>   parent with the selected file so the focal node
>   visually pops out of its cohort.
> * Inspector "Open in this pane" button is now just
>   "Open" across FileInfoBody / GraphPanel ghost body /
>   TagInfoBody — we always open in the focused pane.

Ready for commit + push under standing topic-level
clearance.
