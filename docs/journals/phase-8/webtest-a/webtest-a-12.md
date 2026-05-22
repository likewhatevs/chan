# webtest-a-12 — -a-67 slice 1 visual: Graph scope-path header row

Owner: @@WebtestA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Visual check for `-a-67a` (the first slice of the
right-click menu revamp — Graph hamburger scope-path
header row). Light walk.

## Reference

* `-a-67` task body + slice 1 tail describing the
  shape.
* Commit `af65ebc`.

## Acceptance

1. **Header row renders at top of graph tab menu**:
   open graph; right-click / hamburger to open the
   tab menu; confirm a scope-path header row appears
   ABOVE the depth slider.
2. **Icon matches scope kind**: drive scope → drive
   icon; folder scope → folder icon; file scope →
   file icon. Walk through 3 scope kinds.
3. **Path fades on overflow**: open graph scoped to
   a deeply nested path; confirm the path text fades
   at the right edge (no 2-line wrap).
4. **Separator below header**: confirm a separator
   line between the header row and the depth row.
5. **No click-to-inspector yet** (this is slice 1 —
   display-only). Spec'd in slice 1b; flag if you
   observe a click-handler.

### Walkthrough audit trail

Append to [`webtest-a-1.md`](webtest-a-1.md):
`## 2026-05-22 — fullstack-a-67a graph scope-path header row walk`.

## How to start

1. Confirm `af65ebc` in HEAD.
2. Rebuild chan; spin up test server + seed.
3. Walk checks 1-5.
4. Append verdict; tear down.

## Coordination

* @@WebtestA lane.
* Light walk; ~15 min.

## Numbering

This is `-12`.

## Out of scope

* `-a-67b` click-to-inspector wiring (slice 1b
  pending @@FullStackA pickup).
* `-a-67c`+ remaining surfaces (Hybrid / Terminal /
  FB / Editor revamps).
* `-a-64` + `-a-65` (covered by `webtest-a-11`).
