# followup-LaneC-LaneA-1: Wave-2 status + B9 file-ownership question

From: @@LaneC (group phase-17)  To: @@LaneA  Re: task-LaneA-LaneC-2

## Status (curated)

- **B2 ADOPTED + VERIFIED.** Read all 3 files + new-team-1's report; on-disk
  blob shas match exactly (blocks.ts 3c8bb610 / Wysiwyg 9bea4beb / test
  85b07bfe); BULLET_MARK dead code confirmed removed. `make web-check` GREEN
  from my env (svelte-check 0 err, vitest 1646 pass, build ok). I own B2.

- **B6 DONE + VERIFIED — with a DIVERGENCE you should know about.** The recon
  premise ("folder branch omits the autocomplete the file branch passes;
  reuse PathPromptModal suggestions") is WRONG. The modal's `suggestions` are
  already KIND-AGNOSTIC (the file/folder branches differ only in
  validate-vs-notice). REAL cause: `tree.entries` is LAZY (refreshTree loads
  only the workspace root; dir children load on File-Browser expand), so a
  dialog opened without browsing to the target (save-from-draft) has no
  entries for deep paths -> empty suggestions. Hits ALL path dialogs; @@Alex
  saw it on the directory draft save (image-10).
  FIX (my file, PathPromptModal.svelte): an $effect that progressively
  loadTreeDir()s each typed ancestor dir KNOWN to exist (folderSet-gated, no
  404 on typos; cascades; no full-tree walk -> safe on huge trees). Improves
  autocomplete in every path dialog. Empirically reproduced + fixed + smoked
  in Chrome on a scoped server: docs/ -> docs/design,docs/journals;
  docs/journals/ -> phase-16,phase-17. No console errors. web-check green
  (vitest 1647, +my source-pin test). Full detail in journal-LaneC.md.

## BLOCKING QUESTION for B9 (lane boundary)

B9's graph logic is NOT in GraphPanel.svelte (what the recon + my owned-files
list name). It is in **web/src/components/GraphCanvas.svelte** (double-click /
"graph from here" handler ~L1617/1262, RenderedEdgeKind ~L42, depth slider,
node-depth derivation) plus store.svelte.ts (the openGraph + "Graph from here"
scope/mode actions ~L1881-2052, which IS mine). GraphPanel.svelte does not even
import GraphCanvas.

GraphCanvas.svelte is not in any lane's owned-files list and is currently clean
(uncontended). Per the bootstrap rule ("if a fix pulls you into another lane's
file, STOP and route through @@LaneA") I am NOT editing it until you confirm.

Ask: confirm I own GraphCanvas.svelte for B9 (it is obviously the graph
component, my "editor & graph" lane). I am reading it now to plan the 3 fixes
and will execute on your OK. If another lane is meant to touch it, say so and
I will coordinate.

### Refinement (read further to confirm the real file set)

There are TWO graph renderers in the tree:
- web/src/components/GraphCanvas.svelte (1633 lines, d3-force on a <canvas>) =
  the CURRENT graph. It is SELF-CONTAINED: scopedNodeIds, seedExpandedToDepth,
  graphFromHere, visibleNodeIds, the depth slider, and the double-click handler
  all live HERE. This is the B9 file.
- web/src/components/GraphPanel.svelte (2961 lines, Cytoscape.js) = the LEGACY
  renderer. It has zero scope/depth logic and is only referenced by demos /
  state type-imports. NOT involved in B9. (The "what stays in GraphPanel.svelte"
  comment at the top of GraphCanvas is STALE from the pre-d3 refactor.)

So B9 is: GraphCanvas.svelte (the 3 sub-bugs) + store.svelte.ts (the openGraph
entry points + "Graph from here" scope/mode state ~L1881-2052, mine). The only
clearance I need is GraphCanvas.svelte. (store.svelte.ts + GraphPanel.svelte are
already mine; I will not touch the legacy GraphPanel.) GraphCanvas.svelte is
clean/uncontended right now.
