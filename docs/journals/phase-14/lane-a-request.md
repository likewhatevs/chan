# @@LaneA request - Phase 14

You are @@LaneA, the **backend hot-paths + pre-flight** lane. Rust
only: `chan-server`, `chan-workspace`, `chan-desktop`. You do NOT touch
`web/` or any frontend tree (that is @@LaneB). You MAY spawn 1-2
in-session subagents (one per item below). You run concurrently with
@@LaneB; you share only the seams in `coordination/contracts.md`, which
you propose first.

## Recover context (read in order)

- `/Users/fiorix/dev/github.com/fiorix/chan/CLAUDE.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/design.md`
- `docs/journals/phase-14/roadmap-round-3.md` (theme 1 + theme 2)
- `docs/journals/phase-14/coordination/contracts.md`
- `docs/journals/phase-14/coordination/event-lane-b-lane-a.md` (inbox; may not exist yet)
- `docs/journals/phase-14/lane-b-plan-draft-restore-banner.md` (you own the backend half of its e2e stress test)

## Worktree + branch

Source ONLY in a dedicated worktree off the current phase-14 base:

```
git -C /Users/fiorix/dev/github.com/fiorix/chan worktree add ../chan-p14-lane-a -b phase-14-lane-a
```

Journals, contracts, and inboxes live in the canonical checkout under
`docs/journals/phase-14/` and are edited by ABSOLUTE PATH (never the
worktree copy).

## Scope

### A1. Paced graph delivery (round 3, theme 1, backend half)

The graph endpoints send whole payloads; on a large workspace
(`/tmp/linux`) that hogs the API bus. Make delivery incremental and
bounded, end to end from the on-disk indices out.

- chan-server: `crates/chan-server/src/routes/fs_graph.rs`
  (`api_fs_graph`, `build_fs_graph`, `FsGraphScope`),
  `routes/graph.rs` (`api_graph`, `api_language_graph`),
  `routes/ws.rs` + `bus.rs` (the `/ws` event bus),
  `routes/index.rs` (`api_index_status`, `api_indexing_state`).
- chan-workspace: the indexer / graph producers feeding the above;
  make them yield bounded chunks rather than a full walk.
- Define the delivery contract in `contracts.md` section 1 (request,
  batch unit, transport, backpressure, ordering) and implement the
  producer side. Cap per-frame work so a large workspace fills in
  gradually instead of blocking.

### A2. Depth-slider paging (backend)

Expanding depth returns the next incremental batch for the newly
revealed directories, not a re-walk of the whole tree. Server side of
the slider; the contract is the same one pinned in A1.

### A3. New-workspace pre-flight on chan-server (round 3, theme 2)

Move the pre-flight out of chan-desktop into chan-server's first boot.

- chan-server: add the pre-flight (a module + endpoint(s)) that runs on
  first boot of a new workspace; expose start / state / decision per
  `contracts.md` section 2.
- chan-desktop: `desktop/src-tauri/src/default_workspace.rs`,
  `serve.rs`, `main.rs` - the "add a workspace" (today's Open) action
  starts `chan serve`, which runs the pre-flight server-side before the
  UI is usable; the desktop stops owning the pre-flight logic.
- Same flow for local and remote (inbound/outbound) workspaces.

### A4. Backend half of the draft-banner e2e stress test

Per `lane-b-plan-draft-restore-banner.md`: a chan-server/chan-workspace
integration stress test (reuse the `crates/chan-workspace/tests/`
harness + the draft unit tests in `routes/drafts.rs:228`). Hammer
create-draft -> write/autosave (`/api/files` with `expected_mtime_ns`
CAS) -> re-read, many iterations; assert self-write suppression holds
(own writes never broadcast as external via `bus.rs` / `self_writes.rs`),
CAS mtime round-trips, and no spurious `DraftBroken` / "missing
draft.md". Lane B owns the frontend half + the actual fix.

## Coordination

- Pin `contracts.md` sections 1 and 2 first; announce in
  `event-lane-a-lane-b.md`. @@LaneB renders against them.
- The only shared file with other Rust work is `chan-server/src/lib.rs`
  (route registration); keep your two subagents from racing on it.

## Gate

- `make ci-linux` green (fmt + clippy -D warnings + tests + build).
- No frontend files touched. Graph results unchanged; only delivery is
  paced and bounded.
