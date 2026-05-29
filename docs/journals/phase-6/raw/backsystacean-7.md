# backsystacean-7: surface indexer state

Owner: @@Backsystacean
Status: REVIEW

## Goal

Expose the running indexer's state through an API surface so the
graph panel can replace its static "not in the current file listing
(try Reload / chan index)" hint with a live status when the indexer
is busy.

## Background

Alex noticed in the graph that ghost nodes (a node in the graph DB
that isn't in the current tree listing) carry a static "try Reload
/ chan index" hint. There is no progress signal today; the user has
no way to tell whether the indexer is mid-cycle or genuinely idle.
The hint comes from `web/src/components/GraphPanel.svelte:1146`.

The indexer already keeps internal state in
`crates/chan-server/src/indexer.rs` (event queue, debounce, batch
processing, watcher coalescing per phase 5). This task exposes that
state to the frontend.

## Relevant links

* Request follow-up: ghost-node UX gap discussed 2026-05-18.
* Journal: [journal.md](./journal.md).
* Decisions: [architect-2.md](./architect-2.md) item 4.
* Frontend consumer: [frontend-6](./frontend-6.md).
* Source: `crates/chan-server/src/indexer.rs`,
  `crates/chan-server/src/routes/health.rs`.

## Scope

### Indexer state shape

* Status: `idle | settling | rebuilding` (or richer if the indexer
  already distinguishes more lanes; pick the smallest enum that
  meaningfully covers user-facing behavior).
* `queue_depth`: pending event count, 0 when idle.
* `last_event_at`: timestamp of the most recent watcher event.
* `last_settled_at`: timestamp of the last idle transition.
* `coalesced_rebuild`: bool, true while the phase-5 git/hg / large-
  burst coalesce is in flight.

### Route surface

* Option A: extend `/api/health` with an `indexer` block. Cheap, no
  new route, already polled.
* Option B: add `/api/indexer/state`. Explicit, but adds a route.
* Recommendation: extend `/api/health`. Keeps the surface count
  small; the frontend can poll on a low cadence (1 s) only when the
  graph panel is open on a ghost node.

### Update cadence + cost

* Health is polled today on a low cadence already; adding the
  indexer block costs one mutex read on the indexer state.
* No new event channel needed for this phase; the frontend polls
  while the ghost hint is visible.

## Out of scope

* Pushing indexer state over WebSocket (deferred; polling is enough
  for the current UX gap).
* Restructuring the indexer's internal state machine.

## Acceptance criteria

* `/api/health` (or sibling) returns the new `indexer` block.
* Block reflects the live state (idle vs settling vs rebuilding) on
  a tiny manual test: drop a file into the drive, verify state
  flips, verify it returns to idle.
* Tests covering the serialization shape + a transition.

## Tests

* `cargo test -p chan-server` for the new field shape + transitions.
* Pre-push gate green (fmt, clippy, no-default-features, workspace
  test).

## Dependencies

Unblocks [frontend-6](./frontend-6.md).

## Progress notes

* Extended `/api/health` with an `indexer` block sourced from the
  live server indexer handle.
* Added lightweight indexer telemetry alongside the existing
  `IndexStatus`: queue depth, last watcher event timestamp, last
  settled timestamp, and coalesced rebuild flag.
* Health status maps to `idle`, `settling`, `rebuilding`, or `error`.
  Debounced pending paths report `settling`; active build/reindex or
  queued coalesced rebuild reports `rebuilding`.
* Added tests for the health serialization shape and the settling /
  rebuilding transition mapping.

## Completion notes

Ready for review. `/api/health` now returns:

```json
{
  "status": "ok",
  "indexer": {
    "status": "idle",
    "queue_depth": 0,
    "last_event_at": null,
    "last_settled_at": 1700000000,
    "coalesced_rebuild": false
  }
}
```

Verified:

* `cargo test -p chan-server health`
* `cargo test -p chan-server`
* `scripts/pre-push`
