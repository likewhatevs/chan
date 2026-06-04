# task Lead -> LaneB (5): BUG - graph selected node not persisted across window reload

@@Alex (live-testing): the graph does NOT persist the selected node across a
window reload; it should. Editor/graph state survives reload via the layout
hash, but the graph's selection is lost. Graph lane; I root-caused it for you.

## Root cause (recon done - it is B-only, NO @@LaneC change)
The serialize + restore plumbing ALREADY exists and is correct:
- serializeLayout (tabs.svelte.ts:3674) writes `gn: t.selectedNodeId` (+ gnl
  selectedNodeLabel) into the graph tab's hash form.
- restore (tabs.svelte.ts ~3840-3855 + store.svelte.ts:2183) reads `gn` back
  into selectedNodeId + pendingSelectId.
- @@LaneC's persistStateToHash just CALLS serializeLayout - no C change needed.

The MISSING link is in GraphPanel.svelte (your file): the live selection is a
component-local `let selectedId = $state(...)` (line ~728). It is written on
re-scope (~208) and graph-from-here (~497), and `selectedNode` derives from it
(~1379) - but `selectedId` is NEVER written back to `graphState.selectedNodeId`
(the GraphTab field the serializer reads). Grep shows writes to graphState
.scopeId/.depth/.pendingSelectId/.expanded/.mode/.filters/.inspectorOpen but NO
`graphState.selectedNodeId = ...`. So on a normal node click, the tab field
stays null/stale -> `gn` is omitted -> reload restores nothing.

## Fix (your domain, GraphPanel.svelte)
Sync the live selection into the tab so the (already-correct) serializer
captures it: when `selectedId` (and its label) changes, write
`graphState.selectedNodeId = selectedId` and `graphState.selectedNodeLabel =
<label>` (clear to null when selectedId is null). An `$effect` keyed on
selectedId is the obvious spot; make sure it does not fight the restore path
(pendingSelectId -> selectedId on load) or cause a $state-in-$derived hazard.
NOTE: GraphPanel.svelte triggers the NUL-byte grep-binary issue (line ~308) -
use `grep -a`. You flagged that NUL separator as an out-of-scope nit; since you
are editing this file anyway, your call whether to also fix it to the ` `-escape
form (graphData.svelte.ts:31) - low risk, but only if you are confident.

## Gate + smoke
make web-check + svelte-check + build. BROWSER-SMOKE the actual bug: open a
graph, click a node to select it, reload the window -> the same node is selected
(inspector populated, tab title shows it). Runtime persist/restore; static gates
miss it.

## Note
Graph code already committed (ae22d5a1); this fix APPENDS as its own commit when
green. Cut task-LaneB-Lead-3.md (root cause confirm + fingerprint + the
reload-persist smoke), poke me.
