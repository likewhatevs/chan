# fullstack-44: carousel cycle/stop toggle

Owner: @@FullStackB
Cut by: @@Architect
Date: 2026-05-19

## Goal

The empty-pane carousel (from `fullstack-35`) auto-
rotates every 5 seconds. Add a toggle so @@Alex can
**stop / resume the cycle** without leaving the
welcome state. Default = cycling (current behavior).

## Relevant links

* @@Alex's chat note 2026-05-19 12:45 BST.
* Predecessor: [./fullstack-35.md](../fullstack-b/fullstack-35.md).

## Acceptance criteria

* Subtle toggle affordance on the carousel chrome
  (e.g. a small play / pause icon next to the slide
  indicators).
* Click toggles between **cycling** (auto-rotate
  5s) and **stopped** (current slide stays).
* When stopped, the slide indicator + arrow
  affordances still let the user navigate manually.
* When restarting cycle, it picks up from the
  currently-displayed slide (not from slide 1).
* Pointer-hover-pause behavior from `fullstack-35`
  stays — that's a transient pause, separate from
  the explicit stop.
* Setting persists per-user via the existing
  preferences endpoint (so a user who always wants
  it stopped doesn't have to click every time).

## Out of scope

* Per-pane state (it's a per-user preference).
* Custom rotation intervals (5s stays).

## How to start

1. `EmptyPaneCarousel.svelte` — add the toggle button
   + state.
2. Wire a new persisted preference like
   `empty_pane_carousel_cycling: bool` (default
   true) alongside the existing
   `bubble_overlay_mode` preference.
3. When `cycling` is false, skip the 5s setInterval
   tick; manual navigation only.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-b-architect.md`.
