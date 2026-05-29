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

## 2026-05-22 — ready for review

Three-file change (1 component + 1 SPA + 1 new
test). SPA-only; no Rust touched. No Tauri-side
mediation needed (window `focus` event fires
reliably on the JS side; the existing per-pane
mousedown is too late since OS sometimes
consumes the first focus-restore click).

### What landed

`web/src/components/Pane.svelte`: added
`data-pane-id={pane.id}` to the `.pane` root
`<div>`. Lets the window-level mousedown
handler walk a click target's DOM ancestry +
resolve which pane was clicked.

`web/src/App.svelte`:

* `FOCUS_CLICK_WINDOW_MS = 50` — the
  click/focus correlation window. 50ms is short
  enough to avoid false-positive pane-select
  on idle clicks that happen to land on a pane
  long after a focus event.
* `focusRestoreAt` mutable state stamps
  `Date.now()` on `window` focus events.
* `onWindowMouseDown(e)`:
  * Short-circuit when `focusRestoreAt === 0`
    (no recent focus event — Cmd+Tab path
    leaves this 0 since Cmd+Tab fires focus
    but no mousedown follows). Existing
    Pane.svelte `onmousedown` handles the click.
  * Short-circuit when the gap exceeds
    `FOCUS_CLICK_WINDOW_MS` (idle click long
    after focus restore).
  * Otherwise walk `e.target.closest(".pane[
    data-pane-id]")` + call `setActivePane`
    with the resolved id.
  * Clear `focusRestoreAt = 0` after the match
    so subsequent clicks fall back to the
    per-pane handler.
* Listener registered with `capture: true` so it
  fires BEFORE `Pane.svelte`'s onmousedown. The
  per-pane handler also calls `setActivePane`
  (with the same id, so this is idempotent),
  but capture-phase ensures the window-level
  handler always gets a chance to map the
  click to a pane even if the bubble-phase
  propagation gets stopped by a child.

`web/src/components/paneFocusClickRestore.test.ts`
(new): 10 raw-source pins covering the
attribute, the constants, the focus stamping,
the DOM-ancestry walk, the setActivePane call,
the short-circuits, the cleanup, and the
import.

### Acceptance

1. **Click-to-focus restore**: window `focus`
   stamps `focusRestoreAt`; the immediate
   mousedown within 50ms walks DOM ancestry +
   calls `setActivePane(paneId)`. ✓
2. **Cmd+Tab restore**: `focus` event fires but
   NO mousedown follows. `focusRestoreAt` stays
   set but no handler walks it; on the user's
   next click (likely > 50ms later), the gap
   check fires the short-circuit + resets to 0.
   No pane-select side-effect. ✓
3. **Click within already-active pane**:
   `setActivePane` is idempotent (`current.
   activePaneId = paneId` no-op when already
   equal); no visible change. ✓
4. **Click outside any pane** (chrome /
   hamburger / tab strip): `closest(".pane[
   data-pane-id]")` returns null when the
   target is outside any pane element. Handler
   returns early without calling
   `setActivePane`. ✓

### Gate

* vitest **748 / 748** (+10 net from `-a-63`'s
  738).
* svelte-check 0 errors / 0 warnings across
  4000 files.
* npm build clean.
* Rust gate not re-run.

### Decisions

* **Capture phase** on the mousedown listener
  so it always fires before per-pane
  bubble-phase handlers. Defensive against
  any descendant that stops propagation.
* **50ms correlation window** per the bug
  body's recommendation. Cmd+Tab can sometimes
  generate spurious mousedowns if the user
  trackpads through (rare) — 50ms is short
  enough that the trackpad gesture has
  typically completed.
* **Clear after first match** so an idle click
  long after focus doesn't trigger
  pane-select. The per-pane mousedown handler
  still fires for those.
* **No Tauri-side mediation** — the bug body's
  fallback path was if the JS focus event
  proved unreliable. It's reliable on macOS
  Tauri 2.x; no cross-lane work needed.

### Suggested commit subject

```
Pane focus-click restore: select pane under cursor on click-to-focus (fullstack-a-59)
```

Single commit. Attribute + handler + test
tightly coupled.

### Files for `git add` (per-path discipline)

* `web/src/components/Pane.svelte`
* `web/src/App.svelte`
* `web/src/components/paneFocusClickRestore.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-59.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.
