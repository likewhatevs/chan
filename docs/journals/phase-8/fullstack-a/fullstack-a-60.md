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
