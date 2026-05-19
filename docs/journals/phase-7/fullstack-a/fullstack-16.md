# fullstack-16: Phase 2 — Cmd+K transactional pane mode + keybinds

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-18

## Goal

Layer the Phase 2 transactional pane mode on top of the
`fullstack-15` substrate. The mode is **transactional**:
Cmd+K snapshots the live tree, all keystrokes inside the
mode operate on a draft, Enter commits, Esc discards. No
conflation of "exit" with "undo".

Desktop-first per @@Alex's call. The central shortcut
config handles cross-platform.

## Relevant links

* @@Alex's design:
  [../ui-exploration.md](../ui-exploration.md) — Phase 2
  "Keyboard: pane mode (Cmd+K)" section is the
  authoritative spec.
* Depends on `fullstack-15` substrate (binary tree +
  detach + persistence).

## Acceptance criteria

### Entering / leaving the mode

* `Cmd+K` enters pane mode. Snapshots tree shape +
  ratios + focus pointer. Draft state drives rendering.
* Visual chrome flips:
  * Thin tint on unfocused panes.
  * Brighter border on the focused pane.
  * Small pane-mode pill in the status bar.
  * Enough to make the mode unmistakable without burying
    the layout.
* `Enter` commits the draft (replaces live tree with
  draft).
* `Esc` discards the draft (no change).
* While in pane mode, all keystrokes route through pane-
  mode handlers; xterm and editor keystrokes don't fire.

### Keybinds (no Cmd prefix after entering the mode)

| Key                | Action                                          |
|--------------------|-------------------------------------------------|
| `W` / `A` / `S` / `D` | Move focus up / left / down / right        |
| `↑` / `←` / `↓` / `→` | Swap focused tile with neighbour direction  |
| `[`                | Shrink focused tile horizontally                |
| `]`                | Grow focused tile horizontally                  |
| `-`                | Shrink focused tile vertically                  |
| `=`                | Grow focused tile vertically                    |
| `Shift + [` etc.   | Larger nudge (10% vs 2%)                        |
| `0`                | Equalise siblings at current split level        |
| `Enter`            | Commit transaction and exit                     |
| `Esc`              | Discard transaction and exit                    |

### Semantics

* Focus moves and swaps are no-ops when there is no
  neighbour in that direction.
* Resize operates on the focused tile inside its parent
  split. If parent split is on the wrong axis (e.g. `]`
  while parent is vertical), walk up the tree to the
  nearest ancestor on the right axis and resize there.
  Matches the Hyprland "make me wider always works"
  feel.
* Resize clamps to a sensible minimum.

### State

* Draft tree is a snapshot of (shape + ratios + focus
  pointer) taken on Cmd+K. Small + cheap.
* Live tree only updates on commit. Esc throws away the
  draft, no live mutation occurred.

## Out of scope

* Pane mode entry from menu (keybind only).
* Animations beyond a sensible default.
* Multi-step undo within pane mode (it's transactional
  by design — one Cmd+K session = one transaction).

## How to start

1. State seam in
   `web/src/state/tabs.svelte.ts` (or a sibling state
   module). Add a `paneDraftLayout` slot + boolean
   `paneModeActive`.
2. Capture Cmd+K early in the shortcut handler; clone
   the layout tree into draft.
3. Render path: when `paneModeActive`, the layout
   renderer reads from `paneDraftLayout`, not live.
4. Keybind handler: routes WASD / arrows / [ ] - = / 0
   into draft mutations. Enter swaps draft into live;
   Esc nulls draft.
5. Visual chrome: a CSS class on the root that gates the
   tint / brighter-border / pill styles.

## Hand-off

Standard. Pre-push gate green. Pairs with @@WebtestA on
the keybind matrix walkthrough. Ping via
`alex/event-fullstack-architect.md`.

## 2026-05-18 21:05 BST — implementation

Implemented transactional pane mode locally:

* `Cmd+K` snapshots the live layout into a draft and flips pane mode on.
* While pane mode is active, the workspace renders the draft layout tree
  and panes show lightweight tab previews instead of mounting duplicate
  editor / terminal / graph bodies.
* `Enter` commits the draft back to the live layout and schedules session
  persistence; `Esc` discards the draft.
* `W/A/S/D` move focus through neighbouring panes, arrow keys swap the
  focused pane contents with the directional neighbour, `[ ] - =` resize
  the nearest split on the requested axis, and `0` equalises the current
  parent split.
* Added pane-mode visual chrome: dimmed unfocused panes, brighter focused
  pane border, and a bottom status-bar pill.
* Added state tests for discard and commit paths.

Verification so far:

* `npm run test -- tabs`
* `npm run check`
* `npm run build`

Next: full gate, then commit if green.

## 2026-05-18 21:06 BST — commit readiness
Ready to land.

Changed files:
* web/src/state/tabs.svelte.ts
* web/src/components/Workspace.svelte
* web/src/components/Pane.svelte
* web/src/App.svelte
* web/src/components/AppStatusBar.svelte
* web/src/state/tabs.test.ts
* docs/journals/phase-7/fullstack-a/fullstack-16.md

Verification:
* npm run test -- tabs
* npm run check
* npm run build
* scripts/pre-push

Known risk: neighbour selection is tree-topology based; WebtestA should walk nested split keybind matrix.

Proposed commit: Add transactional pane mode (fullstack-16).
