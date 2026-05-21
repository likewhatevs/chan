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
