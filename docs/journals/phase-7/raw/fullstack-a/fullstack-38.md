# fullstack-38: right-docked file browser mirrors text alignment

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Goal

The left-docked file browser pane (from `fullstack-1`)
aligns its tree text to the left edge. The right-docked
version should mirror that — text aligned to the RIGHT
edge, so the tree visually anchors against the closest
viewport edge. @@Alex flagged 2026-05-19 06:45 BST.

## Relevant links

* Predecessor: [./fullstack-1.md](../fullstack-a/fullstack-1.md)
  (docked side-pane File Browser).

## Acceptance criteria

* When the file-browser pane is docked on the **right**:
  * Text labels (file / dir names) render with
    right-aligned text.
  * Tree indent guides invert: indentation grows from
    the right toward the left, matching how the
    left-docked pane grows from left toward the right.
  * **File / folder icons** sit on the right side of
    the row (between the right edge and the text label).
  * **Expand / collapse arrows** (chevrons) sit at the
    rightmost edge of the row, mirroring how they sit
    at the leftmost edge in left-dock.
  * Row order, right-to-left: chevron → icon → text →
    (indent space).
* Left-docked behavior unchanged.
* Overlay (non-docked) mode behavior unchanged (it
  doesn't have a "side" — uses the default left
  alignment).
* If the user toggles between left/right dock mid-
  session, the alignment updates immediately.

## Out of scope

* The first-class File Browser tab (`fullstack-14`) —
  that's a tab, not a docked pane, alignment stays
  default.
* Drag-to-resize behavior.
* The Graph pane (no docked-graph version exists).

## How to start

1. The docked file browser component lives in
   `web/src/components/` (likely `FileTree.svelte`
   or a sibling). Locate the dock-side prop /
   detection.
2. Branch the row layout CSS on dock side. A flex-
   direction flip + `text-align: right` is probably
   the smallest change.
3. Indent guides: invert the offset direction when
   right-docked.

## Hand-off

Standard. Pre-push gate green. Coordinate with
@@WebtestA for the dock-flip walkthrough. Ping via
`alex/event-fullstack-a-architect.md`.

## 2026-05-19 10:39 BST — @@FullStackA specialist review

### Plumbing

* `web/src/components/FileBrowserSurface.svelte` — passes
  `dockSide={variant === "dock" ? side : undefined}` to the
  inner `<FileTree>`. Overlay and first-class tab variants
  keep the default left-aligned layout (no dock side).
* `web/src/components/FileTree.svelte` — adds the `dockSide`
  prop. A `$derived` `rightDock` flag drives both the inline
  padding swap and the `class:right-dock` class hook on the
  root `<ul>`.

### Row layout flip

* Dir, file, and child-empty rows now render
  `padding-right: <indent>px` instead of `padding-left:
  <indent>px` when right-docked. The 16 px chevron-column
  offset on file rows mirrors with the same flip.
* CSS in `FileTree.svelte`:
  * `.tree.right-dock .row { flex-direction: row-reverse; }`
    flips chevron / icon / name visual order; the chevron
    rides the row's right edge again, icon next-inward,
    text left of that.
  * `.tree.right-dock .name { text-align: right; }` so
    long names anchor on the right edge.
  * `.tree.right-dock .empty { text-align: right; }` for
    the loading / empty / error rows.
  * `.tree.right-dock .row.dir .dir-icon` swaps
    `margin-right: 2px` → `margin-left: 2px` so the
    chevron-to-icon gap still lands on the correct visual
    side after row-reverse.
  * `.tree.right-dock .dirty-dot` similarly swaps
    `margin-left: 4px` → `margin-right: 4px` so the dot
    sits visually trailing the name in reading order.

### What stays unchanged

* The first-class File Browser tab (`fullstack-14`) and the
  overlay variant inherit `dockSide=undefined` and render
  left-aligned as before.
* Drag-to-resize (`ResizeHandle` in
  `FileBrowserSidePane.svelte`) is unaffected.
* The Graph pane has no docked variant.

### Tests

* `web/src/components/revealBrowserActions.test.ts` — added
  a `right-docked file browser mirrors text alignment`
  describe with four raw-source assertions:
  1. FileBrowserSurface forwards the right `dockSide` value
     to FileTree.
  2. FileTree declares the prop and toggles the class.
  3. Both row variants render `padding-right` rather than
     `padding-left` under right-dock.
  4. The CSS contains the `row-reverse` + `text-align:
     right` rules.

Raw-source assertions match the existing pattern in this
test file. They guard the contract without spinning up a
full layout pass under jsdom (which can't measure flex
positions reliably).

### Gate

* `npm run test -- revealBrowserActions` — 8 passed (was 4;
  +4 new).
* `npm run test` — 32 files / 281 tests, all pass.
* `npm run check` — 0 errors / 0 warnings.
* `npm run build` — clean.
* `bash -lc 'ulimit -n 4096; scripts/pre-push'` — green.

### What needs a manual walkthrough

@@WebtestA should:

1. Pin the file browser to the right side.
2. Confirm row layout: chevron at the right edge, icon
   inboard of it, file name text right-aligned, indent
   column visibly on the right side (grows leftward as
   depth increases).
3. Toggle between left and right dock mid-session;
   alignment should swap immediately without a reload.
4. Left-docked + overlay + first-class tab variants
   unchanged.

### Proposed commit message

> Mirror file-browser row layout when docked on the right (fullstack-38)
>
> Pass dockSide through FileBrowserSurface to FileTree; the right
> variant flips row flex direction, right-aligns names, swaps the
> indent column to the right edge, and mirrors the dir-icon /
> dirty-dot margins. Overlay and first-class tab variants stay
> left-aligned.

Ready for commit + push under standing topic-level clearance.
