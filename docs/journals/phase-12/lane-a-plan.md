# @@LaneA plan: graph + File Browser carryover (phase 12)

You are @@LaneA. Full opening context: `bootstrap.md` + `phase-12-backlog.md`.
You own the graph + File Browser carryover.

Scope (priority order, confirm/adjust with @@Architect before slice 1):
1. Overlay / scope-concept WIPE - `../phase-11/overlay-scope-wipe-spec.md`
   (W1-W7, design-resolved). The big one; sub-slice it. The graphOverlay/
   browserOverlay state is LOAD-BEARING (scope resolution + dock) - the spec
   documents the coupling; do not blind-delete.
2. GI-10: drive node pinned to the bottom, spine grows upward (GraphCanvas).
3. Graph loading-state UX (`../phase-11/graph-loading-state-spec.md`).
4. Optional GI-11 `../`/`./` link-target regression locks (else drop).

Surfaces: web/src graph + FB (GraphPanel, GraphCanvas, graphData, store, tabs,
App, FileBrowserSurface, FileTree) + routes fs_graph.rs/graph.rs.

You MAY spawn 2-3 subagents (per @@Alex) to split the work; if the spawn tool
is unavailable, load skills in-session + sub-slice. Each slice independently
gated + merge-ready; report "ready to merge: phase-12-lane-a@<sha>" on
event-lane-a-architect.md.

CONTENTION: @@LaneB's drive->workspace codemod and @@LaneC's cosmetics touch the
same web/src files. Declare touches on the cross-lane channels; expect a
codemod sequencing window from @@Architect. Verify graph/index behavior against
a FRESH binary + reindex before trusting an observation.
