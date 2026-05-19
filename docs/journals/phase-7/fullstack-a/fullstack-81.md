# fullstack-81: Graph tab title from selected node (selection wins, scope falls back)

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex flagged: the Graph tab title should
mirror what `fullstack-65` does for the Files
tab — title reflects the selected node, not
just the spawn-time scope. After `-64`, Graph
title derives from `scopeId` (e.g. `file:foo.md`
→ `foo.md`). After this task, it derives from
the currently-selected node, with the scope
as fallback when nothing's selected.

This is the "tabs are named after what you're
looking at" rule extending from FB to Graph
for consistency.

## Spec

* When a node is selected in the graph
  inspector (file / dir / tag / contact /
  language / etc.), the tab title is the
  basename / label of that node:
  * `file:foo/bar/baz.md` → `baz.md`
  * `dir:foo/bar` → `bar` (or `bar/`)
  * `tag:foo` → `#foo`
  * `contact:Miguel` → `Miguel`
  * `language:rust` → `rust`
  * Other node kinds: human-readable label
    (drop the prefix).
* When nothing is selected → title falls
  back to the basename of the **scope**
  (current `-64` behaviour: `drive`,
  `foo.md`, etc.).
* On selection change → title updates
  reactively in the tab strip.
* Truncation (`-66`'s `truncateTabTitle`)
  still applies on top.
* Tooltip (title attr) shows the full
  identifier — selection path + scope, or
  similar, so basename collisions are
  hover-disambiguated.

## Relevant code

* `web/src/state/tabs.svelte.ts` —
  `GraphTab` type from `-64`. May already
  have a `pendingSelectId` or
  `selectedNodeId` field; if not, the
  selected-node ID needs to flow from
  GraphPanel state through to the tab.
* `web/src/components/GraphPanel.svelte` —
  the `selectedNode` / `selectedFsNode` /
  `selectedId` state. The inspector
  already shows the selected node's info;
  surface the same identifier to the tab
  title path.
* `web/src/state/tabs.svelte.ts` —
  `tabLabel()` helper / `graphTitle()` /
  whichever function `-64` introduced.
  Extend to peek the selected node before
  falling back to scope.
* Hash schema: if selection isn't already
  serialized (the `gp:` field for
  `pendingSelectId` exists from `-43`;
  may not cover live selection state),
  add a hash slot so the title round-trips
  via reload.

## Acceptance criteria

* Graph tab with selected file node →
  tab title is the file basename.
* Graph tab with selected dir node →
  tab title is the dir name.
* Graph tab with selected tag node →
  tab title is `#tagname`.
* Graph tab with selected contact node →
  tab title is the contact name.
* Graph tab with no selection → title
  falls back to scope basename
  (current `-64` behaviour).
* Selection change → title updates in
  the tab strip without a full re-mount.
* Truncation rule (`-66`, max 15ch,
  `head[..]tail`) still applies.
* Tooltip carries full disambiguating
  info on hover.
* Round-trip: a URL with a selected node
  on a Graph tab restores both the
  selection and the title-from-selection
  on reload.
* Multiple Graph tabs in the same pane
  (per `-47`) each carry their own
  selection-driven title independently.

### Tests

* Vitest: title derivation function
  consumes `(selection, scopeId)`:
  * Selection present → basename / label
    of selection.
  * Selection null → fallback to scope
    derivation (current `-64` behaviour).
* Component test: two Graph tabs with
  different selections render different
  titles in the tab strip.
* Regression: Graph tab with no selection
  + drive scope → title is `drive`.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* v0.11.0-blocking-soft. Strong consistency
  win across the marquee tab kinds (Files
  + Graph both selection-driven).
* Coordinate with `-65` (Files title from
  selection) — share the basename-helper
  if `-65` factored one out; otherwise
  add a sibling helper.
* Coordinate with `-66` (truncation) —
  apply the truncation at the same call
  site as the rest of `tabLabel()`.
* Queue position: end of Lane A queue.
  Updated queue: `-75` → `-76` → `-77` →
  `-81`.
* Standing topic-level commit clearance.

## 2026-05-19 19:03 BST — @@FullStackA implementation note

Implementation:

* `tabs.svelte.ts`: extended `GraphTab` with
  `selectedNodeId?: string | null` and
  `selectedNodeLabel?: string | null`. New
  exported `graphTabLabel(t)` returns the
  trimmed `selectedNodeLabel` when present,
  else falls back to `t.title` (the
  `-64`-derived scope title cached at spawn).
  `tabLabel` routes the graph branch through
  it; `tabTooltip` appends the
  `selectedNodeId` to the existing
  `Graph: <scopeId>` template so basename
  collisions hover-disambiguate.
* `tabs.svelte.ts:cloneTab`: graph clone now
  carries the two new fields. The shared
  hash schema gains `gn?: string` (selected
  node id) and `gnl?: string` (cached
  label); both deserialize paths populate
  them and seed `pendingSelectId` from
  `selectedNodeId` so the graph load
  restores the selection on reload.
* `GraphPanel.svelte`: `setSelected(id)`
  writes back to `tab.selectedNodeId` /
  `tab.selectedNodeLabel`. New
  `graphSelectionLabel(id)` derives the
  human-readable label — prefers
  `fsNodes.find(...).name` (FsGraphNode has
  `name` directly, including the drive-root
  empty-name case which falls through), then
  `nodes.find(...).label` for the semantic /
  language views. Selection / clearing
  always updates both fields together.
* Hash serialization: `serializeTab` for
  graph appends `gn` + `gnl` when present;
  restore in both paths (URL hash + session
  layout) decodes them. `pendingSelectId`
  falls back to `selectedNodeId` so the
  graph data load picks up the selection on
  first paint.

Truncation: `truncateTabTitle` (`-66`) still
applies at the Pane.svelte call site since
the graph branch flows through `tabLabel` →
`graphTabLabel` → string.

Tests added in `tabs.test.ts`
(`graphTabLabel (fullstack-81)` describe):

* No selection / empty / whitespace-only
  label → fallback to scope title (`drive`
  / `foo.md`).
* Selection label wins (`foo.md`,
  `#search`).
* `tabLabel` routes graph tabs through
  `graphTabLabel` (regression check —
  catches accidental drift to `t.title`).

Gate green:

* `npm run test` (413 passed — 3 new
  graphTabLabel cases),
* `npm run check` (0 errors / 0 warnings),
* `npm run build`,
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`
  (green).

Visual eyeball worth doing: open a Graph
tab, tap a node — tab strip flips to the
node's label. Tap another node — title
updates reactively. Tap background to
clear — title falls back to the scope's
basename. Reload the page — selection +
title both round-trip.

Proposed commit message:

> Graph tab title from selected node (fullstack-81)
>
> Graph tab title now derives from the currently-
> selected node when one is set, mirroring
> fullstack-65's Files-tab convention. New
> `selectedNodeId` + `selectedNodeLabel` fields on
> `GraphTab` round-trip via URL hash (`gn` / `gnl`
> keys). `GraphPanel.setSelected` writes the
> selection back to the tab so the strip updates
> reactively; restore seeds `pendingSelectId`
> from `selectedNodeId` so the graph data load
> brings the focal node back on reload. No
> selection → tab title falls back to the
> scope-derived label from fullstack-64.
