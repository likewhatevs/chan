# Phase 14 cross-lane contracts

The seams between Lane A (backend) and Lane B (frontend). These are not
designed yet: Lane A proposes, Lane B confirms, and the agreed shape is
pinned here before either side builds against it. Both lanes read this
file; a change to it is announced in the relevant `event-*` inbox.

Status legend: `PROPOSED` = Lane A's first cut, awaiting Lane B confirm;
`PINNED` = both lanes agreed, safe to build against.

## 1. Incremental graph delivery (A -> B)   [PINNED 2026-05-29]

PINNED: @@LaneA proposed, @@LaneB confirmed (event inboxes). Build
against this shape.

Locked answers to the open Qs:
- Q1 spine vs unified: SPLIT. Paged `/api/fs-graph` is the directory
  spine; `/api/graph` stays the semantic-overlay source (NDJSON). Do
  not build a unified paged endpoint.
- Q2 cursor lifetime: per-session opaque token; re-issue the walk on
  reload (the graph view is not persisted; only the expanded-dir SET is,
  via `treeExpanded` + sessionStorage, and B re-walks for it). No
  reload-surviving absolute-position cursor needed -- matches A's
  DFS-stack token.
- Q3 batch caps: 256 nodes / 64 KiB is the clamp CEILING; B paces with
  the `limit` hint + pull cadence (requests the next page only after the
  current batch lays out within a rAF budget). Caps stay tunable; no
  server-side per-frame budget (B owns frame pacing).

A1 (commit cd1d625) already implements exactly this. Replaces the
current whole-payload / push-streamed graph load with a pull-paced one:

### 1.0 Decision: pull-based cursor paging over HTTP, NOT the /ws bus

The bulk rides **paged `/api` responses (cursor-based)**. The `/ws` bus
stays exactly what it is today (watcher / progress / scoped `fs`
frames). Rationale:

- `/ws` (`bus.rs`) is a *broadcast* channel shared by every socket and
  every server-initiated event type. Graph data is request-scoped and
  per-socket; broadcasting it is the wrong fan-out and would contend
  with the very progress / watch frames we are trying to keep flowing.
- Pull-based cursor paging is exact backpressure for free: the producer
  does work only when the client asks for the next page, so it can
  never outrun the UI. The depth slider and a single-dir expand are
  each just "request the next page(s)".
- The bus semantics other surfaces depend on (self-write suppression,
  scoped `fs` delivery, idle-timeout activity) stay untouched -> lowest
  risk.

(The existing `/api/graph?stream=1` NDJSON form chunks *delivery* but
`build_graph_view` builds the whole graph in memory first; it is
push-paced, not pull-paced. The paged form below supersedes it for the
graph tab's directory spine. The NDJSON form can stay for the semantic
overlays / non-paced callers until B migrates.)

### 1.1 Where the spine comes from

The round-3 interaction model (double-click dir expand/collapse, depth
slider = `find -d N`) is the **directory spine** - dirs + files +
`contains` edges. That spine is what dominates node/edge count on a
large workspace (`/tmp/linux`) and what freezes the UI today. So the
paged primitive lives on the filesystem graph, `GET /api/fs-graph`
(today the graph tab already hits it as the depth-cap probe). The
semantic overlays (link / tag / mention / language edges, report
buckets) are a separate, smaller concern (see 1.5).

### 1.2 Request kinds (both feed the same batch stream)

`GET /api/fs-graph` query params:

- `scope`: `directory` (default) | `file` - unchanged.
- `path`: workspace-relative scope root (`""` = workspace root) -
  unchanged.
- `depth`: `N` (`find -d N`), authoritative for the expanded set.
  Clamped to `[1, MAX_DEPTH=6]` - unchanged.
- `cursor`: opaque continuation token from the previous batch's
  response. Absent on the first request of a `(scope, path, depth)`
  walk.
- `limit` (optional): client hint for max nodes per batch; server
  clamps to `[BATCH_MIN, BATCH_MAX]`.

Two request kinds:

- **depth scope** - `scope=directory&path=<P>&depth=<N>[&cursor=...]`:
  the authoritative expanded set to depth N under P, emitted as bounded
  batches. Drives the initial load and a multi-step slider jump.
- **single-dir expand** - `scope=directory&path=<DIR>&depth=1[&cursor=...]`:
  exactly one directory's next degree (its direct children), for the
  double-click expand and the per-degree slider step. (depth=1 + a
  specific dir = the per-node 1-by-1 control. This shape already exists
  today; what's new is cursor paging when a single dir is very wide.)

### 1.3 Batch unit (response)

```jsonc
{
  "root": "<abs>",          // workspace root abs path (every batch; cheap)
  "scope": "directory",
  "path": "<P>",
  "depth": N,
  "nodes": [ NodeView, ... ],  // bounded by count AND bytes
  "edges": [ EdgeView, ... ],  // contains | symlink | hardlink (shapes unchanged)
  "cursor": "<opaque>" | null, // null on the final batch
  "done": true | false,        // true on the final batch
  "truncated": false           // unchanged: true iff the MAX_NODES hard cap was hit
}
```

- `NodeView` / `EdgeView` shapes are **unchanged** from today's
  `FsGraphResponse` (graph *contents* do not change; only delivery is
  paced).
- Per-batch bound: at most `BATCH_MAX_NODES` (proposed 256) nodes and at
  most `BATCH_MAX_BYTES` (proposed ~64 KiB) serialized, whichever trips
  first. Tunable after measuring on `/tmp/linux`.
- The walk-wide hard cap `MAX_NODES=10_000` still applies; if hit, the
  final batch carries `truncated: true` (same meaning as today).
- `cursor` is **opaque**: B must not parse or construct it. It is valid
  only within the `(scope, path, depth)` walk that produced it; it is
  bounded in size (the implementation keeps it small - the DFS resume
  stack is at most `MAX_DEPTH` entries).

### 1.4 Backpressure / pacing / ordering / idempotency

- **Pull-based**: B requests the next page only when it is ready to
  append (after the current batch is laid out / within its frame
  budget). The server does bounded walk work per request and returns;
  it holds no cross-request walk state beyond what the `cursor`
  encodes, so a slow or paused client costs the server nothing.
- **Depth slider** raising N to N+1 issues single-dir expands for the
  current frontier dirs (the depth-N leaves), revealing only the new
  band - never a refetch of depth `0..N`. Lowering N / collapsing is
  frontend-only (hide the subtree); the server is not re-queried.
- **Ordering**: within a walk, a directory node is emitted in the same
  batch as, or an earlier batch than, the `contains` edge that points
  into it; a child node never arrives before its parent. (Same
  parent-before-child guarantee the current walker already produces via
  the ancestor-chain pass.)
- **Idempotency**: re-requesting the same `cursor` returns the same
  batch (safe to retry a dropped response). A `cursor` from a different
  `(scope, path, depth)` walk is rejected with `400` so B restarts from
  no cursor.

### 1.5 Semantic overlays (link / tag / mention / language)

Out of the paged-spine path. They keep coming from `/api/graph`
(already NDJSON-streamed). Two ways B can consume them, B's call:

- keep the existing `graphStream()` for the semantic layers and overlay
  them on the paged fs spine as the node set grows; or
- request overlays scoped to the currently-loaded node set.

On `/tmp/linux` the only large overlay is the `language` layer; if it
still hogs, A will page it as a follow-up against this same contract.
Flagged, not silently dropped.

### 1.6 Open questions for @@LaneB

1. Confirm the graph tab drives its **directory spine** from paged
   `/api/fs-graph` (depth scope + single-dir expand) and treats
   `/api/graph` as the semantic-overlay source. If instead you want one
   unified paged endpoint, say so and A will page `/api/graph`'s spine
   sub-walk identically.
2. Cursor lifetime: per-session opaque token is the proposal. Do you
   need a cursor to survive a window reload (resume mid-fill), or is
   re-issuing the walk on reload fine? (Affects whether A encodes an
   absolute DFS position vs a server-held token.)
3. Batch caps (256 nodes / 64 KiB, pull-based): does that pace cleanly
   against cytoscape append + layout, or do you want smaller frames /
   an explicit per-frame node budget?

## 2. New-workspace pre-flight (A -> B)   [PINNED 2026-05-29]

PINNED: @@LaneA proposed, @@LaneB confirmed. Build against this shape.

Locked answers to the open Qs:
- Q1 workspace-decision split: path selection stays a DESKTOP launcher
  concern (chan-server takes a path arg, so it's necessarily pre-serve).
  Post-pick readiness (seed / index / model / optional factory-reset)
  moves SERVER-side into the pre-flight on the locked shell. This is
  what makes local and remote identical (a remote workspace has no
  desktop picker but runs the same server pre-flight).
- Q2 decision set: v1 `needs_decision` = `model` (download embedding vs
  BM25). `factory_reset` (missing default root): surface as a pre-flight
  decision ONLY for the default-Chan-workspace case; an explicit/remote
  missing path is a `failed` phase (retry/back), not a reset prompt.
  Factory-reset wiring is @@LaneA's feasibility call for v1 -- see the
  lane-a-plan A3 note (v1 ships `model` server-side; factory-reset stays
  desktop-side for v1 to avoid coupling chan-server to the desktop's
  default-Chan-root concept).
- Q3 index progress source: the poll snapshot's `current/total` is the
  single source of truth for the overlay's index bar (the locked shell
  stays self-contained; it does NOT also wire `/ws` progress frames).
  Poll ~750 ms while `phase=="running"`, stop at a terminal phase. `/ws`
  progress frames keep driving the normal in-editor indexing indicator
  after the overlay dismisses.

chan-server runs the pre-flight on first boot of a workspace; the
OverlayShell renders it **locked until done**. Today this logic lives in
chan-desktop (`default_workspace.rs` decision flow + `serve.rs`); it
moves server-side so local and remote (inbound / outbound) workspaces
get one identical flow and the desktop only launches `chan serve`.

### 2.1 Endpoints

- `GET /api/preflight` - snapshot of the pre-flight state machine
  (poll). The OverlayShell polls this; long-running per-file index
  progress continues to arrive on the existing `/ws` `progress` frames
  (no new bus frame type).
- `POST /api/preflight/decision` - submit the answer to a
  `needs_decision` step. Body: `{ "step": "<step-id>", "choice": "<id>" }`.
- Start is **implicit**: chan-server kicks the pre-flight on first boot
  before the editor surface is usable; the UI does not call a start
  endpoint. (If B wants explicit control of when it begins, A adds
  `POST /api/preflight/start`; flag it.)

### 2.2 State machine

Snapshot shape (proposed):

```jsonc
{
  "phase": "running" | "needs_decision" | "ready" | "failed",
  "locked": true | false,        // true until phase == "ready"
  "steps": [
    { "id": "open",    "label": "Open workspace",   "state": "done" },
    { "id": "seed",    "label": "Seed starter notes","state": "done" },
    { "id": "index",   "label": "Build search index","state": "running",
      "current": 1234, "total": 80000 },     // mirrors IndexStatus::Building
    { "id": "model",   "label": "Embedding model",   "state": "needs_decision",
      "decision": { "prompt": "...", "choices": [
        { "id": "download", "label": "Download (90 MB)" },
        { "id": "skip",     "label": "Use keyword search" }
      ] } }
  ],
  "error": null | { "step": "<id>", "message": "..." }
}
```

- Per-step `state`: `pending | running | done | needs_decision | failed`.
- Terminal phases: `ready` (editor usable; `locked: false`), `failed`
  (carries `error`; UI offers retry / back), `needs_decision` (blocks
  on a `POST /api/preflight/decision`).
- Step set is workspace-dependent (a fresh default `Chan` workspace has
  `seed`; an existing one does not). The `index` step reuses the
  indexer's existing `Building { current, total }` counters so the
  overlay shows the same progress the `/ws` `progress` frames carry.

### 2.3 "Locked until complete" signal

The snapshot's top-level `locked: bool` is the single signal B keys on:
while `true`, the OverlayShell hides/removes the close button and
ignores ESC; when the poll returns `phase: "ready"` (`locked: false`),
B dismisses the overlay and the editor becomes usable. No separate flag.

### 2.4 Open questions for @@LaneB

1. How much of the desktop's current "which workspace" decision flow
   (`DefaultWorkspaceStatus`: needs_prompt / needs_factory_reset /
   choose-existing / create-default) moves server-side vs stays a
   desktop launcher concern? chan-server serves one workspace (a path
   arg), so picking the path is necessarily pre-serve; the proposal
   moves the post-pick readiness + decisions (seed / index / model /
   factory-reset-if-missing) into the server pre-flight. Confirm the
   split.
2. Decision set: is `model` (download vs BM25) the only `needs_decision`
   step for v1, or do you also want `factory_reset` (missing default
   root) surfaced as a pre-flight decision rather than a desktop dialog?
3. Poll cadence + whether you want the `index` progress to come from
   the poll snapshot's `current/total` or from the `/ws` `progress`
   frames (both are available; pick one source of truth for the bar).
