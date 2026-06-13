# task-Editor-Conductor-46 — graph keep-alive WKWebView walk: CO-SIGN (30/30)

From: @@Editor. To: @@Conductor. Co-signs:
task-Desktop-Conductor-45 (round-2 graph keep-alive walk, 3fdd4bfe).
Date: 2026-06-13.

## Co-sign: YES — 30/30, every verdict matches my spec gates

Verified the table line-by-line against
round-2-graph-walk-editor-assertion-specs.md. No contests. @@Desktop's
harness exceeded the spec: the per-canvas __xform hook upgraded items
1b/6 from my no-remount proxy to a LITERAL transform-value check, and
the inside/outside boundary fixture made #5 — the gap Chrome
structurally could not reach — a real machine assertion.

## What the walk PROVED on the real engine (my lane)

- **The @@Alex-visible symptom is fixed, mechanism machine-proven.**
  1a: load() 2→2 across 2 switch cycles (zero reloads; the fsProbe=5
  noise is the expected depth-probe, correctly demoted per my
  gold-signal correction). 1b: panned to a distinct offset, transform
  x/y/k byte-identical after the switch (the __xform literal read, not
  a proxy). No remount (same canvas node id). That triad IS the
  no-redraw symptom, closed.
- **Lazy restore exactly 1** (item 3) — the tightening I insisted on
  over the original <=2: after a window reload with 2 graph tabs,
  exactly ONE load() with active=true; the hidden graph stayed lazy. A
  mount-gating regression would have shown 2 and the <=2 bound would
  have false-passed it. The exact-1 caught the regression class it
  exists for.
- **Hidden→dirty→one-on-reactivation** (item 4): zero while hidden,
  +1 on reactivation, zero on a further clean switch.
- **#5 out-of-scope ZERO + in-scope control +1** — the new empirical
  gap, now closed. Same hidden-then-reactivate motion, opposite
  outcomes on the dir-scoped boundary fixture: out-of-scope edit → 0
  reload (changeAffectsScope=false), in-scope edit → +1. The control
  proves the boundary is real and the watcher alive, not a dead path.
- **Console clean** (item 7): 0 state_unsafe_mutation / 0 errors. The
  one new $state-in-$effect (canvasEverShown) is verified safe on
  WebKit — exactly the runtime class static gates miss, now empirically
  closed for this commit.

## Item-6 method correction — verified correct, source-grounded

@@Desktop's correction is right and I checked it against source:
Workspace.svelte wraps each split half in `{#key split.a}` / `{#key
split.b}` (lines 73/89), so `cs pane split` mints new pane-node ids →
keyed remount of the subtree (canvas id changes, +1 load, transform
reset). That is EXPECTED structural behavior, not a keep-alive bug —
the pane-tree shape changed. A divider-drag adjusts only `split.ratio`
/ flex-grow (no key change → no remount), which is the correct way to
resize a kept-alive graph and exercises the resume resize()-not-start()
path. Re-run with a synthetic divider-drag → PASS (refit to the new
617px host, transform preserved). No bug.

### Cross-cutting observation (NOT a finding — consistency note)
The `{#key split.a/b}` remount applies to ALL keep-alive tab kinds,
not just graph: terminals (scrollback) and file editors (scroll/undo)
ALSO remount on a pane SPLIT, because they live under the same keyed
subtree. So graph now behaves CONSISTENTLY with terminal/editor on a
split — the keep-alive contract (round 1 + round 2) targets
tab-switch + flip + Hybrid-Nav (none change the pane-tree shape), and
split is deliberately outside it. Pre-existing, consistent, out of
scope. Candidate future enhancement only if @@Alex ever expects a
split to preserve subtree state; worth a one-line doc note at round
close, no task.

## Hand-smoke (the only one) for @@Alex's list
- 1c node-CLICK selection survival: machine-asserted via the
  selection-hash proxy (readable + unchanged across switch); the
  literal canvas-hit-test click is hand-smoke (synthetic hit-test on
  a node's pixel position isn't reliably synthesizable — my ledger).
  One eyeball: select a node, switch away/back, it stays selected +
  inspector intact. The headline (no redraw) is fully machine-covered,
  so this is a low-stakes confirmation.

## State
Provenance clean (walk binary 36ae19d0 = 3fdd4bfe + worktree-only
instrumentation, never committed; clean smoke binary 36e7e132).
@@Desktop retains harness/fixture for any re-run and rebuilds a clean
stripped dist before any release smoke. My round-2 deliverable
(3fdd4bfe) is now empirically validated on the real engine. No push
(local only). B7 (Xcode CI) remains the release-run watch item.
