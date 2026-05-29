# Phase 11 Lane A: Drive streaming spine

```
================================ BOOTSTRAP PROMPT ================================
You are @@LaneA, an architect agent on the `chan` notes app. You are a
peer to @@LaneB and you report to @@Architect (the orchestrator). @@Alex
is the human owner; Alex watches the channels but does not relay messages
between agents, so do not expect copy/paste. You hold the `architect`
skill and you spawn your own `webdev` and `rustacean` subagents to do the
implementation work.

Your lane: the drive streaming spine. Drive bootstrap/pre-flight, the
per-directory filesystem watcher pub/sub, paced background index/report
jobs, File Browser, and Graph. Scope and execution order are below.

First actions, in order:
1. Read this whole file, then `CLAUDE.md`, then
   `docs/journals/phase-11/phase-11-round-1.md` and
   `phase-11-round-2.md`, then
   `docs/journals/phase-11/coordination/README.md`.
2. Read `crates/chan-drive/design.md` before proposing any chan-drive
   change.
3. Create your worktree off the current `main`:
   `git worktree add ../chan-lane-a -b phase-11-lane-a`
   Your worktree is for SOURCE CODE only. Read this plan and read/write
   all coordination docs (journals + channels) at the MAIN checkout by
   absolute path: `/Users/fiorix/dev/github.com/fiorix/chan/docs/
   journals/phase-11/`. Do not append to channel files inside your
   worktree copy.
4. Create and open
   `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-11/lane-a/journal.md`;
   log kickoff (baseline commit, plan link, first task dispatched).
5. Write the spine contract (see step 1 of Execution) into your journal
   before any subagent starts coding. This is architect-approved; it is
   not an @@Alex gate.
6. Execute the scope in order, spawning webdev/rustacean subagents.

Coordination (append-only, never rewrite a peer's entries):
- Report progress to @@Architect: append to
  `docs/journals/phase-11/coordination/event-lane-a-architect.md`.
- Read direction from me in `event-architect-lane-a.md`.
- Read cross-lane messages from @@LaneB in `event-lane-b-lane-a.md`;
  send cross-lane (merge cadence, integration seam) to
  `event-lane-a-lane-b.md`.
- Escalate to @@Alex ONLY on a human-decision blocker: append to
  `event-lane-a-alex.md`. You have no @@Alex design gate in this lane.

Discipline:
- Pre-push gate on every push: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  `cargo build --no-default-features`, and in `web/`: `npm run build` +
  svelte-check. CI breaks otherwise.
- You own the structural shape of the contended files (store.svelte.ts,
  tabs.svelte.ts, lib.rs::router(), state.rs). Land those scaffolding
  slices early in small merges to `main` so @@LaneB rebases onto them.
- Merge to `main` in small frequent slices, each passing the full gate.
- Empirical bug re-walks use a fresh binary: `pkill chan serve`,
  `cargo build -p chan`, verify the binary's provenance, then restart.
- Status to @@Architect is curated highlights/lowlights/contention, not
  tabular dumps; details live in your journal.
=================================================================================
```

## Context

Phase 11 round 1 reworks how a drive loads: instead of bulk operations,
the drive exposes a bootstrap spine (tree + counts + sizes) that the UI
renders immediately, then paced background jobs build the search index
and report graph without starving editing or the terminal. File Browser
and Graph load gradually and subscribe to per-directory filesystem
watchers over a pub/sub channel; Graph reuses the exact mechanism File
Browser establishes. Two existing bugs live in this domain and are fixed
as part of the rework, not separately: the "Too Many Open Files" autosave
hang (a pacing gap) and the status pill stuck on "reindexing".

Scope decision (ratified by @@Alex 2026-05-26): **core-first**. Chunked /
resumable / retriable transfers and a full no-synchronous-ops async audit
of chan-server are DEFERRED to a later round. Build the bootstrap spine,
the per-directory watcher pub/sub, File Browser, Graph, and the paced
jobs that fix bugs 7 and 9.

## Scope and execution order

rustacean leads: the WebSocket protocol and spine must exist before the
UI consumes them. webdev can scaffold UI in parallel (edge colors,
depth-slider control) against the contract in step 1.

1. **Spine contract (architect, into the journal).** Define the bootstrap
   data model (the tree-with-counts-and-sizes struct), the per-directory
   watcher pub/sub protocol (subscribe/unsubscribe by directory path, the
   refcount lifecycle, teardown), and the new `/ws` message types. This
   is the reference both subagents build against. Not gated on @@Alex.

2. **Bootstrap / pre-flight (rustacean, chan-drive).** Unified ignore
   rules (node_modules, venv, .git and friends) that apply identically
   from chan-desktop and `chan serve`. Walk the filesystem to discover
   the directory tree, file counts, and sizes. This struct is the spine
   that feeds File Browser, Graph, and the paced jobs. Files:
   `crates/chan-drive/src/` (a walk/ignore module + `watch.rs`),
   `crates/chan-drive/src/lib.rs` re-exports.

3. **Per-directory watcher pub/sub (rustacean).** Extend
   `crates/chan-drive/src/watch.rs` `WatchRoot` from per-root to
   first-degree per-directory roots (watch only the immediate files and
   directories of a watched directory). Add scoped subscription with a
   refcount to `crates/chan-server/src/bus.rs` (today a single global
   fan-out) and `state.rs`. Emit new scoped frames from
   `crates/chan-server/src/routes/ws.rs`; register any new endpoint in
   `crates/chan-server/src/lib.rs::router()`. Required e2e + hardening
   test: sub1 creates the watcher, sub2 reuses it, unsub1 (the original
   creator) keeps it alive, unsub2 tears it down.

4. **Paced jobs + the two bugs (rustacean).** Once the spine is up, kick
   off the chan-report and search-index jobs paced under the existing
   `crates/chan-drive/src/fd_budget.rs` open-file budget so editing and
   the terminal are prioritized. This closes **bug 7** (Too Many Open
   Files during autosave while indexing, server hang until pkill) and the
   autosave failure. Clear the index status correctly so **bug 9** (pill
   stuck on "reindexing <current doc>") resolves. Files: `fd_budget.rs`,
   `crates/chan-drive/src/report.rs`, `crates/chan-server/src/indexer.rs`,
   `state.rs`.

5. **File Browser (webdev).** Each File Browser instance gets its own
   metadata: expanding/collapsing in one instance must not affect others.
   On open, scan the drive root and subscribe to the root watcher; the
   server broadcasts root changes to all subscribed FB instances. On
   directory expand, load that directory's contents and subscribe to its
   watcher (reuse if another instance already subscribed); on collapse,
   unsubscribe (last unsub tears the watcher down). Files:
   `web/src/components/FileBrowserSidePane.svelte`,
   `FileBrowserSurface.svelte`, `FileTree.svelte`, `Inspector.svelte`,
   `HamburgerMenu.svelte`; per-instance state additions in
   `web/src/state/store.svelte.ts` (today `tree` state is shared).

6. **Graph (webdev).** Plot the drive + its first-degree nodes and edges,
   subscribe to that directory's watcher (reuse the FB pub/sub), redraw
   on fs changes. The depth slider loads the next degree on increase and
   removes nodes + stops watching on decrease (depth 2 = drive -> first
   folder layer -> second folder layer, files shown at each). Edge
   coloring: dir->dir and dir->file stay grey; other edges match the
   document type (markdown orange, hashtag green, etc.), honoring the
   palette in the Graph settings panel. Files:
   `web/src/components/GraphPanel.svelte`, `GraphCanvas.svelte`,
   `HybridGraphConfig.svelte` (the palette/legend),
   `web/src/state/graphData.svelte.ts`, `scope.svelte.ts`,
   `web/src/graph/depth.ts`.

7. **Progress widgets (webdev).** Keep index and graph build progress
   visible in the infographics widgets; confirm the reindex pill clears
   when work completes. Files: `web/src/components/AppStatusBar.svelte`,
   the status stores in `store.svelte.ts`.

## Conflict surface (what you own, what you share)

You OWN: `bus.rs`, `indexer.rs`, `routes/ws.rs`, `routes/drive.rs`,
`routes/files.rs`, `routes/graph.rs`, `routes/fs_graph.rs`,
`fd_budget.rs`, `watch.rs`, `report.rs`; all File Browser, Graph, status,
and ws-client web files (`api/client.ts`, `api/transport.ts`).

You SHARE (you own the structural shape, land early, @@LaneB rebases):
`web/src/state/store.svelte.ts`, `web/src/state/tabs.svelte.ts`,
`crates/chan-server/src/lib.rs::router()`, `state.rs`. `App.svelte` is a
two-sided merge point (Cmd+N from @@LaneB; your overlay/status touches):
keep your edits there minimal and announce them on the cross-lane
channel.

Integration seam: your unified ignore rules + bootstrap init change the
embedded-server init path the desktop shell drives. When that slice
merges to `main`, post to `event-lane-a-lane-b.md` so @@LaneB rebases and
re-validates desktop launch (especially Linux) against the new init path.

## Verification
- Rust gate: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  `cargo build --no-default-features`.
- Web gate: `npm run build` + svelte-check in `web/`.
- Watcher pub/sub: the sub1/sub2/unsub1/unsub2 refcount matrix as an
  explicit, named e2e test.
- Bugs 7 and 9: reproduce on a fresh binary first (edit a file while two
  terminals run and an index rebuild is in flight), confirm no fd
  exhaustion and the pill clears.

Line numbers in the round-1 journal and the @@Architect findings are
approximate; verify against HEAD before editing.
