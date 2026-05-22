# fullstack-a-59 — pane-focus-click: select pane under cursor on click-to-focus restore (not on Cmd+Tab)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

When chan-desktop loses focus + the user clicks back on
the window, the first click should ALSO select the
Hybrid pane under the cursor (not stay on the previously-
selected pane). Critical disambiguation: only on the
mousedown-driven focus restore. Cmd+Tab keyboard refocus
must NOT change pane selection.

## Reference

[`../phase-8-bugs.md`](../phase-8-bugs.md) "chan-desktop
first click after window-focus restore doesn't follow
the mouse to select the pane under the cursor" — full
bug body with detection shape + Cmd+Tab disambiguation
+ implementation hints.

## Detection shape

SPA listens for window `focus` event + `mousedown`
event. If `mousedown` fires within ~50ms of the `focus`
event, treat as click-to-focus restore + dispatch
paneSelect on the pane under the mousedown target. Map
the mousedown DOM target to a pane via the DOM ancestry
chain. Focus-without-adjacent-mousedown (Cmd+Tab) → no
pane-select change.

## Acceptance

1. **Click-to-focus restore**: chan-desktop unfocused;
   user clicks on a Hybrid pane (different from the
   previously-active one); window refocuses; clicked
   pane becomes active; subsequent typing lands in the
   newly-active pane.
2. **Cmd+Tab restore**: chan-desktop unfocused; user
   Cmd+Tabs back; window refocuses WITHOUT a mousedown;
   pane selection unchanged.
3. **Click within the already-active pane**: no
   visible change (no-op; pane was already active).
4. **Click outside any pane** (chrome / hamburger /
   tab strip): no pane-select change (the click goes
   to whatever it landed on; pane state stays).

### Tests

Vitest pins for the timing-window detection (mousedown
within ~50ms of focus → paneSelect); for Cmd+Tab
(focus-only → no paneSelect); for click-within-active-
pane no-op.

### Gate

* `npm test -- --run` green.
* `npm run check` 0e/0w.
* `npm run build` clean.

## Coordination

* @@FullStackA primary (SPA window-focus + mousedown
  event handling).
* If timing turns out to need Tauri-side mediation
  (e.g. focus event fires before mousedown reliably),
  cross-lane to @@FullStackB. Fire scope poke if so.
* Atomic-audit-commit discipline.

## Authorization

**Yes** for SPA (`web/src/App.svelte` or wherever
window-focus event handling lives + the pane-select
dispatch path).

## Numbering

This is `-a-59`.
