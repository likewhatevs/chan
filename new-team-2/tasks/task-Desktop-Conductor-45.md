# task-Desktop-Conductor-45 — round-2 graph keep-alive WKWebView walk: 30/30 GREEN

From: @@Desktop. To: @@Conductor. Re: task-Conductor-Desktop-43
(joint with @@Editor; their specs round-2-graph-walk-editor-assertion-specs.md).
Date: 2026-06-13. @@Editor co-sign slot below.

## Provenance

- Walk binary `36ae19d0` (clean smoke binary `36e7e132`) = worktree
  at settled HEAD `3fdd4bfe` + worktree-only instrumentation (debug
  IPCs + CSP + throttling). The gold-signal load() counter +
  per-canvas __xform hook were injected into GraphPanel/GraphCanvas
  SOURCE and web/dist rebuilt; the debug binary reads dist from disk,
  so the served bundle carried both hooks (verified embedded + live:
  driver hello reported xformHook=true). Isolated $HOME; no peer
  processes touched.
- @@Editor's correction adopted: raw /api/graph fetch counts are
  NOISY (fs-graph depth-probe on reactivation + watcher nonce
  re-emits 2-3x/edit), so the GOLD reload signal is window.__graphLoads
  (a counter at the top of load()); fetch counts demoted to an
  allowed-noise fs-graph side channel.

## Results — 30/30 machine-asserted PASS, 0 FAIL

| item | verdict |
|---|---|
| 1a no reload on switch (2 cycles) | PASS — load() 2→2 (fsProbe noise=5, the expected depth-probe, NOT a reload) |
| 1 keep-alive structure | PASS — both .graph-tab mount; hidden one visibility:hidden |
| 1 no remount on switch | PASS — same canvas node id |
| 1b transform preserved (the @@Alex symptom) | PASS — panned to a distinct offset (via __xform), byte-identical x/y/k after switch |
| 1c selection survives | PASS (proxy) — selection-hash readable; node-CLICK selection is [hand-smoke: canvas hit-test, per your ledger] |
| 2a menu opens + order | PASS — right-click → .tab-menu-bubble; order Depth → Reload → Copy link to graph |
| 2b Reload = one load, menu closes | PASS — load() +1, bubble gone, graph visible |
| 3 lazy restore | PASS — after window reload with 2 graph tabs, exactly ONE load(), entry active=true (hidden stayed lazy) |
| 4 in-scope hidden edit | PASS — zero while hidden, exactly +1 on reactivation (active), zero on further switch (dirty cleared) |
| 5 OUT-of-scope hidden edit (the new gap) | PASS — ZERO reload on reactivation (changeAffectsScope=false) |
| 5 control (in-scope) | PASS — +1, proving the boundary is real / watcher alive |
| 6 resize-while-hidden (divider-drag) | PASS — no load, no remount, canvas refit to new 617px host, transform PRESERVED (resume resize() not start()) |
| 7 console sweep | PASS — 0 errors / 0 state_unsafe_mutation / 0 advisories (the canvasEverShown $state-in-$effect, clean on the real engine) |

## Methodology note on item 6 (not a finding — a test-method correction)

My first run used `cs pane split` to resize (your spec offered it).
That REMOUNTED the graph (canvas id changed, +1 load, transform
reset) — but that is EXPECTED structural behavior, not a keep-alive
bug: Workspace.svelte wraps each split half in `{#key split.a/b}`,
so changing the pane-tree shape remounts the subtree. That is a
DIFFERENT path from the in-pane tab-switch the feature targets. The
correct resize is a DIVIDER-DRAG (changes split.ratio / flex-grow
only, no key change → no remount), which exercises the resume
resize()-not-start() path. Re-ran item 6 with a synthetic
divider-drag on `.divider` (mousedown + window mousemove/up) → all
green. So: split-remount is expected and out of the feature's scope
(tab-switch is the feature); no bug to file. Worth a one-line doc
note only if @@Alex ever expects pane-split to preserve graph state.

## Hand-smoke ledger (per your spec)

- 1c node-CLICK selection survival: canvas hit-test (compute a node's
  screen pos + click) wasn't synthesized; I asserted the
  selection-hash is readable as a proxy. Eyeball: select a node,
  switch away/back, node stays selected + inspector intact.
- The @@Alex visual "no redraw on switch" is worth one human glance
  regardless — but 1a/1b machine-prove the mechanism (zero load() +
  identical transform).

## State

All walk processes torn down; harness + fixture + drivers RETAINED
(for any @@Editor amendment re-run); web/dist still carries the
instrumented bundle + driver — I rebuild a clean dist + strip before
any release smoke. Worktree at 3fdd4bfe + the source instrumentation
(load()/__xform), worktree-only, never committed. Evidence:
/tmp/chan-rc-report-run3-clean.jsonl (+ run1/run2 archives).

## Co-sign

@@Editor: gold-signal load() counter + __xform hook + the
inside/outside boundary fixture all per your specs; item-6 method
corrected to divider-drag (split was a remount, explained above).
Please co-sign or contest. Findings → none from me.
