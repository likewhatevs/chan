# task-LaneA-LaneC-2: adopt B2 + Wave-2 (B6, B9)

From: @@LaneA  To: @@LaneC (phase-17)  Wave: 2

## First: ADOPT B2 (do NOT redo - it is DONE + quiescent)

The dual-team collision is resolved: new-team-1 is stood down. Its @@LaneC
already implemented B2 + @@Alex's 2x glyph-gap into YOUR files
(blocks.ts/Wysiwyg.svelte/blocks.test.ts), gate-green + browser-smoked +
@@Alex reviewed the render. The files are quiescent (no edits since stand-down).
Report: task-LaneC-LaneA-1.md (margin-right 4.48px = doubled gap; depth cycle
disc/circle/square, marker-agnostic).

Your job is to OWN it, not rebuild it:
1. Read the 3 files + the report. Confirm the impl matches the spec (depth
   cycle, all markers -/*/+ identical glyph, 2x gap, even baseline).
2. Light verify from YOUR env: `make web-check` green; grep for any DEAD/unused
   symbol left behind (the earlier worry was a leftover `BULLET_MARK` - confirm
   there is no unused export/const; svelte-check + the build should be clean).
3. If clean -> you own B2; it commits at round close under your lane. If you
   find a REAL issue (dead code, a glyph/gap miss), fix it in place (your file).
   Note the adoption + any fix in journal-LaneC.

## Then Wave-2

### B6 - save-dialog autocomplete for folder-mode drafts

Saving a draft that is a DIRECTORY (has images) opens PathPromptModal in folder
mode WITHOUT path autocomplete; the file branch passes it. Give folder-mode the
same autocomplete. draft.md image-10.
- tabs.svelte.ts saveDraftTabToWorkspace ~2064-2126; folder branch ~2085-94
  omits what the file branch passes.
- PathPromptModal already computes directory suggestions ~200-251 - reuse.
- This is YOUR tabs.svelte.ts region (saveDraft ~2085). @@LaneB owns the pane
  (~2353/2618) + prompt-sink (~1433) regions - far apart, interleave-safe; I
  commit the merged file at round close.

### B9 - graph bugs a/b/c (GraphPanel.svelte + store.svelte.ts)

RE-READ draft.md's graph bullets VERBATIM (@@Alex re-describes the layer model
precisely). The three interconnected sub-bugs:
- (a) Fresh cmd+shift+m graph opens semantic; double-click on a directory is a
  no-op until "Graph from here" flips to filesystem mode (onGraphDoubleClick
  ~231-240). Make directory-expand work from the fresh graph.
- (b) After expand/collapse the depth slider stops (seedExpandedToDepth
  ~352-376 seeds the set but never re-runs layout). Slider must expand all dirs
  FROM the selected node onward (root+max = whole workspace; node 2 deep = only
  its subtree).
- (c) "Graph from here" on a directory drops the initial layers (graphFromHere
  ~390-414 forces filesystem-only). Keep ALL layers: directory spine, files
  with edges to dirs, markdown link/backlink + hashtag + contact/mention edges,
  language edges to files (scopedNodeIds ~861-999, RenderedEdgeKind ~444-449).
- KEEP B9 in store.svelte.ts/GraphPanel.svelte. App.svelte (the cmd+shift+m
  handler ~654-658) is @@LaneB's - if B9 MUST touch it, STOP and route that one
  line through @@LaneA.

## Gate

- make web-check (vitest) + svelte-check + npm run build.
- Browser-smoke (Svelte-5 reactivity is runtime-only, static gates miss it):
  B6 folder-draft save shows autocomplete; B9 fresh-graph dir expand + slider
  from a selected node + "graph from here" keeps all layers. rust-embed: npm
  run build before cargo build; smoke the SERVED bundle.

## Report

Cut task-LaneC-LaneA-2 (adopt note + B6 + B9 summary + own-gate-green + pathspec
shas) + poke @@LaneA.
