# frontend-6: ghost-node indexer-progress UX

Owner: @@Frontend
Status: REVIEW

## Goal

Replace the static "not in the current file listing (try Reload /
chan index)" hint in the graph panel with a live status when the
indexer is busy, so the user can tell whether to wait or to take
action.

## Background

Today the hint is fired from `web/src/components/GraphPanel.svelte`
around line 1146 whenever a ghost node is selected. The hint is a
fixed string; there is no signal telling the user whether the
indexer is currently chasing recent fs changes or idle.

## Relevant links

* Source of the hint: `web/src/components/GraphPanel.svelte`
  (search for "not in the current file listing").
* Decisions: [architect-2.md](./architect-2.md) item 4.
* Backend surface: [backsystacean-7](./backsystacean-7.md)
  (`/api/health` now has an `indexer` block with
  `status / queue_depth / last_event_at / last_settled_at /
  coalesced_rebuild`).

## Scope

* When the inspector renders a ghost node, poll `/api/health` on a
  1 s cadence while the panel is open.
* If `indexer.status != idle`, replace the static hint with a live
  string. Suggested copy:
  * `settling`: `indexer is catching up (N event(s) pending)` where
    N is `queue_depth`.
  * `rebuilding`: `indexer is rebuilding (full pass)`.
  * `idle`: keep today's "not in the current file listing (try
    Reload / chan index)" hint.
* Stop polling when the inspector closes or the selected node is
  no longer a ghost.
* No new global polling loop; the poll is scoped to the ghost
  inspector body.

## Out of scope

* WebSocket push for indexer state.
* Surfacing indexer state outside the graph panel (e.g., a global
  status badge); flag as a follow-up if the polling pattern proves
  useful elsewhere.

## Acceptance criteria

* Ghost inspector shows the live status while the indexer is busy
  and reverts to the static hint when idle.
* Polling stops when the ghost inspector body unmounts.

## Tests

* Vitest covering the polling subscription lifecycle + hint copy
  based on a mocked `/api/health` response.
* `npm --prefix web run check`, `npm --prefix web test -- --run`,
  `npm --prefix web run build` green.

## Dependencies

Unblocked by [backsystacean-7](./backsystacean-7.md), which ships the
state endpoint.

## Progress notes

* Added typed `/api/health` support on the web API client for the
  indexer block.
* Graph ghost-file inspector now polls `/api/health` once per second
  only while the overlay is open and a non-broken ghost file is
  selected.
* Busy states replace the static hint with:
  `indexer is catching up (N event(s) pending)` or
  `indexer is rebuilding (full pass)`.
* Idle, missing endpoint, or poll failures fall back to the previous
  static hint.

## Completion notes

Verification:
* `npm run check` in `web` passed.
* `npm test -- --run` in `web` passed: 18 files, 170 tests.
* `npm run build` in `web` passed with existing chunk-size warnings.
