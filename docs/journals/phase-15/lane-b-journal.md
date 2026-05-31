# Lane B journal — phase-15

Search overlay cleanup: remove the SCOPE selector, the SEARCH STATUS
button, and (gated) delete the search-status overlay.

## Decisions locked by @@Alex
- **FileTree "Search" right-click entry + `openSearchForFile`/
  `openSearchForDirectory`: drop entirely.** Search is workspace-wide,
  reachable via Cmd+K only. (Not "prefilled-query open".)

## Spec correction (raised + accepted)
- B1/`plan-round-1.md` listed `scope.svelte.ts` for removal. That is an
  over-reach: `scope.svelte.ts` is the **shared** scope picker for both
  Search and Graph. `GraphPanel.svelte` (`synthesizeScope`, `ScopeOption`,
  the scope dropdown), `store.svelte.ts`'s graph helpers
  (`availableScopeOptions`, `defaultScopeId`, `resolveGraphSpawnContext`),
  and `tabs.svelte.ts` (`graphTitle`, per-tab `scopeId`) all depend on it.
  Deleting it would break Graph, which is out of scope.
  -> Approach: keep `scope.svelte.ts` intact; remove only Search's *use*
  of it.

## B1 + B2 — done (commit `fda36d53`)

Search is now workspace-wide; no scope, no client-side result filtering.

### SearchPanel.svelte
- Removed the `<span class="title">Scope</span>` + SCOPE `<select>` and
  the SEARCH STATUS (`Database`) button + `openSearchStatus`.
- Removed `scopeOptions`/`currentScope` derives, both scope `$effect`s
  (snap-to-workspace + re-run-on-scope-change), `pathInScope`, and the
  `lastSearchedScopeId` tracker.
- `scheduleSearch` limit is a flat `25` (was `scoped ? 100 : 25`).
- `searchLanguage` scans all non-dir tree entries (dropped the
  `pathInScope` filter).
- `tagRows`: doc counts are now total tag-edge counts (dropped the
  `filePath` map + scope test); `refs === 0` still drops orphan tags.
- `rows` combiner: dropped every `pathInScope` filter.
- Removed imports: `availableSearchScopes`, `searchStatusOverlay`,
  `ScopeOption`, `defaultScopeId`, `Database`. Removed the `.title` +
  `.scope-select` CSS and the redundant `header { gap }` rule.

### store.svelte.ts (shared file; my searchPanel/scope region only)
- `searchPanel` $state: dropped the `scopeId` field + init.
- Removed `availableSearchScopes`, `openSearchForFile`,
  `openSearchForDirectory`.
- Removed the `HASH_SEARCH_SCOPE` (`search_scope`) URL-hash key + its
  parse (`applyOverlaysFromHash`) and serialize (`persistStateToHash`).
  Old bookmarks with `search_scope=` strip on next write (not in
  HASH_KEYS). Verified the diff contains no `handleWindowCommand`
  (@@LaneC's region).
- Import from `scope.svelte` trimmed to just `defaultScopeId` (still
  used by graph open fns). `searchStatusOverlay` state KEPT (B3-gated).

### FileTree.svelte
- Removed the right-click "Search" menu entry, `searchThis`, the
  `openSearchFor*` imports, and the now-unused `Search` lucide icon.

### App.svelte
- Removed the `void searchPanel.scopeId` line from the hash/session
  persistence `$effect`. SearchStatusOverlay import/render KEPT (B3).

### Tests
- `store.test.ts`: dropped two `searchPanel.scopeId = "workspace"` lines.
- `fileTreeSelectionMenu.test.ts`: flipped "Search label relabelled" ->
  "Search entry removed" (asserts no `<span>Search</span>`).

## Gate (web; change is 100% frontend, Rust unaffected)
- `svelte-check`: 0 errors / 0 warnings.
- `vitest`: all tests touching my 6 files pass. 3 unrelated failures
  remain, all in files I never touched, from other lanes' uncommitted
  WIP in the shared worktree:
  - `altSpaceXtermHandlerRemoved.test.ts`, `terminalGeneratedReplyFanout`
    -> `TerminalTab.svelte` (@@LaneC, BUG-3 keyboard-protocol work).
  - `paneFocusFollowFlip.test.ts` -> `Pane.svelte` (@@LaneA flip).
  The failing set even shifted between two runs, confirming concurrent
  edits by A/C. None are in my commit.
- `vite build`: success (only pre-existing chunk-size / dynamic-import
  warnings).

## B3 — done (commit `eb507ed2`)

CK-INDEX verified for real before deleting: the Index widget now lives
in `dashboard/SearchSlotConfig.svelte` (committed by @@LaneA at
`dbf59875`) with `indexStatus`/`rebuildIndex`/`api.indexRebuild` + the
chunks/vectors/model + building/reindexing display + Semantic +
Embedding model. So the standalone overlay was redundant.

- Deleted `SearchStatusOverlay.svelte` (-383).
- `App.svelte`: dropped the import, the `<SearchStatusOverlay />` render,
  and the `void searchStatusOverlay.open` line in the overlay-stack
  `$effect`.
- `store.svelte.ts`: removed the `searchStatusOverlay` state, the
  `"search-status"` `OverlayId` member, and its `closeOverlay` /
  `syncOverlayStack` arms. `OverlayId` is now just `"search"`. The
  overlay-stack machinery stays (it is the general mechanism; one
  overlay is fine).
- Only remaining textual reference is an intentional provenance comment
  in @@LaneA's `SearchSlotConfig.svelte` ("ported from the former
  SearchStatusOverlay's Index widget") — left as-is.

Gate (whole tree was all-committed code by this point): svelte-check
0/0, vitest **1572/1572** (the earlier terminal/flip failures cleared
once A and C committed), `vite build` clean.

## Coordination
- B1/B2 commit `fda36d53` (`store.svelte.ts` searchPanel/scope region).
- B3 commit `eb507ed2`. By the time I did B3, @@LaneC had committed their
  `handleWindowCommand` work (`41b28e7a`) and @@LaneA their dashboard
  work, so `store.svelte.ts` was clean at HEAD and B3 landed as one
  clean commit (no shared-file staging trap).

## Lane B status: all tasks (B1, B2, B3) done and committed locally.
Nothing pushed (per round rules). Round-2 backlog from Lane B: none.
