# Phase 11 graph dead-ends / loading state (new task)

From @@Alex (2026-05-26). OWNER: @@LaneA (Graph). Sequence after the
inspector, alongside the Graph/partial-load work.

## Observation
The graph shows many "dead-end" nodes - nodes that read as "this file is
not in the filesystem." @@Alex does not believe these are all real. The
open question: are they (a) a faithful reflection of filesystem state
(genuinely broken links / missing targets), or (b) artifacts of the graph
rendering BEFORE indexing completed, i.e. the graph is inaccurate while
the index/walk is still in flight?

## Task
1. INVESTIGATE the root cause. Determine where dead-end/ghost nodes come
   from: link/wikilink targets that do not resolve to a file, vs targets
   that DO exist on disk but are not yet indexed/reconciled when the graph
   is plotted. Distinguish "indexing incomplete" from "genuinely broken
   link." (Relevant: the bootstrap spine knows the real fs tree; the
   semantic graph/links come from the index which may lag.)
2. UX while a scope/depth is still loading or indexing: do NOT render
   inaccurate dead-end ghost nodes. Instead, pull those nodes back and
   show the PARENT directory in a loading state - a pulsing / spinner
   effect, mirroring the File Browser's existing spinning loader on
   expand. Only resolve to real nodes/edges once that scope's data is
   complete.
3. Once indexing for a scope IS complete, any remaining dead-end is a
   REAL broken link and may be shown as such (distinct styling from the
   loading state), so the user can trust it.

## Notes
- This is the Graph half of the partial-load / lazy-loading story: plot
  progressively, show loading state per scope, and never present
  not-yet-known data as fact. Reuse the File Browser's loading-spinner
  pattern for visual consistency.
- Key files: `web/src/components/{GraphPanel,GraphCanvas}.svelte`,
  `web/src/state/graphData.svelte.ts`, the FB loading-spinner component/
  state it should mirror, and the index-status/progress signals (the
  Slice G progress widgets + index status) to know when a scope is
  "done." Backend: confirm whether the graph endpoint can report
  per-scope completeness so the UI knows when to drop the loading state.
- Relates to `watcher-scalability.md` (lazy per-scope loading) and the
  inspector's "Graph from here" (a re-root should show its parent in the
  loading state until that depth resolves).
