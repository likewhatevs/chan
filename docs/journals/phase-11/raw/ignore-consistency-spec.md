# Phase 11 ignore-set consistency (TOP PRIORITY)

Root-cause fix for @@Alex's "we're plotting nodes we should ignore"
(node_modules/target showing in the graph; a repo-root drive hit 60K-131K
nodes). OWNER: @@LaneB (backend). @@Alex approved 2026-05-26.

## The requirement (round-1, drifted)
ONE unified ignore set, DEFAULT-SANE, applied CONSISTENTLY everywhere a
drive is walked: File Browser bootstrap, the search-INDEX build, the
GRAPH build, and the watcher feed. Round-1: "which files and directories
are ignored by us: the usual node_modules, venv, etc; we want consistency
across doing this from chan-desktop and `chan serve`."

## What's already there vs the gap
- `WalkFilter` (crates/chan-drive/src/fs_ops.rs) is the mechanism; the
  Library builds it from `registry.index_excluded_dirs` (library.rs).
- APPLIED: bootstrap/File-Browser spine; Library reindex; and the watcher
  feed (your `c9a9aae`, keep it).
- GAP: a repo-root drive still indexed + graphed node_modules/target ->
  60K+ nodes. So either (a) the DEFAULT exclude set is not sane out of the
  box (doesn't include node_modules/target/venv/.git by default), and/or
  (b) an index/graph walk site bypasses the filter - note `drive.rs` uses
  an UNFILTERED `fs_ops::walk_drive` at ~1226 and ~1320, and the main
  `Drive::reindex` (~2000) walk must be checked.

## The fix
1. Make the DEFAULT ignore set sane (built in, not requiring config):
   node_modules, target, venv, .venv, .git, .hg, .svn, .chan, dist,
   build, and similar. Confirm `WalkFilter::default()` / the registry
   default `index_excluded_dirs` includes them; if not, add them. Keep
   registry `index_excluded_dirs` as ADDITIVE user config on top of the
   defaults.
2. Apply the unified WalkFilter to EVERY walk that feeds the index and
   the graph: replace the unfiltered `fs_ops::walk_drive` sites in the
   reindex paths with the filtered walk (or thread the filter through),
   so node_modules/target are never indexed. The graph builds from the
   index, so once the index excludes them the graph should too - verify
   graph.rs has no separate unfiltered walk.
3. Keep `c9a9aae` (watcher feed) folded in.
4. Editable-text gate is unchanged: a user can still OPEN a file inside an
   ignored dir directly; ignored dirs are only excluded from the
   spine/index/graph/watch, not from explicit open.

## Verification (use a SMALL seeded /tmp drive, NEVER the repo root)
- Seed `/tmp/chan-test-lane-b-ignore` with: a few real `.md` files + a
  fake `node_modules/`, `target/`, `.venv/`, `.git/` each containing junk
  files. Serve it (scoped). Confirm the File Browser, the search index,
  AND the graph all EXCLUDE the ignored dirs (node count reflects only the
  real files; no node_modules/target nodes in the graph). Add an
  end-to-end test asserting the ignored dirs are absent from index +
  graph.
- This should also collapse the runaway node counts entirely.

## Coordination
@@LaneA owns GI-3 (false-broken-link, graph.rs link-target resolution).
The ignore fix is expected to live in the INDEX walk (drive.rs / library /
index facade) + watcher, NOT graph.rs - but if you DO touch graph.rs,
declare it on event-lane-b-lane-a.md so @@LaneA sequences GI-3.
