# fullstack-85: empty-pane border thickness matches top-bar thickness

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex flagged a CSS inconsistency on empty
panes: the focus border around the body
(left / right / bottom) is thicker than the
border around the top-bar chrome (where the
theme toggle + kebab live). Should be
consistent — the **thin** border everywhere,
including the body.

Visual evidence in @@Alex's screenshot: thick
pink border around the empty-pane body, thin
pink border along the top bar.

## Relevant code

* `web/src/components/Pane.svelte` — pane
  focus border CSS. The `data-focus-color`
  attribute (per the earlier work) drives
  the focus border colour; thickness is
  likely a CSS rule on `.pane` or
  `.pane.empty` (or wherever the empty-pane
  branch lives).
* Empty-pane render branch — find the
  styles that apply to the empty-pane body
  specifically vs. the always-present
  top-bar chrome.

## Acceptance criteria

* Empty pane: border thickness on the body
  (left / right / bottom) matches the
  thickness on the top bar. Both are the
  thin variant.
* Non-empty pane (terminal / Files / doc /
  Graph): no regression. Focus border
  thickness stays at the existing thin
  value.
* All focus-color variants (blue / green /
  pink) flip cleanly.

### Tests

* Component test (or source-grep sentinel):
  the CSS rule for the empty-pane body
  border uses the same width value as the
  top-bar rule.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* Trivial CSS fix. Few-minute ship.
* v0.11.0-blocking-soft.
* Visual eyeball worth doing — quick
  ad-hoc chan serve, empty a pane, confirm
  both borders match thickness across all
  three focus colours.
* Queue position: end of Lane A queue.
  Updated queue: `-83` → `-85` (assuming
  `-81` shipping per the 19:03 impl note).
* Standing topic-level commit clearance.

## 2026-05-19 19:23 BST — @@FullStackA implementation note

Root cause: not a thickness mismatch in
`Pane.svelte`, but a **double-ring stack**.

* `Pane.svelte:.pane.focused` paints a 2px
  inset focus glow around the entire pane
  (multi-pane only, gated on `isFocused`).
* `EmptyPaneCarousel.svelte:.carousel:focus-
  visible` was painting ANOTHER 2px inset
  ring around the carousel body when
  keyboard-focused.

When both fire (multi-pane empty pane with
the carousel focused) the body shows two
stacked rings → reads ~2× thicker than the
top-bar chrome, which only has the pane-
level ring.

Fix: drop the `.carousel:focus-visible`
inset glow entirely. The pane-level focus
indicator already covers the multi-pane
focused case; single-pane case keeps no
focus indicator (matching the rest of
single-pane chrome — only one pane to be
focused). Comment block left in place of
the dropped rule documents the rationale
so a future contributor doesn't reflexively
add it back.

Files:

* `web/src/components/EmptyPaneCarousel.svelte`
  — dropped `.carousel:focus-visible { box-
  shadow: inset 0 0 0 2px ... }` rule;
  replaced with a comment explaining why.
* `web/src/components/EmptyPaneCarousel.test.ts`
  — added a regression source-grep:
  asserts the rule is gone.

Acceptance:

* Empty pane: top bar + body now show the
  same 2px inset focus border from the
  pane-level rule, no double-ring on the
  body.
* Non-empty panes: unaffected (no carousel
  → no extra ring).
* All focus-color variants (blue / green /
  pink) flip cleanly — they're driven by
  the same `--pane-active-focus` CSS
  variable.

Gate green:

* `npm run test -- EmptyPaneCarousel`
  (5 passed — the new regression test +
  the existing 4),
* `npm run test` (434 passed — Lane B and
  Lane A both added tests in parallel),
* `npm run check` (0 errors / 0 warnings),
* `npm run build`,
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`
  (green).

Visual eyeball worth doing: split a pane,
empty one side, focus it — body + top bar
borders should look the same thickness. Try
the green and pink focus colours via the
pane hamburger to confirm the cure isn't
colour-specific.

Proposed commit message:

> Drop empty-pane carousel inset focus ring (fullstack-85)
>
> The empty-pane carousel painted its own 2px
> inset focus ring on top of `.pane.focused`'s
> 2px ring, making the empty-pane body look like
> it had a thicker border than the top-bar
> chrome. The pane-level rule already covers the
> multi-pane focused case; drop the carousel's
> own ring so the body and top bar render with
> the same thin variant. Single-pane empty
> carousels keep no focus indicator (matches the
> rest of single-pane chrome). Regression test
> asserts the rule stays gone.
