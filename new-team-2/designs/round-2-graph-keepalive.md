# Round 2 (post-close add-on) — graph tab keep-alive + Reload menu item

Single item, requested by @@Alex before the release. Web-only, one
coherent feature across three coupled files; @@Editor owns it end to
end (this extends their own dadd5e64 keep-alive work to a third tab
kind). Baseline: main @ 00a585b3.

## Why

Clicking onto a graph tab redraws the whole graph. Cause: a REMOUNT —
Pane.svelte renders GraphPanel inside the active-tab if-chain
(`{:else if active?.kind === "graph"}`, ~line 1408), so every
activation destroys + recreates it. nodes/edges are component-local
$state and GraphCanvas.start() refetches + re-lays-out from scratch
(transform reset, sim rebuilt). No explicit focus handler — the
remount IS the reload @@Alex sees. Cheap on small workspaces, painful
on large ones (Linux source tree).

@@Alex wants: (1) graph stops auto-reloading on tab activation;
(2) the Reload menu item BACK, between Depth and "Copy link to graph"
(it existed, removed in ae22d5a1 at his request; now wanted for manual
control).

## Decisions (ratified by @@Alex)

- **Keep-alive**, mirroring dadd5e64 — graph tabs stay mounted, hidden
  via the visibility:hidden contract. No refetch/re-layout on switch;
  pan/zoom/selection survive; first load stays lazy.
- **KEEP the file-watcher auto-reload** — on-disk edits to in-scope
  files still refresh the VISIBLE graph (scope-filtered, 250ms
  debounce). Reload button is the manual "refetch now" on top, not a
  replacement.

Outcome: tab switch never reloads; on-disk in-scope edit still
refreshes the visible graph; a hidden graph that missed an in-scope
edit reloads ONCE on next activation; Reload forces a fresh fetch.

## Files & changes (anchors verified at 00a585b3)

### Pane.svelte — graph joins the keep-alive each-blocks
- REMOVE the front-face branch (lines 1408-1415, the
  `active?.kind === "graph"` → GraphPanel). Leave browser/dashboard/
  `!active` branches and the BACK-face HybridGraphConfig dispatch
  (~1507) untouched.
- ADD a third each-block after the file each-block (after 1491), inside
  `.face.front`, mirroring terminal (1464) + file (1485) exactly:
  ```svelte
  {#each pane.tabs.filter((t) => t.kind === "graph") as t (t.id)}
    <GraphPanel
      tab={t}
      active={!paneMode.active && !pane.showingBack && t.id === pane.activeTabId}
      onClose={() => { void closeTab(pane.id, t.id); }}
      onFlip={() => flipHybrid(pane.id)}
    />
  {/each}
  ```
  Rationale comment mirroring the file/terminal ones. BUG TO AVOID: the
  old branch closed `active.id`; the each-item onClose/onFlip must
  capture `t`. NO `focused` prop (graph owns no keyboard caret — canvas
  focuses on click, menu is portal-anchored).

### GraphPanel.svelte — thread `active`, restructure load gating
- Props (~84): add `active = false`.
- Line 100: replace `const visible: boolean = true;` with
  `const visible = $derived(active);`. Every existing `if (!visible)`
  reader keeps working; only the 3 effects below change. (Verified no
  site assigns visible.)
- New PLAIN locals (NOT $state — read/written only in effects, never
  rendered → no state_unsafe_mutation risk), ~line 592:
  ```ts
  let hasLoadedOnce = false;
  let graphDirty = false;
  let lastLoadedKey: string | null = null;
  ```
- Load effect (2192-2197): first load LAZY on first activation (not
  mount — the each-block mounts all graph tabs at session restore;
  mount-gating fires N loads at once). Hidden + loadKey change → mark
  graphDirty (don't load). Visible → load if
  `!hasLoadedOnce || keyChanged || graphDirty`, then set the latch +
  lastLoadedKey. load() stays inside untrack (same contract).
- Watcher effect (2274-2298): advance nonce + apply changeAffectsScope
  FIRST. Then hidden + in-scope → `graphDirty = true` (no background
  reload — the whole point). Visible + in-scope → debounce-reload as
  today, sync lastLoadedKey. Out-of-scope stays ignored.
- Depth-probe effects (2200-2224): unchanged logic (already gate on
  visible). Smoke note: clearing the workspace probe on hide re-probes
  /api/fs-graph on re-activation — cheap, distinct from a graph reload.
- Re-add `reloadGraph()` near copyGraphLink (~806): closeTabMenu();
  reset workspaceDepthProbe + loadWorkspaceDepthProbe() at workspace
  scope; await load(); sync lastLoadedKey. (Restores the ae22d5a1 body.)
- Menu item between the depth-row trailing msep (2493) and the
  Copy-link button (2494):
  ```svelte
  <div class="msep" role="separator"></div>
  <button class="mbtn" onclick={reloadGraph}>
    <span class="mbtn-icon" aria-hidden="true">
      <RotateCw size={16} strokeWidth={1.75} />
    </span>
    <span class="mbtn-label">Reload</span>
    <span class="mbtn-chord"></span>
  </button>
  ```
  Import RotateCw from lucide-svelte (already used in the repo, e.g.
  FileEditorTab). Order: Depth → Reload → Copy link to graph.
- Keep-alive CSS — root `<div class="graph-tab">` (2373): add
  `class:active`, `role="tabpanel"`, `aria-hidden={!active}`. Rewrite
  the `.graph-tab` style block (currently `display:flex; flex:1;
  flex-direction:column; min-height:0; min-width:0; background:var(--bg)`
  at line 2815) to the `.editor-tab` pattern:
  `position:absolute; inset:0; visibility:hidden; pointer-events:none`
  + `.graph-tab.active { visibility:visible; pointer-events:auto; }`,
  keep the inner flex-column for children, DROP `flex:1` (no longer a
  flex child). NEVER display:none (reports 0x0 → resize() refits →
  loses pan/zoom).

### GraphCanvas.svelte — latch `open`, add `paused`
start() resets transform (1323), stop() discards sim + node arrays
(1438); the open effect (1496) calls them on toggle. So `open` MUST
latch true once shown, or pan/zoom/selection die on every switch.
- In GraphPanel: `let canvasEverShown = $state(false);
  $effect(() => { if (active) canvasEverShown = true; });` and pass
  `<GraphCanvas open={canvasEverShown} paused={!active} … />`.
- Add `paused` prop (default false). While paused, suspend the rAF loop
  (loop() at 1147: `if (paused) { rafId = null; return; }`) → zero
  background paint/sim for hidden graphs (the huge-workspace win; a
  latched-but-unpaused canvas would paint every hidden graph at 60fps).
- Resume effect: when paused flips false AND sim exists → resize() (the
  pane may have resized while hidden) + re-arm the loop. NO start(), no
  transform reset → pan/zoom preserved.

### Server
None. /api/graph (stream) + /api/fs-graph are read-only, invoked only
from load() + depth probes. Confirmed.

## Tests
- Update menuTrims.test.ts (~112-121): flip the Reload pin to positive;
  add an order pin Depth → Reload → Copy-link; keep footer pins.
- Update revealBrowserActions.test.ts (~142): refresh the stale comment
  (chained assertions still pass with Reload inserted); optional
  positive Reload pin.
- NEW paneGraphTabKeepAlive.test.ts (model: paneFileTabKeepAlive.test.ts,
  ?raw source pins): each-block keyed by t.id, NOT under the if-chain;
  the 4 active gates; onClose captures t not active; GraphPanel takes
  active + `const visible = $derived(active)`; root class:active +
  role=tabpanel + aria-hidden; .graph-tab visibility-pair (not
  display:none); latch/dirty locals + the
  `!hasLoadedOnce || keyChanged || graphDirty` condition;
  open={canvasEverShown} + paused={!active}; the loop() pause
  short-circuit + resume-resize() in GraphCanvas.
- No change (verified): Pane.test.ts (pins only the back-face
  HybridGraphConfig), graphDepthFilter.test.ts, paneFocusFollowFlip.test.ts,
  tabMenuReloadInspector.test.ts.

## Verification
- Static: `make web-check` after the FINAL edit.
- Chrome smoke (vite dev + standalone server), Network filtered to
  graph/fs-graph: (1) tab switch fires NO graph request (a
  workspace-scoped graph may re-fire the cheap fs-graph depth probe on
  re-activation — not a reload); (2) Reload fires exactly one; (3)
  watcher works visible via an on-disk in-scope edit (shell echo, NOT
  the API — API writes dedupe) → one reload after 250ms; (4) hidden +
  external in-scope edit → zero while hidden, exactly one on
  re-activation, then zero on a further switch; (5) out-of-scope hidden
  edit → zero on re-activation; (6) pan/zoom/select a node → switch
  away/back → identical transform + selection + inspector, zero
  requests; (7) lazy restore: 3 graph tabs, reload window → only active
  graph(s) fetch; (8) console clean (no state_unsafe_mutation —
  browser-only check).
- WKWebView is the REAL gate (same surface as dadd5e64): @@Desktop
  build, walk items 1/6/7 + console.

## Risks
- Latch is load-bearing: reverting open={canvasEverShown} → open={active}
  kills pan/zoom on every switch with no runtime test catching it — the
  ?raw pin guards it.
- Skipping paused → every hidden graph paints at 60fps, the exact
  huge-workspace cost this targets. Required.
- .graph-tab style: drop flex:1, ensure position:absolute doesn't fight
  an existing rule (read the full block at 2815 first).
- Hidden graphs keep file-watch subscriptions (reconcile early-returns
  on !visible) — DESIRED (reload signals keep flowing for dirty-track)
  but holds watcher refs till close; matches FileBrowser. Glance at
  fbWatch refcount under many mounted graphs.
- Multiple mounted GraphPanels share paneWidths.graph (inspector
  width) — pre-existing for the active graph; only the visible one
  mutates on drag. Low risk; smoke note.

## Process
Lean dispatch (not a full multi-phase round): @@Editor implements +
own-gates + Chrome-smokes; @@TeamFlow cross-reviews (they reviewed
dadd5e64 — same surface, adversarial behaviour-preservation); @@Desktop
builds the WKWebView gate when the tree's ready; @@PromptQueue +
@@CtxPass parked. Round-1 disciplines carry: pathspec-atomic commits
(`git commit -F msg -- <paths>`, staged-stat before / show-stat after),
own-gate with real flags, lean 1-line pokes, verify-before-relay.
@@Conductor runs the integrated gate + WKWebView coordination + @@Alex
close. Local commits only; push on explicit ask. B7 (Xcode CI) still
the release-run watch item.
