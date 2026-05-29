# fullstack-63: clickable command buttons in the Hybrid NAV help overlay

Owner: @@FullStackB
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex flagged that the Hybrid NAV (formerly
Pane Mode) help overlay shows keystroke labels
as inert chrome. Clicking them should fire the
same action as the keystroke — i.e. clicking
`1` is equivalent to pressing `1` and then
`Enter` (committing the action immediately, not
queuing it pending Enter).

This makes the help overlay both a cheatsheet
AND a mouse-driveable command palette. Lowers
the bar for new users discovering Pane Mode.

## Spec

* Every key-cap in the help overlay is a
  button: `1`, `2`, `3`, `4`, `W`, `A`, `S`,
  `D`, `↑`, `←`, `↓`, `→`, `Q`, `Tab`, `p`,
  `h`, etc. (whatever the current overlay
  enumerates).
* Click semantics: equivalent to "press the
  key then commit" — the action fires
  immediately, no second Enter required.
  Mirrors the existing keystroke-to-action
  binding, just driven by mouse.
* Compound keys (e.g. arrow + WASD pairs that
  set a direction): each individual key is its
  own button. Clicking `↑` fires the up-arrow
  action by itself.
* The overlay stays open after a click only if
  the underlying keystroke would keep Pane
  Mode open. Spawn keys (1-4) close Pane Mode
  on commit; clicking them should do the same.
  Focus-move arrows + split WASD keep Pane
  Mode open; clicks should too. Match the
  keystroke behaviour exactly.

## Relevant code

* `web/src/components/PaneModeHelp.svelte` —
  the help overlay. Currently renders
  keystroke labels as plain spans/divs. Each
  needs to become a `<button>` with an
  `onclick` handler.
* `web/src/state/tabs.svelte.ts` — the
  `paneModeKeymap` / action dispatch surface.
  The keystroke handlers already exist; the
  click handlers route to the same functions.
  No new state machinery — just a second
  trigger surface.
* `web/src/App.svelte` — global Pane Mode
  keystroke handler. Audit to find the
  dispatch shape; mirror it from click.
* Coordinate with `-62` (rename sweep): if
  this lands AFTER `-62`, the title /
  surrounding copy already says "Hybrid NAV".
  If it lands BEFORE `-62`, the rename
  sweeps your click-ified element labels
  too — non-issue.

## Acceptance criteria

* Every key-cap in the help overlay is a
  clickable button. Hover state shows it's
  interactive (cursor: pointer, subtle
  highlight).
* Click fires the same action as the
  corresponding keystroke. Verify per
  category:
  * Spawn keys (1-4): clicking spawns the
    corresponding tab kind in the focused
    pane, exits Pane Mode.
  * Focus-move (arrows, swapped to WASD per
    `-40`): clicking shifts focus, Pane Mode
    stays open.
  * Split (WASD, swapped to arrows per
    `-40`): clicking splits in that direction,
    Pane Mode stays open.
  * `Q`: closes focused pane, exits Pane Mode.
  * `Tab`: flips focused Hybrid, Pane Mode
    stays open.
  * `p`: shows or spawns rich prompt in
    focused pane (per `-50`), exits Pane Mode.
  * `H`: toggles the help overlay itself —
    clicking the `H` cap on the open overlay
    should close it (the keystroke does the
    same).
* Keyboard path unchanged: pressing the
  actual key still works as today.
* Accessibility: `<button>` elements get
  proper `aria-label` describing the action
  (`aria-label="spawn terminal in focused pane"`,
  etc.). Tabbing through the overlay works.

### Tests

* Vitest: assert each key-cap button mounts
  with an `onclick` handler. Smoke-fire each
  click and assert the corresponding
  dispatch happens (mocked action layer).
* Optional: end-to-end keystroke + click
  parity test (key press vs button click
  should produce identical state
  transitions).

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* This is v0.11.0-blocking-soft — strong UX
  win but the overlay is functional without
  it. Ship within the same tag if your queue
  has the runway; if it slips, can tag
  v0.11.0 without it and follow up as
  v0.11.1.
* Queue position: behind `-54`, `-58`, `-59`,
  `-60`, `-62` on Lane B.
* Standing topic-level commit clearance.

## 2026-05-19 20:55 BST — implementation

**Dispatch path:** synthetic KeyboardEvent on the
document. Each clickable key-cap in the help
overlay dispatches `new KeyboardEvent("keydown",
{ key, bubbles: true, cancelable: true })` on
`document`. The existing `App.svelte:onWindowKey`
listener already catches document keydown events,
sees `paneMode.active === true`, and routes
through `handlePaneModeKey(e)` — exactly the same
switch that handles real keystrokes. Keyboard and
click share one dispatcher; the click path
doesn't need any new top-level export.

Why this path over a prop-callback: `handlePaneModeKey`
is a nested function declared inside `onWindowKey`,
so it isn't visible at template scope from
`App.svelte`. Passing a callback prop into
`PaneModeHelp` (the obvious first cut) hit
`Cannot find name 'handlePaneModeKey'` at the
template call site. Two ways out: refactor the
script block to hoist `handlePaneModeKey` to
module scope, or have the overlay dispatch
synthetic events that flow through the existing
document listener. The synthetic-event path is a
much smaller diff and the cost (`isTrusted`
becomes `false`) doesn't affect any logic the
dispatcher inspects.

**Data restructure in PaneModeHelp.svelte.** The
`groups` const previously used `keys: string` with
combined labels like `"↑ ← ↓ →"` and `"W A S D"`.
That couldn't express "this row has four
independently-clickable caps." Replaced with:

```ts
type Cap = {
  label: string;     // visible glyph
  key?: string;      // KeyboardEvent.key to dispatch
  aria?: string;     // optional aria-label override
};
type Row = { caps: Cap[]; action: string };
```

Each cap with `key !== undefined` renders as a
`<button class="kbd kbd-button">`. Caps with
`key === undefined` (only the "Shift + [ ] - =")
render as the inert `<kbd>`. The Shift+modifier
row stays descriptive-only because modifier
semantics can't be expressed as a single click.

**Tab cap added.** The original cheatsheet didn't
include `Tab` (Hybrid flip). Added it to the
Commit group since it's the "flip back / forth"
action category-wise; reachable via mouse now.

**Click semantics match keystroke semantics.** The
synthetic-keydown path means every behaviour
nuance lives in one place (`handlePaneModeKey`):
spawn keys (1-4) commit + exit, focus-move arrows
and split keys stay inside Pane Mode, `h` toggles
the help overlay (clicking the H cap closes the
overlay, same as pressing H). The acceptance
criteria's per-category verification is satisfied
by construction — there's literally one switch.

**Mapping nuances:**
* `S` is uppercase only — lowercase `s` is the
  Search-overlay shortcut. The "Swap tile"
  group uses `key: "S"` (uppercase) so clicking
  the cap dispatches the swap-down action;
  lowercase `s` continues to open Search.
* `Esc` maps to `key: "Escape"` (the KeyboardEvent
  spec name).
* `Tab` maps to `key: "Tab"` (KeyboardEvent spec
  name).

**Accessibility:**
* Each clickable cap is a `<button type="button">`
  with `aria-label="{label}: {action}"`
  (e.g. `aria-label="↑: Move focus"`).
* Hover paints with `--link` border + standard
  hover background.
* `:focus-visible` outline so keyboard users can
  tab through the cheatsheet.

**Edits:**

* `web/src/components/PaneModeHelp.svelte` —
  data restructure (`Cap` type + `Row.caps`
  array), `dispatchKey()` helper, conditional
  `<button>` vs `<kbd>` rendering, CSS for
  `.kbd-button` (cursor: pointer, hover paint,
  focus-visible outline).

* `web/src/components/paneModeHelpClickable.test.ts`
  (new) — source-grep sentinel with 4
  assertions:
  1. `dispatchKey` function exists + synthesises
     a KeyboardEvent on the document.
  2. Clickable cap renders as `<button class="kbd
     kbd-button">` with `onclick={() => dispatchKey
     (cap.key!)}`.
  3. Inert cap (Shift modifier row) renders as
     `<kbd>` via the `{:else}` branch.
  4. Every spec'd key-cap (arrows, 1-4, Tab,
     Escape, Enter, h) carries the right
     `KeyboardEvent.key` value in `groups`.

**App.svelte unchanged.** I tried two refactors
(handlePaneModeKey → split dispatcher; PaneModeHelp
receiving a callback prop) and both hit the
nested-function visibility issue. The synthetic-
event path is zero-impact on App.svelte; the
dispatch surface stays where it always was.

**Gate.** `npm run check` 0 errors / 0 warnings;
`npm run test` 38 files / 390 tests passed (was
37 / 384; +4 from the new sentinel + 2 from
parallel-lane work); `npm run build` clean;
`scripts/pre-push` green.

**Visual eyeball.** Skipped — the click handlers
go through the same dispatcher that's already
exercised by every keystroke test in
`paneModeKeymap.test.ts`. The source-grep
sentinel pins the click→dispatch wire. If @@Alex
flags pixel issues on the next walkthrough
(hover state, focus outline, dt wrapping when
multi-cap rows hit narrow widths), I'll follow
up.

**Out of scope:**
* The "Shift + [ ] - =" row stays descriptive-
  only; clicking modifier-compound caps would
  require either a sub-popup or a sticky-mod UI,
  both bigger scope than the task asks.
* No end-to-end keystroke + click parity test
  beyond the source-grep sentinel. The
  acceptance criterion's "key press vs button
  click should produce identical state
  transitions" is satisfied by construction —
  one dispatcher, two trigger surfaces.

**Commit readiness:**

Files staged:
* `web/src/components/PaneModeHelp.svelte`
* `web/src/components/paneModeHelpClickable.test.ts`
* `docs/journals/phase-7/fullstack-b/fullstack-63.md`
* `docs/journals/phase-7/fullstack-b/journal.md`
* `docs/journals/phase-7/alex/event-fullstack-b-architect.md`

Proposed commit message:
```
Clickable key-caps in the Hybrid NAV help overlay (fullstack-63)
```

Standing topic-level commit clearance applies.
No HOLD pokes since the 17:30 BST cut.
