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

## 2026-05-22 — ready for review

One-file change (+1 test). SPA-only.

### Audit verdict

Root cause located at
`web/src/editor/widgets/image.ts:284` — the
image-load handler's gate
`if (Math.abs(headLine - imgLine) > 1) return;`.

The original `-a-71`-adjacent design (per the
comment block at line 270-274) was:

> Re-anchor the scroll once the image lands,
> but only when the caret is on or next to
> THIS image's source line — anywhere else
> means the user is editing elsewhere while a
> distant image streams in, and re-scrolling
> would fight their deliberate position.

The assumption: distant-image-load doesn't
disturb a user editing far away. The
assumption is WRONG when a tall image renders
ABOVE the caret: layout shifts down → caret
moves off-screen → user has lost visibility
even though they haven't touched anything.

@@Alex's repro pattern: list-at-bottom +
image-above → image renders → list pushed
down → caret vanishes from viewport. Exactly
the scenario the gate prematurely guards
against.

### Fix shape

Drop the `headline-distance` gate; rely on
the existing viewport-visibility gate
(`if (cb.top >= sb.top && cb.bottom <= sb.bottom) return;`)
to preserve the "deliberate position" safeguard:

* Caret still visible after image load → no
  dispatch (user undisturbed).
* Caret off-screen after image load →
  scrollIntoView with `nearest` to bring it
  back.

The viewport-check naturally handles both
cases:

* User editing line 1000, image streams at
  line 5: caret stays visible → no scroll.
* User editing list at bottom, image renders
  above + pushes layout down: caret pushed
  off → scroll restores.

### What landed

`web/src/editor/widgets/image.ts`:
* Removed the `Math.abs(headLine - imgLine) > 1`
  early-return.
* New comment block documents the
  layout-shift cause + the viewport-check's
  role as the actual gate.

`web/src/editor/widgets/imageScrollCaretLost.test.ts`
(new): 4 raw-source pins covering the gate
removal, the viewport-check preservation, the
scrollIntoView dispatch, and the rationale
comment.

### Acceptance

1. **Repro doesn't lose cursor**: image above
   caret line + tall layout shift → caret
   visibility restored via the now-unguarded
   scroll dispatch ✓ (mechanism via tests;
   @@WebtestA walk for empirical repro).
2. **No regression on image rendering**:
   image still loads + decorates normally;
   only the post-load scroll behavior
   changed ✓.
3. **No regression on "deliberate position"
   safeguard**: viewport-check still
   short-circuits when caret is on-screen ✓.

### Gate

* vitest **829 / 829** (+4 net from `-a-66`'s
  825). 3 flaky timeouts on first run
  (EmptyPaneCarousel / Pane / TerminalTab
  tests) cleared on re-run; known pattern.
* svelte-check 0 errors / 0 warnings across
  4012 files.
* npm build clean.
* Rust gate not re-run.

### Decisions

* **Drop gate vs add a second branch** —
  the original gate had a "defense" purpose
  (don't disturb far-from-image users), but
  the viewport-check ALREADY provides that
  defense (returns early when caret visible).
  Dropping the redundant gate is cleaner
  than adding an OR clause + preserves the
  same end-state behavior for unaffected
  cases.
* **Comment rewrite** — important for the
  next reader; the original comment's
  framing was load-bearing for the
  too-restrictive gate. Replaced with the
  layout-shift framing so future debugging
  finds the right mental model.

### Suggested commit subject

```
Editor image-load scroll: drop distance gate so off-viewport caret is always restored (fullstack-a-71)
```

Single commit. One-line code change + comment
+ test pin tightly coupled.

### Files for `git add` (per-path discipline)

* `web/src/editor/widgets/image.ts`
* `web/src/editor/widgets/imageScrollCaretLost.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-71.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.
