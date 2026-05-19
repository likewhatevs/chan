# fullstack-42: Cmd+K binding revisions — 3=Graph, s=Search, h=Help

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Goal

Revise the Cmd+K mode key map from `fullstack-39` per
@@Alex 2026-05-19 12:35 BST:

* `3` → **Graph** tab (was Search in -39).
* `4` → vacated (was Graph in -39).
* `s` → **Search** overlay (moved off `3`).
* `h` → **Help** — show an inline cheatsheet of the
  Cmd+K bindings.

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
