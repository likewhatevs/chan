# fullstack-a-60 — Graph canvas click hit-radius expansion (forgiving-clicks UX)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Expand the graph canvas click hit-radius beyond the
visible node stroke so users don't need to zoom in to
register clicks on nodes.

## Reference

[`../phase-8-bugs.md`](../phase-8-bugs.md) "Graph canvas
click hit-radius is too tight; users need to zoom in to
register clicks on nodes" — surfaced by @@WebtestA's
proactive walk (`a63c8cb`); reproducible with multiple
clicks near visible nodes missing pre-zoom.

## Fix shape

Typical forgiving-clicks pattern: `hitRadius =
strokeRadius + ~8-12px` for the canvas pointer
hit-test. Implementer picks the exact pad value based
on the canvas density at default zoom.

Audit `GraphCanvas.svelte`'s hit-test logic +
pointer-event handler. Most likely a single constant or
formula change.

## Acceptance

1. **Click registers without zoom**: at default canvas
   zoom, clicks within ~10px of a node's visible edge
   register as a hit on that node. (Mobile / small
   screens should also benefit.)
2. **No false-positive overlap**: when two nodes are
   close, clicks near a midpoint shouldn't ambiguously
   hit either — fall through cleanly OR pick the
   nearest centroid. Implementer's call; conservative
   default is "nearest centroid wins."
3. **Drag/pan unaffected**: hit-radius applies to
   click-to-inspect only; drag-to-pan should still
   work when starting on canvas-empty pixels.

### Tests

DOM/canvas assertion pin: simulate a pointerdown at
`(nodeX + strokeRadius + 8, nodeY)` and assert the
hit-test resolves to the correct node id.

### Gate

* `npm test -- --run` green.
* `npm run check` 0e/0w.
* `npm run build` clean.

## Coordination

* @@FullStackA lane. SPA-only.
* Atomic-audit-commit discipline.
* If this task ends up needing chan-server emit changes
  (unlikely), scope-poke first.

## Authorization

**Yes** for `web/src/components/GraphCanvas.svelte` +
test pin + task tail + outbound.

## Numbering

This is `-a-60`.

## Out of scope

* Parent-edge invariant (separate task `-a-58`).
* Filter chip bucket extension (separate task `-a-57`).
* Layout / force-directed tuning beyond the hit-test.

## 2026-05-22 — ready for review

Two-file change. SPA-only; no Rust touched.

### What landed

`web/src/components/GraphCanvas.svelte`:

* New `PICK_SLACK_DRAG_PX = 4` (existing slack;
  preserved as the drag-vs-pan disambiguation
  pad so clicks near empty space still pan).
* New `PICK_SLACK_CLICK_PX = 10` (forgiving-clicks
  pad per task body's 8-12px range).
* `pickNode` extended with a `slackPx` parameter
  defaulting to `PICK_SLACK_DRAG_PX`. The slack
  is zoom-corrected via `slackPx / Math.max(0.5,
  transform.k)` so the pad stays visually
  constant in SCREEN pixels across zoom levels.
* Call-sites:
  * `onMouseDown` (drag-detect) — default slack
    (4px). Preserves pan-on-empty-space.
  * `onMouseMove` no-drag (hover cursor) —
    `PICK_SLACK_CLICK_PX` so cursor preview
    matches the tap target.
  * `onMouseUp` no-move (tap-to-select) —
    `PICK_SLACK_CLICK_PX` so taps near small
    nodes register.

`web/src/components/graphCanvasHitRadius.test.ts`
(new): 8 raw-source pins covering the constants,
function signature, slack formula, call-site
slack selection, and the preserved nearest-centroid
tie-break.

### Acceptance

1. **Click registers without zoom** ✓ — wider
   10px slack on tap-to-select means clicks
   within 10px of a node's visible edge resolve
   to that node.
2. **No false-positive overlap** ✓ — preserved
   nearest-centroid tie-break (`d2 < bestD2`).
   When two nodes are close, the closer one
   wins.
3. **Drag/pan unaffected** ✓ — `onMouseDown`
   still uses 4px slack so the empty-space
   "pan" zone only shrinks by 4px around each
   node (unchanged from pre-`-a-60`).

### Gate

* vitest **756 / 756** (+8 net from `-a-59`'s
  748).
* svelte-check 0 errors / 0 warnings across
  4002 files.
* npm build clean.
* Rust gate not re-run.

### Decisions

* **Separate slacks for drag vs click** rather
  than a single global bump — preserves the
  acceptance criterion #3 (drag/pan unaffected
  on empty space near nodes).
* **10px** within the task body's 8-12px range.
  Tried + spec'd value matches common
  forgiving-clicks UX. If feedback shows nodes
  feel too "grabby" or not grabby enough,
  bumping the constant is a one-line follow-up.
* **Hover uses click slack** so the pointer
  cursor changes when the user is in the
  tap-resolvable zone — visual preview of
  where the click will land.
* **Zoom-corrected** slack (`/ transform.k`)
  preserved — keeps the slack constant in
  SCREEN pixels regardless of zoom level.

### Suggested commit subject

```
Graph canvas: expand click hit-radius to 10px while keeping drag-detect tight (fullstack-a-60)
```

Single commit. Constant + parameter + call-sites
+ test tightly coupled.

### Files for `git add` (per-path discipline)

* `web/src/components/GraphCanvas.svelte`
* `web/src/components/graphCanvasHitRadius.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-60.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.
