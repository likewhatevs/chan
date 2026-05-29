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
  (signals backpressure), never a whole refetch. Keep the existing
  gesture model as-is: single click = inspector, double click =
  "graph from here", background tap = clears. Do NOT add a per-node
  expand/collapse gesture; rescope covers that case. This item only
  makes delivery incremental, not the interactions.
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

## Coordination

- Build the graph rendering and the OverlayShell against the pinned
  `contracts.md`; if a shape is missing, request it in
  `event-lane-b-lane-a.md` rather than inventing it.

## Gate

- `cd web && npm run check && npm test -- --run && npm run build`; the
  gateway SPA (`gateway/`) `npm run check`/`build` green.
- No Rust touched. Visible outcomes unchanged on every surface.
