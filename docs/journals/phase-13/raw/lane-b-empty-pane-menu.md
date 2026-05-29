# @@LaneB task - retire the empty-pane right-click menu + close the hamburger gap

Cut by @@LaneA on @@Alex's direction (2026-05-28). This lives entirely
in Lane B's files (`Pane.svelte`, `EmptyPaneWelcome.svelte`,
`Pane.test.ts`), so it routes to Lane B. Queued append-only as a
DISCRETE new task per `feedback_inflight_task_amendments` - do NOT fold
it into the started `lane-b-round-1-closing-2.md` file.

## Why

@@Alex: "today we have a slightly different menu in the pane's
hamburger and in the empty pane's right-click; I'd like to remove the
empty pane's right click menu altogether, and leave just the pane's
hamburger, which already covers all of the options."

Investigation (@@LaneA) found the hamburger does NOT actually cover all
options today, so the removal needs a gap-fix in the same change:

- Pane hamburger (⋮) renders only `spawnActions` (New Draft, Terminal,
  File Browser, Rich Prompt, Graph) + nav/close/focus-color. It has NO
  Dashboard and NO Search.
- Dashboard + Search live only in the empty-pane right-click menu
  (`emptyPaneExtraActions`). `EmptyPaneWelcome`'s front grid has the 5
  spawn tiles + Dashboard (no Search) and renders ONLY in single-pane
  layouts. In a multi-pane layout an empty pane shows just the chan
  mark, so the right-click menu is the only path to Dashboard + Search
  there.
- The empty-pane back-of-card is a "Hybrid" title + hint (no buttons).

@@Alex's call: add Search + Dashboard to the hamburger first, THEN
remove the empty-pane right-click menu, so nothing is lost.

## Goal

Make the pane hamburger (⋮) the single menu for panes. Remove the
empty-pane RIGHT-CLICK context menu. Other right-click menus
(non-empty pane Reload/Inspector, per-tab, content surfaces) stay
untouched.

## Part 1 - close the hamburger gap (do this first)

Render `emptyPaneExtraActions` (Dashboard, Search - already defined at
`Pane.svelte:216-233`) inside the pane hamburger, right after the
`spawnActions` loop and before the first `<li class="sep">` /
"Enter Hybrid Nav" section. The hamburger then carries the full spawn
set (New Draft, Terminal, File Browser, Rich Prompt, Graph, Dashboard,
Search) for ALL panes (empty + non-empty), matching the old right-click
set.

- Hamburger render block: `Pane.svelte:1169-1242` (spawnActions loop at
  1187-1196).
- Reuse the existing `emptyPaneExtraActions` array + `dispatchCommand` +
  `closePaneHamburgerMenu` + `chordLabel` already used in that block.

## Part 2 - remove the empty-pane right-click menu

Delete the empty-pane contextmenu path; leave every OTHER right-click
menu intact (these are separate code paths, confirmed by survey):

- Non-empty pane right-click -> `openPaneContextAt` -> Reload / Open
  Inspector (`Pane.svelte:1243-1270`). KEEP.
- Per-tab right-click -> `openTabMenu` (`Pane.svelte:~1036`). KEEP.
- Surface right-clicks (Terminal/Editor/Graph/FB/Dashboard/Search).
  KEEP.

Remove:
- `onEmptyPaneContextMenu` + `openEmptyPaneMenuAt`
  (`Pane.svelte:253-262`).
- `oncontextmenu={onEmptyPaneContextMenu}` on the `.placeholder`
  (`Pane.svelte:1365`) and on `<EmptyPaneWelcome>` (`Pane.svelte:1377`).
- The triggerless empty-pane `<HamburgerMenu bind:this={emptyPaneMenu}>`
  block (`Pane.svelte:1386-1415`) + the `emptyPaneMenu` /
  `emptyPaneMenuOpen` state (`Pane.svelte:250-251`).
- The `pane.tabs.length === 0` -> `openEmptyPaneMenuAt(e)` branch in the
  tab-strip contextmenu handler (`Pane.svelte:1006-1010`): empty
  tab-strip right-click should no-op (fall through), non-empty keeps
  `openPaneContextAt`.
- `EmptyPaneWelcome.svelte`: drop the `oncontextmenu` prop
  (`:30-36`, `:115`). Its spawn grid stays as-is.
- `emptyPaneActions` alias (`Pane.svelte:215`) just aliases
  `spawnActions`; drop if no longer referenced after the above.

Pre-release per `feedback_pre_release_no_backcompat`: delete outright,
no shims.

## Tests

- `Pane.test.ts`: replace "empty pane right-click shows the welcome
  menu" (~223-264) with an assertion that right-clicking an empty pane
  opens NO menu. Keep "empty pane left-click leaves the welcome menu
  closed" and "loaded pane right-click keeps reload and inspector
  menu". Add/extend a hamburger test asserting the spawn rows now
  include Dashboard + Search.
- `EmptyPaneCarousel.test.ts` "forwards right-click to the parent
  contextmenu handler" (~79-99): the carousel lives in DashboardTab,
  not the empty-pane body; reassess whether the forwarder prop is still
  wired anywhere and update/remove the test accordingly.

## Acceptance

- Right-click on an empty pane (single-pane AND multi-pane) opens
  nothing.
- Pane hamburger ⋮ lists: New Draft, Terminal, File Browser, Rich
  Prompt, Graph, Dashboard, Search, then Enter Hybrid Nav / splits /
  next / prev / close all / kill pane / focus colour.
- Non-empty pane right-click still shows Reload / Open Inspector; tab
  and surface right-clicks unchanged.
- `EmptyPaneWelcome` single-pane grid still renders (5 tiles +
  Dashboard).

## Gate (mandatory)

cargo fmt --check / clippy --all-targets -- -D warnings / test /
build --no-default-features / (web) npm run check / npm run build /
npm test. Browser-smoke the right-click no-op + the hamburger Dashboard
+ Search rows per `feedback_svelte_static_gate_misses_runtime`.

## Sequencing / cross-lane

- Lane B owns all touched files; no Lane A overlap.
- Per `feedback_no_midtask_interrupts`: pick up at a coherent point
  AFTER current in-flight `Pane.svelte` closing-2 work lands - do not
  interleave half-states.
- @@Alex to confirm whether this is a closing-2 tail item or a round-2
  carryover (`roadmap-round-2.md`).
