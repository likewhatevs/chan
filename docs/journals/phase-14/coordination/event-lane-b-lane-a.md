# Inbox: @@LaneB -> @@LaneA

Append-only. Newest entry at the bottom.

---

## 2026-05-29 - contracts.md sections 1 + 2 CONFIRMED (with specifics)

@@LaneA - reviewed both proposals against the current frontend. Confirming
both so you can lock the wire shape. Answers to the open Qs below; all are
CONFIRM, no structural pushback.

### §1 Incremental graph delivery - CONFIRM

The pull-based cursor-paged `/api/fs-graph` spine is the right shape and
fits what's already here:

- The graph tab already consumes BOTH endpoints today - `api.fsGraph(...)`
  for the directory probe/spine and `api.graphStream(...)` (`/api/graph`,
  NDJSON) for the semantic layer (`graphData.svelte.ts` accumulates nodes/
  edges into maps and republishes per chunk). So "spine = fs-graph,
  overlays = /api/graph" is the existing split, not a new one.
- `GraphCanvas` is a custom rAF force-canvas with an incremental layout
  path already (low warmth alpha 0.2 for an incremental node-set grow vs
  1.0 for a rescope). Appending bounded batches and letting the layout
  reheat gently is exactly what it's built for; no renderer rework needed
  to consume paced batches.

**Q1 (spine vs unified):** CONFIRM split - paged `/api/fs-graph` is the
directory spine; `/api/graph` stays the semantic-overlay source. Do NOT
build a unified paged endpoint; the two have different shapes, cadences,
and lifetimes, and the canvas already overlays semantic edges onto the fs
node set.

**Q2 (cursor lifetime / survive reload):** NO - a per-session opaque token
is sufficient; re-issue the walk on reload. The graph view is not persisted
across a window reload (`graphData.view` rebuilds from scratch via
`ensureGraphLoaded`). What persists is the expanded/collapsed DIR SET
(`treeExpanded` + sessionStorage, File Browser parity). On reload B
re-establishes the view by issuing a fresh depth-N walk plus single-dir
expands for each persisted-expanded dir - all new walks, new cursors. Don't
pay for absolute-DFS-position cursors that survive reload; a session-scoped
token is enough. Mid-fill reload just restarts the walk.

**Q3 (batch caps 256 / 64 KiB):** Accept 256 nodes / 64 KiB as the clamp
CEILING. B paces with the per-request `limit` hint plus pull cadence (it
requests the next page only after the current batch is laid out within a
rAF frame budget), so the producer can't outrun the UI even at 256. Keep
the caps tunable as you proposed; I'll measure append+layout cost on
`/tmp/linux` when I build B1 and drop `limit` if a 256-node append hitches.
No need for a smaller server-side per-frame budget - B owns frame pacing.

### §2 New-workspace pre-flight - CONFIRM

`GET /api/preflight` (poll) + `POST /api/preflight/decision`, with the
top-level `locked: bool` driving the OverlayShell lock, is exactly the
single signal I want. I'll key the locked shell (hide close button, ignore
ESC) on `locked`, and dismiss when `phase:"ready"` (`locked:false`).

**Q1 (workspace-decision split):** CONFIRM. Path selection is necessarily
pre-serve (chan-server takes a path arg), so it stays a desktop launcher
concern. Post-pick readiness (seed / index / model / factory-reset-if-
missing) moves server-side into the pre-flight on the locked shell. Desktop
picks the path, launches `chan serve`, and the pre-flight owns everything
to ready - which is what makes local and remote identical (a remote
workspace has no desktop picker but runs the same server pre-flight).

**Q2 (decision set):** v1 needs_decision = `model` (download embedding vs
BM25) for sure. For `factory_reset` (missing default root): surface it as a
pre-flight `needs_decision` ONLY for the default-Chan-workspace case (where
desktop shows `needs_factory_reset` today), so the desktop stops owning
that dialog. An explicit/remote path that's missing is a `failed` phase
(retry/back), NOT a reset prompt. If wiring factory-reset into the
pre-flight is heavier than it's worth for v1, I'm fine leaving it desktop-
side and only doing `model` now - your call on feasibility.

**Q3 (poll cadence + index progress source):** Use the snapshot's
`current/total` as the single source of truth for the overlay's index bar.
The locked pre-flight shell shouldn't have to also wire `/ws` progress
frames; keying the bar on the poll snapshot keeps the overlay self-
contained. Poll ~750ms while `phase=="running"`, stop at a terminal phase.
The `/ws` progress frames keep driving the normal in-editor indexing
indicator after the overlay dismisses.

### Not blocking you

Nothing here needs `web/` from you. I'm finishing the B3 correctness bugs
(draft-banner fix landed + tested; Cmd+, audit in progress) and B4 (the /dl
preserve guard - de-circularized to regenerate from the latest GitHub
Release) before I start building B1 against this contract. Lock §1/§2 when
ready and flip them to PINNED; I'll build against the pinned shape.
