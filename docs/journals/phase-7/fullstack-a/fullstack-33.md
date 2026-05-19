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

## 2026-05-19 10:22 BST — @@FullStackA specialist review

### Root cause

The guide rendering was hard-coded depth-1-through-6:

* `web/src/editor/decorations/blocks.ts:listDepthClass` clamped
  the depth integer at 6 (`Math.min(6, ...)`).
* `web/src/editor/Wysiwyg.svelte` had per-depth CSS rules,
  `cm-md-list-depth-1::before` through `cm-md-list-depth-6::before`,
  each stamping N box-shadow guides at 2ch intervals.

Past depth 6 the class collapsed to `cm-md-list-depth-6`, so:

1. The padding-prefix stayed at 14ch even when actual indentation
   was 16+ spaces, pulling the bullet column out of alignment with
   the text-indent / padding compensation.
2. Only six guide stamps were ever rendered, so deeper nesting
   showed the wrong number of bars.

### Fix

* Generalised guide rendering by reading
  `--cm-md-list-depth` (inline style set per-line) and emitting:
  * `--cm-md-list-prefix: calc((var(--cm-md-list-depth, 0) + 1) * 2ch)`
    so the padding scales with depth at any level.
  * `::before` width + `repeating-linear-gradient` paint a 1px
    stripe per indent level — depth-agnostic, no per-depth selector
    to maintain.
* `blocks.ts`: new `listDepth(text)` returns the integer depth,
  soft-capped at 20 to keep the decoration cache bounded and the
  guide width sane against pathological input. `listDepthClass`
  still returns `cm-md-list-depth-N` for grep / CSS-hook
  compatibility. `listLineDecoration` now attaches the inline
  `--cm-md-list-depth: N` style.

### Tests

* `web/src/editor/decorations/blocks.test.ts` — covers depths 0,
  1, 2, 7, 11 (smoke at the legacy cap and above), the 20-level
  soft cap on pathological indentation, and a sweep that asserts
  every depth 0..20 emits a distinct `cm-md-list-line-depth-N`
  class string (the 20-level smoke from the task's acceptance
  criteria).

### Manual verification

* Full pre-push gate green (fmt + clippy + npm test + svelte-
  check + npm build + no-default-features build).
* Visual check via a built doc with a 20-level nested bullet
  list shows N+1 vertical guides at every depth, bullet glyphs
  staying aligned to the rightmost guide column.

### Gate

* `npm run test -- blocks` — 9 passed (9 added/updated assertions).
* `npm run test` — 30 files / 271 tests, all pass.
* `npm run check` — 0 errors / 0 warnings.
* `npm run build` — clean.
* `bash -lc 'ulimit -n 4096; scripts/pre-push'` — green.

### Proposed commit message

> Render list indent guides at any depth (fullstack-33)
>
> The old box-shadow approach clamped depth at 6, so nesting past
> level 6 drifted: the padding prefix stuck at 14ch while text
> kept indenting, and only 6 guide stamps were ever painted.
> Switch to a CSS variable (`--cm-md-list-depth`, set inline by
> listLineDecoration) plus a repeating-linear-gradient that scales
> 1 stripe per level. Soft-cap depth at 20 so unbounded input
> can't blow up the cache or guide width.

Ready for commit + push under standing topic-level clearance.
