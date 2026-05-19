# fullstack-39: Cmd+K spawn/split/kill keybinds + invisible pane divider

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Goal

Extend the Phase 2 Cmd+K transactional pane mode (from
`fullstack-16`) with content-spawn + split + kill
keybinds. Plus a pane-chrome refinement: the visible
divider between panes goes away, drag-to-resize still
works invisibly on the same gutter.

## Relevant links

* @@Alex's chat note 2026-05-19 12:00 BST.
* Predecessor (Cmd+K substrate):
  [./fullstack-16.md](../fullstack-a/fullstack-16.md).
* Predecessor (pane chrome):
  [../fullstack-b/fullstack-34.md](../fullstack-b/fullstack-34.md).

## Acceptance criteria

### Cmd+K mode keybinds (additions)

Inside Cmd+K pane mode, add these single-key
bindings on top of the existing WASD / arrows /
`[ ]` / `- =` / `0` / Enter / Esc set:

| Key  | Action                                         |
|------|------------------------------------------------|
| `1`  | Open a new terminal tab in the focused pane.   |
| `2`  | Open a File Browser tab in the focused pane.   |
| `3`  | Open the Search overlay.                       |
| `4`  | Open a Graph tab in the focused pane.          |
| `/`  | Split the focused pane to the **right**.       |
| `\\` | Split the focused pane to the **bottom** (down).|
| `x`  | Close all tabs in the focused pane (welcome state remains). |
| `k`  | Kill (close) the focused pane.                 |

Semantics:

* Spawning a tab (`1`/`2`/`3`/`4`) targets the
  draft-tree's currently-focused pane. The action
  commits within the transaction — Enter still seals
  the layout draft + the new tab; Esc still rolls
  back including the spawned tab.
* `3` (Search) opens the Search OverlayShell since
  Search isn't a tab type (per @@Alex's Phase 1
  decision — Search stays as overlay).
* `/` and `\\` reuse the existing `splitPane`
  primitive (right + down only, per `fullstack-21`).
* `x` (close all tabs) reuses the same affordance
  from `fullstack-34`'s hamburger Close-all-tabs
  action. **Honor the existing confirmation prompt
  for terminal tabs** — don't bypass on a keystroke.
* `k` (kill pane) reuses the hamburger Close-pane
  action. **Same terminal confirmation prompt
  applies** — terminal tabs prompt before dying.

The keys fire inside Cmd+K mode only. Outside the mode
they pass through to the normal editor / terminal /
chrome handlers.

### Invisible pane divider

* Today the divider between panes renders a visible
  bar. Make it visually invisible (no border / no
  fill / no shadow) — the panes' rounded chrome from
  `fullstack-34` already gives visual separation via
  margin + shadow.
* The drag-to-resize hit area stays the same size as
  today (don't shrink to <8px or anything brittle).
  Cursor on hover still changes to the resize cursor
  so the user can find it.
* Both axes (horizontal + vertical split dividers)
  get the same treatment.

## Out of scope

* Cmd+K shortcuts outside the existing transactional
  mode (no global hotkeys here).
* New tab types beyond what `fullstack-14` introduced
  (Terminal, File Browser, Graph; doc tabs spawn via
  Cmd+P or file-tree clicks).
* Divider-color customization (it's just invisible
  now).

## How to start

1. `web/src/state/shortcuts.ts` (or the Cmd+K mode
   handler in `tabs.svelte.ts`) — extend the
   single-key dispatch table with the 8 new bindings.
2. Spawn paths: reuse `openTerminal()`,
   `openFilesTab()`, `openGraphTab()`, search-overlay
   open helper. Each takes the focused-pane id in
   the draft tree.
3. Close confirmations: terminal-close confirmation
   already lives in the Close pane / Close all tabs
   flow; reuse, don't reimplement.
4. Divider CSS: locate the `ResizeHandle` (likely in
   `Pane.svelte` or a sibling). Set its visible
   styles to transparent while preserving width +
   hit area + cursor.

## Hand-off

Standard. Pre-push gate green. Coordinate with
@@WebtestA only if a walkthrough is wanted; otherwise
this is bounded enough that the gate is sufficient.
Ping via `alex/event-fullstack-a-architect.md`.
