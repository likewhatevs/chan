# Channel: @@LaneA -> @@Architect

Append-only. @@LaneA writes progress reports here; @@Architect reads. Never
edit prior entries. Curated highlights/lowlights/contention; link your journal.

## 2026-05-27 @@LaneA -> @@Architect
Oriented + worktree up + W1-W7 sub-slice plan (gate before slice A1).

Worktree `../chan-lane-a` @ `phase-12-lane-a` from main fe6e126. Per @@Alex:
IN-SESSION sub-slices, no subagents (W1-W7 are coupled on the same store/
GraphPanel files; parallel subagents would collide). Journal:
`docs/journals/phase-12/lane-a/journal.md` (grounded against the worktree, with
real line numbers + verified symbol counts).

PLAN (W1-W7 grouped into merge-safe slices; each leaves main compiling + non-
regressed since you serialize):
- A1 (W1+W3): graph scope = the tab's own root. Kill the PANE-DERIVED
  enumeration inside `availableGraphScopes`; GraphPanel `graphState=tab`,
  `visible=true`, drop the `?? graphOverlay` fallback + dead `{#if !tab}` bar;
  `currentScope` from the tab's scopeId. Drop group/global/git_repo rootable.
  Overlay state stays ALIVE (deleted in A5).
- A2 (W2): LANGUAGE rooting - NET-NEW (no `language` scope kind today; needs a
  backend dirs-by-majority-language query + entry point). Additive, after A1.
- A3 (W5): retire legacy `?graph=`/`?files=` hash; degrade old bookmarks, never
  crash; stop writing HASH_GRAPH/HASH_BROWSER.
- A4 (W4): dock gets its own `$state` browserState (drop `tab ?? browserOverlay`).
- A5 (W6): DELETE graphOverlay/browserOverlay + mirror + HASH_GRAPH codec + App
  persistence effect + scope.svelte.ts/FileTree leftovers. Gated after A1+A3+A4.
- A6 (W7): re-gate + in-browser walk (FRESH binary + reindex).
Then GI-10, loading-state, optional GI-11 as A7+.

DECISIONS I need you to confirm before A1:
1. ENTANGLEMENT: `availableSearchScopes` also uses the shared
   `availableScopeOptions` pane enumeration. I plan to KEEP search's pane-
   derived client filter and kill ONLY the graph consumer. OK? (Spec names
   graph only; confirm the wipe isn't meant to strip search scopes too.)
2. LANGUAGE rooting is net-new work - bundle in the wipe (A2) or SPLIT to a
   post-wipe additive slice? I recommend SPLIT to de-risk the wipe.
3. Re-root UX: panes-form-scope gone -> the dropdown becomes display/dir-nav,
   re-root via "graph from here" / file browser. Confirm (matches W3 intent).

CONTENTION (declaring touches): store.svelte.ts, GraphPanel.svelte,
scope.svelte.ts, App.svelte, FileBrowserSurface.svelte, FileTree.svelte,
GraphCanvas.svelte + routes fs_graph.rs/graph.rs - overlaps @@LaneB codemod +
@@LaneC cosmetics. Will declare on cross-lane channels when those lanes engage.

Holding at A1 until you serialize + rule on the 3 decisions.

## 2026-05-27 12:20 @@LaneA -> @@Architect
READY TO MERGE: phase-12-lane-a@69a0aea (slice A1 = W1+W3)

Graph scope now resolves from the tab's own scopeId; panes-form-scope is dead.
Highlights:
- Deleted availableGraphScopes (pane-derived graph option list). currentScope =
  synthesizeScope(tab.scopeId); graphState=tab; visible=true. KEPT
  availableScopeOptions + availableSearchScopes (search), per your ruling 1.
- Removed the dead {#if !tab} overlay bar. This cascaded (dropping the
  graphOverlay import forces graphState=tab => tab required => bar dead-typed):
  also removed its overlay-only infra (HamburgerMenu, menu/menuOpen, POPOVER_*,
  the redundant filterChips/menuItems snippets, dead .bar/.chip CSS). The live
  tabMenu bubble renders the same rows inline and is UNCHANGED.
- Fixed synthesizeScope's tag label (#-strip; header renders #${label}).

Gate (worktree, node_modules symlinked from main @ fe6e126):
- web: npm run check 0 errors / 0 warnings; npm run build OK; vitest 1596 pass
  / 11 skip / 0 fail (4 behavior-lock tests updated for the removed bar + the
  deduped single chip-iteration site).
- rust: cargo fmt --check clean; clippy --all-targets -D warnings clean; cargo
  test pass; cargo build --no-default-features OK. (No .rs changed.)
- IN-BROWSER (fresh binary, scratch drive): graph renders at drive scope
  (10 nodes / 15 edges, drive hub + docs + tags + folders); tab-menu bubble
  (Depth/Reload/filters/footer) verified intact. Server + tab torn down.

LOWLIGHT/SCOPE NOTE: A1 grew past the spec's one-line "drop the bar" because the
overlay-menu infra was entangled. Still W1+W3. Net -267 lines.

DEFERRED (needs your call): the ~50 inert group/global/git_repo
currentScope.kind branches. Unreachable now (synthesizeScope no longer yields
those as rootable graph scopes; group was never graph-produced) but ripping them
out is W2 "decide kind fate", not W1+W3. Proposing a small dedicated slice
(A1b) or fold into A5. Left them inert for this merge.

CONTENTION: A1 touched store.svelte.ts + GraphPanel.svelte (your @@LaneB-held
hot files). Next I'll take A4 (W4 dock local browserState) - most self-contained
and independent of A1 - unless you want a different sequence. A3 (W5 hash
retire) after. Journal: docs/journals/phase-12/lane-a/journal.md.

## 2026-05-27 12:26 @@LaneA -> @@Architect
READY TO MERGE: phase-12-lane-a@a6cbacd (slice A4 = W4, STACKED on A1@69a0aea)

Proceeded to A4 while you were processing A1 (it's independent of A1 + the
deferred decision; @@Alex nudged for progress). Linear history A1->A4, so
serializing up to a6cbacd includes 69a0aea.

A4: dock variant of FileBrowserSurface gets its own minimal local browserState;
no longer reads the browserOverlay singleton.
- Scoping confirmed from source: FileBrowserSurface is mounted ONLY as
  variant="tab" (Pane) + variant="dock" (FileBrowserSidePane). NO overlay mount
  - the overlay variant is dead. browserState is only inspectorOpen/
  inspectorWidth, all gated behind isWideSurface (false for docks). Dock tree/
  selection state rides on per-side persistKey + browserSelection, not
  browserOverlay. So this is a no-behavior-change decouple.
- `browserState = tab ?? (isDock ? dockBrowserState : browserOverlay)`. Dead
  overlay branch keeps browserOverlay until A5/W6 deletes the state.

Gate: web check 0/0, build OK, 1596 vitest pass / 0 fail. No .rs changed (Rust
gate unchanged since A1). Per-slice browser smoke skipped (dock inspector
unrendered + tree state untouched); full walk is A6/W7.

STILL OPEN from my A1 report: (1) your A1 serialize/re-gate; (2) the deferred
group/global/git_repo kind-fate branches (A1b vs fold into A5). Neither blocks
me. Next I'll take A3 (W5 hash retire, store+App), then A5 (W6 delete the now-
dead overlay state) gated after A1+A3+A4. Will hold A5 for your A1 ack since it
is the destructive one. Journal updated.

## 2026-05-27 (round 2) @@LaneA -> @@Architect
READY TO MERGE: phase-12-lane-a@9bc0ddb (slice A3 = W5, STACKED on A4@a6cbacd)

Acked your round-2 plan + the FOLD-kinds-into-A5 ruling. A3 = retire the legacy
graph=/files= overlay hash. Kept it STORE-ONLY (out of App.svelte) to shrink the
@@LaneB contention surface - App's void-read persist effect is W6/A5.
- Dropped HASH_GRAPH/HASH_BROWSER restore + persist blocks; removed them from
  HASH_KEYS so dropUnknownHashKeys strips old bookmarks (ignored on restore,
  never crash; tabs persist via layout `s`). Search + settings keys kept.
- Removed the hash-only encodeGraphFilters/decodeGraphFilters codec (the live
  layout-`s` graph-tab codec in tabs.svelte.ts is separate + untouched).
- KEY: the removed HASH_GRAPH restore was the ONLY production setter of
  graphOverlay.open=true; browserOverlay.open is never set true in prod. So the
  "nothing sets .open=true" invariant W6 needs is now satisfied.

Tests: swapped the graph-hash persist/restore + URL-codec tests for a
retirement-lock. Gate: web check 0/0, build OK, 1591 vitest pass. No .rs changed.

NEXT: A5 (W6) is now unblocked (cleared by you + A3 just landed). It's the big
destructive slice across store/App/scope/FileTree/GraphPanel (incl. the folded
group/global/git_repo dead branches). I'll gate hard + do a fresh-binary browser
smoke before reporting it. A5 WILL touch App.svelte (your @@LaneB hot file) -
heads-up that the LaneB-quiescence point is after A5 + GI-10 + loading-state per
your plan. Journal updated.

## 2026-05-27 (round 2) @@LaneA -> @@Architect
A5 SCOPED - bigger than the W6 bullet list. One decision before I cut it.

Fully inventoried graphOverlay/browserOverlay. A5's real surface (beyond W6's
named list of $state defs + mirror + App effect + scope.svelte + FileTree):

IN W6 spec: graphOverlay/browserOverlay $state defs; mirrorGraphTabToOverlay +
11 call sites; App.svelte void-read persist $effect (~215-268) + imports;
scope.svelte.ts browserOverlay reads (162/298); FileTree if(browserOverlay.open)
(505/736). Plus the folded-in 28 dead group/global/git_repo kind-branches in
GraphPanel (+ synthesizeScope global/git_repo cases, scopeAncestors, header).

NOT in the W6 list (found while scoping - flagging):
1. SESSION PAYLOAD (store ~1264 serializeSession + ~1306 applySessionSidecars):
   `overlays.graph = {scopeId,depth,mode}` is saved to + restored from
   session.json from graphOverlay. This is the session-era analog of the hash I
   retired in A3 - redundant with the layout-persisted graph tab. Deleting
   graphOverlay forces dropping it. BACK-COMPAT: old session.json bodies carry
   `overlays.graph`; my plan is to STOP writing it + IGNORE it on restore (old
   sessions degrade gracefully, same as the hash). The SessionPayload.overlays
   field goes vestigial/removed.
2. syncOverlayStack (store ~2095): the ESC/z-order overlay stack reads
   browserOverlay.open + graphOverlay.open. Both are never true in prod after
   A3, so I drop the browser/graph entries (they're tabs, not overlays).
3. Presence-guards `browserOverlay.open || graphOverlay.open || hasBrowserTab()
   || hasGraphTab()` (x3, store 599/986/1025) collapse to the hasTab() halves.
4. FileBrowserSurface: the dead overlay-variant branches (264 isOverlay&&...,
   the A4 `: browserOverlay` fallback) go too.

DECISION I want your nod on before the destructive cut: handle the session
payload by DROPPING overlays.graph (stop write + ignore on restore, degrade old
sessions) - consistent with the hash retire. OK? And keep A5 as ONE slice
(store + App + scope + FileBrowserSurface + FileTree + GraphPanel + 3 test
files), or do you want the session-payload part split out?

This is the destructive centerpiece + touches App.svelte (your @@LaneB hot
file), so I'm holding for your nod rather than blind-deleting beyond the spec.
A1/A4/A3 were within-spec so I proceeded; this one has a spec gap + session.json
back-compat. Standing by. (A1+A4 merged; A3@9bc0ddb reported ready above.)

## 2026-05-27 (round 2) @@LaneA -> @@Architect
A5 back-compat RESOLVED by @@Alex directly - proceeding, no nod needed.

@@Alex reminded (in chat): chan is PRE-RELEASE, no back-compat, write fresh
code. So I'm DROPPING session `overlays.graph` outright (stop write + stop
read), no graceful-degrade. Same fresh-delete posture for any other legacy
field A5 touches. Keeping A5 as ONE slice (you already approved that + folded
the kinds in). Executing A5 now; will gate hard + fresh-binary browser-smoke
(graph + dock + reload) before reporting ready. Flag stands: A5 edits
App.svelte (your @@LaneB hot file) per the W6 surface.

## 2026-05-27 (round 2) @@LaneA -> @@Architect
READY TO MERGE: A5 complete (a4c139b + 760e242) + SMOKED. PLUS a main-graph
BLOCKER my smoke caught (chunk-1 landed without chunk-2).

REBASE: done - phase-12-lane-a onto current main 4cb5ca8 (you said 48b4951; main
moved). A1/A4/A3 already in main; rebase replayed only A5p1 (a4c139b), clean.
A5p2 = 760e242. Branch: 4cb5ca8 -> a4c139b (W6 overlay-state) -> 760e242 (W6
kind-branches).

A5 GATE: web check 0/0, build OK, 1595 vitest pass. No .rs changed.
A5 SMOKE (fresh binary, scratch drive):
- Reload: no crash, graph tab persists via layout `s`, hash restores -> A3+A5
  persistence intact.
- Directory-scoped graph renders 3 nodes / 2 edges + "drive/docs" breadcrumb +
  full inspector -> A5 render/scope/inspector paths SOUND, destructive deletion
  broke nothing.

*** BLOCKER (NOT A5 - main is broken; my smoke surfaced it) ***
DRIVE-scope semantic graph fails: "unknown variant `drive`, expected one of
`workspace`, `directory`, `file`". chunk-1 (@@LaneB) renamed the /api/graph
backend scope variant drive->workspace, but the frontend client.ts (lines
206/955/964 type + 1346 sends `scope: "drive"`) was NOT flipped - that's chunk-2
(frontend wire-flip), which is HELD on my quiescence. So chunk-1 shipped a
backend rename whose frontend half is gated behind me -> the whole-workspace
graph is broken on main RIGHT NOW (drive button -> error; dir/file scopes still
work since scope=directory/file are valid).

NOT mine to fix unilaterally: client.ts scope "drive"->"workspace" IS the
drive->workspace codemod = @@LaneB chunk-2 territory; me editing it would collide
with their held wire-flip. YOUR CALL on sequencing: (a) @@LaneB hotfixes just the
/api/graph scope param now (tiny, unblocks the graph) without the full chunk-2;
(b) expedite chunk-2; or (c) explicitly hand me the scope-param hotfix. Flagging
loud because it's a user-visible release break on main.

NEXT (round 2, mine): GI-10 (drive node bottom) + graph loading-state, A6 walk,
addendum-2 FB tab/dock expansion-independence verify (fresh binary; may be
A4-fixed). Journal has full detail.

## 2026-05-27 (round 2) @@LaneA -> @@Architect
READY TO MERGE: phase-12-lane-a@3d3254b (GI-10, on rebased main abac76c).

Acked: A5 merged (abac76c), drive-scope break confirmed yours-not-mine (@@LaneB
hotfixing, I'm not touching client.ts). Rebased onto abac76c (A5 commits now in
main; branch was 0-ahead, clean).

GI-10: drive root pinned to the BOTTOM, containment spine grows UPWARD.
GraphCanvas-only: the filesystem-spine forceY flips from `depth *
hierarchyYSpacing` to `-depth * hierarchyYSpacing` (deeper nodes rise above their
ancestors; drive root depth 0 anchors the bottom). Updated the spine test +
comments.

Gate: web check 0/0, build OK, 1596 vitest pass (clean exit now - @@LaneD flake
fix is in main). No .rs changed. VISUAL CHECK DEFERRED to the A6 walk per your
hold (drive-scope graph is broken pending @@LaneB's hotfix; GI-10's "drive at
bottom" is most visible at workspace scope, so it pairs with the post-hotfix
walk).

NEXT (mine): graph loading-state UX (graph-loading-state-spec.md) - reading the
spec; likely a per-scope index-completeness signal (may need a small backend
add). Then A6 walk + addendum-2 FB-independence verify once the graph hotfix
lands. Still not quiescent (chunk 2 waits on me). Journal updated.

## 2026-05-27 (round 2) @@LaneA -> @@Architect
GI-10 visual ✓ + wipe verified on fresh binary; loading-state investigated +
de-risked (frontend-only). One UX confirm before I build.

Acked GI-10 merged (a477e62) + @@LaneB graph hotfix in. Rebased onto a477e62
(0-ahead). Ran a fresh-binary empirical session (broken-link + 3-deep drive):
- GI-10 VISUAL: drive root sits at the bottom, spine grows upward. Confirmed.
- WIPE: drive-scope graph renders 13 nodes/19 edges; broken-link ghost renders
  dashed + muted, distinct from real files. A1-A5 verified working. (This covers
  the A6 essentials; a dedicated A6 sweep can fold in after loading-state.)

LOADING-STATE - investigation done, grounded plan (de-risked):
- Root cause: dead-ends are EITHER genuine broken links (already shown dashed -
  spec step 3 done) OR index-not-yet-complete artifacts (spec steps 1-2).
- The per-SCOPE completeness signal ALREADY EXISTS: GET /api/indexing/state ->
  per-directory {state: indexed|indexing|pending} (api.indexingState();
  IndexingStateResponse). Today only EmptyPaneCarousel consumes it. So this is
  FRONTEND-ONLY - NO backend add.
- Plan: GraphPanel/graphData subscribes to indexing-state; while the active
  scope's dir != "indexed", show a loading state (pulse the parent dir, mirror
  the FB spinner) instead of rendering dead-end ghosts as fact; once "indexed",
  render fully (remaining ghosts = real broken links, already dashed).

CONFIRM before I build (small UX call): the "pulse the parent dir while the
scope is indexing" UX per the spec + the frontend-only reuse of
/api/indexing/state. If you (or @@Alex) want a different loading affordance, say
so; else I implement as above + gate + smoke. Then addendum-2 FB-independence
verify -> quiescent -> your chunk-2 window. Journal has detail.

## 2026-05-27 (round 2) @@LaneA -> @@Architect
addendum-2 VERIFIED: it's a REAL bug (NOT A4-fixed). Both items scoped below.

addendum-2 (FB tab/dock expansion independence) - fresh-binary repro CONFIRMS the
bug: expanding docs/ in a File Browser TAB also expands it in the dock (left
sidebar). Root cause: FileTree renders expansion from the GLOBAL `treeExpanded.map`
singleton (FileTree.svelte:332), shared by every FB surface. A4 only decoupled
`browserState` (inspector/selection), not expansion. Subtlety: the tab DOES
persist its own expansion (`be:[...]` in the layout `t`), but the live render
reads the global map -> instances mirror. FIX scope: FileTree must render from a
PER-INSTANCE expansion source (tab -> its `be`; dock -> its own $state map),
instead of the global singleton. Moderate refactor (FileTree expansion source +
per-instance maps + how the tab `be` round-trips); NOT a one-liner.

loading-state (graph) - investigated + de-risked, FRONTEND-ONLY plan ready (reuse
GET /api/indexing/state per-dir indexed/indexing/pending; pulse the parent while
a scope's dir != indexed; real broken links already dashed). Caveat: it's a
genuine UX feature, and EMPIRICAL verification needs a drive large enough for
indexing to visibly LAG (a tiny test drive indexes instantly), so the smoke is
harder to stage than the others.

STATUS: both remaining items are real BUILDS (not quick closes) - flagging so your
freeze/chunk-2 timing reflects that. I'm proceeding to build them as focused gated
slices, loading-state first (your order), then the addendum-2 expansion fix. Will
report each ready + smoke as I go. Still not quiescent. If you'd rather descope/
reorder either given the round's tail, say so.

## 2026-05-27 (round 2) @@LaneA -> @@Architect
READY TO MERGE: phase-12-lane-a@19d5456 (graph loading-state SLICE 1).

loading-state slice 1 (foundation + core trust signal): wired the drive-global
`indexStatus` into GraphPanel; the status bar shows a pulsing "indexing…" cue
while the index is building/reindexing, so an incomplete semantic graph (dead-end
"missing" nodes that may just be unindexed) isn't read as final. Once idle,
remaining dead-ends are real broken links (already dashed). Reduced-motion
respected; source-pattern test locks the wiring.

Gate: web check 0/0, build OK, 1596+4 vitest pass. No .rs changed. (Empirical
cue-visibility has the same index-lag caveat - a tiny test drive indexes
instantly; the source test locks the wiring.)

SLICE 2 (the richer spec behavior) - separate gated slice: while a scope's
directory is still indexing (per-dir GET /api/indexing/state), pull back the
ghost dead-end nodes + pulse the parent dir (mirror FB spinner); resolve to real
nodes once that scope completes. This touches the canvas paint() loop + the
node/edge filter pipeline (visibleNodeIds re-adds nodes via edge endpoints, so
hiding ghosts means filtering edges too) - higher-risk, so I'm landing it as its
own change with index-lag verification rather than bundling into slice 1.

Then addendum-2 (per-instance FB tree expansion). Both remain before quiescence.
Proceeding. Journal updated.

## 2026-05-27 (round 2) @@LaneA -> @@Architect
READY TO MERGE: phase-12-lane-a@915ea29 (addendum-2: per-instance FB tree
expansion). Stacked on loading-state slice 1 (19d5456, also ready).

addendum-2 FIXED via the deferred Slice-E migration. Used a sub-agent (per
@@Alex) for the focused implementation; I reviewed + verified + committed.
- FileTree reads/writes expansion from the per-instance fbTreeInstances registry
  (keyed by instanceId) instead of the shared global treeExpanded.map singleton.
- FileBrowserSurface threads instanceId; reconcile/snapshot/restore/toggle-all/
  fullyExpanded all per-instance. revealAndSelect fans across instances (reveal
  should surface wherever the user looks; user toggles stay independent).

KEY LESSON (validates the empirical-smoke requirement): the sub-agent's static
gate PASSED (svelte-check 0/0 + 1603 vitest, all ?raw source-pattern), but the
live app CRASHED with `state_unsafe_mutation` - it called ensureFbTreeInstance
(which mutates $state) inside a $derived. svelte-check + source-pattern vitest
can't catch a Svelte-5 runtime reactivity violation; only running the app does.
I fixed it (ensure in $effect, READ in $derived) + re-verified.

EMPIRICAL VERIFY (fresh binary, after the fix): no crash; expanding docs/ in a
TAB does NOT expand it in the DOCK, and expanding other/ in the DOCK does NOT
expand it in the tab (independent BOTH directions); tab expansion restores on
reload (via tab.expanded).

CAVEAT (minor, spec-accepted): dock/overlay expansion persistence across reload
is best-effort (sessionStorage snapshot); it didn't restore in my Chrome reload
(snapshot-key timing: write uses drive.info.root, the on-mount seed falls back
to pathname before drive.info loads). The spec says dock-reset-on-reload is
acceptable, so I shipped it + flagged the snapshot-key timing as a follow-up
(fix the key or drop the snapshot).

Gate: web check 0/0, build OK, 1603 vitest. No .rs changed.

REMAINING before quiescence: loading-state SLICE 2 (paint/pipeline ghost
pull-back + parent-dir pulse). That's the last item. Journal updated.

## 2026-05-27 (round 2) @@LaneA -> @@Architect
READY TO MERGE: phase-12-lane-a@73bc625 (loading-state SLICE 2). *** QUIESCENT ***

loading-state slice 2: while the index is building, dead-end ("missing") nodes
are pulled back (with their edges) so a not-yet-indexed link target isn't shown
as a broken link; once idle they reappear as real broken links (dashed). A
`hiddenMissingIds` $derived gated on slice-1's indexBuilding, following the
existing hidden*Ids node-filter convention (pure derived, no $state mutation).

VERIFIED on a fresh binary (300-file drive + a broken link): WHILE INDEXING ->
306/307 nodes, dead-end + edge pulled back, "indexing" cue; ONCE IDLE -> 307/307
nodes, dead-end reappears as a real broken link, cue gone. Clean transition, no
crash, no dangling edges. Gate: check 0/0, build OK, 1606 vitest.

QUIESCENCE: this was my last round-2 item. My full round-2 stack is reported
ready (all on phase-12-lane-a, in order):
- 19d5456 loading-state slice 1 (indexing cue)
- 915ea29 addendum-2 (per-instance FB tree expansion)
- 73bc625 loading-state slice 2 (pull back dead-ends while indexing)
(A1-A5 + GI-10 already merged.)

I am PAUSING all web/src work. Once you merge this stack + re-gate, the graph/FB
frontend is quiescent and you can OPEN THE CHUNK-2 FREEZE for @@LaneB's
drive->workspace codemod (the round's last big piece). I will NOT touch web/src
until the codemod lands, so it rebases onto a settled tree.

DEFERRED carryover (refinements, NOT blocking - for after the codemod / a future
round): loading-state per-parent-dir pulse; the dock reload-snapshot key timing;
the now-dead group-edge/SCOPE_HUB machinery in GraphCanvas (A5 follow-up); the
optional GI-11 regression-lock tests. Journal has the full record.

## 2026-05-27 (round 2) @@LaneA -> @@Architect
addendum-3 DONE (all 3). Rebased onto chunk-2 main (bce6bd3); 2 slices ready +
1 closed-by-GI-10. All gated + browser-smoked on a fresh post-codemod binary.

READY TO MERGE (stacked on bce6bd3):
- 3de22a76 A3-i: graph right-click opens the tab menu ANYWHERE on the canvas bg
  (mirrors the editor; anchors openTabMenu at the cursor). Smoke: right-click on
  empty canvas bg opens the full menu at the cursor. ✓
- 5003a6a2 A3-iii: "Export to PDF" moved from the editor right-click menu to the
  file Inspector (FileInfoBody), shown for MARKDOWN files (@@Alex confirmed
  markdown-only over all-editable), placed [Upload][Download] / Export to PDF /
  Graph from here. The inspector fetches file content (selection isn't always
  open in an editor; editor autosaves so disk == live). Removed it + the dead
  Printer/print imports from the editor. Smoke: alpha.md inspector shows it in
  the right slot. ✓

CLOSED (no commit):
- A3-ii (drive + near dirs at bottom, plot upward): VERIFIED on the fresh
  post-codemod build - GI-10 (merged earlier) already does exactly this; the
  workspace node sits at the bottom with the spine growing up. Done-by-GI-10,
  as you predicted (@@Alex was on a pre-GI-10 build).

(Item 1 of addendum-3 - terminal orange-dot pulsating - was NOT routed to me.)

Gate (both slices): web check 0/0, build OK, 1611 vitest pass. No .rs changed.
Post-merge I'm quiescent again. Journal updated.
