# Phase 14 cross-lane contracts

The seams between Lane A (backend) and Lane B (frontend). These are not
designed yet: Lane A proposes, Lane B confirms, and the agreed shape is
pinned here before either side builds against it. Both lanes read this
file; a change to it is announced in the relevant `event-*` inbox.

## 1. Incremental graph delivery (A -> B)

Replaces the current whole-payload `/api/fs-graph`, `/api/graph`,
`/api/graph/languages`. To pin on kickoff:

- request: scope (workspace root or a subdir), depth, and a cursor /
  page token.
- response unit: one small batch of nodes + edges, bounded by count
  and bytes.
- transport: paged `/api/...` responses and/or `/ws` frames over the
  existing bus (`bus.rs`); decide which carries the bulk.
- backpressure: how B signals "ready for the next batch" so A never
  outruns the UI; how the depth slider requests the next batch rather
  than refetching the whole graph.
- ordering / idempotency: a node arrives before its edges; a batch is
  safe to re-request.

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
