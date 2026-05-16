# syseng-1: Phase 2 kickoff — lane scoping + pre-architect survey

Owner: @@Syseng. Status: PREP.

@@Architect has not yet created the [[chan-pre-release-phase-2/journal.md]]
or dispatched task assignments. This file is the syseng kickoff: a
lane-mapping of the [[chan-pre-release-phase-2/request.md]] work items,
a code survey of the parts the user's complaints lean on, and the open
questions that need @@Architect or Alex before deep work begins.

When @@Architect lands the journal, fold the syseng-lane items below
into the formal task assignment and supersede this file with one or
more numbered syseng-N.md tasks.

## Phase 1 carry-forward

From [[chan-pre-release-phase-1/summary.md]] and prior syseng work:

- Watcher hardening shipped (`apply_watch_change` in
  `crates/chan-server/src/indexer.rs`) — symlink, FIFO, broken,
  missing events no longer pin `/api/index/status` to Error.
- `/api/fs-graph` walker uses lstat semantics, classifies symlinks
  by literal `readlink` (no `resolve_safe_strict` on the target),
  ghosts broken/outside/special targets, dedups hardlinks by
  `(st_dev, st_ino)`, caps at `MAX_DEPTH=6`, and rejects mid-path
  symlink escapes via canonicalized parent check.
- Self-upgrade Windows install split + HTTPS guard centralised in
  `crates/chan/src/update.rs`.
- Open release follow-ups left at phase-1 seal: signed release
  checksums; auto-rollback on new-binary launch failure.

Phase-1 fixture script lives in
[[chan-pre-release-phase-1/syseng-1.md]] under "Fixture drive". Same
script still rebuilds a usable hardening fixture; re-use rather than
reinvent.

## Phase 2 lane mapping

Request items grouped by who owns them. @@Syseng-lane is anything
that touches filesystem semantics, indexer correctness, watcher
coupling, or operational invariants. Items I'd review even when I
don't own are tagged `(review)`.

### Owned by @@Syseng

| Request item | Notes |
|---|---|
| Search → tags only from markdown files | Verify and either land a regression test or document that the perceived bug is elsewhere. See "Tag extraction survey" below — code already gates this. |
| Graph → filesystem as source of truth before plotting | Content graph (`/api/graph`) currently reads from the graph SQLite without re-stating files. A note deleted on disk but not yet reindexed shows up as a real node. Owner of the correctness fix; deliverable can land in chan-server or chan-drive depending on architect's call (see "Content graph FS truth" below). |
| Graph → ghost deleted file/dir while overlay is open | Same fix surface as the previous row. Adds a `missing: true` flag (the `/api/graph` shape already supports it for unresolved link targets — extend to indexed-but-vanished files). |
| Graph → add newly created node while overlay is open | Watcher → frontend signal already exists (`bus::make_watch_bridge` → `routes/ws.rs`). The backend side is mostly already there; @@Frontend consumes the existing /ws stream. Syseng confirms the stream's semantics for graph consumers. |

### Co-owned with @@Backend (review by @@Syseng)

| Request item | Notes |
|---|---|
| Graph → depth slider bound to drive shape | Computing the real max depth requires walking the drive (or asking chan-drive). Backend route + @@Syseng on the walk-cost / symlink-loop behaviour. The fs-graph walker already has the `(dev, ino)` visited set; reuse it. |
| Search → collapse search results per file | The dedupe sits at the search ranker. @@Backend leads; syseng review only if it changes how `index_file` populates tantivy. |
| Search → language elevated to graph (whole-drive Graph This, Language nodes, Language filter) | Currently `language:<name>` is fully client-side (web/src/components/SearchPanel.svelte). Elevating to graph means a new node kind + edges to folders. @@Backend leads; syseng reviews because language counts come from chan-report and the per-folder fan-out shape matters for graph render cost. |

### Owned by @@Frontend (syseng not on the path)

| Request item | Notes |
|---|---|
| Graph → force layout for stacked nodes on folder scope |  |
| Editor → enumerated list indent visual guidance |  |
| Editor → context menu Graph this / Show file |  |
| Search → Graph This link in code report |  |
| Search → click heading shows file inspector |  |
| Search → inspector layout / close-button placement |  |

## Tag extraction survey (request item: "tags only from markdown")

Verified in code at `2026-05-16`:

- `chan_drive::Drive::index_file` → `index_file_inner`
  → `parse_doc` calls `markdown::extract_tokens(body)` then
  `if !fs_ops::is_markdown_file(rel) { tokens.retain(|t|
  !matches!(t, Token::Tag { .. })); }`
  (chan-core `crates/chan-drive/src/drive.rs:2433-2436`).
- `fs_ops::is_markdown_file` accepts only `.md` (case-insensitive)
  (`crates/chan-drive/src/fs_ops.rs:80-83`).
- `index_file` is only invoked for `is_indexable_text` paths, which
  is `EditableText` = `.md` + `.txt` only. Source-class text
  (`.py`, `.rs`, `.c`, `.go`, `.ts`, ...) is **never** read by the
  indexer (`fs_ops.rs:60-73`).

What this means for the user's complaint "today I think we index
#tags from include in source code":

1. `#include` in `.c` / `.h` is **not** an indexable-text file. The
   indexer skips it entirely. No tag, no full-text entry, no
   heading. So that specific case is already not happening.
2. `.txt` files **are** indexed for full-text and headings but
   their `#tag`-looking strings are **not** promoted to graph tag
   nodes (the `retain` strips them). So `.txt` is also fine.
3. The one residual surface is **search results** containing a `.txt`
   file whose body happens to include a `#` sequence — tantivy
   tokenization may or may not match a `#name` query. This is
   "full-text search behaves as full-text search", not a tag-graph
   bug. Worth confirming with Alex whether that is also in scope.

Recommendation: instead of "fix the tag scope" code change, file a
small **regression test** in chan-drive's drive-level tests that
asserts:

- a `.txt` file containing `#urgent` produces zero `EdgeKind::Tag`
  edges after `index_file`
- a `.py` source file is never opened by `index_file`
- a `.md` file with `#urgent` produces exactly one tag edge

That nails the contract so future refactors can't regress it. If
Alex's report is grounded in a specific bad behaviour (e.g.
"`#tag` from a `.txt` file shows up under the Tags list in the
sidebar"), the fixture for that test surfaces it; if it doesn't
reproduce, the user can show us the path they're worried about.

## Content graph FS truth (request item: "use filesystem as source of truth")

`/api/graph` (`crates/chan-server/src/routes/graph.rs:286-480`)
builds its node list from `graph.files()`, which is a SQLite scan
of the chan-drive graph DB (`crates/chan-core/.../graph.rs`).
Nothing in the route re-stats the filesystem before emitting file
nodes. So when a markdown file is deleted on disk but the watcher
event has not yet propagated through `forget_file`, the file
still shows up as a normal node. Clicking it produces the
"not in the current file list" inspector message the user
reported.

Two non-exclusive fixes, pick one or both:

**(A) Stat-on-emit in `api_graph`.**

After computing the `files` set, filter or mark each entry by
calling `drive.stat(rel)` (or a single batched scan). Existing
files emit normally; missing files emit with `missing: true`
(the `GraphNodeView::File` shape already supports it because of
the ghost-from-link case at `graph.rs:474-481`).

Cost: one stat per indexed file per `/api/graph` call. For
realistic drives (low thousands of markdown files) this is
single-digit ms. The route is not called frequently. Acceptable.

**(B) Lazy-reconcile in `Drive::graph().files()`.**

Inside chan-drive: before returning `files`, run a delete pass
that drops rows whose path no longer exists. This makes the
graph DB authoritative again and avoids the per-call stat cost.

Cost: a separate consistency loop with the watcher (race on
add-then-immediately-render). Watcher already eventually calls
`forget_file`, so lazy-reconcile is duplicative; only worth it if
we want chan-drive itself to be self-healing for crash-recovery.

Recommendation: **(A) in chan-server**. It's local to this repo,
keeps chan-drive's contract unchanged, and matches the existing
ghost-on-emit pattern for unresolved links. Symlink/FIFO/special
classification can reuse the lstat helpers already imported by
the indexer.

Edge cases for review:

- File deleted between `stat` and frontend render. Acceptable —
  shows briefly, next poll catches it.
- File replaced by a symlink between scans. Stat-on-emit will see
  a regular file at the symlink's target (if it points back into
  the drive); the existing fs-graph walker classifies this as a
  symlink node. Decision: should the content graph also
  reclassify, or is "still markdown content" the contract?
  Defer to @@Backend; flagging it.
- Hardlink dedup. The content graph keys by path, not `(dev, ino)`.
  Two hardlinked copies of the same `.md` show up as two distinct
  content nodes today. Existing behaviour; not asked to change.

## Graph live update (request item: "add newly created node while overlay is open")

Existing surfaces already cover the backend side:

- `crates/chan-server/src/bus.rs:36-69` — `make_watch_bridge`
  publishes JSON `{"type":"watch","event":...}` frames to all /ws
  subscribers.
- `crates/chan-server/src/routes/ws.rs:14-` — `/ws` subscribes to
  the events broadcast and pumps frames.

What's missing isn't backend; it's the frontend wiring on the
graph overlay to (a) subscribe, (b) filter relevant events
(matching the current scope filter), and (c) call `/api/graph`
again or merge the incoming event into the in-memory graph state.

@@Frontend leads this. Syseng confirms:

1. The /ws stream as-shipped emits one event per filesystem change
   (no batching, no debounce). For bulk operations (`git
   checkout`, `mv folder`) the overlay should debounce before
   re-rendering. Frontend concern but worth saying out loud.
2. Self-writes are suppressed in the events stream (see
   `bus.rs::make_watch_bridge` predicate). Editor writes from
   inside the app won't double-trigger a "new file" pop-in. Good.
3. The event payload includes the `path` and `kind` (file/dir/
   special). Graph filtering by scope is straightforward.

## Indexer / watcher invariants to preserve

Anything that touches the content graph or the indexer should
preserve the watcher hardening that landed in phase 1
([[chan-pre-release-phase-1/architect-syseng-2.md]]):

- `apply_watch_change` lstat-classifies before calling
  `Drive::index_file` / `forget_file`.
- Symlinks, FIFOs, sockets, devices, directories ≠ Error.
- Missing paths are delete races, not errors.

If the FS-truth fix in `api_graph` adds new stat calls, they
should use `std::fs::symlink_metadata`, not `metadata()`, to keep
lstat discipline. Confirmed by reading the existing fs_graph
walker.

## Decisions locked by Alex (2026-05-16)

1. **FS-truth fix locus**: chan-server stat-on-emit (recommendation A).
   Implementation lands in `crates/chan-server/src/routes/graph.rs`,
   reusing the existing `GraphNodeView::File { missing: true }` shape.
   chan-drive's `files()` contract stays unchanged.
2. **Tag-from-source-code complaint**: ship a regression test only,
   no code change. The existing `is_markdown_file` gate at
   chan-core `drive.rs:2434` covers it; tests pin the contract so
   future refactors can't regress it.
3. **Phase-1 self-upgrade follow-ups** (signed checksums,
   auto-rollback): out of scope for phase 2. Stays deferred.

## Open questions for @@Architect (before task dispatch)

1. Depth-slider max-depth probe: reuse `/api/fs-graph` with its
   `truncated: true` signal, or a dedicated `/api/fs-graph/max-depth`
   route? Frontend prefers a number; reusing the existing route
   means walking the drive twice when the slider mounts. Lean
   toward the dedicated route. @@Backend's call.
2. Graph live updates: must-have for phase 2, or stretch?
   Backend signal already exists via /ws. Mostly frontend wiring;
   no syseng dependency either way.

## Proposed dispatch (for @@Architect)

With Alex's decisions locked, the syseng-lane work splits into two
self-contained tasks:

- `syseng-2.md`: regression-test tag extraction scope in chan-core.
  Asserts: (a) `.txt` containing `#urgent` produces zero
  `EdgeKind::Tag` edges; (b) `.py` source file is never opened by
  `index_file` (path-class skip); (c) `.md` with `#urgent` produces
  exactly one tag edge. Lives next to the existing chan-drive tag
  tests. No code change unless a test reproduces a real leak.
- `syseng-3.md`: FS truth + ghost-on-vanish in `/api/graph`. Stat
  each indexed file before emit, mark missing as `missing: true`
  using the existing `GraphNodeView::File` shape. Use
  `std::fs::symlink_metadata` to preserve lstat discipline.
  Regression-tested by deleting an indexed file then calling
  `/api/graph` and asserting the node carries `missing: true`.

Live-update wiring (request item "add new node while graph is
open") is @@Frontend's via /ws — backend already broadcasts. Syseng
reviews when that frontend task lands, no upstream blocker.

## Verification gate (will mirror phase 1)

When the dispatched tasks land:

```
cargo build
cargo test
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cd web && npm run check
cd web && npm test -- --run
scripts/pre-push
```

Plus a fresh syseng probe against the phase-1 fixture script
augmented with: a non-markdown `.py` containing `#include`, a
`.txt` containing `#urgent`, and a deleted-but-still-indexed
markdown file for the FS-truth case.

## Status

PREP. Ready for @@Architect to fold this scoping into the journal
and create the formal syseng-N tasks. Holding here so I don't
front-run the architect's coordination role.
