# fullstack-42: Cmd+K binding revisions — 3=Graph, s=Search, h=Help

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Goal

Revise the Cmd+K mode key map from `fullstack-39` per
@@Alex 2026-05-19 12:35 BST:

* `3` → **Graph** tab (was Search in -39).
* `4` → **New file** (was vacated in the earlier
  rev; @@Alex 12:50 BST assigns it).
* `s` → **Search** overlay (moved off `3`).
* `h` → **Help** — show an OverlayShell-like window
  with survey-like buttons for all Cmd+K bindings,
  grouped (Move / Spawn / Split / Close / Resize /
  Commit), responsive layout for large + small
  screens.

### Drop redundant menu items (menus only — NOT inspectors)

@@Alex 2026-05-19 13:40 BST clarified: keep the
"open in another tab type" buttons inside the
**inspectors** (they're genuinely useful there);
drop them from the **menus** (right-click on tabs,
right-click on tab content, terminal right-click,
etc.).

**Drop from menus**:

* `Graph from here` — wherever it appears in any
  right-click menu (file-tree, doc-tab, terminal
  right-click, etc.).
* `Show Dir` — terminal tab right-click.
* `Show in file browser` — doc-tab right-click.
* `Show Directory` / `Show File` — any right-click
  menu that surfaces these (NOT the inspector
  versions).

**Keep + complete the inspectors**:

* Inspectors on Files / Graph / Search tabs keep
  their `Open` / `Graph from here` / `Show Dir` /
  `Show File` / `Show Directory` affordances. They
  were fixed in `fullstack-29` to spawn the new
  first-class tab types correctly; they stay.
* **Add `Show Dir` to any inspector that's missing
  it** — audit during this pass. Specifically: the
  Search inspector, the Graph inspector when the
  focused node is a directory, etc. Ensure every
  inspector that surfaces a path has an "open this
  in a Files tab" affordance.

Rationale: the inspector is the canonical "drill-
into-this-node" surface; redundant copies in
right-click menus create choice paralysis. Cmd+K
context-aware spawn (`fullstack-43`) covers menu-
gesture use; the inspector buttons cover panel-
gesture use.

### Drop redundant standalone keyboard shortcuts

@@Alex 2026-05-19 12:55 BST: any keyboard shortcut
already covered by a Cmd+K action goes away. Pane
Mode (`Cmd+K`) is the canonical surface; standalone
duplicates are noise.

Drop:

| Standalone shortcut         | Cmd+K equivalent          |
|-----------------------------|---------------------------|
| `Cmd+T` (new terminal)      | `Cmd+K 1`                 |
| `Cmd+Alt+T` (web variant)   | `Cmd+K 1`                 |
| `Cmd+P` (file browser)      | `Cmd+K 2`                 |
| `Cmd+Shift+M` (graph)       | `Cmd+K 3`                 |
| `Cmd+N` (new file)          | `Cmd+K 4`                 |
| `Cmd+Shift+F` (search)      | `Cmd+K s`                 |
| `Cmd+]` / `Cmd+[` (pane nav)| `Cmd+K` + `→` / `←`       |
| `Cmd+Alt+]` / `Cmd+Alt+[`   | `Cmd+K` + `→` / `←`       |

Keep (different action, not a Cmd+K duplicate):

* `Ctrl+D` (close current tab — `fullstack-41`).
  Cmd+K `x` is "close all tabs in pane" + `k` is
  "close pane", neither is the same as "close
  current tab".
* `Cmd+,` (Settings overlay). Not yet in Cmd+K.
* `Cmd+S` (save). Not yet in Cmd+K.
* `Cmd+F` (find in editor). Not yet in Cmd+K.
* `Cmd+\`` if it's the new-terminal alias from
  `fullstack-12` — drop as part of the `Cmd+T`
  removal.

Update `chan serve --help` output + any documented
keymap references (e.g. `ui-exploration.md`) to
match.

## Relevant links

* @@Alex's chat note 2026-05-19 12:35 BST.
* Predecessor: [./fullstack-39.md](../fullstack-a/fullstack-39.md).

## Acceptance criteria

### Updated Cmd+K key map

After `fullstack-40`'s WASD↔arrow inversion + this
task, the full map inside Cmd+K mode is:

| Key                       | Action                                          |
|---------------------------|-------------------------------------------------|
| `↑` / `←` / `↓` / `→`     | Move focus up / left / down / right (per -40)   |
| `W` / `A` / `S` / `D`     | Swap focused tile with neighbour direction (per -40) |
| `1`                       | Open Terminal tab in focused pane               |
| `2`                       | Open File Browser tab in focused pane           |
| `3`                       | Open Graph tab in focused pane                  |
| `4`                       | (vacated)                                       |
| `s`                       | Open Search overlay (commits draft first)       |
| `h`                       | Show Cmd+K cheatsheet (overlay / inline panel)  |
| `/`                       | Split focused pane right                         |
| `\\`                      | Split focused pane down                          |
| `[` / `]` / `-` / `=`     | Resize focused tile (per -16)                   |
| `Shift +` modifiers       | Larger nudge (per -16)                          |
| `0`                       | Equalize siblings at current split level         |
| `x`                       | Close all tabs in focused pane (terminal prompt preserved) |
| `k`                       | Kill (close) the focused pane (prompt preserved) |
| `Enter`                   | Commit draft                                     |
| `Esc`                     | Discard draft                                    |

### Help (`h`) affordance

* `h` renders a cheatsheet of the Cmd+K bindings.
  Layout: a list of key + action rows, grouped (Move,
  Spawn, Split, Close, Resize, Commit/Discard).
* The cheatsheet does NOT commit the draft. It's a
  read-only overlay; Esc on the cheatsheet returns
  to Cmd+K mode (still inside the transaction).
* Pressing `h` again hides the cheatsheet.
* Style: small, dense, TUI-density. Doesn't need to
  be fancy; just legible.

### Tests + spec update

* Update the keymap tests from `fullstack-39` to
  reflect the new bindings.
* Update `ui-exploration.md` Phase 2 keymap section
  to reflect the new bindings + the help key.
* Add a test that `h` toggles the cheatsheet visibility
  without committing the draft.

## Out of scope

* Configurable bindings.
* What `4` does (vacant for future use; no error on
  press, just no-op).

## How to start

1. `web/src/App.svelte:handlePaneModeKey` — adjust the
   dispatch table per the new map.
2. The Help cheatsheet is a new small component (e.g.
   `web/src/components/PaneModeHelp.svelte`) gated on a
   `paneModeHelpVisible` flag.
3. Update tests in `web/src/state/tabs.test.ts`.
4. Update spec in `docs/journals/phase-7/ui-exploration.md`.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-a-architect.md`.
