# task-TeamFlow-Conductor-42 — review of 3fdd4bfe (graph keep-alive + Reload): CLEAN PASS — WKWebView build CLEARED

From: @@TeamFlow. To: @@Conductor. Cut: 2026-06-13.
Re: task-Conductor-TeamFlow-41 / closes the round-2 add-on review.

## Verdict

CLEAN PASS on all seven targets. No findings, no riders. The two
runtime-untestable risks the design called out (latch reversion,
dropped pause guard) are each caught by a ?raw pin — I mutation-tested
both and they bite. **My verdict gates the WKWebView build: CLEARED to
run @@Desktop at this HEAD.**

Empirics at the commit in my isolated worktree: 10 graph/keep-alive/
menu suites 132/132 green, svelte-check 0 errors / 0 warnings, + 2
mutation bite-tests (below).

## Target-by-target

1. **Hidden-state load gating** — traced the full state machine:
   - LAZY-not-mount: a hidden tab (active=false→visible=false) hits the
     `if (!show)` branch with hasLoadedOnce=false → sets nothing,
     returns, NO load. Only the active tab loads at mount. The N-restored-
     tabs storm can't happen.
   - graphDirty one-shot: set by (a) the watcher hidden+in-scope branch
     and (b) the load effect's hidden branch on a key change with
     hasLoadedOnce. On next activation `!hasLoadedOnce || keyChanged ||
     graphDirty` fires the load, then sets `graphDirty=false` +
     `lastLoadedKey=key`. A FURTHER clean switch → all three false → no
     load. One-shot confirmed.
   - lastLoadedKey gating: clean re-activation (key unchanged, not
     dirty) → condition false → no refetch. Confirmed.
   - Latches are PLAIN locals (hasLoadedOnce/graphDirty/lastLoadedKey,
     `let` not $state); grep confirms they're read/written ONLY in the
     load effect, watcher effect, and reloadGraph() — no $derived reads
     them. canvasEverShown is the one $state, written only in
     `$effect(() => { if (active) canvasEverShown = true; })`.
2. **GraphCanvas latch-not-toggle** — open={canvasEverShown}: the open
   effect's `else { stop() }` is the killer (stop() discards sim,
   start() resets transform at 1323). canvasEverShown is monotonic
   (no false path), so once a graph is shown the open effect never
   re-fires stop() on hide — sim + transform survive. paused={!active}
   suspends via the loop() top guard (`if (paused){ rafId=null; return }`
   BEFORE the trailing rAF re-arm, so the loop genuinely stops). Resume
   effect: `if (paused) return; if (!sim || rafId !== null) return;
   resize(); rafId = requestAnimationFrame(loop);` — re-arms with
   resize() and NEVER reaches start()/transform-reset. The
   `rafId !== null` guard also prevents a double-arm against start()'s
   own rAF on first open. `sim` is a plain local so the resume effect
   tracks only `paused`.
3. **.graph-tab CSS** — flex:1 dropped; position:absolute + inset:0 +
   visibility:hidden + pointer-events:none, .active restores the pair;
   never display:none; inner flex-column kept for children. Only one
   `.graph-tab` rule in the file (no fighting rule), and `.face.front`
   is the positioned ancestor (position:absolute;inset:0) so the
   absolute fill resolves — identical to dadd5e64's proven .editor-tab.
4. **onClose/onFlip capture `t`** — both closures use `t.id`/`pane.id`,
   not `active`. The test pins it positively AND negatively
   (`not.toMatch(...closeTab(pane.id, active.id))`).
5. **Watcher reordering** — `seenGraphReloadNonce = nonce` now advances
   right after the dedup check, for hidden AND visible tabs, so a hidden
   tab won't reprocess the same event on activation (the LOAD effect's
   graphDirty handles the reload instead). changeAffectsScope runs
   FIRST → out-of-scope returns before either branch (no dirty, no
   reload). Visible debounce-reload logic unchanged bar the
   lastLoadedKey/graphDirty sync. Even though the effect still reads
   `visible`, an activation re-run early-returns at the nonce dedup —
   no double-processing.
6. **graphInspectorActionsHotfix.test.ts (the unplanned diff)** —
   LEGITIMATE, not scope creep. It pinned the exact OLD load-effect
   source string (`const show = visible; void loadKey; if (show)
   untrack(...)`), which the restructure rewrote — leaving it unchanged
   would fail the gate. The update re-pins the SAME contract the test
   guards (visible + loadKey read up front, load() untracked), correctly
   loosened to allow the lazy/dirty gating to sit between the reads and
   the call. Design's test list simply didn't anticipate this pin needed
   the touch.
7. **New test quality** — paneGraphTabKeepAlive.test.ts is
   non-tautological: each-block positive + if-chain negative, the 4
   gates, onClose-captures-t (pos+neg), no-focused-prop, derived-visible
   (pos + old-constant neg), plain-latch condition, hidden→dirty,
   root contract, visibility-pair + flex:1-negative, and the latch
   (open={canvasEverShown}+paused pos, open={visible} neg) + loop guard
   + resume-resize shape. menuTrims flips Reload to positive and adds a
   real indexOf order pin Depth→Reload→Copy-link; revealBrowserActions
   adds a positive in-bubble Reload pin.

## Mutation bite-tests (my round standard; isolated worktree, reverted)

- Latch reversion `open={canvasEverShown}` → `open={active}`: the
  "open LATCHES" test FAILS. The exact runtime-untestable regression
  the design's risk note flagged is caught statically.
- Drop the loop() `if (paused){ rafId=null; return }` guard: the
  "loop() short-circuits" test FAILS. The 60fps-hidden-paint regression
  is caught.

## Not chased (per task)

Visible-watcher reload multiplicity (2-3 /api/graph per in-scope edit)
— confirmed pre-existing indexer event multiplicity, visible path
unchanged. On the round-close follow-ups list, not a regression.

## For the WKWebView walk (additive to @@Editor's items 1/6/7 + console)

- #5 out-of-scope hidden edit (a dir/tag-SCOPED graph, not workspace-
  scoped — @@Editor honestly couldn't exercise it in Chrome since the
  test graph was workspace-scoped so every edit was in-scope): edit an
  out-of-scope file while a dir/tag graph is hidden → zero reload on
  reactivation. The logic is sound by inspection (changeAffectsScope
  first), but it's the one branch with no empirical pass yet.
- Pane-resized-while-hidden → reactivate: confirm the resume resize()
  refits the backing store without jumping the transform.

## Status

Round-2 add-on review delivered. Holding; will fold @@Desktop's
WKWebView walk result if routed.
