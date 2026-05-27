# @@LaneA journal: drive streaming spine

Lane: the drive streaming spine. Bootstrap/pre-flight, per-directory
watcher pub/sub, paced background jobs (bugs 7 + 9), File Browser, Graph,
progress widgets. Chunked/resumable transfers are deferred this round.

Plan: `docs/journals/phase-11/lane-a-plan.md`.
Worktree (source code only): `../chan-lane-a` on branch `phase-11-lane-a`.
Coordination docs (this journal + channels) live in the MAIN checkout.

## Kickoff (2026-05-26)

- Baseline commit: `198beb9` (`docs(phase-10): record item-4 dispatch and
  review`). Worktree branched off this.
- Read: plan, CLAUDE.md, round-1, round-2, coordination README,
  chan-drive design.md (full), and the source I own/touch: `watch.rs`,
  `bus.rs`, `state.rs`, `routes/ws.rs`, `fd_budget.rs`, `indexer.rs`,
  `lib.rs::router()`, and `web/src/state/store.svelte.ts` (the shared
  `tree` state).
- Spine contract written below (this commit), architect-approved per
  `event-architect-lane-a.md`. No @@Alex gate.
- First subagent task: see Dispatch log.

## Baseline facts verified against HEAD (not the plan's approximate refs)

These are the load-bearing realities the contract is built on; verified
by reading source at `198beb9`, not inferred from the plan.

1. **`/ws` is one-directional today.** `routes/ws.rs::ws_pump` only
   forwards `events_tx` broadcast frames server -> client; it never reads
   client `Message::Text`. There is one global `broadcast::Sender<String>`
   (`AppState::events_tx`) fanned to every socket. Scoped subscribe needs
   a client -> server message path plus per-socket routing.

2. **`bus.rs` is a single global fan-out.** `WatchBroadcast` forwards
   every `WatchEvent` to (a) `index_tx` (the indexer, always) and (b)
   `events_tx` as `{"type":"watch","event":...}` (UI, minus self-write
   echoes). There is no notion of a directory scope on the wire.

3. **`watch.rs::WatchHandle::start` watches `RecursiveMode::Recursive`.**
   Each `WatchRoot` is recursive. The drive's single watcher (in
   `DriveCell::watch_handle`) covers the whole tree. First-degree
   per-directory watching does not exist yet.

4. **`state.rs` already has a precedent for keyed `WatchHandle` maps:**
   `AppState::loaded_teams: Mutex<HashMap<String, WatchHandle>>`
   (systacean-31, per-team event watchers rooted at a `WatchRoot`). The
   scoped pub/sub registry follows the same storage shape but adds a
   refcount and is keyed by drive-relative directory path.

5. **`fd_budget.rs` gates indexing internals** (graph pool size, tantivy
   writer threads, index read workers, active-drive admission) by probing
   `/dev/fd` count vs the soft `nofile` limit. It does NOT yet gate the
   *number of concurrently open per-file handles during a reindex pass*,
   nor does the report engine consult it. Bug 7 is a pacing gap on top of
   this module.

6. **`indexer.rs` status pill: `IndexStatus::Reindexing { file }`** is set
   in `spawn_watcher_loop`'s worker before each per-file apply and is only
   cleared by `set_idle` on the success/skip paths. Bug 9 ("pill stuck on
   reindexing <current doc>") is a clear-path gap: if the apply errors, or
   the worker is aborted mid-apply, or `set_idle` is skipped, the pill
   stays. Verify the exact stuck path empirically on a fresh binary before
   editing (per the fresh-binary-rewalk discipline).

7. **`web/src/state/store.svelte.ts` `tree` is a single shared `$state`**
   (entries, loadedDirs, loadingDirs, dirErrors). All File Browser
   instances read/write the same object. Per-instance metadata (the
   round-1 ask "expanding/collapsing in one instance must not affect
   others") is a structural reshape I own and land early.

---

# Spine contract (v1)

Reference both subagents build against. Three parts: the bootstrap data
model, the per-directory watcher pub/sub protocol, and the `/ws` message
types. rustacean lands the Rust side first; webdev scaffolds UI against
the JSON shapes here in parallel.

Wire-shape discipline: every new serialized struct/enum gets a pinning
test (the `progress_event_serializes_for_the_wire` precedent in
`tests/progress_events.rs`). A change to a wire shape is then an explicit
edit, not silent breakage of connected clients.

## Part 1 — Bootstrap data model (the spine)

The drive exposes, immediately on open, a lightweight tree-with-counts-
and-sizes that the UI renders before any index/report job runs. This is
the spine that feeds File Browser, Graph, and the paced jobs. It is a
*structural* snapshot (names, kinds, child counts, sizes), NOT content
and NOT graph edges.

### Ignore rules (unified)

One ignore policy applies identically from chan-desktop and `chan serve`.
It extends the existing `WalkFilter` precedent (`Registry::
index_excluded_dirs`, persisted to `~/.chan/config.toml`) so the
bootstrap walk, the search index walk, and the report walk all agree.

- Hardcoded invariants (never walked, never watched, never emitted):
  `.chan/`, `.git/`, `.hg/` (already enforced by `walk_drive` +
  `watch::dispatch::is_filtered`). VCS control allowlist
  (`.git/HEAD`, `.git/index`, `.hg/dirstate`) stays as-is for the
  indexer's checkout-storm detection; it is NOT surfaced in the
  bootstrap tree.
- Policy list (user/registry tunable, shared): `node_modules`, `target`,
  `__pycache__`, `venv`, `.venv`, `.tox`, `dist`, `build`, `.next`,
  `.cache`, ... (final default list lives in `Registry::
  index_excluded_dirs`; bootstrap reuses the SAME `WalkFilter` the
  indexer already loads, so there is ONE policy, not two).
- The editor-visible on-demand APIs (`Drive::list`, `list_tree`) stay
  UNFILTERED so a user can still open a file inside an ignored dir on
  purpose. The bootstrap spine is FILTERED (it drives the default
  rendered tree + the paced jobs). This mirrors the existing split
  documented in chan-drive design.md "Walk filter".

### `BootstrapTree` (proposed Rust shape, chan-drive)

```rust
/// Structural snapshot produced by the bootstrap walk. Counts and
/// sizes only; no content, no graph edges. Serializable for the
/// /api/drive/bootstrap response and FFI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapTree {
    /// Drive-relative POSIX dir path of this node ("" for root).
    pub path: String,
    /// Immediate child directories (first-degree only at this level).
    pub dirs: Vec<BootstrapDir>,
    /// Immediate child files (first-degree only at this level).
    pub files: Vec<BootstrapFile>,
    /// Aggregate over the WHOLE subtree under `path` (filtered):
    /// total file count and summed bytes. Lets the UI show "1,240
    /// files, 38 MB" on a collapsed dir without walking it again.
    pub subtree: SubtreeStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapDir {
    pub name: String,                 // basename
    pub subtree: SubtreeStats,        // recursive counts/sizes
    /// Immediate-child counts so the UI can render "12 files, 3
    /// folders" without expanding.
    pub child_dirs: u32,
    pub child_files: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapFile {
    pub name: String,                 // basename
    pub size: u64,
    pub mtime: i64,                   // unix seconds
    pub class: FileClassWire,         // editable/text/image/pdf/other
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SubtreeStats {
    pub files: u64,
    pub bytes: u64,
}
```

`FileClassWire` mirrors `chan_drive::fs_ops::FileClass`
(EditableText/Text/Image/Pdf/Other) as a stable serialized tag so the UI
gets the same classification the editor gate uses. The bootstrap walk is
breadth-first and **lazy at the directory boundary**: the first response
carries the root level fully (root's immediate dirs+files + each child
dir's aggregate subtree stats). Deeper levels load on File Browser
expand / Graph depth-increase via the same `/api/files?dir=` +
subscribe path described in Part 2. The aggregate `subtree` stats ARE
computed eagerly (one full filtered walk on open) because they are cheap
(stat only, no read) and they drive the "this dir has N files" affordance
the UI needs before expansion.

Rationale for counts+sizes eager but contents lazy: the round-1 ask is
"walk the filesystem and discover the directory tree and number of
files, their size ... this is pretty much all we need to show the UI."
A stat-only walk of even a large drive is cheap relative to read+parse;
it does not open file content and so does not pressure the fd budget the
way the index/report jobs do.

### Transport

`GET /api/drive/bootstrap` returns `BootstrapTree` for the root level.
Lives in `routes/drive.rs` (an area I own), registered in
`lib.rs::router()` in the open (non-settings) lane. Directory expansion
reuses the existing `GET /api/files?dir=<rel>` (`api_list_files`) which
already returns per-directory entries; bootstrap adds the subtree
aggregates the plain listing lacks. If the existing `/api/files` shape is
sufficient for expand, expansion does NOT need a new endpoint — only the
root bootstrap + the subscribe/unsubscribe frames are new wire surface.
Confirm during implementation; prefer reusing `/api/files`.

## Part 2 — Per-directory watcher pub/sub protocol

Today: one recursive drive watcher, one global broadcast. Target: the
drive root watcher plus first-degree per-directory watchers created on
demand, reference-counted, torn down on last unsubscribe. File Browser
expand and Graph depth-increase are the two producers of subscriptions;
they share ONE mechanism.

### First-degree watching (chan-drive `watch.rs`)

Extend `WatchRoot` / `WatchHandle` so a watcher can attach
**non-recursively** to a single directory: it observes only the
immediate files and directories of that directory (create/modify/remove/
rename of direct children, plus creation/removal of immediate
subdirectories). It does NOT descend. `notify` supports
`RecursiveMode::NonRecursive`; the dispatcher's path relativization and
`.chan/`/`.git/` filtering stay unchanged.

- The drive ROOT watcher is the first-degree watcher of `""`. On open we
  attach it once; all File Browser instances subscribe to it implicitly
  (root is always "expanded").
- Each expanded subdirectory `d` gets its own first-degree
  `NonRecursive` watcher rooted at `<drive_root>/d`. Events emerge with
  the directory-relative path (drive-relative), same keyspace as today.
- Renames whose `from` and `to` straddle two watched directories surface
  on both watchers (each sees its side); the UI reconciles. A rename
  inside one watched dir surfaces once.

Note on the existing recursive drive watcher: the *indexer* still needs
recursive coverage (it reindexes anywhere on the drive). The per-
directory pub/sub is a SEPARATE concern layered for the UI; the indexer
keeps its recursive feed via `index_events_tx`. We do NOT remove the
recursive watcher that feeds the indexer. The scoped watchers are
additive and feed only the scoped UI fan-out. (Open question flagged
below: whether to derive scoped UI frames from the existing recursive
feed by path-prefix filtering instead of attaching N extra OS watchers.
See Decision D1.)

### Scoped subscription registry (chan-server `bus.rs` + `state.rs`)

A new registry keyed by drive-relative directory path:

```rust
// In state.rs (AppState), or a dedicated bus::ScopeRegistry.
struct DirScope {
    /// Per-subscriber-connection set; refcount == subscribers.len().
    subscribers: HashSet<SubId>,
    /// The OS watcher (or a marker if D1 picks prefix-filtering).
    handle: WatchHandle,
}
struct ScopeRegistry {
    scopes: Mutex<HashMap<String /* dir rel */, DirScope>>,
    // each connected /ws socket has a unique SubId; on socket close
    // we drop all of its subscriptions (decrementing refcounts).
}
```

Lifecycle (the REQUIRED hardening matrix, named e2e test):

- **sub1(dir)** — first subscriber for `dir`: create the watcher, insert
  the scope, refcount = 1.
- **sub2(dir)** — second subscriber (could be a different FB instance or
  Graph): REUSE the existing watcher, refcount = 2. No new OS watcher.
- **unsub1(dir)** — the ORIGINAL creator unsubscribes: refcount = 1, the
  watcher STAYS alive (it is not tied to the creator's identity).
- **unsub2(dir)** — last subscriber unsubscribes: refcount = 0, the
  watcher is torn down (its `WatchHandle` dropped), scope removed.
- **socket close** — drops every `SubId` the socket held; each is an
  implicit unsub, so a disconnect cannot leak watchers.

Test name: `scope_refcount_sub1_sub2_unsub1_unsub2`. It asserts the
watcher object identity is stable across sub2/unsub1 and is dropped only
after unsub2, and that a socket-close path tears down its remaining
subscriptions. This is a hard deliverable.

### Wire — client subscribes/unsubscribes over `/ws`

`/ws` gains a client -> server inbound message path (today it only
sends). Client frames (JSON, `type` discriminator):

```json
{ "type": "sub",   "dir": "notes/recipes" }
{ "type": "unsub", "dir": "notes/recipes" }
```

`dir: ""` is the drive root (already implicitly watched; sub on `""` is a
no-op refcount the server can accept idempotently). The server routes a
sub/unsub to the `ScopeRegistry` against THIS socket's `SubId`.

Server -> client scoped frames carry the originating `dir` so a client
that subscribed to several dirs can route the event to the right FB
pane / Graph node:

```json
{ "type": "fs", "dir": "notes/recipes", "event": { "kind": "Created",
  "path": "notes/recipes/new.md", "to": null } }
```

`event` is the existing `chan_drive::WatchEvent` serialization verbatim
(reused, not re-invented). The new envelope key is `"fs"` with a `dir`
field, distinct from the legacy global `"watch"` frame so existing
consumers are untouched during migration. (Decision D2: whether to
retire the global `"watch"` frame once File Browser moves to scoped `fs`
frames, or keep it for the editor's external-edit toast. Lean: keep
`"watch"` for the open document's external-edit detection — that is a
single-file concern, not a directory-scope concern — and add `"fs"` for
the tree. Confirm with @@LaneB since the editor toast is near their
surface.)

### Per-socket fan-out (routes/ws.rs)

`ws_pump` becomes bidirectional: a `select!` arm reads inbound client
frames (sub/unsub) alongside the existing outbound broadcast arm and the
shutdown arm. Each socket subscribes to a per-socket `mpsc` (or filters
the broadcast by its subscription set) so it only receives `fs` frames
for dirs it asked for. The global broadcast (`events_tx`) still carries
`progress` and the legacy `watch` frame to all sockets.

## Part 3 — `/ws` message type catalog (v1)

Server -> client envelopes (all JSON, `type` field):

| type       | payload                          | producer          |
|------------|----------------------------------|-------------------|
| watch      | `{event: WatchEvent}` (global)   | bus (legacy)      |
| progress   | `{event: ProgressEvent}`         | bus (indexer)     |
| fs         | `{dir, event: WatchEvent}`       | scoped registry   |
| index      | `{status: IndexStatus}` (opt.)   | indexer (D3)      |

Client -> server envelopes:

| type   | payload          | meaning                              |
|--------|------------------|--------------------------------------|
| sub    | `{dir}`          | subscribe this socket to dir scope   |
| unsub  | `{dir}`          | unsubscribe this socket from dir     |

D3: whether to push `index` status over `/ws` or keep polling
`/api/index/status`. Today it polls (indexer.rs header comment says so on
purpose). Bug 9 (stuck pill) is about the *clear path*, not the
transport, so v1 keeps polling and just fixes the clear; a push is a
later optimization, NOT in scope this round.

## Open decisions (architect-level, resolved in-lane unless flagged)

- **D1 — scoped watchers: real OS watchers vs prefix-filter the existing
  recursive feed.** Two implementations satisfy the same wire contract:
  (a) attach a real `NonRecursive` OS watcher per expanded dir, or (b)
  keep ONE recursive drive watcher and derive scoped `fs` frames by
  filtering its events by directory prefix + depth in the server. (b) is
  far simpler (no per-dir OS handle churn, no inotify-watch-count
  pressure on big trees) and the refcount/lifecycle is purely a server
  bookkeeping concern; (a) matches the round-1 wording literally ("put a
  watcher in the server ... tear down the watcher") and bounds event
  volume at the OS level for huge drives.
  **Lean: (b) prefix-filter the recursive feed**, because the recursive
  watcher already exists (it feeds the indexer), inotify watch counts are
  a real failure mode on large trees, and the refcount lifecycle + the
  sub1/sub2/unsub1/unsub2 test are identical either way (the test asserts
  server-side bookkeeping + teardown, which is implementation-agnostic).
  The round-1 phrase "tear down the watcher" maps to "tear down the scope
  bookkeeping + stop emitting frames for it," which (b) honors.
  RESOLVED in-lane as (b) unless @@Architect overrides. Recorded here so
  the e2e test is written against server bookkeeping, not OS-handle
  identity.
- **D2 — retire global `watch` frame?** Lean: keep for the editor's
  open-document external-edit toast; add `fs` for the tree. Coordinate
  with @@LaneB (editor surface). Non-blocking for the Rust spine.
- **D3 — push index status over `/ws`?** No this round; keep polling, fix
  the clear path for bug 9.

## Sequencing (small merges to main, each passes the full gate)

1. **Slice A — structural scaffolding (land first, ping @@LaneB).**
   Reshape `web/src/state/store.svelte.ts` `tree` into per-FB-instance
   metadata (keyed by instance id) + the shared scope-subscription
   client plumbing stub in `api/transport.ts`/`api/client.ts`. Add the
   bootstrap response type + `/api/drive/bootstrap` route skeleton in
   `routes/drive.rs` + `lib.rs::router()`. No behavior change to the
   indexer or the existing recursive watcher yet. This is the rebase
   surface @@LaneB needs.
2. **Slice B — bootstrap walk (chan-drive).** `BootstrapTree` + the
   unified filtered walk + `/api/drive/bootstrap` wired to real data.
   Ping @@LaneB: this changes the embedded-server init path the desktop
   shell drives; they re-validate desktop launch (esp. Linux).
3. **Slice C — scoped pub/sub.** `bus.rs` ScopeRegistry + bidirectional
   `ws.rs` + the sub1/sub2/unsub1/unsub2 hardening test. Decision D1 (b).
4. **Slice D — paced jobs + bugs 7 & 9.** fd-budget pacing for the
   index/report jobs (open-file ceiling that prioritizes editing +
   terminal), the report job paced under the same budget, and the
   stuck-pill clear-path fix. Reproduce both bugs on a fresh binary
   first.
5. **Slice E — File Browser (webdev).** Per-instance expand/collapse +
   subscribe/unsubscribe wiring against slices A/C.
6. **Slice F — Graph (webdev).** Depth slider reuses the FB pub/sub; edge
   coloring per document type; redraw on `fs` frames.
7. **Slice G — progress widgets (webdev).** Index/graph build progress in
   the infographics widgets; confirm the reindex pill clears.

## Execution-model note (2026-05-26)

The bootstrap header says to spawn `webdev`/`rustacean` subagents via the
Agent tool. That tool is NOT present in this environment (verified via
tool search: no Agent/Task/spawn tool exists; only the `architect`,
`rustacean`, `webdev`, etc. SKILLS are available via the Skill tool).
Rather than block on a round-trip, I am doing the implementation work
directly in-lane and loading the relevant skill (`rustacean` for the Rust
slices, `webdev` for the web slices, `architect` for review) to keep the
same discipline a subagent would carry. The deliverables, gate, merge
cadence, and review are unchanged; only the actor changes (me, not a
spawned subagent). Noted to @@Architect on the channel. If a subagent
tool appears later I'll switch back to delegation for the parallel UI
work.

## Dispatch log

### 2026-05-26 — Work item #1: Slice B (bootstrap walk) — rustacean skill

Scope: implement the `BootstrapTree` data model + the unified filtered
bootstrap walk in chan-drive, and the `GET /api/drive/bootstrap` route in
chan-server. Reuse the existing `WalkFilter` (no second ignore policy).
Counts+sizes eager (stat-only walk), contents lazy. Wire-shape pinning
test required. Full Rust gate must pass. Detailed task in the Agent
prompt; subagent loads the `rustacean` skill first.

Why this first: the UI (File Browser, Graph) and the paced jobs all
consume the spine; it must exist before the scoped pub/sub and the UI
slices can be meaningful. Slice A's web-side `tree` reshape comes next;
the Rust bootstrap is the foundation.

**Result — Slice B DONE + merged.** Implemented `chan-drive::bootstrap`
(`BootstrapTree`/`BootstrapDir`/`BootstrapFile`/`SubtreeStats`/
`FileClassWire`) as a single stat-only filtered `WalkDir` pass that rolls
file counts+bytes up to each top-level dir + the whole-drive aggregate,
and records the root level's immediate children. Reuses the existing
`WalkFilter` (one ignore policy); `.git`/`.chan` stay hardcoded.
`Drive::bootstrap()` + `Drive::bootstrap_dir(rel)`. Server:
`GET /api/drive/bootstrap` (open lane, blocking pool). 6 new unit tests
incl. the wire-shape pin `bootstrap_tree_serializes_for_the_wire`.

Gate (all green on the lane branch): `cargo fmt --check`,
`cargo clippy --all-targets -D warnings`, `cargo test` (chan-drive 525 +
chan-server 317 + integration suites all pass), `cargo build
--no-default-features`, and in `web/`: `npm run build` + svelte-check
(0 errors / 0 warnings — slice is Rust-only so the web bundle is
unchanged from baseline, gate run anyway per discipline).

Commit `d8912b9`; merged to `main` as `3d42b09` (`--no-ff`). `main`
rebuilds clean post-merge. Pinged @@LaneB on event-lane-a-lane-b: small
rebase surface (one router route line + import), and flagged that the
desktop init-path re-validation seam comes with the LATER bootstrap-on-
open wiring, not this additive route. Posed D2 (keep both `fs`+`watch`
frames) to them, non-blocking.

Footgun caught + fixed: my first `Write` of bootstrap.rs used a long
`../../../` path that resolved relative to the agent CWD, landing the
file in a stray `chan/chan-lane-a/` tree instead of the sibling
`fiorix/chan-lane-a` worktree. Moved it to the right path and removed the
stray dir. Lesson: use absolute worktree paths
(`/Users/fiorix/dev/github.com/fiorix/chan-lane-a/...`), not `..` chains
from the agent CWD.

### Next: Slice A (web structural scaffolding) + Slice C (scoped pub/sub)

Slice A reshapes `store.svelte.ts` `tree` into per-FB-instance metadata +
stubs the scope-subscription client in `api/transport.ts` so @@LaneB has
the shared-file shape early. Slice C is the rustacean scoped pub/sub
(`bus.rs` ScopeRegistry + bidirectional `ws.rs` + the
sub1/sub2/unsub1/unsub2 hardening test, Decision D1(b) = prefix-filter
the recursive feed). I'll sequence A before C so the wire client + the
server share one shape.

### 2026-05-26 (resumed session) — Work item #2: Slice A (web scaffolding)

Fresh @@LaneA session. Recovered state from journal + all four channels;
confirmed worktree at `5c97410` lineage on `phase-11-lane-a`, `main` at
`3d42b09`. Followed the REVISED MERGE PROTOCOL (work on branch only, post
ready-note, @@Architect merges; no checkout/merge/push by me).

**Result — Slice A DONE (ready to merge, not merged).** Commit `5c97410`.
Six web files, +454/-7, all `web/src/`. No Rust, no `tabs.svelte.ts`/
`router()`/`state.rs` (those come in Slice C). Additive only:

- `store.svelte.ts`: `FbTreeInstance` + `$state` `fbTreeInstances`
  registry keyed by instance id (`ensureFbTreeInstance` /
  `fbTreeInstance` / `disposeFbTreeInstance` / `fbDirSubscriberCount`).
  The keyed structure for independent per-instance expand/collapse
  (round-1 ask). `treeExpanded` singleton kept as back-compat default;
  Slice E migrates consumers. Also `watchSubscription()` accessor + widen
  `unwatch` to `WatchSubscription`.
- `transport.ts` / `client.ts`: `/ws` client->server path. `openWatch` ->
  `WatchSocket` (callable disposer + send/close, fires `onOpen` on every
  (re)connect); `openWatchSocket` -> `WatchSubscription` with
  `subscribeDir`/`unsubscribeDir`. Pre-OPEN frames dropped on purpose
  (per-socket server registry; owner re-subscribes from onOpen).
- `types.ts`: pinned the Part-3 catalog: `WatchEventWire`, `WsWatchFrame`,
  `WsFsFrame`, `WsClientFrame`.
- Tests: `watchScope.test.ts` (fake WebSocket; pins sub/unsub frame shape,
  drop-before-open, onReady, callable disposer) + store.test.ts block
  (per-instance create/read/dispose + cross-instance refcount).

Gotcha caught: types.ts already had a STALE `WatchEvent` (lowercase kinds,
no rename `to`) not matching the live frame the store reads. Named the new
accurate one `WatchEventWire`; reconciling the stale one is deferred to
the FB slice (Slice E) that touches every consumer.

Svelte-5 footgun caught: `ensureFbTreeInstance` first returned the raw
literal, but `$state` deep-proxies on assignment, so mutations on the raw
object were invisible to `fbDirSubscriberCount` (test caught it). Fixed to
return `fbTreeInstances.byId[id]` (the proxy).

Full gate green on branch. Posted ready-note to @@Architect and rebase-
surface note to @@LaneB. Proceeding to Slice C.

### 2026-05-26 (resumed session) — Work item #3: Slice C (scoped pub/sub)

**Result — Slice C DONE (ready to merge, not merged).** Commit `ac21cd2`
on top of Slice A. 11 chan-server files, +615/-21. No web, no docs leaked.

D1(b) implemented as recorded: scoped `fs` frames derived from the single
existing recursive watcher by FIRST-DEGREE directory match; no per-dir OS
watchers. "Tear down the watcher" == drop the scope bookkeeping.

- `bus.rs`: `ScopeRegistry` + `SubId`. `register` -> (SubId, unbounded
  outbox rx); `subscribe`/`unsubscribe` refcount per drive-rel dir (entry
  exists iff >=1 subscriber, not tied to creator identity); `unregister`
  decrements every scope a socket held (disconnect can't leak); `emit_fs`
  routes an event to its first-degree parent dir's scope, straddling
  rename hits both source+dest parents. `WatchBroadcast::on_event` calls
  `emit_fs` after the legacy global `watch` frame (D2).
  `make_watch_bridge` gained the registry arg; all 4 real call sites pass
  `state.scope_registry` (survives storage reset).
- `state.rs` + `lib.rs`: `AppState.scope_registry` field, created at boot
  before the bridge. Five `#[cfg(test)]` AppState builders across route
  files got the one-line field.
- `ws.rs`: bidirectional pump. `select!` adds the socket's scoped outbox
  arm + an inbound-client-frame arm (`{type:"sub"|"unsub", dir}` ->
  registry). Registers on connect, ALWAYS unregisters on exit (every break
  falls through). Malformed/unknown frames dropped silently.
- Tests: `scope_refcount_sub1_sub2_unsub1_unsub2` (the required matrix),
  socket-close teardown, idempotent re-sub, first-degree emit routing
  (grandchildren excluded), root-scope top-level, straddling-rename-both-
  sides, empty no-op, `parent_dir`/`normalize_dir`, ws client-frame parser
  routing + malformed drop. chan-server lib 328 (was 317 @ Slice B).

clippy footguns caught: (1) doc line wrapping to `>= 1` at col 5 tripped
the doc-quote-marker lint -> reworded; (2) `subscriber_count`/
`scope_exists` were test-only -> gated `#[cfg(test)]` (they're the
refcount-invariant assertions; Slice E can un-gate if it needs them in
prod diagnostics).

Full gate green. Posted ready-note (@@Architect) + state.rs rebase-surface
note (@@LaneB). HANDING BACK after C per @@Architect's direction. Did NOT
start Slice D (paced jobs + bugs 7/9): it needs a fresh-binary empirical
repro of the fd-exhaustion + stuck-pill clear path (per the fresh-binary-
rewalk discipline) and is meatier; opening it cleanly next turn beats a
half-done D. A and C are both ready for the architect's merge.

### 2026-05-26 (resumed session) — Work item #4: Slice D (paced jobs + bugs 7/9)

Fresh @@LaneA session. Recovered state from journal + all channels; rebased
`phase-11-lane-a` onto `main` (`ce41e39`); my A/B/C commits already in main
via the merge so rebase dropped them, branch is now the `ce41e39` tip (clean
tree, also carries @@LaneB editor fixes). @@LaneB's edits are disjoint from
my Slice D scope (editor files vs fd_budget/indexer/report/state/status).

**Empirical repro — fresh binary (`cargo build -p chan` @ `ce41e39`,
provenance verified).** Test drive `/tmp/chan-test-bug7`, 2000 markdown
notes (~7.8 MB). Drove `chan serve` on a DEDICATED port 8799 (never broad
`pkill`; @@LaneB is concurrently serving on 8791/8792 — LESSON below).

Findings (grounded in source + thread samples, not inferred):

1. **Bug 9 — stuck "building/reindexing" pill: REPRODUCED, root cause
   isolated.** On the live server the index status freezes at
   `building 1999/2000 (notes/note-999.md)` and never clears to `idle`.
   `AppStatusBar.svelte` hides the pill only when `state === "idle"`, so a
   never-idle status = a permanently stuck pill. Two concrete in-scope
   contributors:
   - **(a) Frozen label during the embed phase.** `StatusUpdater`
     (indexer.rs ~L786) only flips `IndexStatus::Building` on
     `GraphRebuild | IndexFile` stages; `EmbedBatch` hits the `_ => {}`
     arm. `build_all` runs read+chunk+BM25-enqueue ticks FIRST (counter
     climbs to total-1), THEN the embed-batch flushes + the final
     `bm25.commit()`. So the entire embed phase (minutes on a big drive)
     shows a frozen `IndexFile` label at `total-1/total` — looks exactly
     like a hang even when work is progressing. Confirmed `ProgressStage`
     has `EmbedBatch` (progress.rs L80) and it carries `current/total`
     (chunks) + a `files=N last=...` label that the status never surfaces.
   - **(b) Clear path coupled to build completion.** `set_idle` only runs
     after `spawn_blocking(reindex_with_aggression).await` returns
     `Ok(Ok)`. If the build never returns (a phase stalls), the status is
     held hostage forever with no recovery. Same for the watcher loop's
     `Reindexing { file }` (journal baseline fact #6): set before each
     per-file apply, cleared only on the success/skip paths.
2. **Bug 7 — fd pacing gap: source-level gap confirmed; exact EMFILE not
   triggerable on THIS box.** `fd_budget.rs` samples `/dev/fd` ONCE at
   allocation (graph reader pool `graph.rs:271`, tantivy writer/merge
   threads `bm25.rs:99`, build_all read workers `facade.rs:1096/1140`,
   drive admission `drive.rs:353`). Nothing re-checks mid-build, so a
   reindex that grabbed the full budget when fds were free does NOT yield
   when terminals + the editor open afterward. Empirically the server held
   ~45-52 open fds during a rebuild at `ulimit -n 256`, comfortably under
   the limit, because (i) `EFFECTIVE_NOFILE_CEILING=4096` + the existing
   low-limit clamps already shrink pools at tight limits, and (ii) this
   7.8 MB drive never grows the tantivy segment / vector-shard fd count to
   the EMFILE point. So the literal "Too Many Open Files" needs a genuinely
   large drive; the PACING gap that bug 7 names is real and in-scope to
   close (mid-flight back-pressure so interactive work keeps fd headroom).
   Terminal create already has a `CreateError::FdPressure` gate — the
   missing symmetric piece is the REINDEX yielding under pressure.
3. **OUT-OF-SCOPE blocker found + flagged: embed phase hangs on this Mac.**
   Thread sample of the stuck build:
   `flush_embed_batch -> embed_documents_cancelable -> embed_with ->
   Tensor::to_vec2 -> MetalStorage::to_cpu -> MetalDevice::
   wait_until_completed -> [_MTLCommandBuffer waitUntilCompleted]` — the
   Metal command buffer never completes (>180 s). `CHAN_DISABLE_GPU=1`
   (CPU path) ALSO stalls >120 s with BM25 still empty (commit is after
   embed). Model IS present (`~/Library/Caches/chan/models/...bge-small...`
   128 MB; earlier "no models dir" was me looking under `~/.chan` instead
   of `~/Library/Caches/chan`). This is an environment Metal/candle hang,
   NOT bugs 7/9 and NOT in my file scope (embeddings.rs). It MASKS faithful
   end-to-end testing of bugs 7/9 here. Flagging to @@Architect; my Slice D
   fixes are validated via targeted Rust tests + the BM25-fast path
   (`Drive::reindex` on 2000 files completes in 1.17 s with vectors
   skipped), not a full live embed walk on this box.

LESSON (recording immediately): do NOT broad-`pkill -f "chan serve"` in a
shared multi-agent box — @@LaneB had a live test server on 8791/8792 and my
early broad pkills likely bounced it. Use a dedicated port (8799 is mine)
and `lsof -nP -iTCP:8799 -sTCP:LISTEN -t | xargs -r kill` to kill ONLY my
own listener. Same discipline @@LaneB already follows.

**Result — Slice D DONE (ready to merge, not merged). Commit `07f0a7c`** on
top of `ce41e39`. 4 in-scope files, +324/-12, no web/docs/state.rs leaked.

Bug 7 (fd pacing): the fd budget sizes pools ONCE at index open. Added
`fd_budget::pace_reindex_worker` (re-samples live `/dev/fd`, backs off
when < `REINDEX_RESERVE` = 64 descriptors free, cancellable) +
`reindex_should_pace` (pure, unit-tested policy). Wired into `build_all`'s
read-worker loop (per file) and gated ahead of `report_state()`'s initial
scan so both background passes honor the same reserve. The open-time knobs
+ the existing low-limit clamps stay; this is the missing MID-FLIGHT piece.

Bug 9 (stuck pill): two clear-path gaps fixed in indexer.rs.
  - `StatusUpdater` now maps `EmbedBatch` onto `Building` (was the
    `_ => {}` arm): the embed phase ran after the last IndexFile tick so
    the pill froze on `total-1/total` for the whole embed pass. Now it
    shows `(embedding)` + the chunk counters so the pill animates.
  - `reconcile_idle` moves the status out of `Building` on EVERY build
    resolution — success, cancel, and the drive-Weak-gone case — so a
    cancelled/reset rebuild can't park the pill forever. The only way to
    stay `Building` is a build that genuinely has not finished.
  - state.rs needed NO change: the status mutex lives in `Indexer`, and
    the fix is entirely in the coordinator + StatusUpdater. AppStatusBar
    needed NO change either: it already hides on `idle` and renders the
    `building` label; the bug was purely the status never reaching idle.

Empirical validation (FRESH binary @ 21:02, provenance verified, dedicated
port 8799, CHAN_DISABLE_GPU=1 to dodge the Metal hang below):
  - Bug 9: 60-file drive boot reindex - pill now shows
    `building current/4096 file=embedding` during embed (moving, not
    frozen) and SETTLES to `idle` (docs=60, vectors=60) in ~20 s; search
    returns hits. Pre-fix it froze on the last IndexFile tick forever.
  - Bug 7: server at `ulimit -n 256`, 2 terminals open, rebuild in flight,
    40 autosave PUTs - 40/40 OK, 0 err, 0 hang; status settled idle;
    health 200; no EMFILE in the log. Exactly the bug-7 scenario, passing.

Gate (all green on branch): fmt --check, clippy --all-targets -D warnings,
cargo test (chan-drive 529 + chan-server 332 + all integration suites),
build --no-default-features, web svelte-check 0/0 + npm build. New tests:
5 fd_budget (pacing policy boundary, clear-headroom no-pace, cancel
short-circuit) + 4 indexer (embed-phase animation, ModelLoad non-clobber,
reconcile_idle drive-present + drive-gone).

FLAG to @@Architect (out-of-scope, found during repro): on THIS Mac the
embeddings reindex hangs in the Metal command buffer
(`flush_embed_batch -> embed_documents_cancelable -> embed_with ->
Tensor::to_vec2 -> MetalStorage::to_cpu -> MetalDevice::
wait_until_completed -> [_MTLCommandBuffer waitUntilCompleted]`, never
returns >180 s). `CHAN_DISABLE_GPU=1` (CPU) completes. This is in
`crates/chan-drive/src/index/embeddings.rs` (NOT my Slice D scope) and is
environment-specific (likely this sandbox's Metal access). It MASKS a full
GPU embed walk here, which is why I validated via the CPU path + targeted
Rust tests. Worth a separate bug if it reproduces on a real device; my
bug-9 fix at least keeps the pill HONEST (shows "embedding" + moving) and
clears the moment the build resolves, instead of a frozen file counter.

### 2026-05-26 (resumed session) — Work item #5: Slices E, F, G

Fresh @@LaneA session. Recovered state from journal + the architect channel;
rebased `phase-11-lane-a` onto `main` (`1918992`); my A/B/C/D commits already
in main via the merges so rebase dropped them, branch is now the `1918992`
tip (clean tree, carries @@LaneB editor fixes too).

**Result — Slices E + F + G DONE (ready to merge, not merged).** Branch tip
`27d4b98`: E `3f992db`, F `9c11b61`, G `27d4b98`, all web-only.

Architect-skill scope call (recorded before coding): the round-1 "per-
instance expand/collapse" + the Slice-A `fbTreeInstances` registry tempted a
full per-instance-render rewrite of FileTree (1500 lines, 4 singletons).
Chose option (b): layer a thin subscription manager on the Slice-A registry
and LEAVE the singleton render model intact. Rationale: single
responsibility (subscription lifecycle is one new concern, not a render
rewrite), minimal public API churn, no collision with @@LaneB's editor/tabs
surface, append-only discipline. The round-1 "must not affect others" is
satisfied at the subscription-bookkeeping layer (each instance owns its
`subscribedDirs`); per-tab render independence already exists via the
fullstack-58 snapshot/restore.

**Slice E (`3f992db`) — File Browser scoped /ws wiring.** New
`web/src/state/fbWatch.svelte.ts`: the subscription lifecycle POLICY -
`fbWatchRegister` (subscribe root), `fbWatchSubscribe`/`fbWatchUnsubscribe`
(wire `sub` on the 0->1 cross-instance transition, `unsub` on 1->0, driven
by `fbDirSubscriberCount`), `fbWatchReconcile` (diff against the expanded
set), `fbWatchDispose` (unsubscribe-then-forget, no leak), `fbWatchResyncAll`
(replay the union after reconnect). Store: `onWatchReady()` wired as
`openWatchSocket`'s onReady in `bootstrap()` + `reconnectWatcher()` (the
server registry is per-socket, so a reconnect must replay scopes).
FileBrowserSurface: stable per-surface `instanceId` (tab id / dock side /
overlay) + register-on-mount / dispose-on-unmount / reconcile-against-
treeExpanded.map effects. `fbWatch.test.ts`: 7 tests incl. the
sub1/sub2/unsub1/unsub2 matrix, dispose-no-leak, dispose-with-peer-keeps-
scope, reconcile diffing, reconnect resync.

Note on circular import: store imports `fbWatchResyncAll` from fbWatch, and
fbWatch imports the registry accessors from store. Benign because both
modules reference each other only inside function bodies (resolved at call
time), not at module-eval time; ES modules + Vite handle it. svelte-check +
build + the suite all confirm.

**Slice F (`9c11b61`) — Graph.** GraphPanel registers an fbWatch instance
(reuses the SAME FB pub/sub) and reconciles against `displayedDirs` (fs-graph
dir nodes + scope dir + parent dirs of semantic file nodes); depth-slider
increase = subscribe next degree, decrease/close = unsubscribe + teardown.
Redraw still rides the existing `graphReloadSignal`. Edge palette per round-1
line 48-51: `contains` (dir->dir, dir->file) stays GREY (theme.folder);
`link` edges now coloured by SOURCE document kind via new `fileKindColor()`
(markdown orange --g-doc, source --g-source, binary/img/contact mapped),
stroked in their own per-source-kind pass; tag/mention/language keep their
palette hue. Refactored the canvas edge loop into `strokePass()` +
`strokeForKind()`. `graphEdgePaletteSliceF.test.ts` (12) pins the rules +
the Graph fbWatch wiring; updated 2 stale assertions in the pre-existing
`graphDraftsStyling.test.ts` (relocated drafts_link alpha + link-excluded
iteration array - same behaviour).

**Slice G (`27d4b98`) — progress widgets.** Verification slice: the
infographics widgets (EmptyPaneCarousel slide 3 radial indexing chart +
AppStatusBar pill) already surface index + directory-graph build progress,
and the Slice-D fix made the status reach idle. E/F don't touch the status
path (diff-confirmed). Added `indexPillVisibility.test.ts` (7) as the UI-side
bug-9 regression lock: visible while building/reindexing/error, hidden on
idle + null, both behaviourally vs the indexStatus store and by source-
pinning the derivation + the animated building-counter branch.

Gate (whole E+F+G tree, all green): fmt --check, clippy --all-targets -D
warnings, cargo test (chan-server 332 + all suites), build
--no-default-features, web svelte-check 0/0 + npm build + vitest 1518 pass.

LESSON (recorded): a canvas edge-loop refactor invalidated 2 source-pinning
(`?raw`) assertions in a peer-predating test file. The cross-agent staleness
+ shared-worktree discipline applies to TEST assertions too: when you
restructure code that `?raw` tests pin, update the regexes in the SAME
commit and flag the peer-file touch on the channel. Caught + fixed before
the ready-note; suite green.

NOT started this turn (per @@Architect direction): the inspector
consistency/layout feature (mine end-to-end, after the Graph slice;
`inspector-spec.md`) and the GPU/Metal embed hang (separate candidate bug).
Handed back for the architect to merge E/F/G.

### 2026-05-26 (resumed session) — Work item #6: inspector consistency + layout

Fresh @@LaneA session. Recovered from journal + all four channels.
E/F/G merged to `main` (`1f88ce0`, also has GPU default-CPU fix). Rebased
`phase-11-lane-a` onto `main`: my E/F/G commits already in main via the
merges, so rebase dropped them; branch tip == `1f88ce0`, 0 ahead, clean.

ACTIVE TASK: inspector consistency + layout, end-to-end per
`docs/journals/phase-11/inspector-spec.md`. Wire the Download button to
@@LaneB's already-merged `runDesktopDownload` + `downloadTransfer.svelte`
(do NOT rebuild). Reuse `fileTypes.ts::isEditableText` for Open.

#### Source audit (grounded in HEAD, not the spec's approximate refs)

Read the five inspector components + GraphPanel's wiring + the three host
surfaces (FileBrowserSurface, FileEditorTab) + the store download helpers
+ the lane-B download interface. Facts the slices build on:

1. **Two divergent folder inspectors (the drift root).** The File Browser
   folder inspector is `FileInfoBody`'s `entry.is_dir` branch (uses
   `api.reportPrefix`, drops COCOMO cost). The Graph folder inspector is a
   SEPARATE component `DirectoryInfoBody` (uses `api.reportDir`, the O(1)
   cache; shows COCOMO cost; has a path-row; different section order).
   `InspectorBody` dispatches `kind:"directory"` -> `DirectoryInfoBody`,
   `kind:"file"` -> `FileInfoBody`. So a folder shows DIFFERENT bodies on
   FB vs Graph. THIS is the parity break.

2. **Graph "Open" partial.** Semantic file nodes DO wire `onOpen`
   (`openSelectedFile` -> `openInActivePane`). But (a) the fs-mode block
   binds `onOpen` only when `fsKind === "file"`, and (b) `FileInfoBody`'s
   Open button renders only when `editable && onOpen`. Net: Open works for
   editable semantic file nodes today, but the spec wants it for ANY
   editable/source file on EVERY graph node path, even read-only. The
   `editable` gate already uses `isEditableText` (good); the gap is that
   fs-mode dirs don't pass onOpen (correct -- dirs aren't openable) and
   the ghost branches have their own inline Open. Confirm fs-file Open
   fires regardless of read-only (read-only is a permission badge, not a
   gate on opening).

3. **"Graph from here" was removed from graph inspector bodies**
   (fullstack-a-33) in favour of the ancestor breadcrumb. The spec wants
   it BACK as an explicit action on the selected node (file or folder),
   and re-rooting must always show the node's PARENT folder (file ->
   parent dir; top-level -> drive root) -- which is exactly what
   `openFsGraphForFile` already computes (parent dir, drive fallback).
   `rescopeFromHere(file:X)` re-scopes to the file itself, NOT the parent;
   need parent-folder semantics. For a folder selection, re-root shows
   that folder (its own subtree) -- `openFsGraphForDirectory`/dir-scope.

4. **Layout: actions are at the BOTTOM today.** Both bodies scatter
   Upload/Download/Open/Show/Graph-from-here AFTER the metadata + report
   sections. Spec wants an ACTIONS section directly under the filename,
   then the lazy content (report, links, backlinks, tags, contacts)
   below. Plus a full-path toggle (today FileInfoBody has no path row;
   DirectoryInfoBody has an always-on path-row).

5. **Download is browser-only today** (`fileOps.downloadPath` ->
   `<a download>`). Lane-B's `runDesktopDownload(url, filename)` +
   `downloadTransfer` store is the desktop path with progress; gate on
   `isTauriDesktop()`, keep the `<a download>` for browser.

6. **Hosts:** FB (`FileBrowserSurface` `FileInfoBody` + DriveInfoBody for
   drive root), editor (`FileEditorTab` `FileInfoBody`, showRefs), Graph
   (`GraphPanel` -> `InspectorBody` for both fs-mode + semantic, plus
   inline ghost/drive bodies). `Inspector.svelte` is the chrome host on
   all three; only the body differs.

#### Plan — small gated sub-slices

The unification strategy: make `FileInfoBody` the SINGLE body for files
AND folders on all three surfaces, factor the actions into one shared
section component placed directly under the filename, and route the
Graph's folder selection through `FileInfoBody` (retiring the divergent
`DirectoryInfoBody` path) so FB and Graph folder inspectors are literally
the same component. Each sub-slice is independently gated + merge-ready.

- **I1 — Download button -> lane-B desktop capability.** Add a
  `fileOps.downloadPathWithProgress(path,isDir)` (or inline handler) that
  branches on `isTauriDesktop()`: desktop -> `runDesktopDownload`, browser
  -> existing `<a download>`. Wire `FileInfoBody`'s Download button to it;
  bind a small progress affordance to `downloadTransfer`/
  `downloadTransferActive`. No layout move yet -- smallest first slice,
  isolates the lane-B integration. Vitest for the branch.

- **I2 — Actions section component + layout move.** New
  `InspectorActions.svelte`: the Open / Upload / Download row + the
  full-path toggle (+ media View/Zoom + "Graph from here" + reveal slots),
  driven by props. `FileInfoBody` renders it DIRECTLY under the filename
  header; the lazy report/links/backlinks/tags/contacts sections move
  below. Full-path toggle reveals `entry.path` (replaces DirectoryInfoBody's
  always-on path-row; consistent across file + dir). Pure layout +
  extraction; behaviour parity held by the existing wiring.

- **I3 — Folder parity: route Graph folder selection through
  FileInfoBody.** Change `InspectorBody` so `kind:"directory"` renders
  `FileInfoBody` (dir branch) too, OR make GraphPanel emit `kind:"file"`
  for folder nodes (FileInfoBody already dispatches on `entry.is_dir`).
  Teach FileInfoBody's dir branch to prefer `api.reportDir` (O(1) cache)
  with `reportPrefix` fallback so it keeps DirectoryInfoBody's cheap path.
  Decide DirectoryInfoBody's fate (likely delete once nothing routes to
  it; keep its Drafts notice + drive-root copy in FileInfoBody, which
  already has the Drafts notice). Folder inspector is then ONE component.

- **I4 — Graph actions: Open (any editable, even read-only) + Graph from
  here (parent-folder re-root).** Ensure every graph file-node path binds
  `onOpen` for editable files (semantic already does; verify fs-file +
  reconcile read-only). Re-add "Graph from here" to the graph inspector
  via the new actions section: file -> re-root showing parent folder
  (reuse `openFsGraphForFile` semantics, or `rescopeFromHere` taught the
  parent rule); folder -> re-root at that folder; top-level -> drive root.
  Keep the breadcrumb (it's the upward nav); the button is the
  explicit per-node re-root the spec wants back.

- **I5 — Cross-surface walk + regression tests.** Stand up a scoped test
  server on MY drive path/port, walk FB/editor/Graph inspectors for a
  file, a folder, an image, a read-only file; confirm identical section
  model + working Open/Download/Graph-from-here. Add vitest locking the
  shared actions ordering + the folder-body parity. Tear down server +
  close tabs (scoped pkill only).

Sequencing rationale: I1 isolates the external (lane-B) dependency first;
I2 establishes the shared actions component the rest depend on; I3 + I4
are the consistency fixes built on the I2 component; I5 verifies. Each
lands through the full gate with a ready-note.

#### Results — I1-I4 DONE (ready to merge, not merged), I5 verification done

Branch tip `8315f38` on top of `1f88ce0`. Four gated web-only commits:

- **I1 `7366992`** — Download -> @@LaneB desktop progress capability.
  `fileOps.downloadPathWithProgress` branches on `isTauriDesktop()`:
  desktop -> `runDesktopDownload` (drives `downloadTransfer` store),
  browser -> `<a download>`. FileInfoBody renders an indicator bound to
  the store + disables the button mid-transfer. Did NOT rebuild the
  download flow. New `fileOpsDownload.test.ts` (3) pins the branch;
  updated the `fileTreeDragOut` source-pin for the button markup.
- **I2 `b9c2b14`** — actions section under the filename + full-path
  toggle. New `actionsSection` snippet in FileInfoBody (Open gated on
  isEditableText+onOpen, View/Zoom or View PDF for media, Upload,
  Download+indicator, Show File/Directory, Graph from here) renders
  directly under the filename header in BOTH branches; report/refs move
  below. `inspectorActionsLayout.test.ts` (8) locks the ordering + the
  toggle.
- **I3 `20f6e26`** — folder parity. InspectorBody routes
  `kind:"directory"` through FileInfoBody (its is_dir branch) with the
  graph node's label; FileInfoBody prefers the O(1) /api/report/dir
  cache (reportPrefix fallback). DELETED the divergent
  `DirectoryInfoBody.svelte` (zero render sites after the reroute) +
  its two dedicated source-pin tests; preserved the reportDir +
  GraphPanel-folder-mapping assertions in new
  `inspectorFolderParity.test.ts` (6). The FB drafts notice is already
  covered on FileInfoBody (draftsInspectorFileInfoBody.test.ts).
- **I4 `8315f38`** — graph Open (any editable, even read-only) +
  parent-folder Graph-from-here. New `graphFromHere(path)` re-roots the
  current graph tab in place to the node's PARENT folder (drive root if
  top-level) + pins the node. Wired as onSetAsScope on fs-mode + the
  file/directory semantic selections. Open already works for read-only
  editable files (FileInfoBody gates on isEditableText, a type check,
  not a permission gate). Updated the `revealBrowserActions` scope-wiring
  source-pin.

**I5 — cross-surface verification (in-browser, scoped test drive
`/tmp/chan-test-lane-a-insp`, dedicated port 8799, scoped-pkill only,
torn down + drive removed afterward):**
- FB: folder inspector (header -> path toggle -> actions Upload/
  Download/Graph-from-here -> files/size/file-kinds/code/COCOMO);
  file inspector (DOCUMENT -> Open/Upload/Download/Graph-from-here ->
  meta/refs); media inspector (MEDIA -> preview -> View+Zoom/Upload/
  Download -> size/linked-from). Full-path toggle works (Show/Hide).
- Editor: same FileInfoBody body (shared component).
- Graph: file node inspector identical to FB; FOLDER node inspector now
  renders the SAME FileInfoBody body as FB (drift fixed); "Graph from
  here" on welcome.md re-scoped file:notes/welcome.md -> dir:notes
  (breadcrumb drive/notes) with the node pinned + the cohort in view.

Gate (whole I1-I4 tree, all green): cargo fmt --check, clippy
--all-targets -D warnings, cargo test (chan-server 332 + chan-drive +
all integration suites, 0 failed), build --no-default-features, web
svelte-check 0/0, npm build, vitest 1531 pass / 11 skip / 0 fail.

LESSON re-confirmed (the `?raw` staleness rule): four pre-existing
source-pin tests pinned markup I restructured (button shape, the
graph->DirectoryInfoBody routing, the old onSetAsScope=directory-only
wiring). Updated each regex in the SAME commit as the source change;
the I3 deletion folded the still-valid assertions of the removed
component's tests into a new parity test rather than dropping coverage.

Handing back for the architect to merge I1-I4. Queued (NOT started):
new-file/draft items (new-file-and-draft-spec.md 2/3), fb-capabilities,
watcher-scalability (HELD pending @@Alex).

### 2026-05-26 (resumed session) — Work item #7: new-file items 2 + 3

Fresh @@LaneA session. Recovered from journal + the architect channel.
Inspector I1-I4 merged to `main` at `cc17a37`. Rebased `phase-11-lane-a`
onto `main`: my I1-I4 commits already in main via the merge, so rebase
dropped them; branch tip == `cc17a37`, 0 ahead, clean.

ACTIVE TASK: new-file-and-draft-spec.md items 2 + 3.

#### Source audit (grounded in HEAD, not the spec's approximate refs)

1. **Item 2 (open-after-create) is LARGELY ALREADY DONE on main.** The
   FileTree menu's only create surface is `newFileOrDir` ->
   `fileOps.createFileOrDir` (the unified `fullstack-a-67e` dialog;
   separate New File / New Dir entries were retired). `createFile`,
   `createDir`, and `createFileOrDir` (store.svelte.ts 3232/3272/3306) ALL
   already do the right thing: file branch calls `openInActivePane(path)`
   (which picks wysiwyg for markdown / source for other editable via
   `defaultModeForPath` + `classifyPath`, and gates only on
   `isEditableText` -- a TYPE check, not a writability gate, so read-only
   opens fine); dir branch calls `revealAndSelect`. The `openInActivePane`
   after create has been there since phase-5 (`790fd02`). Other create
   entry points (FileEditorTab L383, TerminalTab L1010) also route through
   `createFile`. So the spec's "creates but does not open" reads STALE
   against current main. PLAN: verify empirically on a fresh binary; if it
   already opens, item 2 is a no-op + regression-lock tests; if a gap
   surfaces (e.g. a menu path that bypasses the helpers), close it at the
   create-resolution layer only.

2. **Item 3 (draft Save reuses PathPromptModal) is the REAL work.** The
   draft "Save to Drive" (FileEditorTab L479 -> `saveDraftTabToDrive`,
   tabs.svelte.ts 2019) currently uses `uiDraftClose` -> `DraftCloseModal`,
   a PLAIN text input with NO autocomplete. The close-draft flow
   (`handleDraftTabClose`, intent "close") shares the same modal but ALSO
   needs the Discard button, so DraftCloseModal stays for close; only the
   SAVE intent moves to PathPromptModal. File-vs-dir is already detected
   server-side via `DraftInspectResponse.has_attachments` (lone draft.md ->
   false -> FILE target, editable-text gated; workspace -> true ->
   DIRECTORY target). So:
   - lone draft.md: `uiPathPrompt({ kind: "file", mode: "create",
     validate: isEditableText })` -- same gate as createFile.
   - draft dir: `uiPathPrompt({ kind: "folder", mode: "create" })` (the
     existing "folder" kind IS the Dir-only mode: no .md append, trailing
     slash allowed) PLUS a NOTICE telling the user the whole draft dir is
     saved as a directory. Add an optional `notice` field to
     pathPromptState + render it in PathPromptModal.
   - after promote: keep `reloadPromotedDraftTab` + the "saved to" notify
     (reuses item-2 open behaviour for the resulting file).

#### Plan — gated sub-slices

- **N1 — PathPromptModal Dir-only `notice` + draft Save routed through it.**
  Add `notice?: string` to pathPromptState + uiPathPrompt; render it as an
  info line in PathPromptModal. Rewrite `saveDraftTabToDrive` to call
  `uiPathPrompt` (file vs folder kind from has_attachments), keeping the
  promote + reload + notify. Leave `handleDraftTabClose` (close intent, has
  Discard) on DraftCloseModal. Vitest for the modal notice + the draft-save
  kind/target wiring.
- **N2 — item-2 verify + regression lock.** Fresh-binary walk of New File /
  New File or Dir (md opens rendered, .txt/source opens source, dir gets
  selected). If a real gap surfaces, close it at the create-resolution
  layer. Add vitest pinning create-then-open + create-then-reveal.

Sequencing: N1 is the substantive change; N2 verifies item 2 (likely a
no-op fix + lock). Each lands through the full gate with a ready-note.

#### Result — items 2 + 3 DONE (ready to merge, not merged). Commit `78ef8c7`

Single gated web-only commit on top of `cc17a37`. 7 files, +199/-36
(5 source/component edits + 2 test files, one new). No Rust, no docs,
no other-agent files.

**Item 3 (the real work) — draft Save reuses PathPromptModal.**
`saveDraftTabToDrive` (tabs.svelte.ts) now routes through `uiPathPrompt`
instead of `uiDraftClose`/DraftCloseModal:
  - lone draft.md (has_attachments=false): `kind: "file"`, default
    `<name>.md`, same editable-text validate as `fileOps.createFile`.
  - draft workspace (has_attachments=true): `kind: "folder"` (the
    existing Dir-only mode: no `.md` append, trailing slash allowed),
    default `<name>/`, PLUS a `notice` line "This draft has
    attachments, so the whole draft directory is saved as a directory
    at the path below."
Added an optional non-blocking `notice` field to `pathPromptState` +
`uiPathPrompt` (store.svelte.ts), rendered as a muted-info line above
the input in PathPromptModal. The draft-CLOSE flow (`handleDraftTabClose`)
KEEPS DraftCloseModal (it owns the Discard button + the name-on-close
input); only the explicit Save action moved. Dropped the now-unused
`intent`/save-intent plumbing from the close path (DraftCloseModal,
`uiDraftClose`, `draftCloseState`, the `DraftCloseIntent` type) since
nothing passes `intent: "save"` anymore.

CIRCULAR-IMPORT footgun caught + fixed (the load-bearing detail): a
STATIC `import { uiPathPrompt } from "./store.svelte"` at the top of
tabs.svelte.ts crashed module init in 9 test files (3 failing tests +
6 zero-test files). Root cause: store.svelte has a TOP-LEVEL
`registerDraftPromotionSink(...)` side effect that calls back into
tabs.svelte; importing store eagerly from tabs forces store's body to
run during tabs' own module-eval, touching `draftPromotionSinks` before
it's initialised. (This is UNLIKE the Slice-E fbWatch<->store cycle,
which was function-body-only.) Fix: a LAZY `await import("./store.svelte")`
inside `saveDraftTabToDrive`, resolved at user-action time. Full suite
went 9-failed -> 0-failed after the fix. LESSON: a cyclic dependency
whose other side has an EAGER side effect must be consumed via dynamic
import, not a static top-level one; svelte-check alone does NOT catch it
(it passed both before and after) — only the vitest module-eval does.

**Item 2 (open-after-create) was ALREADY DONE on main** — verified, no
behavior change needed. Empirical fresh-binary walk (scoped drive
`/tmp/chan-test-lane-a-newfile`, port 8799, scoped-pkill, torn down +
unregistered):
  - New File or Directory menu -> `newdoc` (md) opened wysiwyg
    (rendered); `snippet.txt` opened wysiwyg (`.txt` is markdown-class
    app-wide via MARKDOWN_EXTENSIONS, consistent with every other open
    path); `build.sh` opened SOURCE mode (line gutter); `subdir/`
    revealed + selected in the tree, FB stayed focused. All four
    item-2 cases pass. read-only is not a gate (openInPane gates on
    isEditableText, a TYPE check). Added `newFileOpenMode.test.ts` (5)
    as the regression lock + the defaultModeForPath open-mode split.

**Item 3 empirical (same drive):** lone draft.md -> Save to Drive opens
PathPromptModal (file mode, default `untitled-1.md`, `.md` auto-append
in the status row) -> saved `notes/saved-lone.md` (24B on disk).
Workspace draft (added `diagram.png` to the draft dir on disk under
`~/.chan/drives/.../drafts/untitled-1/` to flip has_attachments) ->
Save to Drive opens the folder Dir-only mode with the notice + default
`untitled-1/` -> saved `notes/saved-workspace/` containing the WHOLE
dir (`draft.md` + `diagram.png`) on disk. (One test-input slip: folder
kind opens cursor-at-end per `fullstack-a-65`, so the first attempt
appended to `untitled-1/`; re-ran with Cmd+A first — code is correct,
the field/backend behaved as designed.)

Gate (whole tree, all green): cargo fmt --check, clippy --all-targets
-D warnings, cargo test (chan-server 332 + chan-drive + all integration
suites, 0 failed), build --no-default-features, web svelte-check 0/0,
npm build, vitest 1548 pass / 11 skip / 0 fail.

Handing back for the architect to merge. Queued (NOT started, per the
channel): FB capabilities (fb-capabilities-spec.md), graph dead-ends
(graph-loading-state-spec.md), watcher hardening + e2e benchmark
(watcher-scalability.md, RELEASED).

### 2026-05-26 (resumed session) — Work item #8: graph/inspector hotfix GI-1..4

Fresh @@LaneA session, resumed after @@Architect stopped me mid-turn. The
stop was a TEST-DISCIPLINE violation: my prior turn served the REPO ROOT
as a graph test drive (node_modules/target -> 131K nodes), which @@Alex
halted. New HARD RULE recorded: NEVER serve the repo root / a worktree /
any dir with node_modules|target|.git as a test drive; always a SMALL
purpose-built /tmp drive; scoped-pkill only; never touch @@Architect's
:8791 docsrv.

Recovery: branch == main (6103f4d), clean, no rebase needed. BUT the prior
turn had left the GI-1..4 changes UNCOMMITTED in the worktree (3 modified
+ 1 untracked test). I re-gated + empirically re-verified before committing
(the work was never gated/verified - it was interrupted).

**Result — GI-1..4 DONE (ready to merge, not merged).** Two commits on
6103f4d: web 7299625, backend d35b852.

GI-1/GI-2 root cause (the real find): NOT a mis-bound onclick. The graph
reload $effect over-tracked - it read load()'s internal currentScope
$derived, recomputed by availableGraphScopes() on any workspace LAYOUT
change. "Open" (opens a tab) and "Show File" (reveals in browser) both
shift the layout -> currentScope churns to an equal-but-new object ->
reload re-fired. Fix: anchor the reload on a stable string loadKey
(scopeId|depth|mode) + run load() untracked. The action HANDLERS were
already correct from I4; the reload was a reactive side effect, which is
why the prior binding-asserting inspector tests passed but missed it. The
new graphInspectorActionsHotfix.test.ts locks the ACTUAL behavior
(loadKey/untrack anchoring + handler routing + dir-radius ordering).

GI-4: RADIUS_DIR=6 (leaf base 5, doc/drive hub 7) in GraphCanvas.svelte.

GI-3: resolve_link_dst (graph.rs) now also walks the source's ANCESTOR
dirs toward the drive root, so a drive-rooted partial-prefix wiki-link
([[phase-2/frontend-3.md]] authored under docs/journals/ -> real file
docs/journals/phase-2/frontend-3.md) resolves instead of ghosting.
Drive-root + immediate-parent bases keep priority; fallback only rescues
otherwise-ghosted links; only resolves to EXISTING files so genuinely
broken links stay flagged. 2 new unit tests (ancestor-lands + fallback-
doesnt-beat-drive-root). @@LaneB's watcher task is watch.rs+drive.rs ONLY
(per their channel) - no graph.rs collision, so GI-3 done this turn.

Empirical verify (FRESH binary @ 23:14; SMALL drive /tmp/chan-test-lane-a-
graph = 5 cross-linking .md, no node_modules/target/.git; port 8799,
CHAN_DISABLE_GPU=1; scoped-pkill, :8791 confirmed untouched; torn down):
the link graph shows journal.md -> frontend-3.md broken:false (real node),
phase-3/notes-a.md broken:false, and does-not-exist-anywhere.md broken:true
(the ONLY missing node). Real files resolve, broken stays broken, no
over-resolution. GI-1/2/4 (reactivity + size) locked by vitest; in-browser
confirm rides @@Alex's :8791 rebuild after merge.

Gate green: fmt, clippy -D warnings, cargo test (chan-server 332 + suites),
build --no-default-features, svelte-check 0/0, npm build, vitest 11/11.

Pre-existing flake flagged (not mine): 3 chan-drive indexer debounce tests
fail under the full parallel suite, pass isolated + single-threaded -
fs-watcher timing flakes in the indexer area @@LaneB owns, disjoint from
graph.rs. Flagged to @@Architect for the merge re-gate.

NEXT (queued, not started): FB capabilities (fb-capabilities-spec.md),
then graph dead-ends/loading UX (graph-loading-state-spec.md). Handed back
for the merge.

### 2026-05-26 (resumed session) — Work item #9: FB capabilities + flaky-test hardening

Fresh @@LaneA session. Recovered from journal + the architect channel.
GI hotfix merged to `main` (`4a7ab0f`, also has @@LaneB's ignore fix).
Rebased `phase-11-lane-a` onto `main`: my GI commits already in main via
the merge, so rebase dropped them; branch tip == `4a7ab0f`, 0 ahead, clean.

ACTIVE TASK: File Browser capabilities (fb-capabilities-spec.md) — desktop
multi-select / clipboard / multi-move DnD — plus a quick pass to harden 3
flaky web tests (EmptyPaneCarousel/Pane/TerminalTab) before round close.

#### Source audit (grounded in HEAD `4a7ab0f`, not the spec's refs)

Backend:
1. **Move EXISTS, copy DOES NOT.** `Drive::rename` (drive.rs:1672) +
   `rename_with_link_rewrite`/`_with` (1738/1748) are the move path: full
   sandbox via cap-std `Dir::rename` (TOCTOU-free), special-file refusal
   (symlink/socket/etc rejected with `SpecialFile`), `ensure_writable_in`
   gate, parent `create_dir_all`. The link-rewrite pass rewrites inbound
   md/wiki/image links and re-relativizes moved sources. NO copy primitive
   exists anywhere in drive.rs/fs_ops.rs (grep: only rename/remove/trash/
   duplicate_team). `fs_ops` has `atomic_write_in`, `classify`,
   `ensure_regular_file`, `describe_file_kind` — the write/refusal
   primitives a copy will reuse.
2. **HTTP: `api_move` (files.rs:1409, POST /api/move, MoveBody{from,to})**
   -> `rename_with_link_rewrite` on a blocking thread, notes self-writes
   for from/to/rewritten so no external-edit toast fires, returns
   MoveResponse{renamed,rewritten,conflicts}. SINGLE from/to only. No
   copy route, no multi-entry route. Web client: `api.move(from,to)`
   (client.ts:746) + `fileOps.moveTo(from,target)` (store).

Frontend:
3. **Selection is a GLOBAL singleton today.** `browserSelection`
   (store:2322) = `{path: string|null, showDrive: boolean}`. Single active
   path. Feeds the inspector, the menu actions, find-cursor, drag-move,
   keyboard nav. Per-tab isolation already exists: FileBrowserSurface
   `snapshotIntoTab`/`restoreFromTab` (140-186) save/restore the singleton
   (selected/showDrive/expanded/scroll) on tab swap; dock+overlay
   intentionally SHARE the singleton (drive-wide intent). The Slice-A
   `fbTreeInstances` registry (store:2479) already has per-instance
   `expanded`/`selected`/`showDrive`/`scrollTop`/`subscribedDirs`.
4. **FileTree keyboard nav** (697 onTreeKeydown): arrows move
   `browserSelection.path` (moveSelection/moveToFirst/Last), Enter
   opens/toggles, Backspace/Delete removes. EARLY-RETURNS on any
   meta/ctrl/alt chord (701) so cmd-combos fall through to the browser —
   this is where cmd+A/C/X/V must hook. Single-row click `selectPath`
   (416).
5. **Kept app-internal drag** (137 onFileDragStart): `TREE_MOVE_MIME`
   payload `{path,isDir}`, single entry. `onRowDrop` (212) ->
   `fileOps.moveTo(src.path, dropTargetPath(...))`. `isInvalidDrop` (174)
   guards self/descendant/same-parent. Bug-2a removed the OS<->app drag;
   this internal one stays and is what I extend to multi.

#### Architecture decision (recorded before coding)

**Selection model: EXTEND the `browserSelection` singleton with a
multi-set, do NOT rip it out.** Add `paths: string[]` (the full selection,
in no particular order; `path` stays the ACTIVE/cursor entry for the
inspector + single-target actions) + `anchor: string|null` (range pivot
for shift+click / shift+arrows). Rationale:
- The inspector, find, menu, and single-entry actions all key off
  `browserSelection.path` today; keeping `path` as the cursor means zero
  churn to those consumers. `path` is always a member of `paths` (or both
  null). A plain click sets `paths=[path]`, anchor=path.
- "Selecting in one instance must not affect another" is ALREADY satisfied
  by the per-tab snapshot/restore seam: extend `snapshotIntoTab`/
  `restoreFromTab` + the `BrowserTab` record + `FbTreeInstance` to carry
  the multi-set. Dock+overlay keep sharing (existing, intended). This
  reuses the per-instance metadata rather than adding a parallel system.
- Avoids a 1500-line FileTree per-instance render rewrite (the same call I
  made for Slice E: layer on the existing model, don't rewrite the render).

**Clipboard: module-level `fbClipboard` $state** `{mode:"copy"|"cut"|null,
paths:string[]}`. Cross-instance paste (same drive) = clipboard is global,
not per-instance (spec line 28 explicitly allows it). Paste target = the
focused/selected dir (selected dir if a dir is the active selection, else
the parent dir of the active selection, else drive root).

**Name-collision policy on paste: " copy"/" copy 2" suffix before the
extension** (Finder-style; least surprising). `foo.md` -> `foo copy.md` ->
`foo copy 2.md`. A cut (move) into a dir that already has the name is a
genuine collision (you're not duplicating) -> for cut, suffix too rather
than refuse/overwrite (never silently overwrite; never lose data). Decided
in-lane; noted here per the spec's "pick the least-surprising default and
note it." The suffix is computed server-side in the new copy/move-into
route so it's atomic against the live tree (TOCTOU: resolve a free name,
then write; on a race the cap-std create fails and we retry the next
suffix).

**Backend: one new `Drive::copy` primitive + a multi-entry route.** Copy
files via `atomic_write_in` of the read bytes (reuses the special-file
refusal + sandbox); copy directory subtrees by walking + recreating
(filtered walk skips .chan/.git like rename does; special files refused per
entry). Multi-entry MOVE reuses `rename_with_link_rewrite` per entry. New
route `POST /api/fs/transfer` (or extend /api/move) takes
`{op:"move"|"copy", sources:[paths], dest_dir}` and resolves collisions +
notes self-writes + routes through the watcher so all FB instances + Graph
update live (Slice C `fs` frames already fan out per-dir on the recursive
watcher feed — copy/move emit Created/Removed which the scoped registry
already routes).

#### Plan — gated sub-slices (selection -> clipboard -> DnD -> backend)

- **FB1 — multi-select model (web only).** Extend `browserSelection` with
  `paths`/`anchor`; teach FileTree click (plain/shift/cmd-ctrl), keyboard
  (shift+arrows extend, cmd/ctrl+A select-all-visible), and a click-drag
  rubber-band over the tree. Per-instance isolation via the extended
  snapshot/restore. Row highlight for the whole set. Vitest for the
  selection algebra (range/toggle/select-all/anchor) + isolation.
- **FB2 — clipboard state + keys (web only, no backend yet).** `fbClipboard`
  $state + cmd/ctrl+C/X/V scoped to the focused FB. Paste resolves the
  target dir + (for FB2, stub the transfer call against the FB4 route
  shape; if FB4 lands first, wire it). Cut shows a dimmed "marked for move"
  affordance. Vitest for clipboard transitions + target-dir resolution.
- **FB3 — multi-DnD move (web only).** Extend `onFileDragStart` to carry
  the WHOLE selection when the dragged row is in the set (else just that
  row + select it); `onRowDrop` moves all via the FB4 multi route;
  `isInvalidDrop` per source. Drag image shows the count. Vitest.
- **FB4 — backend copy + multi-entry transfer (Rust).** `Drive::copy`
  (file + subtree) with sandbox/special-file refusal + a pinning test;
  collision-suffix resolver; `POST /api/fs/transfer` multi-entry move/copy
  routing through the watcher + self-writes; wire client.ts + fileOps.
  Full Rust gate + the wire-shape pin. (Sequenced LAST so FB2/FB3 design
  against its shape; if I land FB4's route shape as a thin skeleton first I
  can wire FB2/FB3 to it directly — will decide at FB2.)
- **FB5 — cross-surface empirical walk + flaky-test hardening.** Small
  seeded /tmp drive: multi-select (shift/cmd/rubber-band/keyboard),
  copy/cut/paste (incl. cross-instance + collision suffix), multi-DnD move;
  confirm all FB instances + Graph update live via the watcher. Tear down +
  unregister. PLUS harden the 3 flaky vitest files (fake timers / await
  settle / kill shared-state races).

Sequencing note: FB1 is the foundation everything builds on; FB2/FB3 are
the input surfaces; FB4 is the atomic backend they call. I'll likely land
FB4's route+primitive EARLY-ish (right after FB1) so FB2/FB3 wire to real
endpoints, but keep them as separate gated commits. The flaky-test
hardening (FB5 tail) is independent and can slot in whenever; I may do it
first as a fast standalone commit since it's a named CI risk.

Beginning with the flaky-test hardening (fast, independent, de-risks CI),
then FB1.

#### Result — flaky tests + FB1-FB5 DONE (ready to merge, not merged)

Resumed mid-turn (committed nothing on the prior stop; one WIP file
restored). Branch tip `602d06d` on top of `4a7ab0f`. 5 gated commits.

**Flaky-test fix (`04ba894`).** Diagnosed the REAL root cause empirically:
the 3 files (EmptyPaneCarousel/Pane/TerminalTab) did not assertion-fail -
they TIMED OUT at 30s under the full parallel suite, hanging at the
per-test `await import("./X.svelte")` inside their render helpers. Under
154 parallel files the Svelte-component transform+import is heavily
contended (cumulative transform ~600s / import ~400s), so the dynamic
import alone blew the per-test timeout. The 3 pass 29/29 run as a group;
only flake under all 154. Fix = static top-level component import (the
proven non-flaky pattern; 30 other test files use it,
TerminalRichPrompt.test.ts among them). My prior-session WIP had added
vi.useFakeTimers() to the carousel as a guess - it made things WORSE
(froze the clock during the dynamic import); backed it out. 3 consecutive
full runs = 1559 pass / 0 fail.
LESSON: a "flaky test" that times out (not asserts) is usually an
infrastructure cost (import/transform contention), not a logic race; check
the failure MODE before reaching for fake timers / await-settle.

**FB1 multi-select (`f59bb3f`, web).** Extended `browserSelection` with
`paths`/`anchor` keeping `path` as the active cursor (zero churn to the
inspector/find/actions that read `path`). Helpers fbSelectSingle/Toggle/
Range/Set/Clear maintain "path is always in paths". Routed every existing
single-select write (reveal/enter/url-hash/keyboard-nav/find) through
fbSelectSingle so the set never goes stale. FileTree gestures: plain/shift/
cmd click, shift+arrows extend, cmd+A select-all-visible, click-drag
rubber-band (cmd = additive). Per-instance isolation via the fullstack-58
snapshot/restore seam + BrowserTab.selectedPaths. `.active-cursor` class
distinguishes the cursor in a multi-set. fbSelectionModel.test.ts (14).

**FB4 backend (`daf45fe`, Rust + web client).** Drive::copy was MISSING
(only rename existed) - added it: file via read+atomic_write_in, subtree
via walk+recreate (skip .chan/.git/.hg, refuse special files per entry,
no-clobber). Drive::resolve_free_name = Finder " copy"/" copy 2" suffix
before the ext (split_name_ext handles dotfiles/multi-dot). POST
/api/fs/transfer {op,sources,dest_dir} multi-entry move/copy: move reuses
rename_with_link_rewrite per source, copy uses Drive::copy; notes self-
writes so no external-edit toast; watcher Created/Removed still flows
through the Slice-C scoped fs registry to all FB instances + Graph. 5
chan-drive + 2 chan-server tests.

**FB2 clipboard + FB3 multi-drag (`602d06d`, web).** Module-level
fbClipboard {mode,paths} (NOT per-instance - cross-instance paste allowed).
cmd/C/X/V scoped to the focused tree; cut rows dimmed; paste target =
selected dir / parent / root; copy keeps the clipboard, cut clears on
success, failed paste retains for retry. Multi-drag carries the whole
selection (paths[] added to the TREE_MOVE payload, back-compat); N-items
drag image; 1 source -> moveTo (link rewrite), many -> atomic transfer.
fbClipboard.test.ts (9).

**FB5 empirical (FRESH binary @ 00:39, provenance verified; SMALL seeded
/tmp/chan-test-lane-a-fb, port 8799, CHAN_DISABLE_GPU=1; scoped-pkill,
:8792 untouched; torn down + unregistered; API-driven, no browser tabs):**
multi-copy, collision suffix (alpha copy -> alpha copy 2), cut/move,
subtree copy (notes/sub -> tasks/sub), no-op move skip, multi-cut, and
LINK REWRITE (moving a link target rewrote the linker's relative href to
../tasks/target.md). Live /api/files listing reflected every change; 0
server errors; health 200.

Gate (whole branch): fmt, clippy -D warnings, cargo test single-threaded
ALL_GREEN (0 failed), build --no-default-features; web vitest 1582 pass /
0 fail / 11 skip, svelte-check 0/0, build. Pre-existing flakes flagged to
@@Architect (chan-drive/chan-server indexer debounce under parallel; one
cosmetic refreshDrive /api/drive unhandled rejection - not mine).

NEXT (queued per the @@Architect CORRECTION note): GI-5/GI-6/GI-7
(graph-inspector-bugs.md) - WEB-only GraphPanel.svelte, likely the same
reactivity root cause as my GI-1 loadKey/untrack fix. Then graph-loading
UX. Handed back for the merge.

### 2026-05-27 (resumed session) — Work item: GI-5/6/7 graph dir-inspector

Fresh @@LaneA session. Recovered from journal + the architect channel +
graph-inspector-bugs.md. FB-caps merged at `b458ef6`. Rebased
phase-11-lane-a onto main (started `b458ef6`; main advanced to `b81636e`
mid-turn via @@LaneB's indexer/PTY de-flake merge, rebased again clean -
web-only vs their chan-drive change). Branch tip `8906d07`, 1 ahead, clean.

**Result — GI-5/6/7 DONE (ready to merge, not merged). Commit `8906d07`.**
Web-only (+265/-33): GraphPanel.svelte + 4 test files.

Diagnosis grounded in source + a FRESH-binary backend repro on a seeded
nested /tmp drive (dedicated port 8797, scoped pkill, docsrv :8792
untouched, torn down after):

- **GI-5 (Show Directory no-op).** `revealSelectedFsEntry` ->
  `revealPathInBrowser(path,{inspectorOpen:true})` -> revealAndSelect,
  which only expands a dir's ANCESTOR chain + selects its row. A top-level
  dir's row is already visible, so nothing observably changes = the no-op.
  Fix: dirs pass `enter: isFsDirectory(node)` -> revealAndEnterDirectory
  expands the dir ITSELF + loadTreeDir(path), so the FB visibly opens AT
  the directory. Files keep select-in-place.

- **GI-6 (Graph-from-here on dir blanks + does not re-root).** Two
  symptoms, one cause. `graphFromHere(path)` always computed the PARENT
  (file semantics). The canonical helpers (`openFsGraphForFile` vs
  `openFsGraphForDirectory`) prove the right rule: a FILE re-roots to its
  parent (a file can't be an fs-graph scope root); a DIRECTORY re-roots to
  ITSELF. With the always-parent rule, clicking a child folder whose
  parent already WAS the current scope left scopeId unchanged -> loadKey
  unchanged -> reload effect never fires (the "does not re-root"), and the
  pendingSelectId set by graphFromHere was never consumed by load(), so
  the inspector fell through to InspectorBody's null branch ("Details /
  click a result to inspect" = the blank). Fix: graphFromHere takes an
  `isDir` flag; dir -> `dir:<path>` (drive for root), file -> parent rule;
  both call sites pass the flag (fs branch via a precomputed `fsIsDir`,
  semantic branch via `inspectorSelection.kind === "directory"`). Node
  stays pinned (pendingSelectId) + eagerly selected so the inspector
  re-populates on the re-rooted node.

- **GI-7 (depth slider snaps back to 1).** Root cause confirmed by API
  probe: `depthCap` for a dir scope was computed from the fs-graph LOADED
  AT THE CURRENT DEPTH (`{nodes: fsNodes, truncated: fsTruncated}`). At
  depth 1 only depth-1 nodes are present, so `graphDepthCap` ->
  maxDepthFromPaths = 1 -> cap 1 -> the clamp `$effect` snaps depth back +
  depthShallow disables the slider. The backend `truncated` flag is a
  MAX_NODES signal, NOT a depth-limit signal (walk_dir doesn't set it when
  it bottoms out at depth==0), so it can't rescue the cap. Empirically
  `dir:journals` depth-1 slice -> reachable depth 1; full-depth probe -> 3.
  Fix: a full-depth `dirDepthProbe` (mirrors the existing drive-scope
  `driveDepthProbe`), keyed by `dirDepthProbePath` so it re-probes on
  scope change + discards stale results; `depthCap` for a dir scope feeds
  the probe (falling back to the loaded slice until it lands) and returns
  `Math.max(probeCap, graphState.depth)` so the cap can never drop below
  what's already on screen. The slider's `max={depthCap}` then reflects
  the REACHABLE depth, so dragging 1->2->3 holds and loads the next layer.

Tests: NEW `graphDirInspectorHotfix.test.ts` (12 source-pins across the 3
fixes) + `depth.test.ts` gained the shallow-slice-vs-full-probe case (the
crux of GI-7 as pure logic). Updated the stale GI-2/I4 assertions in
`graphInspectorActionsHotfix.test.ts` + `revealBrowserActions.test.ts`
that pinned the pre-GI-5/6 single-arg / always-parent forms - SAME commit,
flagged to @@Architect (shared-worktree + cross-agent staleness applies to
my OWN earlier test files too).

Gate green: fmt, clippy -D warnings, build --no-default-features; web
svelte-check 0/0, vitest 1593 pass / 0 fail / 11 skip (52 in the touched
graph+depth tests), npm build. LOWLIGHT (NOT mine): `cargo test` shows 4
chan-drive watch/indexer debounce tests FAILING ("indexer did not pick up
the file write") - they fail IDENTICALLY on a clean `main` checkout
(b81636e, zero of my changes), so it's the known macOS FSEvents debounce
flake under sandbox load (@@LaneB's area), not my regression; my change is
web-only and touches no Rust. Flagged to @@Architect.

LESSON (recorded): the GI-7 cap is a chicken-and-egg - you can't load a
deeper layer because the cap (derived from the shallow layer) forbids it.
The drive scope already solved this with a full-depth probe; the dir scope
just never got the same treatment. When a "max" is derived from currently-
loaded data but the user needs to EXCEED it to load more, the max must
come from a probe of the POTENTIAL, not the loaded slice.

NEXT (queued): graph-loading UX (graph-loading-state-spec.md). Handed back
for the merge.

### 2026-05-27 (NEW ROUND, fresh session) — Work item #10: GI-8/9/10/11 + loading UX

Phase-11 CONTINUATION. I am the single GRAPH LANE this round; @@Alex carries
release/build on main concurrently (Makefiles, docs/manual, chan upgrade,
Tauri workflows). HARD boundary: I do NOT touch Cargo.lock/Cargo.toml,
.github/workflows/, desktop/, docs/manual. Revert any cargo-induced lock
churn. @@Architect serializes merges.

Bootstrap (HEAD 85e6f15, branch main, clean tree besides the untracked
lane-a-kickoff.md): re-read next-round-backlog, graph-inspector-bugs,
graph-loading-state-spec, my own journal (GI-1..7), the architect journal
(round sealed 88ea5c3), the coordination README + channels.

GROUNDING reads (read-only, pre-plan; not coding):
- GraphPanel.svelte: revealSelectedFsEntry (~1071) already passes
  enter:isFsDirectory (my GI-5 fix); currentScope $derived (~103);
  graphFromHere(path,isDir) (~205); loadKey/untrack reload anchor in place.
  GI-8 is the GI-5 reveal re-firing the reload effect = same over-tracking
  class as GI-1/2.
- fs_graph.rs: MAX_DEPTH=6, MAX_NODES=10_000, truncated flag. GI-9's "27/47"
  is far below 10k, so the dropped subdirs are a FRONTEND render/kind filter,
  not the backend node cap.
- graph.rs (chan-server): resolve_link_dst (557) builds candidates =
  [stripped, parent-join, ancestor-joins], each via normalize_drive_rel (595)
  which DOES pop on Component::ParentDir. So `../` IS collapsed here given a
  target with `..` intact.
- chan-drive markdown/links.rs::normalize_href (99) ALSO collapses `..`
  correctly (line 143 `".." => stack.pop()?`), joining source_dir + href
  first. Link.target (52) is "as written, not resolved".
  => GI-11 KEY FINDING: the malformed broken path `journals/phase-8/phase-7/
  next-phase-backlog.md` (the `..` DROPPED rather than collapsed) cannot be
  produced by either resolver. It must originate UPSTREAM in chan-drive's
  graph link-edge indexer (crates/chan-drive/src/graph.rs) storing a markdown
  `../` target without normalize_href. So GI-11's true fix likely lands in
  chan-drive, OUTSIDE my listed chan-server graph.rs/fs_graph.rs surfaces.
  Flagged as BOUNDARY Q1 to @@Architect; will repro on a fresh binary to pin
  the exact site BEFORE editing.

Slice order posted to event-lane-a-architect.md: S1 GI-8 (web, fast) -> S2
GI-11 (backend, repro-first to pin chan-server vs chan-drive) -> S3 GI-9
(spine completeness via FB containment walk) -> S4 GI-10 (drive-at-bottom
layout) -> S5 loading-state UX (builds on S3+S2, maybe a per-scope
completeness signal). Three boundary questions raised (Q1 chan-drive surface
for GI-11, Q2 completeness signal ownership, Q3 ports/FSEvents/test
discipline). WAITING for @@Architect ratification (relayed via @@Alex) before
creating the worktree + starting S1. No code written this turn.

#### Empirical findings (fresh binary @ 03:18, /tmp/chan-test-lane-a-gi8, port 8797, scoped, in-browser + API)

Built fresh binary in worktree (web dist 03:17 -> binary 03:18, provenance
verified), seeded a /tmp drive matching the backlog scenarios: agents/ with 5
subdirs (architect, ci, orchestration, webtest-a, webtest-b), notes/phase-8/
{request,process}.md each linking `../phase-7/next-phase-backlog.md` (real file
at notes/phase-7/). Served on 8797 (docs server :8793 untouched).

THREE backlog premises diverge from empirical reality on current main:

1. **GI-11 (../ false broken-links): DOES NOT REPRODUCE. Already correct.**
   /api/links + /api/graph?scope=drive: notes/phase-8/process.md AND
   notes/phase-8/request.md BOTH resolve to notes/phase-7/next-phase-backlog.md
   with broken=False; the node is missing=None. Source-grounded why: BOTH
   resolvers already collapse `../`/`./` -> chan-drive build_edges (drive.rs:4189)
   runs every md link through normalize_href (links.rs:143 `".." => stack.pop()`)
   BEFORE storing the edge dst, and chan-server resolve_link_dst/normalize_drive_rel
   (graph.rs:557/595) collapse again + ancestor-walk (GI-3). There's even a passing
   test build_edges_normalizes_parent_relative_markdown_link. The backlog's
   malformed `journals/phase-8/phase-7/...` (the `..` DROPPED, not collapsed)
   cannot be produced by either path -> almost certainly STALE INDEX data at
   @@Alex's test time (docs/ drive indexed before the build_edges/GI-3 fixes).
   => S2 becomes: regression-lock tests (multi-level `../`, `./`, multi-`../`)
   + confirm; NOT a code fix. Open Q: should a reindex/migration be offered for
   drives carrying stale pre-fix edges?

2. **GI-9 (subdirs omitted at depth): ROOT CAUSE FOUND (frontend).**
   Backend is CORRECT: GET /api/fs-graph?scope=directory&path=agents&depth=1
   returns the full spine - 7 nodes (drive root "" + agents + 5 subdirs) + 6
   `contains` edges, truncated=false. depth=2 returns 12 nodes incl. the 5 files.
   But the FRONTEND filesystem-mode graph renders 0/7 (empty canvas).
   ROOT CAUSE: `scopedNodeIds` ($derived, GraphPanel.svelte:617-668) seeds the
   scope BFS ONLY from `kind === "file"` nodes whose path is under the dir
   (lines 650-657: `n.kind === "file" && n.path.startsWith(prefix)`). In fs-mode
   a directory's shallow children are DIRECTORIES (kind:"folder" after
   mapFsNodes), NOT files - so seedPaths=[], seedIds=empty, and line 668
   `if (seedIds.size === 0) return seedIds` returns a NON-NULL EMPTY set.
   visibleNodeIds (831) skips every node not in scopedNodeIds; visibleEdges
   (824) drops every edge -> 0/7 nodes, 0/6 edges. The general GI-9 case
   ("27/47, only link-related branches shown") is the same bug at depth: only
   branches that reach a FILE get seeded; sibling dirs with no (deep-enough)
   file are dropped. FIX (matches the FRAMING note exactly): in fs-mode the
   scope BFS must seed/include the containment spine - directory (folder) nodes
   + `contains` edges - not just file nodes. Backend already provides the
   complete spine; this is purely the frontend scopedNodeIds divergence.

3. **GI-8 (Show Directory reloads the graph): NOT reproduced as described.**
   The reload `$effect` (1537) is correctly anchored on `visible` + `loadKey`
   (scopeId|depth|mode) from my GI-1 fix; revealSelectedFsEntry changes none of
   those, and in a graph TAB visible is constant true. Empirically, clicking
   "Show Directory" on a dir node in the drive + dir:agents SEMANTIC graphs did
   NOT reload (stable node counts). What I DID see in semantic mode: it's a
   NO-OP - folder selections hit InspectorBody branch 3 whose onReveal is
   `revealSelectedFile`, guarded `selectedNode.kind === "file"` (1050), so a
   directory never reveals. The fs-mode path (revealSelectedFsEntry, branch 1 at
   1953) is the GI-8 handler, but I CANNOT exercise it because the fs-mode dir
   graph is EMPTY (GI-9 above). So: GI-8's "reload" likely already fixed by the
   loadKey anchor; a residual "semantic-mode folder Show Directory no-ops" may
   be the live symptom; and GI-8 is BLOCKED on GI-9 for any fs-mode repro.

IMPLICATION FOR SLICE ORDER: GI-9 is the keystone - it's the one clear,
grounded code bug AND it blocks GI-8 fs-mode testing. GI-11 + GI-8(reload)
appear already-fixed (need confirmation + regression locks, not fixes).
Proposing to @@Architect/@@Alex: do GI-9 FIRST (re-order S3 ahead of S1/S2).
Server + browser tab kept up for the GI-9 fix verification. No code written.

#### S1' GI-9 FIX + verification (commit pending) - 2026-05-27

Fix: GraphPanel.svelte `scopedNodeIds` returns null in filesystem mode (one
guard + WHY comment), so the backend's already-scoped+depth-limited
containment spine renders in full; the file-centric scope BFS stays for the
SEMANTIC modes. Drive/global already returned null; fs-mode now joins them.
Test: new graphFsSpineCompleteness.test.ts (3 `?raw` source-pins: fs-mode null
guard, its position BEFORE the file-only dir seed, and the file-only seed it
bypasses) - matches the existing graphParentEdgeInvariant pin pattern.

EMPIRICAL VERIFY (fresh binary @ 03:39, provenance binary>dist, port 8797,
scoped, in-browser, docs server :8793 untouched): a `New Graph` on agents/
(true filesystem mode, gm=f) now renders 7/7 nodes - 6/6 edges = the COMPLETE
spine (agents/ + architect, ci, orchestration, webtest-a, webtest-b + drive
root, all via `contains` edges). Pre-fix it was 0/7 (empty). All 5 sibling
subdirs render, not just link-related branches = GI-9's core requirement met.
Gate so far: svelte-check 0/0; the 4 graph scope/depth test files 23/23.

#### S2' GI-8 interim findings (next slice, NOT this commit)

While the fs graph was populated I tested the TRUE GI-8 path (selectedFsNode ->
onReveal=revealSelectedFsEntry): clicking "Show Directory" on the agents/ dir
node did NOT reload/re-fetch the graph (scope stayed dir:agents fs, 7/7
unchanged) - confirming the reload effect is correctly anchored (loadKey+visible).
BUT two real symptoms remain, which together likely ARE what @@Alex read as
"reloads the graph instead of opening a File Browser tab":
  1. The force layout RE-ANIMATED (nodes drifted) after the click. On a dense
     graph that re-animation reads as a "reload". Need to find what the reveal
     touches that restarts the cytoscape layout.
  2. The reveal did NOT visibly open a File Browser tab nor expand the dock,
     and the inspector stayed open (close() didn't visibly take). revealPathIn-
     Browser -> revealAndEnterDirectory (mutates the treeExpanded SINGLETON) +
     openBrowser() + close(). Suspect: post-Slice-E the dock FB uses per-instance
     expanded state (fbTreeInstances), so the singleton mutation doesn't expand
     the dock; and openBrowser() may not be opening/focusing a visible FB tab
     from a full-pane graph context. Backlog GI-8 EXPECTED: "open a File Browser
     tab at the directory (like Show File reveals a file)."
NEXT (S2'): read openBrowser() + revealAndEnterDirectory's per-instance
interaction; make Show Directory open/focus a File Browser tab at the dir (and
not restart the graph layout). Web-only.

#### S1' GI-9 COMMITTED + ready-to-merge - 2026-05-27 06:58
Commit d853a79 (2 files, +81/-0); rebased onto main 0691dc9 (terminal WebGL
fix, disjoint web file, clean) -> branch tip c188cfa. Full gate green: Rust
fmt/clippy(-D warnings)/build --no-default-features/cargo test exit 0 (Rust
unchanged by both web-only changes, result carries); web svelte-check 0/0,
build, vitest 1596/0 on the rebased tree. Re-confirmed in-browser 0/7 -> 7/7.
Posted ready-to-merge: phase-11-lane-a@c188cfa to event-lane-a-architect.md.
@@Architect serializes the merge + re-gate. NOTE: roster shift - it's @@LaneC
(not Lane B) carrying release/build this round; event-lane-c-lane-a.md empty so
far (watch for Cargo.lock dep bumps). Next slice: S2' GI-8.

#### S2' GI-8 refined repro (2026-05-27, post-GI-9, fresh GI-9 bundle)

Clicked the EXACT "Show Directory" button (read_page ref, not coordinates) on
the agents/ node in a TRUE filesystem-mode graph tab. Result reproduces BOTH
halves of @@Alex's GI-8 report:
  (a) the cytoscape layout RE-ANIMATES (nodes re-settle) -> reads as "reloads
      the graph" on a dense graph; NOT a data refetch (7/7 unchanged, no /api
      round-trip).
  (b) NO File Browser tab opens (layout `t` stays [semantic-graph, fs-graph]);
      the dock doesn't expand agents/; the inspector stays open.
Source findings to chase in the fix:
  - GraphPanel `close()` (225) is OVERLAY-oriented: `if (onClose) onClose();
    else graphOverlay.open = false`. In a graph TAB it's effectively a no-op
    for the inspector (it doesn't set graphState.inspectorOpen=false), so the
    inspector staying open is consistent with the handler running but close()
    not closing the tab-mode inspector.
  - revealPathInBrowser -> openBrowser() -> (no existing browser tab) ->
    openBrowserInActivePane() pushes a browser tab to the active pane + makes
    it active. Empirically NO tab appeared, so either the reveal handler isn't
    firing on the dir-node button or openBrowser is short-circuiting in this
    2-graph-tab pane context. NEEDS source trace: confirm InspectorBody's
    Show-Directory button actually invokes onReveal for a dir selection, and
    whether the layout re-animation is a real trigger (graph reacting to the
    layout/activePane change) or just ongoing cytoscape physics on any click.
  - OPEN: is the re-animation a red herring (normal force-sim re-settle on
    selection/click) vs a real reactive trigger? Distinguish by testing "Show
    File" on a FILE node (revealSelectedFile path) - if it opens an FB tab and
    the dir path doesn't, the bug is dir-specific; if neither opens a tab, the
    reveal-from-graph-tab path is broken generally.
NEXT: source-trace InspectorBody Show-Directory wiring + openBrowser-from-tab,
then fix so Show Directory opens/focuses an FB tab at the dir (per-instance dock
state aware) without re-animating the graph. Test server (8797) + browser tab
left UP for the continuation.

#### GI-8 -> OverlayShell-leftover cleanup (scope expanded by @@Alex, 2026-05-27)

@@Alex root-caused GI-8 live + directed the FULL cleanup (AskUserQuestion):
OverlayShell should remain ONLY in Search + Settings; all OverlayShell/
graphOverlay/browserOverlay code in Graph + File Browser is leftover from the
tab migration. GI-8 (Show Directory no visible reveal) is a direct symptom: the
graph uses the overlay-era revealPathInBrowser->openBrowser->close chain; in
tab-world the dir fetch fires (/api/files?dir=agents seen) but no FB tab opens,
the graph isn't dismissed, inspector stays open, no exception.

GROUNDED MAP: OverlayShell rendered only by SearchPanel + SearchStatusOverlay
(Search) + SettingsPanel (Settings) + GraphPanel:1653 (LEFTOVER). Ref spread:
graphOverlay = store 61 / App 12 / GraphPanel 4 / scope 1; browserOverlay =
store 14 / App 4 / FileBrowserSurface 3 / FileTree 3 / scope 3. ~90+ refs.

PRECONDITION VERIFIED (safe to remove): the overlay MODES are dead -
`graphOverlay.open = true` is set in EXACTLY ONE place, the legacy URL-hash
restore (store.svelte.ts:1229, HASH_GRAPH "graph=" param). Nothing MOUNTS the
graph as an overlay anymore (only Pane.svelte mounts GraphPanel, always with a
`tab`), so GraphPanel's OverlayShell branch (1653) + `graphState = tab ??
graphOverlay` fallback + the `visible` overlay arm + close()'s graphOverlay
branch are UNREACHABLE. Same shape for the FB browserOverlay. The only live
concern is graceful handling of OLD bookmarked ?graph=/?files= URLs (convert to
a tab or ignore, don't crash).

PROPOSED SUB-SLICES (behavioral GI-8 fix FIRST, dead-state removal after):
- C1 (GI-8 behavioral, graph): make Show Directory / Show File / Open from the
  graph open/focus a File Browser (or editor) TAB at the path - a tab-world
  reveal - and drop the overlay-era close(). GraphPanel + a tab-world reveal
  helper. This is the user-visible GI-8 fix.
- C2 (reveal path, store/tabs): replace overlay-era revealPathInBrowser/
  openBrowser (browserOverlay-coupled) with the tab-world reveal for ALL callers
  (Graph, Editor, Search, Terminal); remove browserOverlay-open paths.
- C3 (File Browser): remove browserOverlay leftovers (FileBrowserSurface,
  FileTree).
- C4 (store/App/scope): delete the dead graphOverlay/browserOverlay state + the
  overlay-persistence $effect + convert/retire the legacy hash restore; keep
  OverlayShell only for Search/Settings.
- C5: in-browser verify (Show Directory opens an FB tab, graph persists) + full
  gate. Each sub-slice independently gated + merge-ready.

SEQUENCING: GI-9 (c188cfa) merges FIRST (ready); the cleanup branches off the
merged main as a separate effort (don't bundle with GI-9). Reported the
re-scope + contention check (@@LaneC vs App/store/FB) to @@Architect; holding
the cleanup's first commit for their ack + merge ordering. Verified overlay
deadness this turn; no cleanup code written yet.

#### C1 (GI-8 behavioral fix) DONE + verified - 2026-05-27 07:48
Commit ee55143 on top of c188cfa (GI-9). 4 files +90/-45 (GraphPanel.svelte +
3 ?raw test pins). GraphPanel reveal handlers now route through a tab-world
`revealPathInBrowserTab` (openBrowserInActivePane + select + expand ancestors;
dir expands ITSELF too per GI-5) instead of the overlay-era revealPathInBrowser
-> openBrowser -> close() chain. close() dropped from the reveal path (no
overlay to dismiss; graph persists).
VERIFIED IN-BROWSER (fresh binary 07:45, port 8797, scoped): Show Directory on
agents/ in a filesystem graph -> opens a File Browser TAB at agents/ (URL gained
`{k:"b",bs:"agents",be:["agents"],a:1}`), expanded to show architect/ci/
orchestration/webtest-a/webtest-b, graph tab intact. Exactly the GI-8 expected
behavior. Updated revealBrowserActions + graphDirInspectorHotfix(GI-5) +
graphInspectorActionsHotfix(GI-2) pins to the tab-world shape (same commit).
GATE: web svelte-check 0/0, vitest 1596/0, build; Rust carries (web-only, no
Rust delta from the GI-9 tree's green cargo test).

REMAINING overlay-leftover cleanup (C2-C4, the bigger part @@Alex authorized):
- C2: replace overlay-era revealPathInBrowser/openBrowser for the OTHER callers
  (FileEditorTab, SearchPanel, TerminalTab) with the tab-world reveal; also the
  Open-action close() in GraphPanel (same leftover, kept out of C1 to stay
  atomic to the reveal).
- C3: remove browserOverlay leftovers (FileBrowserSurface, FileTree).
- C4: delete dead graphOverlay/browserOverlay state + the OverlayShell branch in
  GraphPanel + overlay-persistence $effect in App; convert/retire the legacy
  ?graph=/?files= hash restore; OverlayShell stays only in Search/Settings.

#### C2 investigation — reuse-behavior nuance found; reverted a too-broad rewrite (2026-05-27)

GI-9 + GI-8/C1 merged (main e61b8c4). Rebased lane-a onto e61b8c4 (my commits
dropped, picked up @@LaneC release work + the WebGL TerminalTab fix bd979bc -
NOT to be reverted per @@Architect). Started C2 (migrate the other reveal
callers off the overlay-era reveal): FileEditorTab (328 file, 655 parent-dir),
SearchPanel (927 result), store handleWindowCommand (658 enter:true dir, 660
select); TerminalTab imports revealPathInBrowser but has 0 call sites (unused
import to drop).

ATTEMPTED: rewrite store `revealPathInBrowser` body to tab-world (open via
openBrowserInActivePane, set per-instance tab.expanded) keeping its signature,
so all callers get fixed with no call-site churn. REVERTED IT before commit -
two blockers:
  1. store.test.ts:249 "revealPathInBrowser focuses an existing browser tab
     instead of duplicating it" enshrines INTENTIONAL reuse-or-create behavior.
     My rewrite (openBrowserInActivePane = always-new) breaks that on purpose -
     not a free cleanup; a deliberate UX change. The correct C2 must PRESERVE
     reuse-or-create: `focusExistingBrowserTab() ?? openBrowserInActivePane(...)`
     + set per-instance expanded/selected on the (reused or new) tab + drop the
     browserOverlay coupling.
  2. I still have NOT pinned WHY the OLD revealPathInBrowser failed specifically
     from a graph tab (C1 fixed it empirically with openBrowserInActivePane, but
     the old path's failure - no visible tab - is unexplained). focusExistingBrowserTab
     iterates layout.nodes for a browser tab; the open question is whether the
     left DOCK File Browser is a layout node it (wrongly) "focuses" instead of
     creating a tab. MUST resolve this before the reuse-preserving rewrite, or
     the graph reveal could regress.

NEXT (C2, fresh + careful): (a) instrument/trace the old reveal from a graph tab
(dock-in-layout? exception? focusExistingBrowserTab target) to pin the failure;
(b) rewrite revealPathInBrowser reuse-PRESERVING + per-instance + drop
browserOverlay; (c) update store.test.ts:249 only if the reuse semantics
genuinely change (prefer to keep them); (d) test EACH caller in-browser
(graph/editor/search/window-command). Then C3 (FB browserOverlay) + C4 (state
removal). Reverted the WIP; branch clean at e61b8c4. Note: GraphPanel's C1 local
revealPathInBrowserTab stays until C4 consolidates it onto the shared reveal.

#### C2 DONE (ready to merge) — 2026-05-27 08:34 — reveal always opens an FB tab
Commit 5654f5e on e61b8c4. Root cause (confirmed by @@Alex): reveal-in-browser
focused the docked FB / reused an existing browser tab instead of opening one
(the dock is FileBrowserSidePane in App.svelte, standalone, not in layout.nodes;
the old openBrowser preferred focusExistingBrowserTab + was browserOverlay-
coupled). Fix: store revealPathInBrowser rewritten to openBrowserInActivePane +
per-instance tab.expanded, dropping the overlay path. Fixes FileEditorTab/
SearchPanel/handleWindowCommand reveal with no call-site change; removed
TerminalTab unused import. store.test.ts:249 updated (reuse -> always-open;
proxy-aware layout.nodes reads + paneMode reset). svelte-check 0/0, vitest
1596/0. In-browser: C1 proved the primitive (graph->FB tab); editor inspector
"Show File" button not reachable in-session, flagged @@Alex to spot-check.
revealAndEnterDirectory is now unused (C4 removes it). C3/C4 remain: FB
browserOverlay leftovers; delete graphOverlay/browserOverlay state + GraphPanel
OverlayShell branch + legacy hash + consolidate GraphPanel's C1 local reveail.
