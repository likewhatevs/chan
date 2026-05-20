# fullstack-a-32: Chord migration + context-aware spawn semantics + surface unification

Owner: @@FullStackA
Date: 2026-05-20

## Goal

Pull the chord migration drafted in
[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
"Chord migration + surface unification" forward into the
rich-prompt mini-wave, **expanded** with @@Alex's
2026-05-20 ask for context-aware spawn semantics.

Three coupled pieces, land as one commit:

1. **New chord set** — top-level chords replace the
   `Cmd+K <key>` spawn family.
2. **Context-aware spawn semantics** — each spawn chord
   picks up context from the focused surface.
3. **Surface unification** — empty-pane carousel slide 1
   + pane hamburger menu + empty-pane right-click menu
   all show the same first-class spawn entries.

## Background

Source: `../architect/round-2-plan.md` "Chord migration +
surface unification (added 2026-05-20)" section + @@Alex's
2026-05-20 refinement: "from a doc, cmd+shift+m does graph
from here using the doc; or cmd+t new terminal from current
cwd or doc's parent dir".

`fullstack-b-9` (Round-1) already landed Cmd+T native +
Cmd+Alt+T web + universal Hybrid NAV `t`. That part stays.

### Chord set (final)

| Action          | Native (Chan.app) | Web fallback     | Universal (Hybrid NAV) |
|-----------------|-------------------|------------------|------------------------|
| New terminal    | Cmd+T (-b-9)      | Cmd+Alt+T (-b-9) | Mod+. t (-b-9)         |
| File browser    | Cmd+O             | Cmd+Alt+O        | Mod+. o                |
| Rich prompt     | Cmd+P             | Cmd+Alt+P        | Mod+. p                |
| Graph           | Cmd+Shift+M       | (Hybrid NAV)     | Mod+. v                |

`Cmd+Shift+M` is the placeholder per @@Alex's
"pick anything else, cmd+shift+m for ex for now"; chan-
desktop overrides Tauri-side so Chrome's people-menu
doesn't capture it.

### Removals

Drop these (verify in shortcuts.ts + remove the keymap +
the help text):

* `Cmd+K 1` (was terminal — Cmd+T now)
* `Cmd+K 2` (was file browser — Cmd+O now)
* `Cmd+K 3` (was graph — Cmd+Shift+M now)
* `Cmd+K 4` (was new file — no top-level chord; reachable
  via FB context-menu / FB plus button)
* `Cmd+K p` (was rich prompt — Cmd+P now)

Keep: `Cmd+K t/T`, `Cmd+K f/F`, `Cmd+K h/H`, `Cmd+K <`/`>`,
`Cmd+K Backspace` (per existing -b-9 / phase-7 work).

### Context-aware spawn semantics

Each spawn chord derives its starting context from the
focused surface at the moment the chord fires. The
context-resolution rule:

* **Cmd+T (new terminal)**:
  * Focus on a terminal → cwd = focused terminal's cwd
    (the terminal session knows its own cwd).
  * Focus on a doc (editor surface) → cwd = doc's parent
    directory (the path one level above the doc's file
    path).
  * Focus elsewhere / unclear → cwd = drive root.

* **Cmd+O (file browser)**:
  * Focus on a doc → FB opens with the doc's parent
    directory expanded + the doc highlighted.
  * Focus on a terminal → FB opens with the terminal's
    cwd directory expanded.
  * Focus elsewhere → FB opens at drive root (today's
    behaviour).

* **Cmd+P (rich prompt)**:
  * Already context-bound to the active terminal (existing
    `showOrSpawnRichPromptInFocusedPane` does this). Wire
    Cmd+P to that same function — no new semantics needed.

* **Cmd+Shift+M (graph)**:
  * Focus on a doc → graph from here, source node = the
    doc. Pairs with [`fullstack-a-33`](fullstack-a-33.md)
    (graph "from here" as default mode).
  * Focus on a terminal → graph from here, source node =
    the terminal's cwd (graph the directory).
  * Focus elsewhere → drive-scoped graph (today's
    default).

The context-resolution helper should be a single named
function called by all four chord handlers — single source
of truth for "what's the focused surface and its
context?". Suggested shape:

```ts
function focusedSurfaceContext(): {
  kind: "terminal" | "doc" | "fb-tree-row" | "none";
  cwd?: string;      // terminal sessions
  filePath?: string; // docs / fb rows
  parentDir?: string; // derived for docs / fb rows
};
```

### Surface unification

Three menus must show the same first-class spawn entries
in the same order:

| Surface                            | File                          |
|------------------------------------|-------------------------------|
| Empty-pane carousel slide 1        | `EmptyPaneCarousel.svelte`    |
| Pane hamburger menu                | `Pane.svelte::paneMenu`       |
| Empty-pane right-click menu        | `Pane.svelte::emptyPaneMenu`  |

Items + ordering:

1. Terminal (icon + label + chord hint "Cmd+T")
2. File Browser (icon + label + chord hint "Cmd+O")
3. Rich Prompt (icon + label + chord hint "Cmd+P")
4. Graph (icon + label + chord hint "Cmd+Shift+M")
5. Separator
6. Existing items (highlight colour picker, etc.) stay
   below the separator.

Clicking each first-class item triggers the same
chord handler as the chord — so the context-aware spawn
semantics apply uniformly whether the user fires a chord
or clicks a menu item.

## Acceptance criteria

* `Cmd+O`, `Cmd+P`, `Cmd+Shift+M` all spawn the right
  surface, with context-aware semantics per the table
  above.
* Web fallbacks (`Cmd+Alt+O`, `Cmd+Alt+P`) reachable on
  both Mac + non-Mac browsers.
* Hybrid NAV `o` / `p` / `v` work universally (web +
  native).
* Old `Cmd+K 1/2/3/4/p` chords no longer fire.
* `PaneModeHelp` + `SERVE_LONG_ABOUT` reflect the new
  chord set (audit + resync; the chord-table-drift sweep
  from `fullstack-a-19` is the template).
* The three surfaces (carousel slide 1 + pane hamburger +
  empty-pane right-click) show identical first-class
  spawn entries in identical order; clicking any entry
  triggers the same chord handler.
* Context-aware spawn: spawning a terminal while focused
  on `crates/chan-drive/src/lib.rs` (in an editor tab)
  produces a terminal at `crates/chan-drive/src/`.
  Spawning a graph while focused on the same file
  produces a graph rooted at that file (depends on -a-33
  for the default rendering mode).
* `vitest` green; pin the context-resolution helper +
  the chord-handler routing.

## How to start

1. Read `web/src/state/shortcuts.ts` for the current
   chord registry. The four spawn chords each have a
   descriptor; extend with the new universal +
   web-fallback bindings.
2. Read `EmptyPaneCarousel.svelte` slide 1 (today the
   shortcuts table) + `Pane.svelte::paneMenu` +
   `Pane.svelte::emptyPaneMenu` to find the existing
   menu shapes; converge them.
3. Implement the context-resolution helper first; it
   becomes the load-bearing piece every chord handler
   calls.
4. Wire `Cmd+O` / `Cmd+P` / `Cmd+Shift+M` to call the
   helper + dispatch the spawn.
5. Drop the old `Cmd+K 1/2/3/4/p` keymap entries.
6. Resync the help cheatsheets.

## Coordination

* Couples with [`fullstack-a-33`](fullstack-a-33.md) —
  the graph "from here" default that this task's
  `Cmd+Shift+M` handler depends on. Land -33 first OR
  in the same commit if scope overlaps; the chord
  handler in -32 expects the default rendering mode in
  -33 to land first or land defensively.
* Coexists with [`fullstack-a-28`](fullstack-a-28.md) /
  [`-29`](fullstack-a-29.md) / [`-30`](fullstack-a-30.md) /
  [`-31`](fullstack-a-31.md) — different surfaces; no
  file conflict expected on the chord layer + the
  carousel / hamburger / right-click menus.
* @@WebtestA verifies on lane-A; @@WebtestB welcomed for
  native-chan-desktop chord verification (`Cmd+Shift+M`
  capture under Tauri).
* Push held for the patch-release commit-grouping cut.
