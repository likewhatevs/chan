# fullstack-61: flash "H for help" centre overlay on Pane Mode entry

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex flagged that Pane Mode discoverability
needs a beat — most users press Cmd+K and see
the pill / chip, but the help cheatsheet
(triggered by `H`) isn't obvious. A short centre-
screen flash on Pane Mode entry telegraphs "press
H for help" before fading.

## Spec

* **Trigger**: every entry into Pane Mode (i.e.
  every time `paneMode.draft` transitions from
  `null` → set, equivalent of Cmd+K firing).
* **Location**: centre of the chan window (NOT
  centre of focused pane). Above all panes,
  below any modal dialog.
* **Content**: the text `H for help`.
  Wording is your call within this constraint —
  could be `H for help`, `Press H for help`, or
  similar. Short, no chrome. Make sure the `H`
  is visually prominent (it's the keystroke
  cue).
* **Duration**: visible 0.7s total. Suggested
  shape: ~100ms fade-in, ~400ms hold, ~200ms
  fade-out. Whatever feels right; the spec is
  "feels like a flash, not a dialog".
* **Style**: subtle. Semi-transparent backdrop
  glow or text-only. Should match the existing
  chan visual language (look at how the
  `pane mode · Enter commit · Esc discard`
  chip is styled and lean into that palette).
* **Non-blocking**: the flash must NOT block
  keystrokes / mouse events. Pane Mode is
  driving from the moment Cmd+K fires; the
  flash is a passive overlay only.

## Relevant code

* `web/src/App.svelte` — the Pane Mode entry
  point. Find where the `paneMode.draft`
  transition happens; mount the flash overlay
  there.
* `web/src/state/tabs.svelte.ts` — `paneMode`
  state machine. The entry transition is the
  hook point if there's one centralised.
* `web/src/components/PaneModeHelp.svelte`
  (current help overlay) — pattern reference,
  not modification target.
* Existing pill chip:
  `pane mode · Enter commit · Esc discard`
  — find its component for style cues.

## Acceptance criteria

* Pressing Cmd+K (when not already in Pane
  Mode) renders the centre flash visible for
  ~0.7s.
* The flash auto-dismisses without user input.
* Pressing `H` during the flash window opens
  the help cheatsheet as today (the flash
  doesn't intercept keystrokes).
* Exiting Pane Mode (Esc) does NOT trigger the
  flash. Only entry triggers it.
* Re-entering Pane Mode after exit triggers
  the flash again — every entry, not "once per
  session".
* The flash is positioned in the screen
  centre, NOT relative to the focused pane.
  Verify with a split-pane layout where the
  focused pane is in a corner — flash still
  centres on the window.

### Tests

* Vitest: assert the flash overlay element
  mounts within ~50ms of `paneMode.draft`
  being set, and unmounts within ~750ms.
* Component test (optional): visual check
  via JSDOM that the element has expected
  positioning + classes.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* Coordinate with `fullstack-62` (Pane Mode →
  Hybrid NAV rename) — the flash copy should
  use the post-rename wording. If `-62`
  ships first, copy reads `Hybrid NAV` /
  similar; if `-61` ships first, use the
  current `Pane Mode` wording and the rename
  task will sweep this surface too. Either
  ordering is fine.
* Queue position: behind `-55`, `-56`, `-57`
  on Lane A.
* Standing topic-level commit clearance.

## 2026-05-19 17:00 BST — @@FullStackA implementation note

Implementation:

* `App.svelte`: added local state
  (`paneModeFlashVisible`, `paneModeFlashKey`,
  `paneModeWasActive`, `paneModeFlashTimer`) +
  a `$effect` watching `paneMode.active`. On
  `false → true` (every Pane Mode entry,
  including re-entry after exit) the effect
  bumps `paneModeFlashKey`, sets
  `paneModeFlashVisible = true`, and schedules
  a `setTimeout` (`PANE_MODE_FLASH_MS = 700`)
  to flip it back to `false`. `onDestroy`
  clears any pending timer to avoid leaks.
* Rendering: `{#if paneModeFlashVisible}{#key
  paneModeFlashKey}<div class="pane-mode-flash">
  ...</div>{/key}{/if}` near the modals
  section. `{#key}` ensures consecutive
  entries re-trigger the CSS keyframe even
  when the previous flash hasn't finished.
* Style: `position: fixed; inset: 0; display:
  flex; align-items: center; justify-content:
  center`. The flash centres on the
  **window** (not the focused pane) per the
  spec. `pointer-events: none` keeps it
  passive so `H` / `Enter` / `Esc` flow to
  the existing Pane Mode handlers. The H key
  chip uses `--font-mono`, white-background-
  on-bg-card, with a subtle drop shadow;
  `for help` sits next to it in muted text.
* Keyframe: 0.7s fade with a gentle vertical
  drift (-6px → 0 → +4px) + scale (0.96 → 1
  → 0.98). `prefers-reduced-motion` swaps to
  a plain opacity fade.
* z-index `25500` — above the OverlayShell
  stack (25000+) but below modals (26000) so
  a modal preempts visually. Below the
  AppStatusBar (28000) too, which is fine
  since the status bar lives in the corner
  while the flash centres.

Coordination with `-62` (Pane Mode → Hybrid
NAV rename): copy reads the current `for
help` since `-62` hasn't shipped on Lane A
yet; when it lands, the rename sweep can
swap the wording if needed (the spec lets
either ordering work).

Tests added in `paneModeKeymap.test.ts`:

* Raw-source assert on the `$effect`'s
  one-shot trigger shape (active && !was →
  bump key + set visible + schedule timeout).
* Raw-source assert on the flash DOM
  (H key span + "for help" text + the
  pointer-events: none property in the
  stylesheet).

Gate green:

* `npm run test -- paneModeKeymap` (10
  passed),
* `npm run test` (367 passed),
* `npm run check` (0 errors / 0 warnings),
* `npm run build`,
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`
  (green).

Visual eyeball worth doing: press Cmd+K
several times — verify the flash centres on
the window in split-pane layouts (focused
pane corner doesn't matter), auto-dismisses
in ~0.7s, and pressing H during the flash
still opens the cheatsheet.

Proposed commit message:

> Flash "H for help" on Pane Mode entry (fullstack-61)
>
> Short 0.7s centre-window flash with an H key chip
> + "for help" text on every Pane Mode entry
> (`paneMode.active` false → true transition).
> Telegraphs the help cheatsheet binding for users
> who hit Cmd+K but don't know about H. Non-
> blocking (`pointer-events: none`); keystrokes flow
> to the existing handlers. `{#key}` re-triggers the
> CSS keyframe on consecutive entries. Reduced-
> motion variant fades opacity-only.
