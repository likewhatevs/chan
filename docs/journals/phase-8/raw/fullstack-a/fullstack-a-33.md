# fullstack-a-33: Graph "from here" as default mode + parent-inspector ancestor navigation + drop explicit button

Owner: @@FullStackA
Date: 2026-05-20

## Goal

Today the graph view has an explicit affordance to engage
"graph from here" mode — scoping the graph to the subtree
rooted at the current selection. @@Alex 2026-05-20: make
"graph from here" the default behaviour, with the graph's
parent inspector rendering the ancestor chain so the user
can navigate back up to the drive root scope. The explicit
button goes away (default mode means no button needed).

Three pieces:

1. **"Graph from here" is the default render mode** —
   when the graph view opens, the active selection (the
   doc, terminal cwd, or whatever the spawn-context
   passed in per `fullstack-a-32`) is the root of the
   rendered graph. No explicit button click required.

2. **Parent inspector shows the ancestor scope chain** —
   the graph panel's parent / breadcrumb inspector
   renders the path from the current scope root back to
   the drive. Clicking any ancestor re-scopes the graph
   to "from here" rooted at that ancestor. The drive
   root is always reachable as the topmost ancestor.

3. **Remove the explicit "graph from here" button** —
   default behaviour means the button no longer adds
   value. Drop it from the graph panel's UI.

## Background

Bug entry (this task is the dispatch):
[`../phase-8-bugs.md`](../phase-8-bugs.md) "Graph: 'graph
from here' should be default; parent inspector should
render ancestor scope navigation".

@@Alex's verbatim ask (2026-05-20):

> i want the graph's parent inspector to show the graph
> from here, enabling to go all the way back to drive
> where graph from here is the default and dont need a
> button

> e.g. from a doc, cmd+shift+m does graph from here using
> the doc; or cmd+t new terminal from current cwd or
> doc's parent dir

Paired chord migration: [`fullstack-a-32`](fullstack-a-32.md)
wires `Cmd+Shift+M` (context-aware spawn) to fire with
the focused surface as the graph's root node. This task
makes the "rooted-at-X" mode the default rendering shape
the chord fires into.

Relevant code (verify at task start):
* `web/src/components/GraphPanel.svelte` — graph view
  component; today's explicit-button affordance lives
  somewhere here or in a sibling inspector / toolbar
  component.
* `web/src/state/tabs.svelte.ts` — graph-tab SerTab
  shape; may need a new field for "scope root path" if
  not already present (or repurpose an existing one).
* Graph data fetch path — likely
  `crates/chan-server/src/routes/graph.rs::api_graph`;
  today's `?scope=drive` parameter is the wide-scope
  fetch. A new / existing `?scope=subtree&root=<path>`
  shape is needed (verify whether the endpoint already
  supports rooted-scope fetches; if not, this task adds
  the parameter wiring on the server side too).

## Acceptance criteria

* Opening the graph view defaults to "from here" mode,
  rooted at the spawn context (doc / terminal cwd /
  drive root if no context).
* The graph panel's parent inspector shows an ancestor
  breadcrumb / list from the current root back to the
  drive root. Each ancestor is clickable.
* Clicking an ancestor re-scopes the graph to "from
  here" rooted at that ancestor. The breadcrumb updates.
* The drive-root scope is always reachable as the
  topmost ancestor — clicking it produces the full
  drive-scoped graph (today's `?scope=drive` shape).
* The previous explicit "graph from here" button is
  removed.
* Existing graph features (pan, zoom, recenter from
  `fullstack-b-4`; inspector "not in current file
  listing" with the `fullstack-a-12` fix) all continue
  to work.
* `vitest` green; pin the default-mode-on-mount + the
  ancestor-click re-scope behaviour.

## How to start

1. Read `GraphPanel.svelte` — find the current explicit-
   button affordance + the data-fetch call site +
   today's default-mode behaviour.
2. Map out the ancestor-list derivation: given a root
   path P, walk up to the drive root, emit each ancestor
   as a clickable breadcrumb entry.
3. Check whether `routes/graph.rs::api_graph` already
   accepts a rooted-scope parameter. If yes: wire the
   default render to use it. If no: add it (server-
   side scope change is in scope here).
4. Drop the explicit-button affordance + any "is in
   from-here mode?" state plumbing that becomes
   redundant.
5. Verify against `systacean-2` + `systacean-4`'s graph
   fixes — directory-typed-as-file ghost emission was
   removed; the ancestor chain in this task should
   never hit those paths since it's walking real
   directories from disk.

## Coordination

* **Hard pair** with [`fullstack-a-32`](fullstack-a-32.md):
  -32's `Cmd+Shift+M` handler depends on this task's
  default-mode rendering. Land -33 first OR commit -32
  and -33 together. If -33 lands first, -32's chord
  handler reads the spawn-context from the focused
  surface + passes it to the graph spawn — the graph
  defaults to that scope per this task. If they land
  separately, -33's commit comes first.
* Coexists with [`fullstack-a-28`](fullstack-a-28.md) /
  [`-29`](fullstack-a-29.md) / [`-30`](fullstack-a-30.md) /
  [`-31`](fullstack-a-31.md). No file conflict expected
  (graph panel + chord layer + rich prompt are different
  surfaces).
* @@WebtestA verifies on lane-A. The seeded chan-source
  drive is a good test bed since it has a deep
  directory tree (`crates/chan-server/src/routes/`,
  `web/src/components/`, etc.) for the ancestor
  navigation. Verify drive-root scope still works as
  the topmost ancestor.
* Push held for the patch-release commit-grouping cut.

## 2026-05-20 — impl note + ready for review (fresh @@FullStackA session)

Two-file change: `GraphPanel.svelte` + `revealBrowserActions.test.ts`.

### Design read

Mapped out the existing shape first: graph scopes are
`drive` / `dir:path` / `file:path` / `tag:nodeId` /
`git_repo:root` / `global`. Only the first three are
path-based, so the breadcrumb only renders for those.
Tag / git_repo / global scopes hide the band (empty
`scopeAncestors` list).

Today's spawn paths already pass `scopeId: "dir:foo"` /
`"file:foo/bar.md"` to `openGraphInActivePane`, so
"default from-here mode" is structurally what happens
when a chord passes spawn context. `-32` will wire
`Cmd+Shift+M` to do exactly that for a focused doc /
terminal. -33 ships the in-graph navigation affordance
that goes with it.

The "Graph from here" buttons in the inspector live on
`DriveInfoBody` / `FileInfoBody` / `TagInfoBody` via the
shared `onSetAsScope` prop. Same component is also
consumed by `FileBrowserSurface` for the FB sidepane's
"open a graph from here" action — that surface still
needs the button. So I dropped `onSetAsScope` only from
GraphPanel's four call sites (Drive / fs-mode file+dir /
semantic-mode); the component-level prop stays.

### `scopeAncestors` derivation

```ts
type Crumb = { label: string; scopeId: string; current: boolean };
const scopeAncestors = $derived.by<Crumb[]>(() => {
  // drive  → [{drive, current}]
  // dir:a/b/c → [{drive}, {a, dir:a}, {b, dir:a/b}, {c, dir:a/b/c, current}]
  // file:a/b.md → [{drive}, {a, dir:a}, {b.md, file:a/b.md, current}]
  // tag: / git_repo: / global → []
});
```

The chain always starts with the drive root entry so
the user can hop back to drive scope from any depth.
The final entry is the CURRENT scope, rendered as plain
text (no button) since clicking it would be a no-op.
Intermediate hops are always directory scopes; the
leaf mirrors the current scope's kind so file-scoped
graphs end on `file:...` and dir-scoped ones on `dir:...`.

### `rescopeFromHere(scopeId)` semantics

Mutates `graphState.scopeId` in place — NO new graph
tab. Mirrors the existing semantic-mode handler
behaviour (line 1342 pre-fix): depth resets to 1 so a
freshly-scoped graph starts tight; selection clears so
the inspector lands on the new scope's drive-root or
file/dir body. Early return when the user clicks the
current crumb (no-op, no flash from the load effect).

Distinct from `scopeFsGraphFromHere(path, isDir)` (in
`state/store.svelte.ts`) which spawns a new graph tab —
still used by `FileBrowserSurface` for the FB sidepane's
"open a graph from here" action. Different semantics:
in-graph navigation mutates; out-of-graph entry spawns.

### Render shape

Breadcrumb band sits at the top of the `<Inspector>`
body, above the existing `{#if}` chain. Mounted only
when `scopeAncestors.length > 0` so tag / git_repo /
global scopes hide it. Each non-current segment is a
`<button class="crumb">` wired to
`rescopeFromHere(crumb.scopeId)`; current segment is a
`<span class="crumb current">`. Separator slash between
segments. Wraps on narrow inspector widths.

Styling: monospace 12.5px, `--text-secondary` ground,
`--link` colour for clickable hops, `--text` weight 600
for the current segment. Sits in a `--bg-card` band
with a `--border` bottom edge so it reads as inspector
chrome rather than file-body content.

### Tests

`revealBrowserActions.test.ts`:

* **Dropped** the old "GraphPanel passes a re-scope
  callback to DriveInfoBody" test (pinned the
  now-gone `scopeFsGraphFromHere("", true)` /
  `graphState.scopeId = "drive"` block).
* **Added** four pins:
  * Negative: no `onSetAsScope` on `<DriveInfoBody`
    anymore.
  * Negative: no `onSetAsScope` on any `<InspectorBody`
    instantiation in the graph.
  * Positive: `scopeAncestors` derived + `scope-crumbs`
    nav + button-bound `rescopeFromHere` handler +
    drive-root head of the chain.
  * Positive: `rescopeFromHere` mutates `scopeId` +
    resets `depth`; `scopeFsGraphFromHere` is gone from
    GraphPanel (still in store.svelte.ts and used
    elsewhere).

The component-test side of the breadcrumb behaviour
(actual click + scope mutation) is better verified via
the lane-A walkthrough since the graph panel's
mounting depends on Cytoscape internals jsdom doesn't
fully exercise. The text/regex pins lock the wiring
shape; @@WebtestA verifies the click + re-scope on
the seeded chan-source drive.

### Composition

* Hard-pair prereq for [`fullstack-a-32`](fullstack-a-32.md):
  -32's `Cmd+Shift+M` handler will spawn graphs at
  `dir:parentOf(focusedDoc)` / `file:focusedDoc` /
  `dir:focusedTerminalCwd`; -33's default-mode +
  breadcrumb make the spawn-into-scope land naturally
  (no extra wiring needed in -32 beyond `scopeId` +
  `pendingSelectId`).
* `fullstack-a-12` (lazy-tree ghost cleanup) untouched
  — the ghost-body branch in the inspector still
  handles broken-link nodes the same way; -33 just
  adds the breadcrumb above.
* `systacean-2` (resolver universe) + `systacean-4`
  (drop directory ghost emission) untouched — the
  ancestor chain walks string segments, never hits the
  server resolver.
* `fullstack-b-4` (graph pan/zoom/recenter) untouched
  — the breadcrumb lives in the inspector, not the
  canvas, and doesn't touch the GraphCanvas
  component.

### Gate

* vitest: **525 / 525** passed (+4 pins replacing the
  one dropped → +3 net from the 522 baseline @@FullStackA
  last reported).
* svelte-check: 0 errors / 0 warnings across 3976 files.
* npm build: clean (existing chunk-size warnings only;
  no new ones).
* No Rust changes; cargo gate skipped.

### Files touched

* `web/src/components/GraphPanel.svelte` — import drop,
  ancestor derivation, rescope helper, breadcrumb
  snippet, CSS, four `onSetAsScope` props removed.
* `web/src/components/revealBrowserActions.test.ts` —
  test pin updates per above.

### Suggested commit subject

```
Graph: ancestor breadcrumb in inspector + drop explicit "from here" buttons (fullstack-a-33)
```

### Notes for the -32 follow-on

The breadcrumb already handles drive→dir→file walks.
-32's `Cmd+Shift+M` from a focused doc just needs:

```ts
openGraphInActivePane({
  mode: "filesystem",
  scopeId: `file:${focusedDoc.path}`,
  depth: 1,
  pendingSelectId: focusedDoc.path,
});
```

The new graph spawns scoped to the doc; its inspector
breadcrumb renders `drive / parent / doc.md` with the
last segment current. User clicks `parent` → graph
re-scopes to the doc's parent dir. Clicks `drive` →
drive-wide graph. No additional wiring on the -32
side; it's literally a spawn helper that passes
context to the existing `openGraphInActivePane`.

Standing by for review.
