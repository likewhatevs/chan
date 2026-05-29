# systacean-18: GET /api/indexing/state for the empty-pane carousel

Owner: @@Systacean
Cut by: @@Architect
Date: 2026-05-19

## Goal

Expose chan-server's per-directory indexing state via a
small read-only HTTP endpoint. @@FullStackB needs this
for the empty-pane carousel's slide 3 (`fullstack-35`):
a dir-only graph where each directory node renders grey
(not indexed), orange (in flight, pulsating), or green
(fully indexed).

## Relevant links

* Consumer: [../fullstack-b/fullstack-35.md](../fullstack-b/fullstack-35.md)
  ("Slide 3 — Indexing-state graph").
* Indexer machinery lives in `crates/chan-drive/` per
  the workspace layout in CLAUDE.md.

## Acceptance criteria

### Endpoint

* `GET /api/indexing/state` — returns the current
  indexing state for the active drive's directory tree.
* Response shape (JSON):
  ```json
  {
    "root": "<drive-relative-path>",
    "nodes": [
      {
        "path": "<drive-relative-path-from-root>",
        "state": "indexed" | "indexing" | "pending",
        "children_count": <int>
      },
      ...
    ]
  }
  ```
  Notes:
  * `state` is one of three values; map to colors on the
    SPA side (`indexed`=green, `indexing`=orange,
    `pending`=grey).
  * Include directories only — no file entries. The
    carousel doesn't plot files.
  * `children_count` lets the SPA decide whether to
    render expand/collapse affordances.
  * Path is drive-relative; root is the drive root.
* Auth: same per-launch bearer token as the rest of the
  API.
* Returns 200 with the JSON body. 401 if unauthenticated.

### State source

* Read from the existing BGE-small embedding index in
  `chan-drive`. Whatever per-directory progress signal
  the indexer already tracks (in memory or persisted),
  surface it through this endpoint.
* If the indexer doesn't currently track per-directory
  state explicitly: derive it lazily. A dir is
  `indexed` if all editable-text files in it have been
  embedded; `indexing` if any file in it is currently
  being processed (the indexer's work queue probably
  has this); `pending` otherwise.

### Lightweight

* The endpoint is hit on every empty-pane render +
  every 5s carousel rotation. Keep the response cheap:
  no recomputation per call — read the indexer's
  cached state. If the cache is stale, snapshot once
  and serve.
* Response should be < 10ms for a typical drive
  (~hundreds of dirs).

### Tests

* Unit test with a synthetic indexer state: assert the
  JSON shape + that each state value is one of the
  three.
* Integration test that hits the endpoint via the test
  router (existing pattern in `chan-server` tests).

## Out of scope

* Per-file state (carousel only shows dirs).
* Streaming updates / WebSocket push. The SPA polls or
  re-fetches on carousel rotation.
* Triggering reindex from this endpoint — that's a
  separate concern.

## How to start

1. Locate the indexer's per-dir progress signal in
   `chan-drive`. If it doesn't exist, add a small
   tracker (HashMap<PathBuf, State> behind a Mutex /
   RwLock, updated as the indexer enqueues + completes
   work).
2. New route in `crates/chan-server/src/routes/` — a
   small handler that snapshots the tracker + walks
   the drive root for unindexed dirs (`pending`
   state).
3. Coordinate with @@FullStackB on the exact JSON
   shape before they commit `fullstack-35` slide 3.

## Hand-off

Standard. Pre-push gate green. Coordinate with
@@FullStackB on the schema. Lands after `systacean-17`
clears. Ping via
`alex/event-systacean-architect.md`.
