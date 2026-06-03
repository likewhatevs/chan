# task-LaneC-LaneA-2: B2 adopt + B6 + B9 - DONE

From: @@LaneC (phase-17)  To: @@LaneA  Re: task-LaneA-LaneC-2

All three items done, own-gate GREEN, empirically smoked. No commit/push (round
close + your call). Boundaries respected: no App.svelte, no Pane.svelte (B4),
no GraphCanvas edits needed after all (B9 fit entirely in GraphPanel + store).

## B2 - adopted + owned

Verified the in-tree impl (new-team-1's @@LaneC's work): blob shas match its
report, dead BULLET_MARK removed, depth cycle disc/circle/square marker-agnostic,
2x glyph gap. make web-check green from my env. Already @@Alex-reviewed + smoked.

## B6 - save-dialog autocomplete (DONE, diverged from recon - root-caused)

Recon premise (folder branch omits the file branch's autocomplete) was wrong:
the modal's suggestions are KIND-AGNOSTIC. Real cause: tree.entries is lazy
(refreshTree loads only the root; children load on FB expand), so any dialog
opened without browsing to the target (save-from-draft) had no entries for deep
paths. FIX (PathPromptModal.svelte): an $effect that progressively loadTreeDir()s
each typed ancestor dir KNOWN to exist (folderSet-gated -> no 404; no full-tree
walk -> safe on huge trees). Fixes save-from-draft AND every path dialog.
EMPIRICAL: Chrome on a scoped server - docs/journals/phase-1 now autocompletes
to phase-16/phase-17 (was empty); no console errors. (Confirmed with your note
that B3 uses api.list-direct, so no overlap.)

## B9 - graph bugs a/b/c (DONE, all 3 empirically confirmed)

Implemented by a general-purpose SUBAGENT under my direction (the 2961-line
GraphPanel needed a fresh context; bash grep was returning empty on that file
this session), then I VERIFIED: diff-reviewed the logic, ran an independent
make web-check, and browser-smoked all three on a live d3-graph. Fits entirely
in GraphPanel.svelte + store.svelte.ts + 3 test files (GraphCanvas untouched).

- (a) Fresh Cmd+Shift+M graph is semantic; the spine ships in the /api/graph
  payload, so onGraphDoubleClick now toggles graphState.expanded[path] for a
  folder node client-side (toggleSemanticDirExpand) - no fetch, no mode flip.
  SMOKE: fresh graph, dbl-click notes/ -> expands (7->8 nodes), stayed semantic
  (gm:s), no prior "Graph from here" needed. FIXED.
- (b) Depth slider re-seeds from the SELECTED node (seedExpandedFromSelected):
  selected dir (else scope root) + ancestors + every loaded dir within `depth`.
  Wired into both load paths; authoritative. SMOKE: select notes/, slider 1->2
  -> expanded set grew to [notes, notes/daily], 4->8 nodes. Works AFTER manual
  expand/collapse (the broken case). FIXED.
- (c) "Graph from here" on a directory now STAYS semantic (was filesystem =
  directories-only), and scopedNodeIds uses the expanded-ancestor tree model
  with tags/mentions/languages always visible. store.openFsGraphForDirectory
  also switched to semantic for the FB entry point. SMOKE: graph-from-here on
  notes/ -> stayed semantic, 16-node scope, layer counts tag 5 / contact 2 /
  language 1 / markdown 5 (NOT directories-only). FIXED.

Subagent flagged one intended behavior to eyeball: a slider move while a deep
node is selected collapses sibling branches (rebuilds from the selected origin)
- matches @@Alex's stated model + the prior authoritative-slider semantics.

## Own-gate: GREEN (independently re-run from my env)

make web-check: svelte-check 0 ERRORS (1 pre-existing a11y WARNING in
RichPrompt.svelte = @@LaneB's, not mine); vitest 1650 passed / 167 files
(B2 + B6 + B9 tests incl. new ones); build OK.

## Pathspec shas (uncommitted WIP; for the round-close commit)

B2:
  3c8bb610...  web/src/editor/decorations/blocks.ts
  85b07bfe...  web/src/editor/decorations/blocks.test.ts
  9bea4beb...  web/src/editor/Wysiwyg.svelte
B6:
  5a978ac6...  web/src/components/PathPromptModal.svelte
  e5b98e40...  web/src/components/PathPromptModal.test.ts
B9:
  c4396c14...  web/src/components/GraphPanel.svelte
  6dfcb50f...  web/src/state/store.svelte.ts
  a40a3ef5...  web/src/state/store.test.ts
  bca151b0...  web/src/components/graphDepthFilter.test.ts
  878616fd...  web/src/components/graphFsSpineCompleteness.test.ts

Test server (/tmp/chanc-b6 :8842, binary /tmp/chanc-LaneC, Chrome tab) being
torn down now. Full detail + the recon-imprecision notes for the retro are in
journals/journal-LaneC.md. Ready for round-close commit on your signal.
