# fullstack-a-75b — Carousel relocate to Infographics tab; welcome pane becomes static spawn grid

Owner: @@FullStackA
Cut: 2026-05-23 by @@Architect
Status: dispatched
Round: 2 wave-3 (follow-up)

## Goal

Per @@Alex's route on the `-a-75` walk: the
carousel widget moves from the welcome surface
(back of empty pane) into the Infographics tab.
The welcome pane becomes a static spawn grid only.

## Reference

@@Alex (`d4a3fc8`): "This is correct we will no
longer have the carousel in the back of the pane
and it will only live in the tab from now on."

@@WebtestA's walk + addendum (`7cc48a0` + `2dded48`)
flagged the UX gap: Infographics shipped as a static
page; carousel rotation lived on welcome. @@Alex's
direction inverts the placement.

## Scope

1. **Move carousel component** from EmptyPane /
   welcome surface to InfographicsTab body. Preserve
   rotation + play/pause + pagination UX end-to-end.
2. **Strip back-of-pane carousel**. Welcome surface
   reduces to:
   * 5-tile static spawn grid.
   * Footer hint ("Each pane's visible tab is part of
     the scope for Graph.").
3. **Infographics tab default slide order**:
   Shortcuts first; future slides appended.

## Acceptance

1. Empty pane: spawn grid + hint only; no carousel
   widget, no rotation, no play/pause.
2. Infographics tab opened from anywhere: hosts the
   full carousel widget with rotation + pagination +
   play/pause + slide ordering.
3. No regression on the spawn-tile click handlers
   (Cmd+T / Cmd+O / etc.).

### Tests

Vitest pins:
* Welcome surface markup contains spawn-grid +
  hint; no carousel component import.
* InfographicsTab body imports + mounts the
  carousel.
* Carousel rotation + play/pause behaviors
  preserved (existing pins migrate if needed).

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA. SPA-only.
* Autonomous-commit per batch dispatch standing
  auth.
* Atomic-audit-commit.

## Authorization

Yes for `web/src/components/EmptyPane.svelte` (or
welcome component) + `InfographicsTab.svelte` +
carousel component file move + tests + task tail +
outbound.

## Numbering

This is `-a-75 slice 2` (filename `-75b` for
clarity; slice 1 was the initial Carousel +
Infographics ship at `ba381f6`).

## Out of scope

* Re-styling the spawn tiles.
* Adding new slides to the carousel.
* Multi-pane carousel sync (if any was hypothetical).
