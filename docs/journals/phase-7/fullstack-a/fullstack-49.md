# fullstack-49: right-docked file browser chevron direction

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Goal

`fullstack-38` mirrored the right-docked file
browser's row layout (chevron at the rightmost edge,
icon next to it, text right-aligned) but the chevron
glyph still points right (`>`) for collapsed
directories. When the dock is on the right, the
visual model is "children open inward" (toward the
left), so the chevron should mirror: point LEFT for
collapsed, down for expanded (down stays — symmetric
on both axes).

@@Alex screenshot 2026-05-19 13:30 BST confirms the
chevron is still right-facing in the right dock.

## Relevant links

* @@Alex's screenshot + chat note 2026-05-19 13:30 BST.
* Predecessor: [./fullstack-38.md](../fullstack-a/fullstack-38.md).

## Acceptance criteria

* Right-docked file browser:
  * Collapsed dir chevron: `<` (left-facing).
  * Expanded dir chevron: `v` / `⌄` (down, unchanged
    — symmetric on the horizontal axis).
* Left-docked + overlay + first-class tab variants:
  unchanged. Collapsed chevron stays `>`.
* The chevron toggles correctly on click (collapse /
  expand) — orientation is purely visual.
* Add a regression test asserting the chevron glyph
  differs between left-dock and right-dock for the
  collapsed state.

## Out of scope

* Chevron animation on expand/collapse.
* Other directional glyphs (icons next to file names
  stay symmetric).

## How to start

* `FileTree.svelte` — find the chevron render. It's
  likely a single `<svg>` or icon component. Either
  swap the icon under `:global(.tree.right-dock
  .row.dir .chevron)` or apply `transform:
  scaleX(-1)` to the existing chevron in the right-
  dock CSS class.
* Verify the click handler stays untouched.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-a-architect.md`.

## 2026-05-19 14:32 BST — @@FullStackA implementation note

Small change: `FileTree.svelte` now imports `ChevronLeft`
alongside `ChevronDown`/`ChevronRight` and the collapsed-dir
chevron branches on `rightDock`. Left-dock + overlay + tab
variants keep `ChevronRight`; right-dock uses `ChevronLeft`.
Expanded chevron stays `ChevronDown` (symmetric on the
horizontal axis). Click handler is untouched — orientation
is purely visual.

Files touched:

* `web/src/components/FileTree.svelte` — `ChevronLeft` import
  + the `{:else if rightDock}` branch in the collapsed-dir
  render block.
* `web/src/components/revealBrowserActions.test.ts` — two
  regression asserts: the icon imports and the branch
  ordering inside the `{#if expanded}` block.

Gate green:

* `npm run test -- revealBrowserActions` (10 passed),
* `npm run test` (338 passed),
* `npm run check`, `npm run build`,
* `bash -lc 'ulimit -n 4096; scripts/pre-push'` (green).

Proposed commit message:

> Flip right-docked file browser chevron direction (fullstack-49)
>
> Collapsed-directory chevron in the right-docked file browser
> now mirrors with the row layout: ChevronLeft instead of
> ChevronRight, since the mirrored tree reads children as
> "opening inward" toward the editor pane on the left. Expanded
> chevron stays ChevronDown (already symmetric on the horizontal
> axis). Left-dock + overlay + first-class tab variants
> unchanged. Click handler untouched — orientation is purely
> visual.
