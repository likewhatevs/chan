# fullstack-a-71 — Auto-scroll cursor-lost when editing list near bottom with image around

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Fix the cursor-lost-on-auto-scroll bug from
[`../alex/addendun-a.md`](../alex/addendun-a.md):

> While writing this very file, on a list of items
> that is at the bottom of the screen while an image
> is around, the editor scrolls down automatically to
> make space/show the image but the cursor gets lost
> and is confusing for the user

@@Alex repros by editing the addendum itself —
list-at-bottom-of-screen + image nearby → editor
auto-scrolls to make room for the image render → user's
cursor visually disappears.

## Reference

* `addendun-a.md` "## Bugs" — auto-scroll item.
* Repro: edit a markdown file with an image near a
  list at the bottom of the viewport.

## Audit hooks

1. Find the auto-scroll trigger — likely an image-
   rendered side-effect in `web/src/editor/markdown/`
   (image decoration / extension).
2. The cursor goes off-screen because the scroll happens
   without compensating for the cursor's viewport
   position.

## Fix shape (likely)

Two options to investigate:

* **(A)** Don't auto-scroll when the cursor is near
  the scrolled region — let the user see the image
  in the layout as-is, even if it pushes content
  partially off.
* **(B)** Compensate the scroll target so the cursor
  stays visible: after the image render, ensure the
  cursor's viewport position is within bounds (scroll
  to keep cursor visible).

Recommend (B): preserve cursor visibility. The image
still renders correctly; user's editing context isn't
disrupted.

Implementer's call after audit.

## Acceptance

1. **Repro doesn't lose cursor**: edit a markdown file
   with a list at the bottom of the viewport + image
   nearby; type/edit the list; image renders +
   auto-scroll fires; cursor stays VISIBLE within
   viewport.
2. **No regression on image rendering**: images still
   display correctly; layout integrity preserved.

### Tests

Vitest pin on the scroll-after-image-render handler
preserving cursor position OR DOM/visual assertion
that cursor's bounding-rect stays within editor
viewport.

### Gate

* `npm test -- --run`, `npm run check`, `npm run build`
  green.

## Coordination

* @@FullStackA SPA primary.
* Atomic-audit-commit.

## Authorization

Yes for editor image / scroll handling SPA files +
tests + task tail + outbound.

## Numbering

This is `-a-71`.
