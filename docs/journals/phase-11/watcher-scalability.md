# Phase 11 watcher scalability (analysis + hardening task)

@@Alex (2026-05-26) raised: with "expand all directories" in the File
Browser and "Graph from here" plotting first-degree forward connections,
could we end up with one filesystem watcher per expanded/visible
directory and hit max file descriptors (or inotify watch limits),
especially under git branch switching? When should the algorithm step
back and aggregate watchers into a narrow-enough common ancestor?

## Grounded answer: the UI cannot multiply OS watchers

The implementation already does the aggregation @@Alex is describing. It
does NOT place one OS watcher per directory:

- `crates/chan-drive/src/watch.rs` starts ONE `RecommendedWatcher` with
  `RecursiveMode::Recursive` on the drive root (line ~164). One watcher
  covers the whole tree. (Plus optional Drafts team subtrees.)
- `crates/chan-server/src/bus.rs` `ScopeRegistry` is the per-directory
  pub/sub: subscriptions are LOGICAL, refcounted scope filters keyed by
  first-degree directory path over that single recursive feed
  (`emit_fs` delivers an event only to subscribers of its first-degree
  directory). The code comment is explicit: "one recursive OS watcher on
  the drive ... without the inotify-watch-count pressure on big trees."

So "expand all" and "Graph from here" add only LOGICAL subscriptions
(zero OS cost) and tear them down on collapse via refcount. The OS
watcher count is fixed at one, independent of how many directories the
user expands or graphs. The common ancestor is already the drive root,
watched once. NO file-descriptor or watch explosion from UI actions.

## The residual real risks (different axis)

1. Event-storm VOLUME (any platform): the single broad recursive watch
   sees every change. A git branch switch touches thousands of files ->
   thousands of events through one stream. FD count is fine (one
   watcher); the cost is processing/broadcast load.
2. Linux inotify watch count: `notify`'s `RecursiveMode::Recursive`
   emulates recursion by adding ONE inotify watch per directory (Linux
   has no native recursive inotify). A huge tree (node_modules, .git,
   target) can exhaust `/proc/sys/fs/inotify/max_user_watches`. macOS
   FSEvents is a single stream and has no such limit. (Linux desktop is
   deferred this round, so this is a documented follow-up.)

## Mitigations (some already in place; rest is the hardening task)

- Ignore-rule filtering at the watcher boundary: drop events under
  `.git/`, `node_modules/`, `target/`, `venv/` using the SAME unified
  bootstrap ignore set (WalkFilter), as early as possible - before
  broadcast and before indexing. VERIFY this is applied to the watcher
  feed (today `watch.rs` dispatch filters some `.git` internals via
  `is_chan_internal`; confirm node_modules/target/venv are dropped too).
- Debounce / coalesce bursts (the indexer already debounces ~150ms-1s);
  confirm a storm coalesces rather than fans out per-event.
- fd_budget pacing (the bug-7 fix) caps concurrent index reads, so a
  storm cannot starve the editor or terminal. This is the key guarantee
  that protects interactive features.
- Scoped broadcast: `ScopeRegistry.emit_fs` already delivers only to
  first-degree subscribers, so the UI is not flooded by distant changes.
- Lazy loading: subscriptions exist only for expanded/visible scopes and
  are refcount-torn-down on collapse; bootstrap is eager-counts /
  lazy-contents. Keep it that way.
- Linux inotify-watch-count (follow-up, deferred with Linux desktop): add
  watches only to non-ignored directories (manual recursive walk that
  skips ignored subtrees) instead of `RecursiveMode::Recursive` over
  everything, OR document a `max_user_watches` requirement.

## Hardening task (RELEASED - @@Alex agreed with the approach 2026-05-26)

OWNER: @@LaneB (reassigned 2026-05-26 to parallelize - this is backend
chan-drive/chan-server work, separable from Lane A's web graph/FB/inspector
cluster). @@Alex ratified the single-recursive-watcher + ignore-filtered-
feed + fd-budget approach. Coordinate any chan-drive index/link-resolution
overlap with @@LaneA (who owns GI-3) on the cross-lane channel.
- Verify/extend the unified ignore filter on the WATCHER feed (not just
  the walk): node_modules/target/venv/.git events dropped before
  broadcast + index.
- Empirically confirm a git branch switch on a large repo, while editing
  a file and running terminals, does not starve the editor/terminal
  (the fd_budget + debounce path should hold; this is the bug-7 scenario
  at watcher scale).
- Document the Linux inotify-watch-count follow-up (non-recursive
  per-non-ignored-dir watching) for when Linux desktop is un-deferred.

### End-to-end indexing benchmark (@@Alex add-on, 2026-05-26)
Add an end-to-end test/benchmark that:
- takes a SHALLOW copy of THIS repo as the test drive (e.g.
  `git clone --depth 1 <this repo> <tmp>` or a filtered copy that honors
  the ignore set), so it is realistic and reproducible;
- measures wall-clock time to index END-TO-END in two modes: (1) WITH
  chan-report language analysis, and (2) WITHOUT chan-report; report both
  and the delta, so we can see chan-report's cost;
- runs with the bge* embedding index DISABLED entirely (embeddings are
  opt-in now per the GPU fix; assert no embedding work happens in these
  runs). The benchmark is about the structural index + chan-report only.
- Record the numbers + analysis in the lane journal (per the writing
  rules: include the benchmark, say whether the timings meet expectations).
