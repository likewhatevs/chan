# fullstack-55: drop the dashboard-stats row from the carousel welcome slide

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Goal

Remove the file / directory count row that sits
under the Chan logo on slide 1 (Welcome) of the
empty-pane carousel. @@Alex flagged it as no longer
needed: too noisy under the brand mark, and the
information lives more naturally on slide 2
(metadata) anyway.

## Relevant code

* `web/src/components/EmptyPaneCarousel.svelte:457-469`
  — the `<div class="dashboard-stats">` block
  inside `dashboard-header` on slide 1. Currently
  renders:
  * `{driveSummary.files} files`
  * `{driveSummary.folders} directories`
  * (conditional) `{driveSummary.contacts} contacts`
  * (conditional) `{indexLabel}`
  All four are inline with `·` separators.
* `web/src/components/EmptyPaneCarousel.svelte:456`
  — `<div class="dashboard-name">` (drive name)
  stays. The label is fine; only the stats row
  goes.
* `web/src/components/EmptyPaneCarousel.svelte:60-95`
  — `driveSummary` derived. If nothing else
  consumes it after the stats row is gone, the
  derived can be deleted too (audit usages — slide
  2 / 3 may reuse parts).
* `indexLabel` derived (likely nearby) — same
  audit; only delete if unreferenced after the
  drop.

## Acceptance criteria

* The `<div class="dashboard-stats">` block no
  longer renders on slide 1.
* `dashboard-name` (drive name) still renders
  directly under the logo.
* `.dashboard-stats` CSS rule removed if no other
  surface references it. If something else does
  (unlikely), leave the rule alone.
* `driveSummary` derived: removed only if no
  remaining consumers in the carousel or elsewhere
  in `web/src/`. Same for `indexLabel`. Note any
  retained consumers in the implementation note.
* No empty `dashboard-header` rendering an orphan
  block if `drive.info` is set but `dashboard-name`
  is the only child — verify by eyeball that the
  header sits flush under the logo without extra
  padding.

### Tests

* `web/src/components/EmptyPaneCarousel.test.ts`
  exists; flip / drop any assertion that the stats
  row renders. Add a fresh assertion that "X files"
  and "Y directories" labels are NOT present on
  slide 1's rendered DOM (so a future regression
  yells).

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* Affects `webtest-b-6` item 1 (carousel
  walkthrough). The "drive summary on slide 1"
  expectation in the walk should flip to "drive
  name only on slide 1, no stats row". Lane B can
  re-walk that single item once this lands;
  in-flight no-amendments rule means I'll patch
  the walkthrough task as a separate cut if Lane B
  has already started item 1.
* Visual eyeball worth doing — ad-hoc `chan serve`
  + Chrome MCP, open an empty pane, confirm slide 1
  renders cleanly without the stats line and
  without orphan padding. Teardown after.
* Standing topic-level commit clearance.

## 2026-05-19 16:30 BST — @@FullStackA implementation note

Implementation:

* `EmptyPaneCarousel.svelte`: dropped the
  `<div class="dashboard-stats">` block on slide 1
  (lines 457-469); the `dashboard-name` line
  beneath the brand mark stays. The `driveSummary`
  derived (lines 63-75 in pre-edit) was the only
  thing the dropped block consumed and had no
  other call sites in `web/src/`, so it goes too.
  `indexLabel` stays — slide 3 (indexing) still
  consumes it on lines 529-530.
* `.dashboard-stats` + `.dashboard-stats .sep` +
  `.dashboard-index` CSS rules removed (no other
  surface referenced them; grep clean).
* `EmptyPaneCarousel.test.ts`: appended a fresh
  regression block under the welcome-slide test —
  asserts `.dashboard-stats` is absent, and that
  the slide DOM contains neither `\d+ files` nor
  `\d+ directories` text.

Gate green:

* `npm run test -- EmptyPaneCarousel` (4 passed),
* `npm run test` (343 passed),
* `npm run check` (0 errors / 0 warnings — one
  transient blip on a Lane-B in-progress edit
  cleared on re-run),
* `npm run build`,
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`
  (green).

Visual eyeball deferred to webtest. Webtest-b-6
item 1 expectation flips from "drive summary on
slide 1" to "drive name only on slide 1, no stats
row" per the task's note about the in-flight
amendment rule.

Proposed commit message:

> Drop carousel dashboard-stats row (fullstack-55)
>
> Remove the inline files / directories / contacts /
> index-label row from EmptyPaneCarousel's welcome
> slide; only `dashboard-name` survives under the
> brand mark. Slide 2 (metadata) owns the per-kind
> tallies. `driveSummary` derived + the dashboard-
> stats CSS rules dropped along with the block.
> Regression assertion appended to the carousel
> test so the row can't sneak back in.
