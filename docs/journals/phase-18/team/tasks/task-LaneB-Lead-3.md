# task LaneB -> Lead (3): selected-node-not-persisted - FIXED + smoked

Done, B-only, GraphPanel.svelte only. Fingerprint: +32 lines, content sha
1feff3c. Appends as its own commit when you're ready.

## Root cause (corrected one detail of the recon)
Your recon: "GraphPanel never writes graphState.selectedNodeId." It actually
DOES - setSelected writes `tab.selectedNodeId` (GraphPanel:2279); the grep for
`graphState.selectedNodeId =` missed it because `graphState = $derived(tab)`,
so the code uses the `tab.` form. The REAL missing link is the persist
TRIGGER: App.svelte's layout-persist effect (lines 154-165) tracks the graph
tab's mode/scopeId/depth/inspectorOpen/pendingSelectId/filters but NOT
selectedNodeId - so a pure selection change never scheduled a hash write, and
`gn` was omitted. (Serialize+restore plumbing is correct, as you found.)
Secondary gap: the programmatic selection paths (graphFromHere, load's
pendingSelectId-resolution) set selectedId without writing the tab field.

## Fix (B-only)
A guarded `$effect` in GraphPanel mirrors `selectedId` (the live source of
truth, covering clicks AND programmatic paths) into
graphState.selectedNodeId/Label and triggers `schedulePersistStateToHash()` +
`scheduleSessionSave()` (both already exported, both debounce internally).
Guard: skip while `pendingSelectId !== null` so a restore/re-scope resolving
through load() neither clobbers the restored `gn` with the initial null nor
persists a transient mid-resolve value. setSelected now clears pendingSelectId
(a user tap supersedes any pending auto-select, so the guard never blocks a
click). No $state-in-$derived hazard (writes are in an $effect, not a derived).

ALTERNATIVE (your call at convergence): the pattern-cleanest fix is ONE line
in App.svelte's persist effect - `void t.selectedNodeId;` next to the other
graph fields (~159). That's @@LaneC's region, so I did the equivalent B-only
(GraphPanel triggers the persist directly, matching how the store's open* fns
already call scheduleSessionSave). If you'd rather centralize it in App.svelte
and drop my trigger calls, say the word - trivial swap. I kept it B-only per
your task-5 framing.

## Gate + smoke
- svelte-check: 0 errors. vitest: 171 files / 1699 tests pass. npm build: OK.
  (Frontend-only; no Rust change.)
- BROWSER-SMOKE (the actual bug): semantic workspace graph -> click index.md
  -> node selected + hash gains `gn=index.md,gnl=index`; `cmd+r` window reload
  -> index.md restored (inspector populated, node selection-ring, tab title
  `path=index`). No console errors. Confirmed `gn` was NOT written on a pure
  select before the fix.

## Notes
- File: web/src/components/GraphPanel.svelte only (graph.rs in the tree is
  @@LaneF's uncommitted doc-comment scrub, not mine).
- NUL-byte (line ~308): left as-is per your deference - out-of-scope nit,
  semantically-risky edge-key change for no payoff.
- Gate ran AFTER my final edit this time (re: the rustfmt stale-green lesson).

Server + Chrome tab + throwaway drive torn down. Standing by.
