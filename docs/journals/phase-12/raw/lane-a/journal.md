# @@LaneA journal (phase 12) - graph + File Browser carryover

Append-only, self-documenting. Branch `phase-12-lane-a`, worktree
`/Users/fiorix/dev/github.com/fiorix/chan-lane-a`. Channels + this journal
edited in the MAIN checkout by absolute path.

## 2026-05-27 - Kickoff + orientation

Identified as @@LaneA. Read bootstrap, lane-a-plan, phase-12-backlog,
the W1-W7 overlay/scope-wipe spec (`../phase-11/overlay-scope-wipe-spec.md`),
coordination README + my channels.

@@Alex decisions at kickoff (via AskUserQuestion):
- Set up worktree + post plan, start slice 1 once @@Architect serializes.
- Drive W1-W7 as IN-SESSION sub-slices (no subagents): W1-W7 are tightly
  coupled on the same store.svelte.ts / GraphPanel files; parallel subagent
  edits would collide in the shared worktree.

Created worktree `../chan-lane-a` on branch `phase-12-lane-a` from main fe6e126.

### Source grounding (verified against worktree, not the spec's estimates)

- `graphOverlay` = 114 refs across web/src; `browserOverlay` = 40 (spec's
  "61 / 14" counted store.svelte.ts only). Symbols all present:
  `availableGraphScopes` (store 1613), `graphOverlay` $state (store 1785),
  `browserOverlay` $state (store 2065), `mirrorGraphTabToOverlay` (store 2003,
  11 call sites 1832-1999), `ScopeOption` (scope.svelte.ts 41).
- Scope kinds today (scope.svelte.ts 41-90): file, dir, git_repo, group, tag,
  drive, global. Spec keeps: dir (base), tag (depth-1), file (->parent dir),
  drive (->root dir). Drops: group, global, git_repo. ADDS: language (NO
  language kind exists today - this is net-new work).
- `availableGraphScopes()` (store 1613-1690) does TWO things:
  1. enumerates PANE-DERIVED options via shared `availableScopeOptions(...)`
     -> this IS "panes form scope", the wipe target.
  2. injects the active tab's own scope (tag:/file:/dir:) so the dropdown
     displays + restores it (1644-1688). Entry points openGraphForTag /
     openGraphForFile / file-browser dir menu already exist INDEPENDENT of the
     pane list.
- ENTANGLEMENT: `availableSearchScopes()` (store 1697) ALSO calls the shared
  `availableScopeOptions(...)`. Search scope is a separate feature (client-side
  result filter over visible files). So I plan to keep `availableScopeOptions`
  + `availableSearchScopes` intact and only kill the GRAPH consumer's pane
  enumeration. Flagged to @@Architect as a decision.

### Proposed W1-W7 -> merge-slice decomposition

Constraint: @@Architect serializes merges to main; each merged slice must leave
main compiling + non-regressed. Grouping reflects that, not the raw W-step
numbering.

- A1 (W1+W3): graph scope = the tab's own root. Kill the pane-derived
  enumeration in `availableGraphScopes` (keep only tab-scope resolution); drop
  group/global/git_repo as graph-rootable. GraphPanel: `graphState = tab`,
  `visible = true`, drop `?? graphOverlay` + dead `{#if !tab}` bar +
  close()'s graphOverlay branch; `currentScope` derives from the tab's scopeId
  (dir rooted at + drive-root special case + file->parent + tag). graphOverlay/
  browserOverlay STILL EXIST (deleted A5); mirror still feeds HASH_GRAPH until
  A3. Search untouched. Re-root becomes context-menu "graph from here" /
  directory nav, not a pane-option dropdown (W3-endorsed UX shift).
- A2 (W2): add LANGUAGE rooting (net-new: scope `language` kind + backend
  dirs-by-majority-language first-degree query + openGraphForLanguage entry).
  Additive; follows A1. Candidate to SPLIT to a post-wipe follow-up to de-risk
  the wipe - flagged.
- A3 (W5): retire legacy `?graph=`/`?files=` hash. Stop writing HASH_GRAPH/
  HASH_BROWSER; degrade old bookmarks gracefully on restore (never crash).
- A4 (W4): dock gets its own local `$state` browserState instead of
  `tab ?? browserOverlay`. Independent of A3.
- A5 (W6): DELETE dead state - graphOverlay/browserOverlay $state,
  mirrorGraphTabToOverlay + call sites, HASH_GRAPH codec, App overlay-
  persistence $effect, scope.svelte.ts browserOverlay reads, FileTree
  `if(browserOverlay.open)` leftovers. Gated only after A1+A3+A4.
- A6 (W7): re-gate + in-browser walk (fresh binary + reindex): graph renders +
  navigates dirs, reveal opens FB tab, old `?graph=`/`?files=` bookmark loads
  without crashing.

Then GI-10 (drive node bottom, spine up), graph loading-state, optional GI-11
locks as A7+.

### Decisions surfaced to @@Architect (event-lane-a-architect.md)

1. Keep `availableSearchScopes`/`availableScopeOptions` (search's pane-derived
   client filter is separate); only kill the GRAPH consumer. Confirm the wipe
   doesn't intend to also strip search scopes.
2. LANGUAGE rooting is net-new (backend + frontend). Bundle in the wipe (A2) or
   split to a post-wipe additive slice? Recommend split to de-risk.
3. Re-root UX: with panes-form-scope gone, the dropdown becomes display/dir-nav,
   not an option list; re-root via "graph from here" / file browser. Confirm
   (matches W3).

Cross-lane touch declaration to follow: store, GraphPanel, scope, App,
FileBrowserSurface, FileTree, GraphCanvas + routes fs_graph.rs/graph.rs overlap
@@LaneB codemod + @@LaneC cosmetics.

Status: awaiting @@Architect serialization of the slice plan before slice A1.

## 2026-05-27 - @@Architect (@@Lead) green-light + slice A1 implemented

@@Lead serialized the A1-A6 spine and ruled all 3 decisions:
1. KEEP `availableSearchScopes`/`availableScopeOptions` (search's pane filter is
   separate); kill only the graph consumer. 2. LANGUAGE rooting SPLIT to a post-
   wipe additive slice (after A6). 3. Re-root UX confirmed per W3.
Cross-lane: @@LaneB's frontend wire-flip is HELD until I report graph/FB merged
+ quiescent; I'm not blocked by anyone. The codemod rebases onto my tree.

### Slice A1 (W1+W3): graph scope = the tab's own root

Turned out BIGGER than the spec's one-line "drop the dead {#if !tab} bar"
implied, because dropping the `graphOverlay` import (W3) forces
`graphState = tab` => `tab` becomes a required prop => the `{#if !tab}` overlay
bar is dead-typed => removing it cascades into its overlay-only menu
infrastructure. All still W1+W3 (the spec lists the bar removal under W3), just
larger. What changed:

store.svelte.ts:
- DELETED `availableGraphScopes()` (W1: the pane-derived graph option list).
  Only callers were GraphPanel + a test comment. `availableScopeOptions` +
  `availableSearchScopes` KEPT (search, per @@Lead ruling 1).
- Refreshed the `openGraphForDirectory` doc comment (no longer references the
  removed function).

GraphPanel.svelte:
- `graphState = $derived(tab)` (was `tab ?? graphOverlay`); `visible: boolean =
  true` (was `tab ? true : graphOverlay.open`); `tab` prop now REQUIRED.
  Dropped the `graphOverlay` import; `close()` = `onClose?.()` (dropped the
  graphOverlay.open branch).
- `currentScope = $derived(synthesizeScope(graphState.scopeId))` - resolved
  straight from the tab's scopeId; removed `scopeOptions`/`availableGraphScopes`.
  Fixed `synthesizeScope`'s tag label to strip the leading `#` (the header
  renders `#${label}`, raw `#search` nodeId would double-hash). synthesizeScope
  KEPT as the resolver (spec W3 floated dropping it, but tag + file are RESOLVED-
  kept kinds, so it stays). group/global/git_repo kind-BRANCHES left inert for
  now; their removal is W2-character "decide kind fate" - deferred (see below).
- Removed the dead `{#if !tab}` overlay bar + its overlay-only infra: the
  `HamburgerMenu` import, `menu`/`menuOpen` state, `POPOVER_WIDTH/HEIGHT`, and
  the `filterChips` + `menuItems` snippets (the bubble renders its own inline
  rows; `reloadGraph`/`flipToSettings`/`doReopenClosedTab`/`closeFromMenu` +
  the FILTER_COLORS/counts/show deps + Settings2/History/X icons all stay used
  by the bubble). The `menu?.close()`/`menu?.openAtCursor` calls were no-ops in
  the tab variant (menu was only bound in the deleted overlay bar; the live menu
  is the `tabMenu` bubble, closed via `closeTabMenu()`), so removing them
  preserves behavior. `onGraphContextMenu` keeps preventDefault (swallow canvas
  right-click) minus the dead `menu` call.
- Removed the now-unused CSS: `.bar`, `.bar-menu`, `.filters`, `.chip*`.
- REVERTED two gratuitous "tab is always set" micro-simplifications that broke
  behavior-lock tests for no benefit: kept `data-theme={tab ? ... : undefined}`
  and `{#if tab && tabMenuOpen}` as-is.

Test updates (locked OLD structure that intentionally changed):
- revealBrowserActions: "hides the chrome bar when rendered as a tab" ->
  "has no chrome bar (overlay variant removed)" (asserts no `<div class="bar">`
  / no `{#if !tab}`).
- graphDepthFilter + graphFileBucketChips: the two chip-iteration sites
  (overlay filterChips + bubble) deduped to ONE; `.toBe(2)` -> `.toBe(1)`.

DEFERRED out of A1 (flagging to @@Lead): the inert group/global/git_repo
`currentScope.kind` branches (~50 sites). They're unreachable now (synthesizeScope
no longer produces global/git_repo as rootable graph scopes in practice; group
was never produced for graph), but ripping them out is W2 "decide kind fate"
cleanup, not W1+W3. Proposing a dedicated small slice (A1b or fold into A5).

Gate (worktree, node_modules symlinked from main - same fe6e126 baseline):
- web: `npm run check` 0 errors/0 warnings; `npm run build` OK; vitest 1596
  passed / 11 skipped / 0 failed.
- rust: `cargo fmt --check` clean; clippy + test + build --no-default-features
  running (no .rs files changed, so baseline-determined).
- In-browser graph-render verification: deferred to A6/W7 walk per the approved
  spine; may smoke-test sooner since A1 reworked scope-resolution.

### A1 in-browser smoke (fresh binary) + commit

Built a fresh `chan` (cargo build -p chan, embeds the new bundle), served a
scratch drive (/tmp/chan-lane-a-smoke: index/alpha/beta linked + docs/note,
tags #project/#idea) standalone on :7799 --no-token, browsed it:
- Graph at drive scope renders: 10/10 nodes, 15/15 edges - drive hub + 4 doc
  nodes + 2 tag nodes + folder nodes all draw. Confirms synthesizeScope("drive")
  -> currentScope.kind="drive" render path + graphState=tab + visible=true +
  the loadKey reload effect.
- Tab right-click bubble renders correctly: "Drive" header, Depth slider
  (disabled on drive, correct), Reload, vertical filter rows with live counts
  (tag 2 / folder 3 / markdown 4, matching the seed), Settings / Reopen / Close.
  Confirms the menu-infra teardown did NOT break the live bubble - all handlers
  wired via its inline rows.
Torn down: killed my server (pid 55544), rm'd the scratch drive, closed my tab.
A stray :8477 "Chan" tab belongs to another lane - left untouched.

Also refreshed a stale present-tense `availableGraphScopes` reference in
graphInspectorActionsHotfix.test.ts (historical-bug comment).

COMMITTED: phase-12-lane-a@69a0aea (6 files, +60/-327). Reported merge-ready to
@@Architect. A1 DONE.

Next: A3 (W5, retire legacy ?graph=/?files= hash) or A4 (W4, dock local
browserState) - both independent of A1; A2 (language rooting) split post-wipe
per @@Lead. Awaiting any @@Architect sequencing note; will proceed on A4 (dock)
as it is the most self-contained.

### Slice A4 (W4): dock gets its own browserState

@@Alex poked twice with @@Architect still silent on A1 serialization; A4 is
independent of A1 + the deferred decision, so I proceeded (stacked as its own
commit so A1->A4 still serializes cleanly).

Scoping found from source (answered my own A4 unknowns, didn't ask):
- FileBrowserSurface is mounted in exactly TWO places: Pane (variant="tab") and
  FileBrowserSidePane (variant="dock" {side}). NO variant="overlay" mount exists
  - the overlay variant is DEAD (like the graph bar). So `browserOverlay` backs
  only the live dock + dead overlay-variant branches.
- browserState is `{ open?, inspectorOpen, inspectorWidth? }`; the only field
  accesses are inspectorOpen/inspectorWidth, ALL gated behind `isWideSurface`
  (false for docks) except one write at L376 (harmless: sets a never-rendered
  field). Dock tree/selection/expansion state rides on the per-side `persistKey`
  (`fb-dock-{side}`) + module-level `browserSelection`, NOT browserOverlay.

Change: added a minimal local `dockBrowserState = $state({inspectorOpen:false})`;
`browserState = tab ?? (isDock ? dockBrowserState : browserOverlay)`. The dock no
longer reads browserOverlay; the tab variant uses its tab; the dead overlay
branch keeps the browserOverlay fallback until A5/W6 deletes the state. No
behavior change (decoupled fields are unrendered for docks).

Gate: web check 0/0, build OK, 1596 vitest pass / 0 fail. No .rs changed -> Rust
gate unchanged from A1. (Per-slice browser smoke skipped: dock inspector fields
are unrendered + tree state untouched; the full in-browser walk is A6/W7.)

COMMITTED: phase-12-lane-a@a6cbacd (stacked on 69a0aea). Reported ready-to-merge.

Next: A3 (W5, retire legacy ?graph=/?files= hash) - store + App. Then A5 (W6,
delete browserOverlay/graphOverlay + mirror + HASH codec + the now-dead overlay
branches), gated after A1+A3+A4. A6 = full in-browser walk. Still awaiting
@@Architect on A1 serialize + the deferred kind-fate decision (A1b vs A5).

## 2026-05-27 (round 2) - @@Architect acks + A3 done

@@Architect: A1 (69a0aea) + A4 (a6cbacd) MERGED to main (merge cf756ca),
combined re-gate green. Rulings: deferred group/global/git_repo kind-branches
FOLD INTO A5 (no separate A1b); A5 cleared (gates on A3 landing first). Round-2
order: A3 -> A5(+kinds) -> GI-10 + loading-state -> A6. @@LaneB wire-flip stays
HELD on my graph/FB quiescence; I signal when A5 + GI-10/loading-state land and
I'm paused, then @@Architect opens the codemod freeze window. New @@LaneD lane
appeared in the roster.

### Slice A3 (W5): retire the legacy graph=/files= overlay hash (STORE-ONLY)

Kept A3 out of App.svelte deliberately (App's void-read persist effect is W6/A5;
staying out shrinks @@LaneB's contention surface). Scoping from source:
- applyOverlaysFromHash had HASH_BROWSER + HASH_GRAPH restore blocks; the
  HASH_GRAPH block was the ONLY production setter of graphOverlay.open=true.
  browserOverlay.open is never set true in prod (HASH_BROWSER restore called
  openBrowser() which sets it false). So removing these clears the
  "nothing sets .open=true" invariant W6 needs.
- encodeGraphFilters/decodeGraphFilters were hash-only (the LIVE layout-`s`
  graph-tab filter codec is encodeGraphTabFilters in tabs.svelte.ts, separate +
  untouched). Removed them. splitInspectorBit stays (search uses it).
- Removed HASH_GRAPH/HASH_BROWSER from HASH_KEYS -> dropUnknownHashKeys strips
  old graph=/files= bookmarks on next write. Retire = IGNORE on restore (spec
  permits "ignore - never crash"); current tabs persist via layout `s`.

Tests: removed the 3 graph-hash persist/restore tests + the 3 URL-hash-codec
tests (store.test.ts + graphFileBucketChips.test.ts); added a retirement-lock
(legacy graph=/files= ignored on restore; retired keys stripped on persist).

Gate: web check 0/0, build OK, 1591 vitest pass / 0 fail. No .rs changed -> Rust
gate unchanged. Per-slice browser smoke skipped (unit tests exercise the exact
restore/persist paths via real URLs); old-bookmark reload walk is in A6/W7.

COMMITTED: phase-12-lane-a@9bc0ddb (A1->A4->A3; net -136). Reporting ready.

Next: A5 (W6) - the big destructive slice (now CLEARED + gated on A3 which just
landed): delete graphOverlay/browserOverlay state + mirrorGraphTabToOverlay +
the App void-read effect + scope.svelte.ts/FileTree browserOverlay reads + the
folded-in dead group/global/git_repo kind-branches in GraphPanel. Touches the
most files (incl. App.svelte). Will gate hard + browser-smoke before reporting.

## 2026-05-27 (round 2) - A5 part 1 committed (overlay-state deletion)

@@Alex unblocked the back-compat question in chat: chan is PRE-RELEASE, no
back-compat, write fresh code (saved as [[feedback_pre_release_no_backcompat]]).
So I dropped session `overlays.graph` outright.

A5 scoping found the surface bigger than W6's bullet list (session payload +
syncOverlayStack + presence-guards), so I split A5 into TWO commits on the
branch (still ONE merge slice reported together):
- A5 part 1 = overlay-STATE deletion (DONE, committed ca86e34). graphOverlay +
  browserOverlay $state + mirror + 11 calls + session overlays.graph +
  syncOverlayStack/closeOverlay entries + OverlayId narrowed + presence-guards
  collapsed + App void-reads + scope.svelte/FileBrowserSurface/FileTree reads.
  Production compiled CLEAN first try; only tests needed fixups (watcher reload
  + activeFbScopes rewired to tabs/dock; overlay-restore lock retired). Gate:
  check 0/0, build OK, 1590 vitest. Net -172.
- A5 part 2 = dead graph scope-KIND branches (PENDING): synthesizeScope drops
  global/git_repo cases (graph tabs never carry those scopeIds), then the ~28
  group/global/git_repo `currentScope.kind` branches in GraphPanel become
  unreachable -> remove. NOTE: the group/global/git_repo KINDS stay in
  ScopeOption (SEARCH's availableScopeOptions still produces them); only the
  GRAPH handling is dead.

Then: fresh-binary BROWSER SMOKE of the whole A5 (graph render + dock + reload/
persistence - destructive change, warrants empirical check) before reporting A5
(both SHAs) ready-to-merge. Then GI-10 + loading-state -> A6, then I signal the
@@LaneB quiescence point.

## 2026-05-27 (round 2) - A5 part 2 done + rebased + SMOKED; found a main bug

Per @@Architect: rebased phase-12-lane-a onto current main (was 48b4951, now
4cb5ca8 with lanes d/e merged). A1/A4/A3 are already in main; rebase replayed
only A5p1 (-> a4c139b), clean (web/src disjoint from @@LaneB chunk-1 rename).
A5 part 2 (kind-branches) committed 760e242. Branch: 4cb5ca8 -> a4c139b -> 760e242.
Gate green: check 0/0, build OK, 1595 vitest. (Pre-existing tabs.test.ts vitest
exit-1 flake is @@LaneD's, per @@Architect - not mine.)

A5 BROWSER SMOKE (fresh binary on rebased branch; note it now compiles
chan-workspace = @@LaneB chunk-1 rename is in main):
- ✓ Reload: no crash; graph tab persists via layout `s`; hash restores cleanly
  -> A3 hash-retire + A5 session-payload deletion don't break persistence.
- ✓ Directory-scoped graph (New Graph on docs/): renders 3/3 nodes / 2/2 edges,
  breadcrumb "drive / docs", full DETAILS inspector -> A5's render/scope/
  inspector/breadcrumb paths are SOUND; the destructive deletion broke nothing.
- BUG (NOT A5, NOT mine): the DRIVE-scope semantic graph fails to load:
  "Failed to deserialize query string: unknown variant `drive`, expected one of
  `workspace`, `directory`, `file`". @@LaneB chunk-1 renamed the /api/graph
  backend scope variant drive->workspace, but the frontend client.ts (206/955/
  964/1346) still sends scope="drive". This is chunk-2 (frontend wire-flip,
  HELD on my quiescence) -> chunk-1 landed without it -> graph broken at drive
  scope on main. dir/file scopes work (scope=directory/file still valid).
  Reported to @@Architect as a release/integration blocker. A5 verified A1
  didn't regress this (A1 merged BEFORE chunk-1; the break is the rename).

A5 (a4c139b + 760e242) reported ready-to-merge. STILL AHEAD (round 2): the
drive-scope fix sequencing (@@Architect), GI-10, loading-state, A6 walk,
addendum-2 FB tab/dock expansion-independence verify (may be fixed by A4).

## 2026-05-27 (round 2) - A5 merged; GI-10 done

@@Architect merged A5 (abac76c); overlay concept fully retired. Confirmed the
drive-scope /api/graph break is @@LaneB's chunk-1 wire slip (they're hotfixing by
pinning the scope variant back to "drive"); I must NOT touch client.ts (chunk-2
territory). Rebased onto abac76c (branch 0-ahead, clean).

GI-10 (committed 3d3254b): drive root pinned to the BOTTOM, spine grows UPWARD.
One-spot GraphCanvas forceY flip (`depth` -> `-depth` * hierarchyYSpacing) +
spine test/comment updates. Gate: check 0/0, build OK, 1596 vitest. Visual check
deferred to the A6 walk (held on @@LaneB's graph hotfix). Reported ready.

NEXT: graph loading-state UX (graph-loading-state-spec.md). Then A6 walk +
addendum-2 verify once the graph hotfix lands. Not quiescent yet.

## 2026-05-27 (round 2) - GI-10 merged; unblocked; empirical session + loading-state investigation

@@Architect: GI-10 merged (a477e62); @@LaneB's graph hotfix is IN (2256aa8 pins
the serde scope tags back to "drive") -> drive-scope graph WORKS again -> A6 walk
+ GI-10 visual unblocked. Rebased onto a477e62 (branch 0-ahead, clean).

FRESH-BINARY EMPIRICAL SESSION (drive: broken link ghost.md + 3-deep nesting):
- ✓ GI-10 VISUAL: drive (storage glyph) sits low-center; folders/files arrayed
  ABOVE it; spine grows upward. Confirmed.
- ✓ WIPE / A6-essentials: drive-scope graph renders 13 nodes / 19 edges; ghost
  node (ghost.md broken link) renders DASHED outline + muted icon + dashed edge,
  distinct from solid real files. (A1-A5 verified working on fresh binary.)
- LOADING-STATE INVESTIGATION (root cause): on a small instant-indexed drive the
  dead-end is a GENUINE broken link, correctly dashed (spec step 3 ALREADY done).
  The index-lag phenomenon (spec steps 1-2) needs the index-completeness signal.

LOADING-STATE PLAN (grounded in source - de-risked, FRONTEND-ONLY):
- The per-scope signal EXISTS: `api.indexingState()` -> GET /api/indexing/state
  -> IndexingStateResponse { nodes: [{path, state: "indexed"|"indexing"|
  "pending", children_count}] }. Per-DIRECTORY. Currently consumed only by
  EmptyPaneCarousel.svelte. NO backend add needed.
- Implementation: GraphPanel/graphData subscribes to indexing-state; while the
  active scope's directory is not "indexed" (indexing/pending), show a loading
  state (pulse the parent dir, mirror FB spinner) instead of rendering dead-end
  ghosts as fact. Once "indexed", render fully; remaining ghosts = real broken
  links (dashed styling already exists).
- Surfaced to @@Architect: confirm this UX (pulse-parent-while-indexing) + the
  frontend-only/reuse-/api/indexing/state approach before I build.

A6 walk is essentially covered by this session (graph + wipe verified on fresh
binary); the dedicated A6 sweep can fold in once loading-state lands. Still not
quiescent (loading-state + addendum-2 FB-independence verify remain).

## 2026-05-27 (round 2) - addendum-2 verified (real bug); loading-state slice 1 shipped

addendum-2 VERIFIED on fresh binary: REAL bug, NOT A4-fixed. Expanding a dir in a
FB tab mirrors into the dock because FileTree renders expansion from the GLOBAL
`treeExpanded.map` singleton (FileTree:332). Tab persists its own `be` but the
live render uses the global map. FIX = per-instance expansion source (tab->`be`,
dock->own). Reported.

loading-state SLICE 1 (committed 19d5456): wired `indexStatus` into GraphPanel +
pulsing "indexing…" status-bar cue while building/reindexing (so an in-flight
graph isn't trusted as complete; reduced-motion respected; source-test locks it).
Gate: check 0/0, build OK, 1596+4 vitest.

Decomposed loading-state into slices because the full spec (pull back ghost
dead-ends while a scope indexes + pulse the parent dir) touches the canvas
paint() loop + the node/edge filter pipeline (visibleNodeIds re-adds nodes via
edge endpoints -> hiding ghosts needs edge filtering too) - higher-risk, so it's
SLICE 2 with its own gate + index-lag verification, not bundled into slice 1.

REMAINING before quiescence: loading-state slice 2 (paint/pipeline ghost
pull-back + parent pulse) + addendum-2 (per-instance FB tree expansion). Both
substantial. Then @@LaneB chunk-2.

## 2026-05-27 (round 2) - addendum-2 FIXED (per-instance expansion, via sub-agent)

@@Alex authorized sub-agents. Spawned ONE general-purpose sub-agent for the
Slice-E per-instance FileTree expansion migration (a877c9e8); I gave it a tight
spec, then reviewed + verified + committed.

The migration: FileTree reads/writes expansion from the per-instance
fbTreeInstances registry (keyed by instanceId) instead of the shared global
treeExpanded.map. FileBrowserSurface threads instanceId; reconcile/snapshot/
restore/toggle-all/fullyExpanded all per-instance; revealAndSelect fans across
instances (reveal surfaces wherever the user looks; user toggles independent).
Global singleton left intact for its other consumers.

LESSON (saved [[feedback_svelte_static_gate_misses_runtime]]): the sub-agent's
static gate PASSED (svelte-check 0/0 + 1603 vitest, all ?raw source-pattern) but
the live app CRASHED with state_unsafe_mutation - ensureFbTreeInstance (mutates
$state) was called inside a $derived. Static gates can't catch Svelte-5 runtime
reactivity violations; only the browser smoke did. I fixed it (ensure in
$effect, READ in $derived) + re-verified.

EMPIRICAL (fresh binary, post-fix): no crash; tab<->dock expansion independent
BOTH directions (docs/ in tab not dock; other/ in dock not tab); tab expansion
restores on reload via tab.expanded. CAVEAT: dock reload-persistence is
best-effort (sessionStorage snapshot didn't restore in my Chrome reload -
snapshot-key timing vs drive.info load); spec-accepted (dock reset on reload is
fine); flagged the key timing as a follow-up.

COMMITTED: phase-12-lane-a@915ea29 (gate: check 0/0, build OK, 1603 vitest).
Reported ready. Branch: a477e62 -> 19d5456 (loading slice 1) -> 915ea29.

REMAINING before quiescence: loading-state SLICE 2 only (paint/pipeline ghost
pull-back + parent-dir pulse). Then I'm quiescent -> @@LaneB chunk-2.

## 2026-05-27 (round 2 CLOSE) - loading-state slice 2 done; QUIESCENT

loading-state slice 2 (committed 73bc625): `hiddenMissingIds` $derived gated on
slice-1's indexBuilding, following the existing hidden*Ids node-filter pattern
(pure derived, no $state mutation -> no state_unsafe_mutation risk). visibleNodeIds
skips them; visibleEdges drops edges touching them. While indexing, dead-end
nodes + their edges are pulled back; once idle they reappear as real broken links.

VERIFIED fresh binary (300-file drive + broken link), BOTH states caught:
- building: 306/307 nodes, dead-end + edge pulled back, "indexing" cue. 
- idle (settled): 307/307 nodes, dead-end reappears as a real (dashed) broken
  link, cue gone. Clean transition, no crash, no dangling edges.

*** QUIESCENT *** - my full round-2 stack is reported ready on phase-12-lane-a:
19d5456 (loading slice 1) -> 915ea29 (addendum-2) -> 73bc625 (loading slice 2).
A1-A5 + GI-10 already merged. Pausing all web/src work so @@Architect can open
the chunk-2 freeze for @@LaneB's drive->workspace codemod (rebases onto my
settled tree).

ROUND-2 SCORECARD (lane-a): overlay/scope WIPE W1-W6 (A1-A5) merged; GI-10
merged; loading-state slices 1+2 ready; addendum-2 (per-instance FB expansion)
ready; caught + reported the chunk-1 /api/graph drive-scope wire break (fixed by
@@LaneB); caught + fixed a sub-agent state_unsafe_mutation crash via empirical
smoke (lesson saved). Used 1 sub-agent (addendum-2) per @@Alex.

DEFERRED carryover (refinements, post-codemod / future round - NOT blocking):
- loading-state per-parent-dir pulse (slice 3): pulse the specific parent dir of
  a pulled-back dead-end, + per-scope (not just global indexBuilding) gating via
  /api/indexing/state.
- dock/overlay reload-snapshot key timing (fbInstanceReloadKey uses drive.info.root
  on write but the on-mount seed falls back to pathname before drive.info loads).
- now-dead group-edge kind + SCOPE_HUB_ID machinery in GraphCanvas (A5 follow-up,
  unreachable since group scope retired).
- optional GI-11 ../ / ./ link-target regression-lock tests.
- A6 dedicated walk (essentials already covered across this round's smokes).

## 2026-05-27 (round 2, post-codemod) - addendum-3 cosmetics DONE

chunk-2 (drive->workspace codemod) merged (main bce6bd3); freeze lifted. Rebased
onto bce6bd3 (codemod renamed my files: WorkspaceInfoBody, showWorkspace, scope
"workspace", etc.). @@Alex routed 3 addendum-3 items (item 1, terminal orange-dot
pulsating, was NOT mine).

- A3-i (committed 3de22a76): graph right-click opens the tab menu ANYWHERE on the
  canvas bg (openTabMenu anchored at the cursor; mirrors the editor). 
- A3-iii (committed 5003a6a2): "Export to PDF" moved from the editor menu to the
  Inspector (FileInfoBody), MARKDOWN files only (@@Alex chose markdown-only over
  all-editable via AskUserQuestion). Inspector fetches content (editor autosaves
  so disk==live); removed it + dead Printer/print imports from the editor.
  isMarkdown added to fileTypes. Tests flipped (editor: NOT present; inspector:
  present + markdown-gated).
- A3-ii: CLOSED done-by-GI-10. Verified on the fresh post-codemod build - the
  workspace node sits at the bottom, spine grows up (GI-10, merged earlier).
  @@Alex was on a pre-GI-10 build.

All gated (check 0/0, build OK, 1611 vitest) + browser-smoked on a fresh
post-codemod binary (right-click-anywhere menu; drive-at-bottom; Export-to-PDF in
the inspector slot). Branch: bce6bd3 -> 3de22a76 (A3-i) -> 5003a6a2 (A3-iii).
Post-merge: quiescent again.
