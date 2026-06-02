# Phase-16 wave-3 @@LaneB - dashboard-config polish (DB1 + DB2)

Source: round-1-wave-3.md "## WAVE-3 FOLLOW-UP (@@Host): dashboard-config
polish (@@LaneB - your phase-15 area)". Two changes on the dashboard config
BACK side. All files are dashboard-area (collision-free with @@LaneD's editor).

## DB1 - carousel navigator replaces the segmented slot selector

Today (image #9, "weird widget selector"): `dashboard/DashboardSlotBack
.svelte` renders a segmented `About | Workspace | Search` button group
(`.slot-picker`) that moves the shared `tab.carouselSlide` cursor.

@@Host wants a carousel navigator (image #8): `<` prev + next `>` chevrons,
a dot pager (one dot per slot, active dot filled), and a pause/play toggle.

### Wiring
- prev / next / dot -> set `tab.carouselSlide` (same shared cursor the front
  carousel reads), then `scheduleSessionSave()`. Same hook the old picker
  used, so the title band + the dispatched config body still switch per slot
  and the front carousel lands on the same slot on flip-back.
- Slots shown: ALL three (About/Workspace/Search), wrapping at the ends. The
  config back must reach every slot's config even when a slot is toggled off
  for the front rotation, so it does NOT filter by `disabledSlots` (the old
  segmented picker showed all three; behaviour preserved). The FRONT carousel
  still shows enabled-only dots; the two faces intentionally differ in the dot
  SET but match in nav STYLE.
- pause/play -> toggles `tab.autoRotate` (default true). Rationale: autoRotate
  is a Lane-B-owned per-tab DashboardTab field (store.svelte.ts:987) whose
  documented role is "suppress auto-advance for THIS tab even when the global
  cycling pref is on" - i.e. a per-tab override. The config back is per-tab, so
  the per-tab knob is the right one, and it matches @@Host's exact word
  ("controls the carousel autoRotate"). The FRONT carousel's own inline
  play/pause stays wired to the GLOBAL `empty_pane_carousel_cycling` pref; the
  front respects BOTH axes via its `paused` derived
  (`... || !cycling || !autoRotate`), so a config pause genuinely stops the
  front. (Alternative considered: wire the config toggle to the global cycling
  pref for one-control consistency. Rejected: autoRotate is per-tab + Lane-B
  owned + the literal ask. One-line flip if @@Host meant the global pref.)
- The config navigator does NOT auto-rotate its own body (you are configuring,
  not watching) - unchanged from the old picker's intent.

### Affordances
Mirror the front carousel's controls (EmptyPaneCarousel.svelte
`.carousel-controls`): ChevronLeft/Right size 16, Pause/Play size 14, 7px
dots, the same muted/hover treatment. Built inline in DashboardSlotBack (not a
shared component) to avoid editing the 1170-line EmptyPaneCarousel or
store.svelte.ts (the latter is @@LaneA's this round); a future shared
`CarouselNav` extraction is possible once ownership frees up.

## DB2 - matrix screensaver preview fidelity

Today the preview (`screensaver/MatrixRainPreview.svelte`, mounted in
`dashboard/AboutSlotConfig.svelte` when theme=matrix) renders `drawStaticMatrix`
- a STATIC FULL GRID where every cell carries a glyph (mostly body-green, sparse
head/lead/mid). The REAL screensaver (`screensaver/MatrixRain.svelte`) ANIMATES
SPARSE FALLING COLUMNS (bright head -> lead -> mid -> body-green trail -> sparse
flicker -> fade to black) over a mostly-BLACK background. A dense wall of glyphs
vs sparse falling rain is @@Host's "ridiculously inaccurate". Root cause: the
two impls DRIFTED (the column state machine was "intentionally not extracted").

### Fix
1. Extract the column rain engine into shared `screensaver/matrixRain.ts`
   (`createRainColumns(numCols, numChars)` + `stepRain(ctx, cols, numCols,
   numChars)` = one frame of the existing `drawScreen`). Single source of truth
   so the two surfaces can never drift again.
2. Refactor `MatrixRain.svelte` to drive its animation off the shared engine
   (behaviour-preserving; same constants, same per-cell logic).
3. Rewrite `MatrixRainPreview.svelte` to ANIMATE the real rain via the shared
   engine instead of re-rolling the static full grid. Gate the rAF on
   `prefers-reduced-motion` (-> a single static SNAPSHOT frame, not a loop),
   `document.hidden`, and an IntersectionObserver (stop when off-screen).
   Known trade-off: the dashboard back face is latched-mounted + rotated away
   (backface-visibility:hidden) when showing the front, which IntersectionObserver
   cannot detect, so a tiny managed rAF may run while flipped-to-front+About+
   matrix; it is cancelled on destroy. Accepted vs editing the shared Pane.svelte
   (not my file) to thread a visibility prop.
4. `drawStaticMatrix` becomes a SNAPSHOT of the rain (sparse columns frozen at
   random fall positions) rather than a full grid, so the reduced-motion path
   (used by BOTH the real screensaver and the preview fallback) also looks
   accurate.

VERIFY: `make web-check` green; browser-smoke the config navigator (slot
switch + pause/play) and the matrix preview side-by-side with the real
screensaver (Mod+L / Test).
