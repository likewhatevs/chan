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

## 2026-05-19 14:25 BST — @@FullStackA implementation note

Implementation complete. Shape:

* `SpawnContext` type in `tabs.svelte.ts` (`{ dir: string;
  file?: string }`). `dir` = anchor directory for terminal CWD
  / new-file parent / file-browser fallback / graph dir-scope.
  `file` = file the source tab points at, used by Browser +
  Graph spawns for "select this exact node".
* `resolveSpawnContext()` in `store.svelte.ts`. Reads
  `activeLayout()` so Pane Mode's draft is honored. Branches
  per active tab kind:
  * Terminal → `{ dir: tab.cwd ?? "" }`.
  * File editor → `{ dir: parentDir(path), file: path }`.
  * Browser → consults `browserSelection.path` + `tree.entries`
    for `is_dir`; directory selection → `{ dir }`, file
    selection → `{ dir: parent, file }`. No selection →
    drive root.
  * Graph → parses `scopeId`: `file:foo` → `{ dir: parent,
    file }`, `dir:foo` → `{ dir }`, `drive` / `tag:` etc. →
    drive root. Per-tab graph selection (`selectedId`) lives
    in `GraphPanel.svelte` and isn't exposed to the layout
    layer, so we use `scopeId` exclusively for now.
* `paneModeOpenTerminal/Browser/Graph` in `tabs.svelte.ts` now
  accept an optional `SpawnContext`. Terminal forwards
  `ctx.dir` as the new tab's `cwd`. Browser sets
  `inspectorOpen: true` when a contextual node is present so
  the auto-selected node lands with its info pane open. Graph
  derives `scopeId` + `pendingSelectId` from `ctx.file ??
  ctx.dir` so `GraphPanel`'s `setSelected` pops the inspector
  on mount (matches `fullstack-32`'s "Graph from here" rule).
* `App.svelte` `handlePaneModeKey`: each spawn case calls
  `resolveSpawnContext()` and forwards. The `2` (File Browser)
  case primes the module-level `browserSelection` via
  `revealAndSelect(ctx.file ?? ctx.dir)` before the spawn so
  the new browser tab's tree lands expanded to + selecting
  the contextual node. The `4` (new file) case resolves the
  context BEFORE `commitPaneMode()` so we capture the focused
  tab at the moment of keypress.

Files touched:

* `web/src/state/tabs.svelte.ts` — `SpawnContext` type +
  context-aware `paneModeOpenTerminal/Browser/Graph`.
* `web/src/state/store.svelte.ts` — `resolveSpawnContext()` +
  `parentDirOf` + `resolveBrowserSpawnContext` +
  `resolveGraphSpawnContext`. Imported `activeLayout` and
  re-exported `SpawnContext` indirectly via tabs.
* `web/src/App.svelte` — Pane Mode handlers for `1/2/3/4`.
* `web/src/state/store.test.ts` — 12 new tests covering
  resolveSpawnContext across every source-tab kind +
  selection-vs-no-selection fallbacks.
* `web/src/state/tabs.test.ts` — 3 new tests asserting
  `paneModeOpen*` propagates the context into the spawned
  tab.
* `web/src/components/paneModeKeymap.test.ts` — raw-source
  asserts on the new App.svelte dispatch shape; catches the
  same class of accidental drift that `fullstack-40`'s
  inversion gate guards against.

Gate green:

* `npm run test -- paneModeKeymap` (7 passed),
* `npm run test -- store tabs` (84 passed),
* `npm run test` (336 passed),
* `npm run check` (0 errors / 0 warnings),
* `npm run build` (clean),
* `bash -lc 'ulimit -n 4096; scripts/pre-push'` (green).

No HOLD pokes on `event-architect-fullstack-a.md`. Standing
topic-level commit clearance applies; proceeding with the
commit.

Proposed commit message:

> Context-aware Pane Mode spawn keys (fullstack-43)
>
> Cmd+K 1/2/3/4 now anchor on the focused tab instead of always
> spawning at the drive root. New `SpawnContext` (parent dir +
> optional file) plus `resolveSpawnContext()` infer the anchor
> per source tab: terminal CWD, file's parent dir, file-browser
> selection (file vs dir via tree.entries), graph scopeId
> (file:* / dir:*). Browser spawns auto-expand and select the
> contextual node; Graph spawns set scopeId + pendingSelectId
> so the inspector pops on mount (matches fullstack-32's
> "Graph from here" rule); New file (4) creates inside the
> contextual directory.
