# Lane B — Search overlay cleanup

## Bootstrap prompt

You are **@@LaneB**. Read `bootstrap.md`, then this file, then
`plan-round-1.md`, then `coordination.md` (do not read `roadmap-round-1.md` —
it is @@Alex's and already decomposed here). Mission: remove the now-redundant
Search
SCOPE selector, the SEARCH STATUS button, and the search-status overlay
entirely. Most of this runs in parallel with @@LaneA; only the final overlay
deletion is gated on @@LaneA relocating the Index widget. Coordinate directly
with @@Alex; cut a task to @@LaneA/@@LaneC for anything in their files.
Confirm understanding, then wait for the go.

**You own:** `SearchPanel.svelte`, the `searchPanel`/scope region of
`store.svelte.ts`, `scope.svelte.ts`, and the deletion of
`SearchStatusOverlay.svelte`. @@LaneC owns the `handleWindowCommand` region
of `store.svelte.ts` — stay in your region.

## Tasks

- **B1.** Remove the SCOPE `<select>` + all client-side scope filtering it
  drives (`pathInScope`, scope `$effect`s, `scopeId`/`availableSearchScopes`,
  `scope.svelte.ts`). Search becomes workspace-wide. Resolve
  `openSearchForFile`/`openSearchForDirectory` call sites (drop vs
  prefilled-query open — raise with @@Alex).
- **B2.** Remove the SEARCH STATUS button + `openSearchStatus`.
- **B3 [GATED on `CK-INDEX`].** Delete `SearchStatusOverlay.svelte` + its
  registration, once @@LaneA confirms the Index widget is live in the
  Dashboard Search back. You are the sole deleter; @@LaneA only reads it.

## Coordination checkpoints
- `CK-INDEX`: wait for @@LaneA before B3.

## Open decisions to raise with @@Alex
- Fate of `openSearchForFile` / `openSearchForDirectory` once scope is gone.
  **RESOLVED (@@Alex): drop entirely** — search is workspace-wide via Cmd+K.

## Progress log

- **2026-05-30 — B1 + B2 done, committed `fda36d53` (local, main).**
  Search is workspace-wide; SCOPE selector + STATUS button removed; the
  File Browser "Search" right-click entry + `openSearchFor*` dropped.
  Spec correction accepted: `scope.svelte.ts` is shared with Graph and
  was NOT deleted (only Search's use of it removed). Gate: svelte-check
  clean, build clean, all my-file vitest tests pass (3 unrelated
  failures are @@LaneA `Pane.svelte` / @@LaneC `TerminalTab.svelte`
  WIP). Full write-up in `lane-b-journal.md`.
- **2026-05-30 — B3 done, committed `eb507ed2` (local, main).**
  CK-INDEX verified: Index widget confirmed live in
  `dashboard/SearchSlotConfig.svelte` (@@LaneA `dbf59875`) before
  deleting. Deleted `SearchStatusOverlay.svelte` + de-registered it from
  App.svelte + removed the `searchStatusOverlay` state / `"search-status"`
  `OverlayId` / `closeOverlay` / `syncOverlayStack` arms in
  `store.svelte.ts`. Landed as one clean commit (C + A had committed, so
  `store.svelte.ts` was clean at HEAD). Gate: svelte-check 0/0, vitest
  1572/1572, build clean.

**Lane B complete: B1, B2, B3 all done and committed locally
(`fda36d53`, `eb507ed2`). No round-2 carryover.**
