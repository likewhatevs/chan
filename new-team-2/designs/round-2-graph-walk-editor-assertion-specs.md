# Round-2 WKWebView walk — @@Editor assertion specs (graph keep-alive, 3fdd4bfe)

Author: @@Editor (task-Conductor-Editor-39 walk; checklist in
task-Conductor-Desktop-43). Consumer: joint walk with @@Desktop.
Build: 3fdd4bfe — provenance-verify the served binary first
(@@Desktop owns). One driver at a time on the shared harness;
@@Desktop owns build/harness/fixture/teardown, I own these specs and
read-back predicates. Carries the round-1 walk's synthetic-event
contract — see designs/round-1-walk-editor-assertion-specs.md and the
[[chrome-focus-probes]] / WKWebView-suspension lessons. Honest-split:
anything not genuinely assertable → [hand-smoke] with a reason.

## The gold signal: instrument load(), not the fetch hook

From the Chrome smoke: counting `/api/graph` fetches via a fetch hook
is NOISY — the workspace-scoped graph re-fires the cheap `/api/fs-graph`
DEPTH PROBE on reactivation (NOT a reload), and the file watcher
re-emits the reload nonce 2-3x per single edit as indexing/embedding
progress (raw-modify + index + embedding events >250ms apart, so the
250ms debounce doesn't coalesce them — pre-existing, not a keep-alive
bug). Both contaminate raw fetch counts.

So the load-bearing instrumentation @@Desktop should inject into the
served GraphPanel (debug build, web/dist read from disk) is a counter
at the TOP of `load()`:

    // top of GraphPanel.svelte load()
    (window.__graphLoads ??= []).push({ active, visible, hasLoadedOnce, graphDirty, scope: currentScope?.kind, t: Math.round(performance.now()) });

`__graphLoads.length` is the true reload count; each entry's `active`/
`visible` tells which tab triggered it. The depth probe and watcher
nonce noise never call load() through this path, so this counter is
clean. (In Chrome I used console.warn forwarded through the vite log;
in the harness, a window array read back through the report channel is
simpler.) Distinguish the depth-probe separately if needed via a
second hook on `/api/fs-graph`, but it is NOT a reload and must not
fail item 1.

## Fixture (the #5 boundary is the new requirement)

@@Desktop builds, isolated $HOME. Need a DIR-scoped graph with a real
scope boundary:

    mkdir -p WS/inside WS/outside
    # inside/: the graph's scope
    for i in 1 2 3 4 5; do
      echo "# Inside $i\n\nlinks [[inside/in-$((i%5+1))]] #scoped" > WS/inside/in-$i.md
    done
    # outside/: NOT in the dir-scoped graph
    for i in 1 2 3; do echo "# Outside $i\n\n#elsewhere" > WS/outside/out-$i.md; done
    echo "# Root\n\n[[inside/in-1]] [[outside/out-1]]" > WS/index.md

Open a graph SCOPED TO `inside/` (dir scope) — via the file browser's
"Graph from here" on the inside/ dir, or a chan://graph dir-scope
link. The scope boundary: edits to `inside/*` are in-scope; edits to
`outside/*` are out-of-scope. (A workspace-scoped graph makes
everything in-scope — that's exactly why Chrome couldn't do #5.)

Also open a plain markdown FILE tab (e.g. index.md) as the
switch-away target, and for items 3/7 a SECOND graph tab.

## Item 1 — the @@Alex-visible symptom: switch = no redraw/reload

This is the headline. Two independent reads, both required:

- 1a NO RELOAD on switch (machine): with the graph loaded + active,
  record `n = window.__graphLoads.length`. Switch to the file tab,
  switch back to the graph. Assert `window.__graphLoads.length === n`
  (zero new loads). Do it across 2 full cycles. (The return MAY add
  ONE `/api/fs-graph` depth-probe entry to a separate fs-hook — that
  is allowed and is NOT a reload.)
- 1b NO VISUAL REDRAW / pan preserved (the actual symptom): before
  switching, pan the graph to a distinctive offset and read the
  GraphCanvas transform. The transform is component-local; expose it
  for the walk via a tiny debug hook (@@Desktop): e.g.
  `(window.__graphXform = () => transform)` at GraphCanvas top, or
  read it off a data-attr. Switch away/back → assert the transform
  matrix is IDENTICAL (x/y/k unchanged, not re-fit to center). If the
  transform can't be exposed cheaply → [hand-smoke: eyeball that the
  graph does NOT visibly redraw/re-layout on switch — the @@Alex
  repro]. Pair it with a screenshot before/after for the record.
- 1c selection survives: select a node (its id persists to the tab
  hash as `gn`); switch away/back → the hash `gn` is unchanged and the
  inspector still shows that node.

## Item 2 — Reload menu item

- 2a right-click the graph canvas → the tab-menu-bubble opens; assert
  the row order is Depth → Reload → Copy link to graph (read the
  `.mbtn-label` text sequence inside `.tab-menu-bubble`, or the
  handler order `reloadGraph` before `copyGraphLink`).
- 2b click Reload → `window.__graphLoads.length` increases by exactly
  1; the menu closes (`.tab-menu-bubble` gone); graph stays visible.

## Item 3 — lazy restore (the session-restore perf claim)

- Session with >=2 graph tabs (the boundary fixture's dir-graph + a
  second graph) + a file tab, ONE graph active. Record
  `window.__graphLoads = []` is fresh, then reload the window (the
  app's Cmd+R path; the injected driver rides web/dist so it survives).
- After restore settles (poll: every `.graph-tab` present, one
  `.active`, active host content non-empty): assert
  `window.__graphLoads.length === 1` AND that the single entry's
  `active === true` (only the active graph fetched; the hidden graphs
  stayed lazy — no load until activated). This is the mount-vs-
  activation gate; a regression to mount-gating would show N loads.

## Item 4 — hidden → dirty → exactly one on reactivation

On the IN-SCOPE side (edit a file under `inside/`):
- Graph loaded + active, then switch to the file tab (graph hidden;
  assert `getComputedStyle('.graph-tab').visibility === 'hidden'`).
- Record `n = window.__graphLoads.length`. Shell-edit an in-scope file
  (`echo >> WS/inside/in-2.md` — on-disk, NOT the API; API writes
  dedupe). Wait ~1.5s.
- Assert `window.__graphLoads.length === n` (ZERO loads while hidden —
  the keep-alive win; the edit set graphDirty instead).
- Switch back to the graph. Assert exactly ONE new load
  (`length === n+1`), entry `active===true` (the dirty one-shot).
- Switch away + back again with NO edit → ZERO new loads (dirty was
  cleared).

## Item 5 — out-of-scope hidden edit (the new empirical gap)

The reason this walk needs the boundary fixture. On the DIR-scoped
(`inside/`) graph:
- Graph loaded + active, switch away (hidden). Record `n`.
- Shell-edit an OUT-of-scope file (`echo >> WS/outside/out-1.md`).
  Wait ~1.5s.
- Switch back to the graph. Assert `window.__graphLoads.length === n`
  (ZERO loads — changeAffectsScope returned false, so no dirty was
  set, so no reload on reactivation). Contrast with item 4's in-scope
  +1: same hidden-then-reactivate motion, opposite outcome, proving
  the scope filter gates the dirty flag correctly.
- Control: then edit an IN-scope file while hidden, reactivate →
  exactly +1 (shows the fixture's boundary is real, not that the
  watcher is dead).

## Item 6 — resize-while-hidden → resume refits without transform jump

Exercises the resume effect's resize()-not-start():
- Graph loaded + active, pan to a distinctive transform, record it
  (via the 1b hook).
- Switch away (graph hidden). Resize the pane while hidden — split the
  pane (`cs pane split right` worked in round 1) or drag a pane
  divider, so the graph's host gets a different width.
- Switch back. Assert: the canvas backing store matches the NEW host
  size (no clipping/letterbox — `canvas.width/height` ≈ host rect ×
  DPR), AND the transform's pan/zoom (x/y/k) is preserved (resize()
  must not have reset it; only `pendingInitialFit` resets, which is
  false for an already-shown graph). If transform read-back isn't
  available → [hand-smoke: eyeball that the graph fills the resized
  pane and is NOT re-centered/re-fit].

## Item 7 — console sweep (whole walk)

Driver installs at t0 (survives the item-3 reload since web/dist is
disk-read): `window.onerror`, `unhandledrejection`, and console.warn +
console.error hooks pushing to `window.__errs`. At end: dump. FAIL the
sweep on any `state_unsafe_mutation` or uncaught error. The graph adds
ONE new `$state` write inside an `$effect` (canvasEverShown) — the
exact pattern that throws state_unsafe_mutation if it were in a
$derived; this sweep is its empirical proof on the real engine. The
plain latch locals (hasLoadedOnce/graphDirty/lastLoadedKey) are NOT
$state, so they can't trip it — but the sweep covers the whole
component regardless.

## Hand-smoke ledger (for the report table)

Likely [hand-smoke] depending on harness capability: 1b/6 transform
read-back if the debug hook isn't cheap (fall back to eyeball + the
machine-checkable canvas-size half of #6); the @@Alex visual no-redraw
is worth one human glance regardless. Everything else (load() counter,
visibility, hash `gn`, menu order, console sweep, canvas dimensions)
is machine-assertable per @@Desktop's round-1 driver capabilities.

## Session protocol

@@Desktop: provenance + fixture (with the boundary) + harness +
inject the load()/transform/console hooks. Then run items in order;
items 4/5 need shell-edit steps interleaved with the driver
read-backs (sync via the report channel). I'm on the bus for live spec
amendments; ambiguities resolve toward [hand-smoke]. @@Desktop writes
the completion table; I co-sign through @@Conductor.
