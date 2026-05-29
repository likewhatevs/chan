# Phase 14 coordination

How the lanes talk. Two kinds of file live here:

- `contracts.md` - the cross-lane interface contracts the lanes build
  against (incremental graph delivery, pre-flight states). Pin a
  contract here BEFORE the dependent lane builds on it; it is the
  single source of truth. Changes are announced in the inboxes below.
- `event-<from>-<to>.md` - append-only inboxes, one per directed pair
  (e.g. `event-lane-a-lane-b.md` is @@LaneA writing to @@LaneB).
  Newest entry at the bottom; created on first use.

## Lanes

- **Lane A** - backend hot paths (paced graph delivery) + the
  new-workspace pre-flight (chan-server side) + the chan-desktop launch
  change. Rust only.
- **Lane B** - all frontend: the incremental graph rendering + the
  pre-flight OverlayShell lock, then the round-2 pristine cleanup.
- **Lane C** - docs. C2 = the docs/journals second-brain reorg
  (concurrent; touches only `docs/journals/`). C1 = the round-2
  `/architect` pass over frontend comments / docs / user-facing copy
  (a closing wave, after A and B merge).

## Concurrency

A and B run at the same time and share only the seams in
`contracts.md` (so pin those first). C2 runs at the same time and
collides with nobody (journals only). C1 runs last.

Round scope docs (`roadmap-round-{2,3}.md`) are the WHAT; these lanes
are the HOW, re-cut by subsystem so no two concurrent lanes edit the
same files.
