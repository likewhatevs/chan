# @@LaneA plan + journal - Phase 14 (round 3 + carryovers)

Backend hot-paths + pre-flight lane. Rust only (`chan-server`,
`chan-workspace`, `chan-desktop`). Worktree `../chan-p14-lane-a`,
branch `phase-14-lane-a`. Journals/contracts/inboxes edited by absolute
path in the canonical checkout.

## Status board

| Item | Title                                            | State |
|------|--------------------------------------------------|-------|
| A1   | Paced graph delivery (fs-graph spine, cursor)    | DONE `cd1d625` |
| A2   | Depth-slider paging (server side)                | DONE via A1 single-dir-expand |
| A3   | New-workspace pre-flight on chan-server          | DONE (server core) `0f727ff`; desktop relocation deferred |
| A4   | Backend half of draft-banner e2e stress test     | DONE `20db58d` |
| A5a  | De-flake indexer serial-lock test                | DONE `b864328` |
| A5b  | De-flake macOS-PTY real-terminal test            | DONE `b864328` |
| A5c  | Remove vestigial `team-work-N` convention        | DONE `ed21c91` |
| A5+  | De-flake drafts-subtree watcher test (3rd, caught in gate) | DONE `82d54dd` |
| A5d  | WKWebView desktop walk (verify gate, not code)   | PENDING @@Alex (human verify) |

Contracts §1 + §2 PINNED (Lane B confirmed). Lane gate
(`make ci-linux` Rust half) GREEN: fmt --check + clippy --all-targets
-D warnings + test --all-targets (all pass) + build
--no-default-features. No frontend touched (web-check/web-marketing
are Lane B's trees).

### A3 deferrals (flagged for @@Alex / next round)

- **Desktop relocation** (`default_workspace.rs` / `serve.rs` /
  `main.rs`): NOT done this pass. The server pre-flight is additive and
  the desktop already launches `chan serve`, so no desktop code change
  is required for the pre-flight to run + for Lane B's locked
  OverlayShell to render it. Ripping out / rewiring the desktop's
  existing default-workspace decision dialog should land once Lane B's
  OverlayShell exists so the end-to-end flow can be verified in
  WKWebView (couples to A5d). Held to avoid speculative desktop surgery.
- **Model-prompt policy (product call, please confirm):** the `model`
  `needs_decision` step fires only when the workspace has semantic
  search ENABLED but the model is absent -- NOT on every fresh
  workspace. Rationale: chan is local-first and defaults to BM25; a
  forced "download 90 MB" lock on first launch of every new workspace
  is naggy and arguably wrong. The "offer semantic on first run"
  onboarding, if wanted, is better as a settings affordance than a
  forced pre-flight lock. This also keeps the snapshot fully derived
  (the "skip" decision sticks via the existing `semantic_enabled` flag,
  no new persisted state). If you DO want every new workspace prompted,
  it's a small rule change (prompt whenever the model is absent) plus a
  persisted "dismissed" flag so "skip" sticks.
- **factory_reset decision:** kept desktop-side for v1 (a missing/locked
  path is a generic `failed` phase server-side); wiring the
  default-Chan factory-reset prompt into the pre-flight would couple
  chan-server to the desktop's default-Chan-root concept for marginal
  v1 value. Noted to Lane B in the inbox.

### A5d — WKWebView desktop walk (verification gate, NOT code)

Cannot be executed by an agent (Chrome/Blink can't reproduce WKWebView,
cf. terminal-webgl note). For @@Alex to run from the merged base:
`make macos-chan-dmg`, then verify: Cmd+Shift+N new window of the
focused workspace; Cmd+I no longer opens Dashboard; Cmd+P fires Team
Work; self-upgrade 0.17.0 -> 0.18.0 from `/dl`. Track as a gate, not a
change.

### A1/A2 done (commit `phase-14-lane-a`)

`/api/fs-graph` gains opt-in cursor paging (`limit` switches it on,
`cursor` resumes). Bounded DFS batches (<=256 nodes / 64 KiB), opaque
base64url cursor bound to `(path, depth)` and capped at MAX_DEPTH stack
frames. Whole-scope path (no params) is byte-identical to before, so the
depth-cap probe and CLI keep working. Per-child logic factored into
`emit_child` so paged + recursive walks emit identical node/edge
content (one caveat: hardlink edges are per-batch in paged mode; flagged
in `build_fs_graph_paged` docs + contracts §1.3). A2's "next degree, not
a re-walk" is the depth=1 single-dir-expand primitive (server returns
only that dir's children). 29 fs_graph tests green incl.
reassembly==whole-walk, bounded batches, idempotent cursor, scope-bound
cursor rejection. fmt + clippy -D warnings clean.

## Decisions taken

- **Graph transport (contracts §1):** pull-based cursor paging over
  HTTP on `/api/fs-graph`; `/ws` bus untouched. The directory spine is
  the freeze source on `/tmp/linux` and matches the round-3 dir
  expand/collapse model; the semantic overlays stay on the
  already-streamed `/api/graph`. Pinned PROPOSED in `contracts.md`,
  announced in `event-lane-a-lane-b.md`. Awaiting Lane B confirm on the
  three open Qs but starting the producer side now (shapes are
  additive; cursor is opaque so encoding can flex).
- **Pre-flight (contracts §2):** `GET /api/preflight` poll +
  `POST /api/preflight/decision`, auto-started on first boot,
  `locked` bool drives the OverlayShell lock. Proposed; desktop's
  "which workspace" decision split is an open Q for Lane B / the
  desktop integration.

## Grounding notes (verified in source 2026-05-29)

- `/api/graph` (graph.rs) already has `?stream=1` NDJSON with
  Meta/Nodes(128)/Edges(256)/Done/Error and bounded mpsc(8)
  backpressure - but `build_graph_view` builds the whole graph in
  memory first (production not paced). Graph tab renders `/api/graph`.
- `/api/fs-graph` (fs_graph.rs) is whole-payload, depth-bounded
  (MAX_DEPTH=6), node-capped (MAX_NODES=10_000), `truncated` flag.
  Graph tab uses it as the depth-cap probe (`workspaceDepthProbe`,
  `dirDepthProbe` at FS_GRAPH_DEPTH_MAX). The walker emits
  parent-before-child via the ancestor-chain pass.
- Dashboard indexing graph (GraphCanvas) polls `/api/indexing/state`
  every 3s (dir-only spine, bounded by dir count).
- `bus.rs` `/ws` is a broadcast for watcher + progress + scoped `fs`
  frames; self-write suppression at `event_is_self_echo`. Indexer feed
  does NOT honor self-write suppression (in-app saves must reindex).
- Desktop pre-flight today: `default_workspace.rs`
  (`DefaultWorkspaceStatus` / create / choose / factory-reset) + Tauri
  commands in `main.rs` (`default_workspace_status`,
  `choose_default_workspace`, `create_default_workspace`,
  `factory_reset_default_workspace`); boot calls
  `ensure_fresh_default_workspace()`.
- A4/A5a share the indexer/self-write area - do together. A5a target:
  `chan-workspace::tests::write_text_does_not_wait_for_indexer_serial_lock`.
  A5b target: `chan-server::routes::terminal::tests::
  conditional_pty_programs_validate_real_terminal` (terminal.rs:1132),
  reds `main` CI on the GitHub macOS runner (no /dev/ttys device path).

## Gate

`make ci-linux` green (fmt + clippy -D warnings + tests + build). No
frontend files touched. Graph *contents* unchanged; only delivery paced.
