# fullstack-b-4: Indexing-chart pan/zoom parity with Graph view

Owner: @@FullStackB
Date: 2026-05-19

## Goal

The indexing-graph slide in the carousel currently clips at the
viewport edges and cannot be panned. Bring it to parity with the
main Graph view: drag-to-pan, wheel-to-zoom, recenter affordance.

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md) under the
"Index chart in the carousel is trimmed and not pannable" item.

The main Graph view (phase-7 `fullstack-N`-series — `GraphCanvas`)
already implements pan + zoom. The carousel's indexing-graph
slide should reuse the same component or, at minimum, the same
input handlers.

Coordination note: phase-8 Round-2 backlog item 4 refactors the
carousel into Infographics tabs. This fix should survive that
refactor — keep the pan/zoom behaviour at the chart-component
level, not the slide-container level, so it carries forward.

## Acceptance criteria

* Indexing-chart slide in the carousel supports drag-to-pan.
* Wheel scrolling zooms in/out (matching Graph view behaviour).
* A recenter affordance restores the default view.
* Visual parity with the main Graph view for input gestures.
* No regression in the carousel's existing cycle / DnD behaviour.

## How to start

* `web/src/components/EmptyPaneCarousel.svelte` (or wherever the
  indexing-graph slide lives) — find the chart render.
* Compare against `GraphCanvas.svelte` for the pan/zoom
  reference implementation.
* If the chart uses a different rendering primitive, lift the
  shared pan/zoom helper out (consult @@FullStackA if a shared
  component refactor is in scope).

## 2026-05-19 - Implementation landed (pre-commit)

The indexing chart in `EmptyPaneCarousel.svelte` is a static SVG
hierarchy (no force simulation, no Canvas), so pulling in
`GraphCanvas`'s full machinery would have been overkill. Added a
local pan + zoom layer:

* `chartTransform: { tx, ty, scale }` — SVG-space transform
  applied to a wrapper `<g>` that contains both the edges and
  nodes groups (so panning keeps them aligned).
* Pointer drag → updates `tx, ty`. Pointer capture is taken on
  the SVG element. Pointerdown on a `.node` bails out before
  setting `panStart` so the existing single-click selection
  still works.
* Wheel → updates `scale` (clamped to [0.5, 6]) using the same
  `exp(-deltaY * 0.0015)` smoothing as `GraphCanvas` so the two
  views feel the same. Zoom anchors the world point under the
  cursor across the transform change.
* Recenter affordance: a small `Locate` icon button pinned to
  the chart's bottom-right corner that resets the transform
  to identity.
* `slideIndex` watcher: when the carousel rotates away from
  slide 2 (indexing graph), the transform resets so the next
  return-to-slide visit lands on a fitted view instead of
  picking up wherever the user left it.
* CSS: `touch-action: none` on the chart so touch devices don't
  fight the gesture; `cursor: grab` → `grabbing` while panning.

Coordination-note compliance: the gesture wiring sits on the
`<svg>` element itself, not on the slide container, so the Round-2
Infographics-tabs refactor (backlog item 4) can move the chart
without losing the behaviour.

While I was in `shortcuts.test.ts` I picked up @@FullStackA's
in-flight `fullstack-a-7` chord swap (Hybrid NAV: Mod+K -> Mod+.)
that had left the "advertises Hybrid NAV (Cmd+K)" test stranded.
Trivial single-line label update; called out here so the audit
trail attributes it correctly. Also resynced
`crates/chan/src/main.rs::SERVE_LONG_ABOUT` so `chan serve --help`
matches the new chord.

Files changed:

* `web/src/components/EmptyPaneCarousel.svelte`:
  - Imported `Locate` icon.
  - Added `chartTransform`, `panStart` (both `$state` so the
    `class:panning` binding flips reactively), `chartSvg`
    ref, plus `recenterChart`, `chartLocalCoords`,
    `onChartPointerDown`, `onChartPointerMove`,
    `onChartPointerUp`, and `onChartWheel`.
  - Wrapped the edges + nodes groups in a transform-driven `<g>`.
  - Added the recenter button.
  - Reset effect for the `slideIndex !== 2` case.
  - CSS additions for the panning cursor and the recenter
    button.
* `web/src/components/EmptyPaneCarousel.indexingChart.test.ts`
  (new): eight pinned-source assertions covering transform
  state, recenter, the slide-leave reset, pointer/wheel
  wiring, the cursor-anchored zoom math, the recenter button,
  and the transform group wrapping both edges + nodes.
* `web/src/state/shortcuts.test.ts`: the stranded "Hybrid NAV
  (Cmd+K)" test's chord updated to Cmd+. so the suite passes
  alongside @@FullStackA's in-flight `fullstack-a-7`.
* `crates/chan/src/main.rs::SERVE_LONG_ABOUT`: Hybrid NAV
  chord resync (Cmd+. instead of Cmd+K).

Acceptance criteria status:

| Criterion                                          | Status |
|----------------------------------------------------|--------|
| Drag-to-pan on the indexing chart                  | done   |
| Wheel-zoom in/out                                  | done   |
| Recenter affordance                                | done   |
| Visual parity with main Graph view gestures        | done [^1]|
| No regression in cycle / DnD                       | done [^2]|

[^1]: Same exp-smooth wheel curve + cursor-anchored zoom + drag
      math as `GraphCanvas.svelte::onWheel` and the
      pointerdown/move/up gestures.
[^2]: Pointer capture takes the gesture exclusively while
      panning; wheel `stopPropagation`s so the carousel's
      auto-rotate doesn't pick up the wheel as a swipe.
      Hover-pause is unaffected (the SVG bubbles `pointerover`
      to the carousel root).

Gate status:

* `cargo fmt --check` — clean.
* `cargo clippy --all-targets -- -D warnings` (RUSTFLAGS=-D
  warnings) — clean.
* `cargo test --all-targets` — green.
* `cargo build --no-default-features` (RUSTFLAGS=-D warnings) —
  green.
* `npm run check` — 0 errors, 0 warnings (warning about non-
  reactive `panStart` resolved by promoting it to `$state`).
* `npm run build` (vite) — green.
* `npx vitest run` — 464/464 green.

WebtestB walkthrough plan:

1. Open the empty pane carousel (lone-pane empty workspace).
2. Wait for slide 3 (Indexing) to surface, or use the dots to
   navigate there.
3. Drag-pan the chart — nodes + edges slide together.
4. Wheel-zoom in / out — the point under the cursor stays
   anchored; the zoom is smooth on both trackpad and mouse.
5. Press the Locate button → transform resets.
6. Rotate to a different slide → transform resets when slide 3
   comes back around.
7. Confirm clicking a node still toggles selection (no pan-
   gesture interception).

Held for commit clearance from @@Architect. Picking up
`fullstack-b-6` next (scope FB watcher — promoted from Round 2
backlog).

## 2026-05-19 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Local pan + zoom on the SVG was the right scope; not lifting in
`GraphCanvas`'s force-simulation machinery just to get gestures
is the lighter, more aligned shape. Same `exp(-deltaY * 0.0015)`
wheel smoothing as `GraphCanvas` keeps the two views feeling
consistent — that's the visual parity the spec asked for. The
gesture sitting on the `<svg>` (not the slide container)
preserves the behaviour through the Round-2 Infographics-tabs
refactor — exact coordination-note compliance.

Picking up @@FullStackA's `fullstack-a-7` stranded shortcuts
test + the `SERVE_LONG_ABOUT` resync was a small but
thoughtful catch — avoids a lane-collision regression. Audit
trail correctly attributes it here.

**Commit clearance**: approved. Suggested subject:

```
Indexing chart: drag-pan + wheel-zoom + recenter, parity with Graph view (fullstack-b-4)
```

Also clear to commit the stranded `shortcuts.test.ts` +
`SERVE_LONG_ABOUT` resync as part of this commit (single
landing for the chord-related drift).

Push waits for Round-1 close. Pick up `fullstack-b-6` next
(scope FB watcher).
