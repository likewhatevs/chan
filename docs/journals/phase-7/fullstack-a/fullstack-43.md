# fullstack-43: context-aware Cmd+K spawn

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Goal

Make Cmd+K spawn keys (`1`/`2`/`3`/`4`) inherit context
from the currently-focused tab, instead of always
spawning at the drive root.

## Relevant links

* @@Alex's chat note 2026-05-19 12:45 BST.
* Predecessors: [./fullstack-39.md](../fullstack-a/fullstack-39.md),
  [./fullstack-42.md](../fullstack-a/fullstack-42.md).

## Acceptance criteria

### Context per source tab

When the user hits a spawn key in Pane Mode, the new
tab's "starting point" derives from the focused tab:

* **Focused = Terminal tab**:
  * If we can discover the shell's CWD (e.g. via the
    PTY proxy / OSC 7 / similar): use it.
  * Otherwise default to the drive root. Don't block
    on CWD discovery; if it's hard to wire, skip.

* **Focused = File Editor tab** (markdown doc):
  * Context = the doc's **parent directory**.

* **Focused = File Browser tab**:
  * Context = the currently-selected node. If the
    selection is a file, use its parent directory
    for terminal / file browser spawns; use the file
    itself for graph spawns (so the graph auto-
    selects it per `fullstack-32`).
  * If nothing is selected, fall back to the
    browser's current root.

* **Focused = Graph tab**:
  * Context = the graph's current scope (directory)
    OR the selected node if any. Same file-vs-dir
    handling as the file browser case.

* **No focused tab / empty pane**:
  * Default to drive root.

### Spawn key behavior

| Key | Action with context                                          |
|-----|--------------------------------------------------------------|
| `1` | Open Terminal in the contextual directory (CWD = ctx).       |
| `2` | Open File Browser scoped to / selecting the contextual node. |
| `3` | Open Graph scoped to / selecting the contextual node.        |
| `4` | New file — create a new untitled file under the contextual directory (parent dir if context is a file). |

### File Browser + Graph: select on open

* When `2` lands a File Browser tab with a contextual
  file/dir, the tree auto-expands to + selects that
  node, and the inspector opens.
* When `3` lands a Graph tab with a contextual file/dir,
  same — graph scope set + node selected + inspector
  popped (matches `fullstack-32`'s rule for "Graph
  from here").

### Tests

* Per-source-tab context resolution function tested
  in isolation.
* Each spawn key tested with each source-tab type to
  assert the right context is passed.
* "No context" fallback to drive root tested.

## Out of scope

* Custom context overrides per workflow.
* Persisting context across sessions.
* CWD discovery in the terminal if it's non-trivial
  to wire — best-effort; default to drive root if
  hard.

## How to start

1. Add a small `resolveSpawnContext(focusedTab):
   { dir, file?, kind }` helper in
   `web/src/state/tabs.svelte.ts`. Each tab type
   knows how to report its context.
2. Update the Cmd+K spawn handlers in `App.svelte` to
   call the helper + pass the context to the spawn
   functions.
3. The spawn functions (open Terminal / Files / Graph
   / NewFile) gain a `context?` parameter; default
   handling is current behavior (drive root).
4. Coordinate with @@FullStackB if File Browser /
   Graph auto-select on open needs a small API
   tweak; reuse existing reveal-in-browser path
   (`fullstack-29`) if it fits.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-a-architect.md`.
