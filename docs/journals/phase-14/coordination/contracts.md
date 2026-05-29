# Phase 14 cross-lane contracts

The seams between Lane A (backend) and Lane B (frontend). These are not
designed yet: Lane A proposes, Lane B confirms, and the agreed shape is
pinned here before either side builds against it. Both lanes read this
file; a change to it is announced in the relevant `event-*` inbox.

## 1. Incremental graph delivery (A -> B)

Replaces the current whole-payload `/api/fs-graph`, `/api/graph`,
`/api/graph/languages`. To pin on kickoff:

- request kinds (both feed the same batch stream):
  - depth scope: scope (workspace root or a subdir) + depth `N`
    (`find -d N`), authoritative for the expanded set.
  - single-directory expand: fetch just one directory's next degree
    (the double-click expand), so B can grow one node in place without
    a whole reload.
- request also carries a cursor / page token for paging within a kind.
- response unit: one small batch of nodes + edges, bounded by count
  and bytes.
- transport: paged `/api/...` responses and/or `/ws` frames over the
  existing bus (`bus.rs`); decide which carries the bulk.
- backpressure: how B signals "ready for the next batch" so A never
  outruns the UI; how the depth slider and a single-dir expand each
  request the next batch rather than refetching the whole graph.
- ordering / idempotency: a node arrives before its edges; a batch is
  safe to re-request. (Collapse + persistence are frontend-only; the
  backend just serves the batches B asks for.)

(unfilled - fill at kickoff)

## 2. New-workspace pre-flight (A -> B)

chan-server runs the pre-flight on first boot; the OverlayShell renders
it locked until done. To pin on kickoff:

- endpoint(s): start the pre-flight, poll or stream its state, submit
  the user's decisions.
- state machine: the steps and states, their UI representation, and the
  terminal states (ready / failed / needs-decision).
- the "locked until complete" signal B uses to hide the OverlayShell
  close button and ignore ESC.

(unfilled - fill at kickoff)
