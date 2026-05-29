# Phase 11 graph/inspector hotfix (found in live testing)

@@Alex found these while testing the merged inspector (I1-I4) on the
docs/ drive, 2026-05-26. OWNER: @@LaneA. URGENT - regressions in the
just-shipped inspector + a false-data bug; @@Alex is testing live. Do this
batch as the IMMEDIATE next task when handing back from new-file items
2/3, BEFORE FB-capabilities.

## GI-1: Graph inspector "Open" reloads the graph instead of opening the editor
Click a document (orange) node -> inspector -> click Open. Expected: opens
an Editor tab (the I4 editable-open: markdown rendered, source in source
mode). Actual: it RELOADS THE GRAPH. The onOpen action is mis-wired.
File: `web/src/components/GraphPanel.svelte` (the `onOpen` path; see the
"Open button (onOpen)" comment ~line 1069). Suspect the button click
bubbles into a node-click / graph-refresh, or onOpen calls a graph
scope/reload instead of the editor-open helper.

## GI-2: Graph inspector "Show File" reloads the graph instead of revealing in the File Browser
Click Show File. Expected: File Browser opens with the file selected.
Actual: RELOADS THE GRAPH. File: `web/src/components/GraphPanel.svelte`
`revealSelectedFile()` / `revealPathInBrowser(path, {inspectorOpen:true})`
(~lines 981, 1001). Same smell as GI-1 - the reveal handler is triggering
a graph reload rather than the File-Browser reveal+select.

## GI-3: existing files mislabeled "file does not exist (broken-link target)"
The inspector shows real files as broken-link targets. CONFIRMED FALSE:
`docs/journals/phase-2/frontend-3.md` EXISTS (also phase-3/5/6), yet the
graph node for it reads "file does not exist (broken-link target)".
Likely root cause: relative-link resolution / existence-check base
mismatch. A link like `[...](phase-2/frontend-3.md)` inside a doc under
`journals/` resolves to `journals/phase-2/frontend-3.md` (exists), but the
broken-link existence check resolves "phase-2/frontend-3.md" against the
drive root (or the wrong base) and finds nothing -> false "does not
exist". Fix the link-target resolution so existing files are not
mislabeled. This is the real bug behind the "ghost nodes" @@Alex
distrusted; the loading-state UX in `graph-loading-state-spec.md` still
applies for the genuinely-incomplete-index case and for GENUINELY broken
links (which should remain shown, distinctly, once the index is complete).
Investigate where link targets are resolved + existence-checked
(chan-drive graph/link indexer + the graph endpoint + graphData).

## GI-4: directory nodes slightly bigger (clickability)
Directory nodes should be SLIGHTLY bigger than other nodes (not much) so
they are clearly clickable. File: `web/src/components/GraphCanvas.svelte` /
the cytoscape node-style by node kind.

## Notes
- GI-1/GI-2 are the priority (the inspector's headline actions are broken).
- GI-3 is a correctness/trust bug; coordinate with the graph-loading-state
  task (same Graph area).
- After this batch resumes the queue: FB capabilities, then the rest of
  graph-loading-state + watcher hardening + benchmark.
- @@Architect will rebuild + restart @@Alex's test server after the merge
  so @@Alex can re-verify.

## GI-5 / GI-6: DIRECTORY-node actions still broken (found in re-verify 2026-05-26)

GI-1/GI-2 fixed the FILE-node Open / Show File (`revealSelectedFile`). The
DIRECTORY (fs-mode) node actions were not covered and are broken. @@Alex,
on a `dir:journals` graph, clicked a directory node (e.g. phase-11):

- GI-5: "Show Directory" does NOTHING. Handler is `revealSelectedFsEntry`
  (GraphPanel.svelte ~995 -> `revealPathInBrowser(selectedFsNode.path,
  {inspectorOpen:true})`, wired at template ~1857 `onReveal=...`). Make it
  actually reveal + select the directory in the File Browser (mirror the
  GI-2 file fix; `revealPathInBrowser` may need to handle a DIRECTORY
  path / open the FB at that dir, not just files).
- GI-6: "Graph from here" on a directory flips the inspector to
  "Details / click a result to inspect" (blank) and does NOT re-root the
  graph. Handler is `graphFromHere(fsPath)` (wired at ~1865
  `onSetAsScope`). The GI-1 reactivity fix (stable scopeId|depth|mode +
  untracked load) likely didn't cover the dir re-root path. Fix so
  re-rooting on a directory re-plots the graph rooted there - per round-1,
  "Graph from here picks a new starting point (file or folder), always
  shows its own parent folder, or the drive root" - and KEEPS the node
  selected so the inspector stays populated (not blank).

Add tests that lock the dir-node behavior: Show Directory -> FB reveal+
select (not a no-op); Graph-from-here on a dir -> graph re-roots + the
inspector stays populated (not blank). Verify on a small seeded /tmp drive
with subdirectories. WEB-only (GraphPanel.svelte).

## GI-7: depth slider resets to 1 (found in re-verify 2026-05-26)

On a `dir:journals` graph, dragging the depth slider from 1 to any value
RESETS it back to 1. Expected (round-1): "only when the user increases the
depth slider, we then load the second-degree and so forth"; decreasing
removes layers. If there is genuinely no deeper structure the slider must
still hold/cap gracefully, NOT snap back. Likely the SAME reactivity root
cause as GI-1: a depth change is being treated as (or triggers) a scope
recompute that resets depth to 1 (see GraphPanel.svelte ~176 "depth resets
to 1 [on new scope]"). Fix so a depth-slider change loads THAT depth with
the scope stable (depth is the var that changes), without resetting.
Files: GraphPanel.svelte, web/src/graph/depth.ts, state/graphData.svelte.ts.
Verify on a /tmp drive with nested dirs: depth 2/3 actually loads deeper
layers and the slider holds its value. WEB-only.
