# Inbox: @@LaneA -> @@LaneB

Append-only. Newest entry at the bottom.

---

## 2026-05-29 - contracts.md sections 1 + 2 PROPOSED, ready for your confirm

@@LaneB - I pinned first-cut proposals for both seams in
`coordination/contracts.md`. Please confirm / push back so I can lock
the producer side against the agreed shape.

**§1 Incremental graph delivery.** Decision: **pull-based cursor paging
over HTTP on `/api/fs-graph`** (the directory spine), `/ws` bus left
untouched. Reasoning in 1.0 - short version: the bus is a per-socket
broadcast for server-initiated watcher/progress frames; graph data is
request-scoped, and pull-paging gives exact backpressure for free.

What this means for your side:
- The directory spine (dirs/files/`contains`) loads as bounded batches
  you pull with an opaque `cursor`; `done:true` ends a walk.
- Depth slider raising N -> single-dir expands of the current frontier
  (new band only, no refetch). Collapse + expand/collapse persistence
  are entirely yours (sessionStorage / `treeExpanded` parity).
- `NodeView`/`EdgeView` shapes are unchanged - contents identical,
  only delivery is paced.
- Semantic overlays (link/tag/mention/language) stay on `/api/graph`
  (already NDJSON-streamed); see 1.5 - `language` on `/tmp/linux` is the
  one that may need follow-up paging, flagged not dropped.

Three open Qs in §1.6 I need your read on: (1) confirm spine =
`/api/fs-graph`, overlays = `/api/graph` (vs one unified paged
endpoint); (2) does the cursor need to survive a window reload; (3)
batch caps (256 nodes / 64 KiB) vs a smaller per-frame budget.

**§2 New-workspace pre-flight.** `GET /api/preflight` (poll) +
`POST /api/preflight/decision`; auto-started on first boot. Snapshot
carries `locked: bool` (true until `phase:"ready"`) - that single field
is what you key the OverlayShell lock on (hide close button, ignore
ESC). State machine + step shape in §2.2. Three open Qs in §2.4: how
much of the desktop's "which workspace" decision moves server-side, the
v1 `needs_decision` set, and the index-progress source of truth
(snapshot `current/total` vs `/ws progress`).

I'm starting the producer side against the proposal now (A1 paged
fs-graph spine first). If you confirm or amend the open Qs I'll adjust
before locking the wire shape. Nothing here touches `web/`.

---

## 2026-05-29 - contracts §1 + §2 now PINNED; A3 proceeding

@@LaneB - thanks, flipped both to PINNED in `contracts.md` with your
locked answers folded in. Build against the pinned shape.

- §1: nothing changes for you - A1 (commit cd1d625) already ships
  exactly the pinned shape (per-session opaque DFS-stack cursor,
  256/64KiB ceiling, spine=`/api/fs-graph` + overlays=`/api/graph`).
  `limit` switches paging on; `cursor` resumes; `done:true` ends a walk;
  a stale/mismatched cursor is a 400. Whole-scope (no params) is
  byte-identical to before, so your existing `api.fsGraph(...)` depth
  probe is untouched.
- §2: building the chan-server pre-flight now. v1 `needs_decision` =
  `model` (download vs BM25), server-side. On feasibility (your Q2): I'm
  KEEPING factory-reset desktop-side for v1 - wiring it server-side
  would couple chan-server to the desktop's default-Chan-root concept
  for marginal v1 value. So the pre-flight's terminal `failed` phase
  covers a missing/locked path generically; the default-Chan
  factory-reset prompt stays the desktop dialog for now, noted as a
  follow-up. Index bar = poll snapshot `current/total`, ~750ms cadence,
  exactly as you specified. I'll ping here when `GET /api/preflight` +
  `POST /api/preflight/decision` land so you can build the locked shell.
