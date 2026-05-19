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

## 2026-05-19 12:08 BST — @@FullStackA specialist review

### Cmd+K mode keybinds

* `web/src/state/tabs.svelte.ts` — new draft-aware
  helpers:
  * `paneModeSplit("row" | "column")` reuses a
    refactored `insertSiblingPaneIn(state, ...)` so the
    split lands inside the draft. Honors right/down only
    via the call sites in App.svelte.
  * `paneModeOpenTerminal()` spawns a `TerminalTab` on
    the draft's focused pane. The terminal WebSocket
    only connects when the tab mounts, so an Esc
    rollback leaves no backend state behind.
  * `paneModeOpenBrowser()` and `paneModeOpenGraph()`
    mirror `openBrowserInActivePane` /
    `openGraphInPane`'s dedup-by-existing semantics
    against the draft pane, so pressing `2` or `4`
    repeatedly doesn't pile duplicates.
* `web/src/App.svelte:handlePaneModeKey` — added 8 cases
  on top of the existing WASD/arrows/`[ ]`/`- =`/`0`/
  Enter/Esc set:
  * `1` / `2` / `4` → `paneModeOpenTerminal` /
    `paneModeOpenBrowser` / `paneModeOpenGraph`. Stay
    inside the transaction so Esc rolls the new tab
    back along with any layout edits.
  * `3` (Search) commits the draft + opens the existing
    Search OverlayShell. Search isn't a tab type
    (per @@Alex's Phase 1 call), and the overlay needs
    normal keyboard context — staying inside the
    pane-mode keydown handler would eat its input.
  * `/` and `\\` → `paneModeSplit("row" | "column")`.
    Right + down only, matching `fullstack-21`'s
    hamburger menu constraint.
  * `x` / `X` and `k` / `K` commit the draft + call
    `closeTabsInPane(layout.activePaneId)` /
    `closePane(layout.activePaneId)`. The existing
    terminal-close confirmation modal lives in those
    helpers — keystrokes don't bypass it.

### Invisible pane divider

* `web/src/components/Workspace.svelte` — the
  `.divider` element keeps its dimensions (4px, 6px
  hover), `flex-shrink: 0`, and `cursor: col-resize /
  row-resize`, but now paints with
  `background: transparent` instead of `var(--border)`.
  Drag-to-resize still works invisibly on the same
  gutter; the pane chrome's margin + shadow (from
  `fullstack-34`) carries the visual separation
  between halves.

### Tests

* `web/src/state/tabs.test.ts` — four new tests under
  the existing pane-mode describe:
  * "pane mode spawn keys add tabs to the draft and Esc
    rolls them back" — asserts the draft sees the new
    tabs while the real layout is untouched, and Esc
    leaves the original tab count.
  * "pane mode commits the draft's spawned tabs into
    the real layout" — Enter commits, real layout
    gains the new tabs in order, focus lands on the
    most-recently-spawned tab.
  * "pane mode browser/graph spawn dedupes against
    existing tabs" — pressing the spawn keys with
    matching tabs already in the focused pane is a
    no-op on tab count.
  * "pane mode split inserts a new pane to the right/
    down in the draft" — asserts the draft gains a
    SplitNode at the root with the new pane on side
    "b" (placement: "after"), draft focus follows, and
    Enter commits.

### Gate

* `npm run test -- tabs` — 50 passed (was 46; +4 new).
* `npm run test` — 32 files / 289 tests, all pass.
* `npm run check` — 0 errors / 0 warnings.
* `npm run build` — clean.
* `bash -lc 'ulimit -n 4096; scripts/pre-push'` — green
  (fmt + clippy + tests + no-default-features build).

### Proposed commit message

> Cmd+K spawn/split/kill keybinds + invisible pane divider (fullstack-39)
>
> Extend the Phase 2 Cmd+K transactional pane mode with eight
> new single-key bindings: 1/2/4 spawn terminal/files/graph in
> the draft (Enter commits, Esc rolls back), 3 commits + opens
> the Search overlay, / and \\ split right/down inside the draft,
> x and k commit + delegate to the existing close-all-tabs /
> close-pane affordances so the terminal-close confirmation
> still fires. Drops the visible divider bar between panes
> (the pane-chrome margin + shadow carries the separation);
> hit area, cursor, and drag-resize behaviour are unchanged.

Ready for commit + push under standing topic-level
clearance.
