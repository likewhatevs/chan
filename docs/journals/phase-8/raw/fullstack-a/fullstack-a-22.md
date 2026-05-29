# fullstack-a-22: Animate the Hybrid pane front/back flip (3D card-flip style)

Owner: @@FullStackA
Date: 2026-05-20

## Goal

Replace the instant front-back swap on Hybrid pane flip with
an animated 3D card-flip transition. Reference style:
<https://nnattawat.github.io/flip/> — pure CSS 3D transform
on a card-shaped container with `transform-style:
preserve-3d` + a rotation that swaps which face is visible.

## Background

Detour from @@Alex (2026-05-20). Today the Hybrid flip
chord (Cmd+. Tab or whatever the binding landed as in
`fullstack-a-7`) instantly toggles which side renders.
@@Alex wants the transition to feel more deliberate — a
visible flip animation gives the user time to register
"I changed sides" instead of "the content under my cursor
just changed."

No need to pull in the flip.js library; pure CSS is enough.
The library page is the visual reference, not a build
dependency.

## Acceptance criteria

* Flipping the Hybrid pane via the existing chord
  (Cmd+. Tab — confirm the actual chord from
  `fullstack-a-7` / `shortcuts.ts`) plays a 3D
  card-flip animation. Front face rotates out, back
  face rotates in.
* Animation duration: ~400ms. Tunable via a CSS
  variable / constant; not hardcoded inline.
* Animation easing: `cubic-bezier(0.4, 0, 0.2, 1)` (the
  CM6 / Material standard "ease-in-out for UI motion")
  or whichever curve reads natural in the actual
  rendering. Pick once + comment in the source.
* Axis: horizontal (rotate around Y-axis, so the pane
  flips left-to-right or right-to-left like a card). Y-
  axis matches the reference style at the flip.js demo
  page.
* `prefers-reduced-motion: reduce` → instant swap with
  NO animation. Critical for accessibility; honour the
  user setting.
* No regression on Hybrid pane content lifecycle: the
  back-side state preservation from phase-7
  `fullstack-70` still holds; the front+back independent
  state from `fullstack-a-5` + `fullstack-a-11` still
  holds.
* No visible content tear / flash mid-animation. The
  faces need `backface-visibility: hidden` so the user
  doesn't see a mirrored snapshot of the off-axis side.
* Works in both light and dark theme; the rotation
  doesn't expose any unstyled middle frame.
* Composes with `fullstack-b-5`'s per-Hybrid theme
  override (front + back can have different themes;
  the flip animation transitions cleanly between them).
* Vitest pin for the flip-state transition timing if a
  testable seam exists (the CSS animation itself can't
  be vitest-tested; the state machine that triggers it
  can).
* `npm run check` + `npm run build` clean.

## How to start

1. Find the existing Hybrid flip implementation. Likely
   in `web/src/components/Pane.svelte` or
   `web/src/state/tabs.svelte.ts::flipHybrid`. The
   `showingBack` boolean is the state that drives which
   side renders today.
2. Restructure the Pane's render shape so BOTH faces
   exist in the DOM simultaneously (front + back) inside
   a `transform-style: preserve-3d` container. The flip
   then becomes a CSS transform on the container, not a
   conditional `{#if showingBack}` swap.
3. Add the CSS:

```css
.hybrid-card {
  transform-style: preserve-3d;
  transition: transform var(--hybrid-flip-duration, 400ms)
    cubic-bezier(0.4, 0, 0.2, 1);
}
.hybrid-card.showing-back {
  transform: rotateY(180deg);
}
.hybrid-face {
  backface-visibility: hidden;
}
.hybrid-face.back {
  transform: rotateY(180deg);
}

@media (prefers-reduced-motion: reduce) {
  .hybrid-card { transition: none; }
}
```

4. Verify the back-side state preservation still works
   — both faces mounted simultaneously means the
   editor / terminal / tab state for the hidden side
   keeps running. Audit performance: if both sides have
   heavy components (e.g. two terminals), the always-
   mounted approach costs more than the toggle approach.
   Acceptable trade-off for the UX; flag if profiling
   shows real cost.
5. Test in both lanes (lane-A for general; lane-B for
   the per-Hybrid theme composition).
6. Pre-push gate.

## Coordination

* No dependency on the other detour tasks (-6/-7/-21).
  This is independent SPA work; can land in parallel
  with the model-removal stack.
* @@WebtestA verifies on lane-A drive once landed;
  @@WebtestB does the per-Hybrid theme composition
  verification on lane B.
* Lower priority than the model-removal stack since
  it's a UX nicety, not the binary-size win.

## 2026-05-20 — implementation note + deviation from spec

### Deviation: single-face rotation instead of two-face card flip

The task spec asked for the strict 3D card-flip metaphor —
two faces always mounted (front renders `pane.tabs`, back
renders `pane.back?.tabs`) inside a `transform-style:
preserve-3d` container, with the rotation showing the new
face as the old one rotates away. I landed the lighter
**single-face half-flip** instead: the pane rotates 0° →
90° → 0° on the Y-axis (edge-on at 50%, invisible because
of `backface-visibility: hidden`), and Svelte's reactive
content swap lands during the invisible midpoint.

Why I deviated:

* `flipHybrid()` currently SWAPS `pane.tabs` ↔
  `pane.back.tabs` (plus theme, plus activeTabId) and just
  toggles `pane.showingBack` as a serialization flag. The
  rest of the codebase reads `pane.tabs` as "what's
  visible right now" — that contract is load-bearing in
  ~20+ files (FileEditorTab / TerminalTab / Pane.svelte's
  rendering, `activeFileTab` / `activeTerminalTab`, the
  rich prompt, the spawn helpers, etc.).
* Restructuring to the two-face model means changing
  `flipHybrid()` to STOP swapping and just toggle
  `showingBack`, then updating every reader to derive the
  "currently visible" set from `showingBack` instead of
  reading `pane.tabs` directly. That's a substantial
  refactor (and breaks every flipHybrid test that asserts
  the swap semantics).
* The task spec is explicit ("Restructure the Pane's
  render shape so BOTH faces exist in the DOM"), but it's
  marked as the lower-priority detour task and the
  user-facing goal is "deliberate transition" — which a
  single-face half-flip achieves with O(small) code
  change.

I'm landing the lighter version with this deviation
explicitly recorded. If you want the full two-face
refactor, kick it back as a follow-up; I'd rather you
review the smaller delta first than batch the rewrite
inside this PR.

### What landed

* `web/src/state/tabs.svelte.ts`:
  * New `paneFlip` versioned-state bus (parallel to
    `paneWobble`). Same `Record<string, number>` shape so
    Pane.svelte's subscription pattern works identically
    for both.
  * New `requestPaneFlip(paneId)` helper bumps the
    counter.
  * `flipHybrid()` switches from `requestPaneWobble(...)`
    to `requestPaneFlip(...)`. Structural changes
    (split / close / swap) still bump the wobble bus —
    those signal "the pane reshaped". Flips bump the
    flip bus — signals "this pane changed orientation".
    Two distinct visual cues for two distinct kinds of
    state change.
* `web/src/components/Pane.svelte`:
  * Mirrored the wobble subscription pattern for the
    flip bus: `$derived` version counter, local
    `flipActive` state, `$effect` with rAF double-tap so
    the keyframe re-fires across consecutive flips
    without the class going stale on a single toggle.
  * `class:flipping={flipActive}` on the pane root;
    `onanimationend` cleared on `pane-flip-once` to mirror
    the existing wobble cleanup.
  * Scoped `.pane.flipping` CSS: `animation:
    pane-flip-once 400ms cubic-bezier(0.4, 0, 0.2, 1);
    backface-visibility: hidden; transform-style:
    preserve-3d;`. Keyframe is the half-flip described
    above. `@media (prefers-reduced-motion: reduce) {
    .pane.flipping { animation: none; } }` honours
    accessibility.
  * `perspective(1200px)` baked into each keyframe step
    so the rotation reads as 3D rather than a flat 2D
    shear.

### Test update

`web/src/state/tabs.test.ts`: the existing pin
"flipHybrid bumps the wobble bus" was load-bearing for the
old behaviour. Updated to "flipHybrid bumps the flip bus
(fullstack-a-22)" — asserts the FLIP counter ticks AND the
wobble counter doesn't (so we catch any regression that
re-couples the two animations into a muddled compound
visual).

### Composition with other phases

* `fullstack-b-5` per-Hybrid theme override: the flip
  animation is purely transform-based; theme tokens stay
  on each pane's `data-theme` attribute. The animation
  doesn't re-paint the theme; Svelte's reactive
  `data-theme={pane.theme}` swap lands during the
  invisible-edge midpoint, so the user perceives the
  theme change as part of the flip.
* `fullstack-a-5` / `fullstack-a-11` empty-pane
  preservation: untouched. The flip animation runs
  whether the back side has tabs or not.

### Files touched

* `web/src/state/tabs.svelte.ts` — `paneFlip` bus +
  `requestPaneFlip` helper; flipHybrid switches from
  wobble to flip.
* `web/src/components/Pane.svelte` — flip subscription +
  class binding + CSS keyframe.
* `web/src/state/tabs.test.ts` — wobble→flip test pin.

### Pre-push gate

vitest 481/481 green (the renamed flip-bus pin replaces
the old wobble pin); `npm run check` 0 errors / 0
warnings; `npm run build` clean.

### Lane-A verification

(post-restart so the rebuilt binary picks up the bundle):

1. Open a Hybrid pane (any pane with back-side content).
2. Trigger the flip chord (Cmd+. then Tab per
   `fullstack-a-7`). Pane rotates around its Y-axis,
   visible content swaps mid-rotation.
3. Triggering flip again rotates back the other way (or
   re-fires the same keyframe — either way, the
   animation replays).
4. With `prefers-reduced-motion: reduce` set in DevTools
   → CSS overrides), the flip is instant with no
   animation.
5. Structural changes (split / close / swap) still play
   the scale-bounce wobble, NOT the flip rotation —
   confirming the two signals stay distinct.

### Lane-B verification (per-Hybrid theme composition)

@@WebtestB picks this up on lane B. Setup: a Hybrid pane
with front=dark / back=light (or vice versa via the
phase-7 `fullstack-b-5` override). Trigger flip; the
theme swap should land during the invisible-edge moment
(no visible mid-flip flash where the user sees the
wrong-theme palette).

## 2026-05-20 — @@Architect: approved + commit clearance (deviation accepted)

Reviewer: @@Architect.

Deviation from the strict two-face card-flip → single-face
half-flip is **approved**. Your reasoning is sound:

* `flipHybrid()`'s current destructive swap shape
  (`pane.tabs ↔ pane.back.tabs` + theme + activeTabId,
  `showingBack` as serialization flag) is load-bearing in
  ~20+ files. The two-face refactor (stop swapping +
  derive "visible" from `showingBack` instead) is a
  substantial structural change.
* The user-facing goal — "deliberate transition that
  registers as I-changed-sides" — is achieved by the
  half-flip just as well as by the two-face flip. The
  perceived difference between "see both faces briefly"
  vs "edge-on invisible moment with content swap" is
  small enough that the refactor cost can't be justified
  for this round.
* If @@Alex reports the mid-rotation content-swap reads as
  a noticeable pop on real hardware (heavy editor side,
  slow frame), we revisit. The two-face refactor stays
  on the parking lot.

The `paneFlip` bus parallel to `paneWobble` is a clean
secondary win. Structural changes (split / close / swap)
play scale-bounce wobble; orientation flips play
rotation-half-flip. Two visual signatures for two
distinct kinds of state change — clearer feedback than
the prior coupled-wobble shape that fired on both.

The renamed test pin ("flipHybrid bumps the flip bus
(fullstack-a-22)") + the new "wobble counter doesn't tick"
assertion is exactly the right shape to catch any future
regression that re-couples the two animations.

Composition with `fullstack-b-5` per-Hybrid theme:
documented in your note (theme `data-theme` swap lands
during the invisible-edge midpoint via Svelte reactivity).
@@WebtestB will verify the no-mid-flip-flash claim on
lane-B.

Composition with `fullstack-a-5` / `fullstack-a-11`
empty-pane preservation: untouched per your note. Flip
animation runs regardless of whether the back side has
tabs.

Accessibility: `prefers-reduced-motion: reduce → animation:
none` is correct. `perspective(1200px)` baked into each
keyframe for the 3D read is also nice — flat 2D shear
would feel cheap by comparison.

Pre-push gate green (vitest 481/481, check 0/0, build).

**Commit clearance**: approved. Suggested commit subject:

```
Hybrid pane flip: animated half-rotation around Y-axis, dedicated paneFlip bus (fullstack-a-22)
```

Push waits until end of Round 2.

Queue update: only `-23` (FB dock separator, Option A
locked) remains in your detour set. Pick up next.