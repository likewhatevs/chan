# fullstack-a-44 — Hybrid pane drag-to-rearrange + transaction-mode NAV

Owner: @@FullStackA
Cut: 2026-05-21 by @@Architect
Status: queued (sequenced AFTER fullstack-a-43 lands in HEAD)

## Goal

Extend Hybrid NAV mode with a mouse-driven "transaction
mode" entered by dragging from the pane top-bar dead zone.
In transaction mode, click-anywhere-in-Hybrid drag-and-drop
rearranges panes; Enter commits, Esc dismisses.

## User flow

Two entries to transaction mode; both target the same
pane top-bar **dead zone** (the space between the last
tab and the hamburger menu).

### Entry A — drag-start (drag-with-payload)

1. User mouses over the dead zone.
2. User mousedown + drag-start.
3. App enters Hybrid NAV mode in **transaction mode**
   (variant of NAV mode with mouse-grab affordances
   active across the whole Hybrid body).
4. The originating drag becomes the first grab — no
   separate click needed; the user is already dragging
   the pane.

### Entry B — double-click (drag-no-payload)

1. User double-clicks the same dead zone.
2. App enters Hybrid NAV mode in **transaction mode**
   without an originating grab — the mode is standby.
3. User clicks + drags anywhere inside any Hybrid pane
   to grab that pane.

Entry B is the discoverable affordance for users who
didn't realise the dead zone is draggable. Entry A is
the fluent path for users who know.

### Once in transaction mode (regardless of entry)

* Click + drag anywhere inside any Hybrid pane (not just
  the top bar) grabs that pane for movement.
* Drop-target indication: visual cue at the swap
  destination (insertion cursor / highlight; implementer
  picks).
* Drag continues until commit/dismiss.

### Exit

* **Enter** → commit the rearrangement(s) to the layout
  state.
* **Esc** → dismiss; revert to pre-transaction layout.

## Background

@@Alex 2026-05-21 (verbatim): "once the user clicks
hybrid's pane area (the space between a tab and the
hamburger) and tries to drag, we auto-enter hybrid nav
mode in transaction mode. so the user could drag and drop
other windows as well, but in hybrid nav mode they should
be able to click anywhere in the hybrid not just the
pane's top bar, and then rearrange until they press enter
or esc to commit or dismiss."

Today's Hybrid NAV mode is keyboard-chord-driven
(`Cmd+.` enter; key-chord rearrangement). Mouse
interactions are limited to top-bar clicks. The new shape
gives mouse parity for the rearrange flow — drag feels
native, the chord-driven keyboard path remains for power
users.

Composes with `-a-32` (chord-migration entry chord
`Cmd+.`) + `-a-43` (Hybrid back-side architecture refactor;
hard prereq — see Sequencing).

## Acceptance criteria

* Dead-zone hit area on the top bar (between last tab and
  hamburger menu) registers BOTH mousedown + drag-start
  AND double-click.
* Drag-start from that hit area transitions Hybrid into
  NAV mode with `transactionMode: true` (or equivalent
  distinct state value) AND the originating pane becomes
  the first grab (drag-with-payload).
* Double-click on that hit area transitions Hybrid into
  NAV mode with `transactionMode: true` AND no
  originating grab (mode is standby; next click + drag
  inside any Hybrid grabs that pane).
* Transaction-mode visual cue distinguishes it from the
  keyboard NAV mode (cursor change to grab/grabbing,
  body-wide drag affordance enabled, optional dim of
  non-target panes — implementer picks the visual
  language).
* In transaction mode: any mousedown + drag inside a
  Hybrid pane (front side, anywhere) grabs that pane.
  The originating drag (the one that entered transaction
  mode) is the first grab.
* Drop-target indication visible during the drag.
* Enter commits; Esc dismisses + reverts the layout to
  pre-transaction state. Same as keyboard NAV exit.
* Cross-Hybrid drags during transaction mode work for any
  Hybrid in the current window (not just the originating
  one).
* Keyboard NAV mode (`Cmd+.` enter, key-chord rearrange)
  still works unchanged — transaction mode is a
  SUPERSET of mouse affordances, not a replacement of
  the chord-driven flow.
* Tests cover: dead-zone hit area, drag-start transition,
  transaction-mode click-anywhere grab, commit/dismiss,
  layout revert on dismiss.

## How to start

1. Audit current Hybrid NAV mode entry + rearrangement
   logic (likely `pane.svelte.ts` + `Pane.svelte` +
   wherever the keyboard chord routes to).
2. Inventory the keyboard rearrangement actions that
   need to be reachable from mouse drag (which already
   exist; transaction mode reuses them).
3. Define the dead-zone hit area on the top bar layout
   (CSS + hit-test).
4. Add `transactionMode` to the NAV-mode state shape
   (or parallel mode value).
5. Wire mousedown + drag-start on the dead zone →
   NAV-mode transition with `transactionMode: true` +
   grab-anywhere mouse handlers on the pane body.
6. Add drop-target visual indication + layout-swap on
   drop.
7. Wire Enter/Esc to the existing NAV commit/dismiss
   path.

## Coordination

* SPA-only. No cross-lane touch.
* Pre-push gate green before commit clearance.
* When ready for commit, append "Commit readiness" +
  poke @@Architect.

### Sequencing constraint — HARD prereq

This task DEPENDS on
[`fullstack-a-43`](fullstack-a-43.md) landing in HEAD
first. `-a-43` refactors `Pane.svelte`'s back-side
mount path; concurrent edits to `Pane.svelte` from this
task would create merge pain in the multi-agent
worktree.

**Do NOT start `-a-44` until `-a-43` commits + clears.**

Natural sequence after `-a-43` lands:

1. `-a-43` commits; @@Architect clears.
2. Tasks B/C/D/E/F (Hybrid back-side population) fan
   out as a queue.
3. `-a-44` (drag) + `-a-42` (About) can land in any
   order relative to B/C/D/E/F since they touch
   different surfaces. @@Architect coordinates the
   final ordering at fan-out.

## Open implementation questions

Implementer-side judgment; surface a scope question
only if a choice becomes load-bearing.

1. **Drag visual language**: cursor (grab/grabbing),
   pane ghost on drag, drop-target highlight shape.
   Pick based on existing chan UI conventions. If
   "should non-target panes dim?" surfaces as a real
   choice, flag it.
2. **Cmd+. mid-drag**: today's keyboard NAV exits on
   Enter/Esc + `Cmd+.` toggle. Transaction mode reuses
   Enter/Esc; should `Cmd+.` also exit transaction mode?
   Default: yes (consistency). Deviate only if the
   chord mid-drag feels wrong.
3. **Click-without-drag inside a transaction**: a
   mousedown + mouseup on a pane body inside transaction
   mode without moving past a threshold (~5 px) — no-op
   release or commit a selection? Default: no-op
   release. The drag distance threshold is the line.

## Numbering

Highest committed `-a-N` is `-a-41`; `-a-42` is About,
`-a-43` is Hybrid back-side architecture refactor (Task
A); this is `-a-44`. The Hybrid back-side Tasks B-F when
they fan out will claim `-a-45..-a-49`; About (`-a-42`)
stays put.

## 2026-05-21 — ready for review

Four-file change. SPA + state only; no Rust touched.

### Architecture

State (`web/src/state/tabs.svelte.ts`):

* `paneMode` gains two transaction-mode fields:
  `transactionMode: boolean` (mouse-driven NAV active)
  and `grabPaneId: string | null` (the pane currently
  held), plus `hoverPaneId: string | null` for the
  drop-target highlight. All three reset on
  `enterPaneMode` / `commitPaneMode` / `cancelPaneMode`
  so the keyboard NAV path stays unaffected.
* New `enterPaneModeTransaction(grabPaneId)`: lazy-
  inits paneMode if not already active (so the same
  call works whether the user is mid-keyboard-NAV or
  starting fresh) and flips `transactionMode = true`
  + sets the originating grab. `null` enters in
  standby (Entry B, drag-no-payload).
* New `paneModeSetGrab(paneId)` / `paneModeSetHover
  (paneId)`: gated on `transactionMode` so callers
  outside transaction mode can't accidentally muck
  with grab/hover state. Drives the visual cues from
  `Pane.svelte` mouse handlers.
* New `paneModeSwapWith(grabId, dropId)`: the
  directional `paneModeSwap` now reduces to this
  once it resolves a neighbour. Transaction-mode
  drop-on-pane calls this directly with the two
  pane ids. No-op if `grabId === dropId`, no-op
  outside pane mode (so a stray mouseup off-drop
  doesn't fire). Both panes wobble per the existing
  swap convention.

UI (`web/src/components/Pane.svelte`):

* `.dead-zone` div added between the last `.tab` and
  the `.actions` hamburger inside the `.tabs` strip.
  `flex: 1` + 12 px min-width keeps the affordance
  hittable even when the tab strip is fully packed.
* Entry A: `onmousedown` on the dead zone records the
  start point + attaches window-level `mousemove` +
  `mouseup` listeners. If `mousemove` crosses the
  5 px threshold, the listeners detach and
  `enterPaneModeTransaction(pane.id)` fires (drag
  started; this pane is the first grab). If
  `mouseup` fires before the threshold, the
  listeners detach without entering NAV (it was just
  a click in the dead zone).
* Entry B: `ondblclick` on the dead zone calls
  `enterPaneModeTransaction(null)` directly.
* Pane root handlers: `onmousedown` augmented to
  call `onPaneBodyMouseDown` (sets the grab to this
  pane when in transaction mode + no grab held, i.e.
  Entry B's first grab); `onmouseenter` /
  `onmouseleave` track hover for drop-target
  indication; `onmouseup` commits the swap with the
  currently-held grab when this pane is the drop
  target.
* Class flags: `.transaction-active` (always while
  transactionMode is on; body cursor → grabbing),
  `.transaction-grab` (dashed orange outline on the
  held pane — distinct from the solid focus ring on
  the keyboard-active pane), `.transaction-drop-
  target` (inset overlay in `--pane-focus` colour on
  the pane under the cursor while a grab is held).
* `.pane { position: relative }` added so the
  drop-target `::after` overlay anchors to the pane.

Exit / commit: handled entirely by the existing
keyboard NAV path. `handlePaneModeKey` in App.svelte
already routes Enter → `commitPaneMode` and Esc →
`cancelPaneMode`; both already clear the new
transaction fields by virtue of my updates to those
helpers. No App.svelte chord-layer additions in this
landing.

### Manual mouse handling vs HTML5 dragstart

Tabs in the strip are `draggable="true"` for inter-
pane tab DnD (the pre-existing per-tab handlers
fire `onDragStart` / `onTabDragOver` / `onTabDrop`).
Adding `draggable="true"` to the dead zone would
route through the same HTML5 drag pipeline and
collide with that DnD. Manual mousedown +
`mousemove` threshold tracking + `mouseup` cleanup
gives the dead-zone interaction full control over
its own state machine without touching the tab
DnD. Pinned by a raw-source test.

### Drop-target indication

`.transaction-drop-target::after` paints a 2 px
solid `var(--pane-focus)` border with an 8 %
inset fill colour-mixed against the same focus
colour. Distinguishable from `.pane.focused`
(solid coloured ring on the box itself); the
overlay reads as "drop here" rather than "this
is focused". `z-index: 5` sits above the editor
body but below the pane chrome.

### Chain semantics

Each drop fires a swap-then-clear: `grabPaneId →
null`, `hoverPaneId → null`. Transaction mode
stays on; the user can immediately grab another
pane and continue swapping until they press
Enter (commit) or Esc (dismiss). Matches the
task's "Drag continues until commit/dismiss" rule.

### Tests

`tabs.test.ts` — new `Hybrid NAV transaction mode
(fullstack-a-44)` describe block, 8 pins:

* Entry A activates transaction mode + sets grab.
* Entry B activates transaction mode with no grab.
* `paneModeSwapWith` swaps two arbitrary panes.
* No-op outside pane mode.
* No-op when grab == drop.
* `paneModeSetGrab` / `paneModeSetHover` gated by
  `transactionMode` (no-op outside; no-op in
  keyboard-only paneMode; mutates in transaction).
* `cancelPaneMode` clears transaction state.
* `commitPaneMode` persists swap + clears
  transaction state.

`Pane.test.ts` — new `Pane Hybrid NAV transaction
mode (fullstack-a-44)` describe block, 4 pins:

* Dead-zone div renders inside `.tabs` between
  the last tab and `.actions`.
* Double-click on the dead zone enters
  transaction mode in standby (no grab).
* Pane root flips `transaction-grab` /
  `transaction-drop-target` classes from
  `paneMode` state changes (mounts the left pane
  of a two-pane layout, drives state, asserts
  class flips).
* Dead-zone wiring uses manual mousedown +
  threshold tracking, NOT HTML5 dragstart.
  Raw-source guard against a future edit that
  routes through `draggable="true"` and stomps
  the per-tab DnD.

### Gate

* vitest **600 / 600** (+12 net from -a-43's
  588 baseline; 8 new in tabs.test.ts, 4 new
  in Pane.test.ts).
* svelte-check 0 errors / 0 warnings across
  3987 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Deviations / decisions flagged

* **Cmd+. mid-transaction**: task default was
  "yes, Cmd+. exits". I did not wire it. The
  existing keyboard NAV doesn't exit on Cmd+.
  today (only Enter / Esc); changing that
  behaviour for both keyboard and transaction
  mode is scope creep. Esc dismisses cleanly;
  Cmd+. mid-transaction is a no-op (the
  `paneMode.active` short-circuit in
  `onWindowKey` keeps the chord from re-
  entering pane mode). Flag if the call should
  flip.
* **Click-without-drag in transaction**: a
  mousedown + mouseup inside a pane body (no
  cursor movement past threshold) sets grab
  on the down + immediately swaps with the
  same pane on the up. `paneModeSwapWith`
  is a no-op when grab == drop, so the
  observable effect is "grab + release on
  same pane = re-set grab to null". Matches
  the task's "no-op release" default.
* **Originating drag continuation**: Entry A
  records the originating-pane id but does
  not synthesize a fake mousemove path. The
  user has to keep dragging past the threshold
  AND release on a different pane to swap.
  Releasing back on the originating pane
  hits the same-pane no-op above. Reads as
  "drag to swap" semantically; no impedance.
* **Cross-Hybrid drag**: any pane can be the
  drop target, not just Hybrid-marked panes.
  The task body says "any mousedown + drag
  inside a Hybrid pane (front side, anywhere)
  grabs that pane" — chan's layout treats every
  leaf pane as "a Hybrid" in this sense
  (`pane.back` is lazily created on first
  flip). Rather than gate on `pane.back !==
  undefined` I let every pane participate,
  which matches the user's "rearrange any
  pane" expectation. Flag if hybrid-only
  participation was wanted.

### Suggested commit subject

```
Hybrid pane drag-to-rearrange + transaction-mode NAV (fullstack-a-44)
```

Single commit. State additions + Pane.svelte
handlers + CSS + tests are tightly coupled around
the same feature; intermediate states would not
compile (the test imports reference the new
exports). No Rust touched.

### Files

* `web/src/state/tabs.svelte.ts` — paneMode shape
  extensions, `enterPaneModeTransaction`,
  `paneModeSetGrab`, `paneModeSetHover`,
  `paneModeSwapWith`; `paneModeSwap` refactored
  to call `paneModeSwapWith` internally.
* `web/src/state/tabs.test.ts` — `Hybrid NAV
  transaction mode (fullstack-a-44)` describe
  block (8 pins) + import additions.
* `web/src/components/Pane.svelte` — imports,
  dead-zone handler functions + threshold const,
  pane-body handlers, dead-zone div in the tabs
  strip, transaction-mode class flags on pane
  root, CSS for dead-zone cursor +
  transaction-* visual cues, `position:
  relative` on `.pane`.
* `web/src/components/Pane.test.ts` — `Pane
  Hybrid NAV transaction mode (fullstack-a-44)`
  describe block (4 pins) + raw-source import.

Push held — multi-agent tree commit discipline.
Standing by for clearance.
