# @@LaneB request - Phase 14

You are @@LaneB, the **frontend** lane. You own all frontend trees:
`web/` (editor SPA, also embedded in desktop), `gateway/crates/identity/web`,
`gateway/web-common`, `web-marketing/`. You do NOT touch Rust (that is
@@LaneA). You MAY spawn 1-2 in-session subagents, but they must be
SERIALIZED on shared `web/` files: do the structural work first, then
the cleanup over the result. You run concurrently with @@LaneA; you
share only the seams in `coordination/contracts.md`.

## Recover context (read in order)

- `/Users/fiorix/dev/github.com/fiorix/chan/CLAUDE.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/design.md`
- `docs/journals/phase-14/roadmap-round-2.md` (the pristine-cleanup principles)
- `docs/journals/phase-14/roadmap-round-3.md` (theme 1 frontend half + theme 2 OverlayShell)
- `docs/journals/phase-14/coordination/contracts.md`
- `docs/journals/phase-14/coordination/event-lane-a-lane-b.md` (inbox; may not exist yet)
- `docs/journals/phase-14/lane-b-plan-draft-restore-banner.md` (correctness item; the backend half is Lane A)
- `docs/journals/phase-14/lane-b-plan-cmd-comma-flip.md` (correctness item)
- `docs/journals/phase-14/addendum-1.md` (phase-13 r2 carryovers; your item is in B4)

## Worktree + branch

```
git -C /Users/fiorix/dev/github.com/fiorix/chan worktree add ../chan-p14-lane-b -b phase-14-lane-b
```

Journals/contracts/inboxes are edited by ABSOLUTE PATH in the canonical
checkout under `docs/journals/phase-14/`.

## Scope (do B1 before B2; do not run them concurrently on shared files)

### B1. Structural: incremental graph + pre-flight lock (round 3)

- Incremental graph construction in the graph tab and the dashboard
  indexing graph (cytoscape): consume @@LaneA's batched delivery per
  `contracts.md` section 1, appending nodes/edges as they arrive. The
  UI must stay responsive at all times; the editor, file browser,
  terminal, and other graphs stay interactive while a large workspace
  (`/tmp/linux`) fills in. The depth slider requests the next batch
  (signals backpressure), never a whole refetch.
- Graph gesture model (revised). Single click = select + open the
  inspector (unchanged; the inspector keeps "graph from here").
  **Double click on a directory node = expand/collapse it IN PLACE
  with no graph reload** - expanding reveals the dir's next degree
  (fetched incrementally via the contract if not already loaded),
  collapsing hides its subtree. **Remove the old double-click "graph
  from here"** (rescope stays in the inspector / right-click / chord).
  The depth slider's `find -d N` scope is authoritative and overrides
  individual expand/collapse. **Persist** the expanded/collapsed set
  across a window reload like the File Browser - reuse the
  `treeExpanded` + sessionStorage persistence in
  `web/src/state/store.svelte.ts` (`persistTreeExpanded` and friends).
- Pre-flight OverlayShell lock (round 3, theme 2): render the
  chan-server pre-flight per `contracts.md` section 2, LOCKED until
  complete - hide/remove the close button, ignore ESC, and guide the
  user toward booting the workspace.

### B2. Pristine cleanup (round 2)

Over the result of B1, across all four frontend trees:

- Correctness first: preserve today's working outcomes; verify against
  the live surfaces before/after; do not refactor in a way that risks
  an outcome.
- Remove obvious duplication; introduce only abstractions that clarify.
- Consistency + idiomatic TS / Svelte / Vite across the four trees;
  converge divergent styles.
- First-public-release discipline: delete back-compat shims, aliases,
  dead transitional code, and changelog-style comments; keep only
  WHY-snapshot comments. The source must read fresh-like-new.

### B3. Correctness bugs (live; have their own plan docs)

Two reported bugs with dedicated plans; each ships with the
regression/stress test it needs. They can lead the lane since they are
concrete and user-facing.

- Draft "unsaved changes from a previous session" false banner -
  `lane-b-plan-draft-restore-banner.md`. Lane B owns the
  `editorBuffer.ts` / `FileEditorTab.svelte` fix; Lane A owns the
  backend half of the e2e stress test (coordinate via the inbox).
- Cmd+, queued-shortcut "panes flip" desync -
  `lane-b-plan-cmd-comma-flip.md`. Audit + cleanup of the
  shortcut-dispatch / pane-focus path.

### B4. Addendum-1 follow-up (see `addendum-1.md`)

`/dl` preserve-release-metadata is a circular self-perpetuating guard
(addendum-1 #1). `web-marketing/scripts/preserve-release-metadata.mjs`
fetches the LIVE `https://chan.app/dl/releases.json`; on 404 it
preserves nothing, so once `/dl` is clobbered every later marketing
deploy keeps it 404 until a release regenerates it. Fix: source `/dl`
from the GitHub release assets (as `release.yml`'s
`generate-release-metadata.mjs` does), not by self-fetching the live
site. Optional (CI, not strictly Lane B): put `pages.yml` +
`release.yml` in a shared concurrency group so they can never deploy
`github-pages` at once. `web-marketing/` is one of this lane's trees.

## Coordination

- Build the graph rendering and the OverlayShell against the pinned
  `contracts.md`; if a shape is missing, request it in
  `event-lane-b-lane-a.md` rather than inventing it.

## Gate

- `cd web && npm run check && npm test -- --run && npm run build`; the
  gateway SPA (`gateway/`) `npm run check`/`build` green.
- No Rust touched. Visible outcomes unchanged on every surface.
