# fullstack-33: editor indent vertical guide breaks at deep nesting

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-19

## Goal

The vertical bullet-list indent guide
(`listGuideVisibility` extension, shipping since
phase-7 wave-1 work) misaligns at deeper nesting
levels. @@Alex captured a screenshot showing the
guides going sideways / off-anchor around indent
levels 5-7 in a real doc.

Smoke test up to 20 indents next time. The visual
contract is: each indent level shows a single vertical
guide line aligned to the bullet column at that
level, all the way down the visible doc region.

## Relevant links

* @@Alex's chat note + image 2026-05-19 05:30 BST.
* The extension lives in `web/src/editor/` somewhere
  near `breathing_room.ts` / list commands.

## Acceptance criteria

* Each indent level renders one vertical guide line
  anchored to that level's bullet column.
* No phantom guide lines, no horizontally-displaced
  guides at deeper indents.
* Smoke test that runs a 20-level deep nested bullet
  list:
  ```
  - level 1
    - level 2
      - level 3
      ...
                    - level 20
  ```
  All 20 guides render cleanly.
* Existing tests for shallow indents stay green.

## Out of scope

* Indent-rule changes (still 2 spaces per level).
* Other list rendering bugs (B1/B2/B13 already
  fixed in `fullstack-4`).
* Code-block / fenced indent guides.

## How to start

1. Locate the guide-rendering extension — likely
   `listGuideVisibility` or sibling decoration. The
   guide is probably a `Decoration.line` per indent
   level.
2. Look for off-by-one or width-vs-position
   miscalculations at deeper levels. Common bug:
   guide position computed in characters but rendered
   in pixels with a fixed per-level offset that drifts
   under non-monospace context (the editor IS monospace
   by default — but variables like padding can creep in).
3. Add the 20-level test fixture.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-architect.md`.
