# fullstack-22: BCAST as window-wide group + fix stuck self-toggle

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-19

## Goal

Reshape BCAST around the correct mental model: there's a
**single BCAST group per Hybrid window**, every tab sees
it, and each tab's own "Broadcast input on/off" button is
the canonical add/remove for that tab. Fix the live bug
where removing a tab from the group leaves its own toggle
stuck off with no way back in.

## Relevant links

* [../request.md](../request.md) — B18 cluster, see the
  2026-05-19 02:00 BST clarification sub-bullet under the
  "tab's `[BCAST]` -> broadcast icon" bullet.
* Predecessor: [./fullstack-8.md](./fullstack-8.md)
  (BCAST/mute cluster). This task corrects the
  membership semantics; the icon swap and the
  membership-leak fixes from `-8` stay.

## Acceptance criteria

### Single window-wide group

* One BCAST group per Hybrid. All tabs in the same
  window see the same group state.
* No "self" entry in the membership checklist menu —
  it's implicit (the current tab's own toggle handles
  it).

### Per-tab toggle is the canonical add/remove

* Each terminal tab's "Broadcast input on" button:
  * When OFF and clicked → add this tab to the group
    (transitions to ON).
  * When ON and clicked → remove this tab from the
    group (transitions to OFF).
  * Always live and clickable. Never disabled because
    of membership state.
* The chip strip + the menu update to reflect the new
  membership when any tab's own toggle fires.

### Membership menu lists OTHER tabs

* The membership menu shows the other tabs in the
  Hybrid (not self), each with a checkbox for joining /
  leaving the group.
* Toggle in the menu = same effect as toggling that
  other tab's own button (isolated to that tab; no
  leak — already covered in `fullstack-8`).
* `Cmd+Shift+I` bulk toggle still affects all tabs
  (existing B17 spec) and preserves per-tab MUTE.

### Live bug fix

* After: create a group with N tabs → remove tab X
  from group → switch to tab X → its own "Broadcast
  input off" button is clickable → click → tab X
  re-joins the group with text flipping to "Broadcast
  input on".
* Add a regression test for the remove-then-rejoin
  cycle.

## Out of scope

* MUTE state — already independent per `fullstack-8`.
* The pill-to-icon swap — landed in `fullstack-8`.
* Cross-window BCAST (no such concept; single Hybrid
  per phase-2 model).

## How to start

1. Locate the per-tab BCAST toggle and the membership
   menu in `web/src/components/Pane.svelte` (and the
   shared store in `web/src/state/tabs.svelte.ts` or
   sibling).
2. Replace any "self entry in menu" rendering with
   `null` for the current tab.
3. Audit the toggle's disabled-when conditions —
   anything that disables the toggle based on
   membership is the live bug. Remove it.
4. Coordinate with @@WebtestB for the stress repro
   (their lane already has a 6-terminal harness).

## Hand-off

Standard. Pre-push gate green. Insert in @@FullStack's
queue after `fullstack-21`. Ping via
`alex/event-fullstack-architect.md`.

## Result

2026-05-19 05:22 BST — Implemented window-wide BCAST membership in
`web/src/state/tabs.svelte.ts` while preserving the existing persisted
tab fields as a projection of the single group. A tab's own broadcast
toggle now adds/removes only that tab from the shared group, the
membership menu updates the same shared group for other tabs, and the
terminal chip strip reflects shared membership across tabs. Tabs that
are not currently members no longer render the inline `off` chip.

Added regression coverage in `web/src/state/tabs.test.ts` for the live
remove-then-rejoin bug: remove a terminal from the group, switch to that
terminal's own toggle, and re-add it to the same window-wide group.

Verification:

* `npm run test -- tabs`
* `npm run check`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`
