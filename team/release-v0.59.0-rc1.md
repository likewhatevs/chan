# v0.59.0-rc1: graph force tuning, focus-on-select, deeper fs-graph

## Theme

Graph tuning. A live force-tuning playground that drives the real renderer
against real data, plus the interaction and depth improvements that fell
out of tuning it against a full source tree.

## What landed

### Graph physics as a shared, tunable module

- `web/packages/workspace-app/src/graph/force.ts`: the `GraphForce` type
  and `DEFAULT_FORCE`, the single source of truth for the d3-force
  physics. `GraphCanvas` takes an optional `force` prop defaulting to it;
  every production caller omits the prop and gets the default.
- `DEFAULT_FORCE` carries the tuned values: charge -90, link distance
  125/128, link strength 1.12, collide 8, center 0.05, hierarchy 90/0.45,
  parent-X 0.18.

### graph-tuner playground (replaces graph-demo)

- Mounts the real `GraphCanvas`, not a re-implementation, so what you tune
  is what the live graph does.
- Live sliders for all ten force params, a Copy FORCE button that emits a
  literal to paste into `force.ts`, plus theme, root-anchor, and
  regenerate controls.
- Data-source toggle: a synthetic generator or a real `/api/graph`
  snapshot of this repo's own source (1361 nodes, 2636 edges), captured to
  `src/graph-tuner/sampleGraph.json`.
- Depth slider matching the Graph tab's workspace-scope depth (path-depth
  via `relativeDepth`), capped at `FS_GRAPH_DEPTH_MAX`.

### Focus-on-select in GraphCanvas

- Clicking a node spotlights its 1st-degree neighbourhood: the selection
  and its neighbours stay full-strength with labels, incident edges light
  up, and everything else greys out.

### Bottom anchor as the default

- `GraphCanvas` `focalAnchor` defaults to `bottom`, so the main Graph tab
  and the Dashboard slide both grow the workspace spine upward from the
  root.

### Deeper fs-graph

- `FS_GRAPH_DEPTH_MAX` (frontend) and `MAX_DEPTH` in the chan-server
  `fs_graph` route both set to 10. The workspace depth slider reaches the
  full depth of a deeper, source-style workspace; a single request stays
  bounded by `MAX_NODES`, so a huge tree truncates by node count.

### Removed

- The sphere-tuner and d3-compare demos, both dead cytoscape-era
  playgrounds.

## Highlights

- Zero fidelity gap: the tuner mounts the production renderer, so a tuned
  value maps one-to-one onto `force.ts`, which both the tuner and the live
  graph read.
- Tuning against a real 1361-node graph of chan's own source surfaced two
  improvements that synthetic data would have hidden: the bottom-anchor
  preference and the depth ceiling.
- The whole flow now exists as a reusable tool: real sample plus sliders
  plus depth plus focus, all against the actual renderer.

## Low lights

- Writing-style slips. I put em dashes and backward-looking "archeology"
  comments (for example "was 6", "no longer hides", "moved to") into
  several new comments. Alex flagged both. Corrected across the change and
  saved as a standing rule; it should not have taken a review pass to
  catch.
- Layered cap missed on the first pass. I raised the frontend depth
  constant and reported the cap lifted, but the chan-server route still
  clamped at 6, so the first check still read 6. Alex caught it. The
  lesson: trace a limit through the client request and the server clamp
  before calling it lifted.
- The `sampleGraph.json` fixture is 381 KB. It is a legitimate real-data
  sample but heavy for the tree. It slims to roughly 307 KB by deriving
  the `contains` edges from paths; left as-is pending a call.
- The effort ran on a debug build, so indexing the seeded workspace was
  slow (semantic embeddings on an unoptimized binary). The workaround was
  to move the model out of the cache for a BM25-and-graph-only reindex,
  which cost a few round-trips.

## Validation

- svelte-check 0/0/0; workspace-app vitest 2094 pass; chan-server
  `fs_graph` tests 30 pass; `cargo fmt --check` clean.
- Browser-verified in the tuner: real data renders, depth slider runs 1
  to 8 after the bump, focus-on-select spotlight, bottom anchor.
- The `/api/fs-graph` endpoint at depth 10 returns nodes to depth 8 on the
  chan-source workspace (1230 nodes, not truncated).
- Empirically unverified: the main Graph tab inside chan-desktop. All
  browser checks ran against the web SPA on a local `chan open` server.

## Open items

- `sampleGraph.json` size: keep at 381 KB, slim to about 307 KB, or drop
  it from the tree.
- The tuner HTML lives at the vite root
  (`web/packages/workspace-app/graph-tuner.html`) so `npm run dev` serves
  it directly.
- No version bump or tag in this commit. This is the rc notes plus the
  code; the version pins stay at 0.58.0 until a release cut.
