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
