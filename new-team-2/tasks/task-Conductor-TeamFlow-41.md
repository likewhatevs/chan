# task-Conductor-TeamFlow-41 — cross-review: graph keep-alive + Reload (3fdd4bfe)

From: @@Conductor. To: @@TeamFlow. Cut: 2026-06-13.

## Scope

Adversarial cross-review (behaviour-preservation + design conformance,
your dadd5e64 standard — this is the SAME keep-alive surface extended
to graph tabs). 3fdd4bfe, verified on main, 7 files (+356/-36):
Pane.svelte, GraphPanel.svelte, GraphCanvas.svelte +
menuTrims/revealBrowserActions/graphInspectorActionsHotfix tests + new
paneGraphTabKeepAlive.test.ts. Spec:
new-team-2/designs/round-2-graph-keepalive.md. @@Editor's completion
(evidence + their own flags): task-Editor-Conductor-40.md.

## Specific targets

1. **Hidden-state load gating** — the round's trickiest logic:
   - First load LAZY on first activation, NOT mount (the lazy-restore
     perf claim). Verify the load effect can't fire on mount for a
     hidden tab.
   - graphDirty: set ONLY on hidden + in-scope watcher event (and
     hidden loadKey change); reloads EXACTLY once on next activation;
     cleared so a further clean switch fires nothing. Trace the
     one-shot.
   - lastLoadedKey gating: a re-activation with unchanged key + not
     dirty must NOT refetch.
   - The latches are PLAIN locals (not $state) — confirm, and confirm
     no $derived reads them (the state_unsafe_mutation guard). The one
     $state added is canvasEverShown — confirm it's written only in an
     $effect.
2. **GraphCanvas latch-not-toggle** — open={canvasEverShown} must
   latch true once shown and never flip false (start() resets the
   transform at line 1323; stop() discards the sim — toggling kills
   pan/zoom). paused={!active} suspends the rAF loop; resume effect
   resize()s + re-arms WITHOUT start(). Verify the resume path never
   reaches start()/transform-reset.
3. **.graph-tab CSS reconciliation** — flex:1 dropped,
   position:absolute + inset:0 + visibility contract; never
   display:none; inner flex-column kept for children. Confirm no
   existing rule fights it.
4. **onClose/onFlip capture `t`, not `active`** — the bug the design
   called out (old branch closed active.id). Confirm each-item
   closures.
5. **Watcher effect reordering** — changeAffectsScope runs FIRST
   (out-of-scope edits return before either branch); nonce advances
   for hidden tabs so the same event isn't reprocessed on activation.
   Visible path functionally unchanged.
6. **The extra test file** — graphInspectorActionsHotfix.test.ts was
   NOT in the plan's test list but is in the diff. Confirm the change
   is a legitimate menu-structure accommodation (the Reload row), not
   unrelated scope creep.
7. **New test quality** — paneGraphTabKeepAlive.test.ts pins the
   each-block-not-if-chain, the 4 gates, the latch/dirty condition,
   open={canvasEverShown} + paused, the visibility pair — not
   tautologies. menuTrims order pin Depth→Reload→Copy-link.

## Not a finding (Editor pre-flagged)

Visible-watcher reload multiplicity (2-3 /api/graph per single
in-scope edit) is PRE-EXISTING indexer event multiplicity, predates
this commit, visible path unchanged — do NOT chase it as a regression.
It's on the round-close follow-ups list.

## Completion

Findings (or clean pass) → task-TeamFlow-Conductor-<n>.md + 1-line
poke. Findings become tasks routed by me; @@Editor fixes their own
lane. Your verdict gates the WKWebView build (I hold @@Desktop until
this clears, so the walk runs once at a settled HEAD).
