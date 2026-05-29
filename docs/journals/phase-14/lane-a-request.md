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
- `docs/journals/phase-14/addendum-1.md` (phase-13 r2 carryovers; your items are in A5)

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

### A5. Addendum-1 follow-ups (see `addendum-1.md`)

Phase-13 round-2 carryovers that fall in this lane's area:

- **De-flake the tests that gate CI / releases** (addendum-1 #2 + an
  observed CI failure on `main`). A timing/env-sensitive test must not
  red-light CI or a release. Two known offenders:
  - `chan-workspace::tests::write_text_does_not_wait_for_indexer_serial_lock`
    (the indexer-flake family, cf.
    `writes_to_drafts_subtree_get_indexed_...`): failed once on the
    v0.18.0 re-publish and gated the release. Same indexer/self-write
    area as A4 - do it alongside.
  - `chan-server::routes::terminal::tests::conditional_pty_programs_validate_real_terminal`
    (`crates/chan-server/src/routes/terminal.rs:1132`): fails on the
    GitHub macOS runner (`make ci-macos`) because the headless PTY's
    `tty` does not report a `/dev/ttys…` device path, so the
    "tty should report a device path" assertion trips. It currently
    leaves `main` CI red. Guard it for a runner with no real tty
    (detect + skip/relax) or mark it so it cannot gate CI; keep the
    real-terminal assertion when a device tty is present.
  De-flake or mark both; neither should be able to gate a release.
- **Remove the vestigial `team-work-N` draft convention** (addendum-1
  #5): nothing creates `team-work-N` dirs anymore (Team Work uses the
  standard `untitled-N` path). Drop it from `chan-workspace`
  (`drafts.rs` / `workspace.rs` / `paths.rs` comments + test examples).
  Pristine, first-public-release discipline. (Confirm it is not needed
  by the returning Team Work work - see Lane C / addendum-1 #4.)
- **WKWebView desktop walk** (addendum-1 #3): NOT code - a human
  WKWebView verify of the round-2 desktop changes (Cmd+Shift+N new
  window, Cmd+I no longer Dashboard, Cmd+P Team Work, self-upgrade
  0.17->0.18 from `/dl`). Build the dmg from the merged base
  (`make macos-chan-dmg`). Track as a verification gate, not a change.

## Coordination

- Pin `contracts.md` sections 1 and 2 first; announce in
  `event-lane-a-lane-b.md`. @@LaneB renders against them.
- The only shared file with other Rust work is `chan-server/src/lib.rs`
  (route registration); keep your two subagents from racing on it.

## Gate

- `make ci-linux` green (fmt + clippy -D warnings + tests + build).
- No frontend files touched. Graph results unchanged; only delivery is
  paced and bounded.
