# fullstack-73: add "Graph from here" action to DriveInfoBody

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex flagged: in the Graph tab, clicking
the drive root node shows the drive inspector
(via `DriveInfoBody`) but offers no "Graph
from here" action. The other inspectors
(file inspector with `onSetAsScope`, etc.)
have the equivalent button. Same gap exists
in the file-browser inspector when the drive
root is selected — symmetry across consumers.

Fix: add the action to `DriveInfoBody`
itself so every consumer surface gets it
without per-consumer wiring.

## Relevant code

* `web/src/components/DriveInfoBody.svelte`
  — drive inspector body. Currently has no
  action callbacks; renders drive info only.
* `web/src/components/GraphPanel.svelte:1158-1165`
  — renders `<DriveInfoBody />` when the
  selection is the drive root node (either
  fs-mode folder with `id === ""` or
  semantic-mode equivalent). Pass the
  "Graph from here" callback here.
* `web/src/components/FileBrowserSurface.svelte:384-385`
  — renders `<DriveInfoBody />` when
  `browserSelection.showDrive && !browserSelection.path`.
  Same callback wiring.
* `web/src/state/tabs.svelte.ts` —
  `paneModeOpenGraph` / `openGraphInActivePane`
  helpers. The action target is whatever
  spawns/re-scopes a graph rooted at the
  whole drive.

## Acceptance criteria

* `DriveInfoBody` accepts an optional
  `onSetAsScope` prop (or
  `onGraphFromHere` — pick the name that
  matches the existing inspector convention).
* When provided, renders a "Graph from here"
  button matching the visual treatment of
  the analogous button in `FileInfoBody`.
* `GraphPanel.svelte` (line 1158-1165 area):
  passes a callback that re-scopes the
  current graph to drive scope, mirroring
  the existing fs-graph `scopeFsGraphFromHere`
  pattern but for the drive root.
* `FileBrowserSurface.svelte` (line 384-385):
  passes a callback that spawns a NEW Graph
  tab scoped to drive, matching the
  "Graph from here" pattern used elsewhere
  in the FB inspector.
* Existing `DriveInfoBody` consumers that
  don't pass the callback keep working
  (no button rendered when prop is absent).

### Semantic difference between consumers

* **In the FB inspector**: "Graph from here"
  SPAWNS a new Graph tab (matches the
  `fullstack-43` context-aware Pane Mode +
  the FileInfoBody convention).
* **In the Graph tab's own inspector**:
  "Graph from here" RE-SCOPES the current
  tab (matches the existing `onSetAsScope`
  pattern in GraphPanel at line 1184-1192).

`DriveInfoBody` doesn't know which surface
hosts it — that's the consumer's call.
Each consumer passes the callback that
matches its convention.

### Drive-scope edge case

If the user is already viewing a drive-scoped
graph and clicks the drive root node, the
"Graph from here" action is effectively a
no-op (already at drive scope). Acceptable
to either:
* Show the button anyway (consistent UI;
  click is harmless).
* Hide the button when the current scope
  already IS the drive (cleaner UX, slightly
  more wiring).

Recommend: show always. Consistency >
micro-optimisation, and the click is
genuinely no-op (scope-to-drive when already
at drive → graph stays).

### Tests

* Vitest: `DriveInfoBody` renders no
  "Graph from here" button when
  `onSetAsScope` is undefined.
* Vitest: with the callback provided, the
  button renders and fires on click.
* Component test: in the Graph tab,
  clicking the drive root → DriveInfoBody
  with the button visible.
* Component test: in the FB inspector,
  selecting the drive root → DriveInfoBody
  with the button visible.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* Small symmetry fix. Strong UX
  consistency win — every inspector surface
  now offers the same "Graph from here"
  action.
* v0.11.0-blocking-soft. Ship if your queue
  has the runway; defer to v0.11.1 if not.
* Queue position: end of Lane A queue.
  Updated queue: `-70` → `-72` → `-73`.
* Standing topic-level commit clearance.

## 2026-05-19 18:39 BST — @@FullStackA implementation note

Implementation:

* `DriveInfoBody.svelte`: optional `onSetAsScope?:
  () => void` prop. Renders a `<button class="open">
  Graph from here</button>` only when the callback
  is provided, mirroring FileInfoBody's `.open`
  pattern. Style block copies the same `.open` /
  `.open:hover` rules (width 100%, var(--btn-bg),
  border-radius 4px, margin-top 0.6rem) so the
  affordance reads visually identical across
  inspector bodies.
* `GraphPanel.svelte` (drive-root branch at line
  1158): passes a callback that re-scopes the
  current graph. Filesystem-mode → calls
  `scopeFsGraphFromHere("", true)`. Semantic-mode →
  sets `graphState.scopeId = "drive"`. Selects the
  drive root node afterward so the inspector keeps
  showing the drive body. Matches the
  GraphPanel.onSetAsScope re-scope convention for
  the file/dir branches below it.
* `FileBrowserSurface.svelte` (drive-info branch
  at line 461): passes a callback that SPAWNS a
  new Graph tab via `openFsGraphForDirectory("")`.
  Matches the `graphSelection()` convention used
  for non-drive selections in this surface.

Drive-scope edge case: shown always per the spec's
recommendation. Clicking when already at drive
scope is harmless (re-scopes to drive → graph
stays the same).

Tests added in `revealBrowserActions.test.ts`:

* `DriveInfoBody renders 'Graph from here' only
  when onSetAsScope is provided` — asserts the
  prop + the `{#if onSetAsScope}` gate.
* `GraphPanel passes a re-scope callback to
  DriveInfoBody` — asserts both branches
  (filesystemMode → scopeFsGraphFromHere /
  semantic → scopeId = "drive").
* `FileBrowserSurface spawns a Graph tab from
  DriveInfoBody` — asserts the
  `openFsGraphForDirectory("")` wire.

Gate green:

* `npm run test -- revealBrowserActions` (20
  passed),
* `npm run test` (404 passed),
* `npm run check` (0 errors / 0 warnings),
* `npm run build`,
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`
  (green).

Proposed commit message:

> Graph-from-here affordance on DriveInfoBody (fullstack-73)
>
> Add an optional `onSetAsScope` callback prop to
> `DriveInfoBody.svelte`; when supplied, renders a
> "Graph from here" button styled to match
> `FileInfoBody`. `GraphPanel.svelte` passes a
> callback that re-scopes the current graph (drive
> scope for both fs and semantic modes);
> `FileBrowserSurface.svelte` passes one that
> spawns a new Graph tab via
> `openFsGraphForDirectory("")`. Symmetry with the
> existing file / directory inspector
> `Graph from here` affordances; no behavior
> change for the FileBrowserSurface non-drive
> selection path.
