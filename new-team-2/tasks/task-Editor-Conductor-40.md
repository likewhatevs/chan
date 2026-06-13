# task-Editor-Conductor-40 — graph keep-alive + Reload menu COMPLETE

From: @@Editor. To: @@Conductor. Cut: 2026-06-13.
Closes: task-Conductor-Editor-39.md.

## Commit

- **`3fdd4bfe`** feat(web): graph tab keep-alive + re-add Reload menu
  item. Pathspec-atomic (pre/post verified): Pane.svelte,
  GraphPanel.svelte, GraphCanvas.svelte + 3 test updates + new
  paneGraphTabKeepAlive.test.ts. 7 files, +356/-36.
- Implemented exactly per designs/round-2-graph-keepalive.md: the
  two non-obvious cores landed as specified — (1) hidden-state load
  gating with PLAIN latch locals (lazy-first / graphDirty /
  lastLoadedKey), (2) GraphCanvas `open` LATCHES via canvasEverShown
  + new `paused` prop suspends the rAF loop, resume re-arms with
  resize() not start().

## Gate

- Own-gate `make web-check` GREEN after the FINAL edit (instrumentation
  removed first): svelte-check 0 errors / 0 warnings, vitest 178 files
  / 1765 tests, build OK, exit 0.
- Sweep confirmed no debug residue before commit.

## Chrome-verified (vite dev + standalone server; gold signal was
load() instrumentation, added/removed within the smoke, gate re-run
after removal)

All assertions on a 25-file workspace, Network filtered to
graph/fs-graph:
- **Switch never reloads**: 2 full graph↔file switch cycles, no
  edits → ZERO load() calls. (design verification #1, #6)
- **Reload button**: exactly 1 /api/graph fetch, menu closes. The
  menu order is Depth → Reload → Copy link to graph as specified.
  (#2)
- **Visible watcher**: on-disk in-scope edit (shell echo) while
  visible → graph live-reloads. NOTE: a single edit fires 2-3
  reloads, not 1 — this is PRE-EXISTING watcher/indexer event
  multiplicity (raw modify + index update + embedding each bump the
  reload nonce >250ms apart, so the 250ms debounce doesn't coalesce
  them). The visible-path logic is functionally unchanged from before
  this commit, so it is not a keep-alive regression; flagging it as
  an observation, not a finding. (#3)
- **Hidden watcher (the new logic)**: in-scope edit while hidden →
  ZERO load() calls while hidden (instrumented, on a fully-loaded
  graph after quiescence); exactly 1 load() on reactivation (the
  graphDirty one-shot); 0 on a further clean switch. (#4)
- **pan/zoom/selection survive**: panned the graph + selected a node
  (inspector open) → switch away/back → identical layout position +
  selection + inspector, ZERO graph reloads (the return only
  re-fires the cheap fs-graph depth probe, exactly as the design
  predicted — distinct from a graph reload). (#6)
- **Lazy restore**: 3 graph tabs + 1 file tab, active = a graph,
  window reload → exactly 1 load() (only the active graph; the 2
  hidden graphs stayed lazy, no fetch until activated). The
  mount-vs-activation gating is the load-bearing perf claim for
  session restore on big workspaces. (#7)
- **Console clean**: no state_unsafe_mutation / onerror / console.error
  across the whole walk (the $state canvasEverShown is written only
  in an $effect; everything else is plain locals). (#8)

### Honest split — what I did NOT instrument
- **Out-of-scope hidden edit → zero on reactivation (#5):** the test
  workspace graph is WORKSPACE-scoped, so every edit is in-scope —
  can't exercise the out-of-scope path here. But changeAffectsScope
  is unchanged and now runs FIRST in the watcher effect: an
  out-of-scope edit returns before reaching either the hidden
  (dirty) or visible (reload) branch, so it sets no dirty flag and
  triggers no reload, visible or hidden — behavior preserved. Covered
  by the unchanged filter + existing graphDepthFilter tests; a
  dir/tag-scoped smoke would confirm empirically (cheap WKWebView
  hand-check add).

### WKWebView-pending (the real gate, same surface as dadd5e64)
Route a @@Desktop build when the tree's ready; I'll drive (same
harness as the round-1 walk). Specifically: the switch-no-redraw
visual (the actual @@Alex-visible symptom), pan/zoom survival, lazy
restore, and the console sweep on the real engine. Fold into the next
walk; the Chrome evidence above is necessary-but-not-sufficient there.

## Review routing

@@TeamFlow cross-reviews (they reviewed dadd5e64). Sha `3fdd4bfe`.
Their targets per the design: latch/dirty correctness (esp. the
hidden-edit dirty path + lazy-first gating), the GraphCanvas
latch-not-toggle + paused/resume, the .graph-tab CSS reconciliation
(flex:1 dropped, position:absolute), and the onClose-captures-t
bug-avoidance. I noted the visible-watcher reload-multiplicity above
so they don't chase it as a regression.

## Follow-ups for round close
- Visible-watcher reload multiplicity (2-3 /api/graph per single
  in-scope edit) — pre-existing, pre-dates this commit; worth a
  separate look at whether the reload nonce should coalesce indexer
  re-emissions, but out of scope here.
- Out-of-scope hidden-edit empirical smoke (dir/tag-scoped graph) on
  the WKWebView pass.
- Multiple mounted GraphPanels share paneWidths.graph (inspector
  width) — pre-existing for the active graph; low risk, design noted.

## Process
Local commit only (no push). Test server + Chrome tabs torn down;
/tmp/graph-lane-ws + the renamed binary copy deleted. B7 (Xcode CI)
remains the release-run watch item.
