# Phase 11 - drive streaming spine, editor/graph fixes, release contract

Status: closed
Span: 2026-05-26 to 2026-05-27 (two calendar days; based on git author dates and dated journal headers)
Versions: none recorded in the phase journals
Tags: #features #bugfixes #performance #editor #graph #release

## Roadmap (the asks)

There was no single request file. The phase opened from three lane plans and a round-1 bug list. The asks grouped into four areas:

**Drive streaming spine (@@LaneA):** Stream the drive listing so large drives do not block the UI. The goal was progressive file-tree hydration, so first paint would not stall while the full listing was computed.

**Editor and desktop bug bundle (@@LaneB):** A collected bug list that included a trailing-slash directory reject, an idle terminal garbling, a stuck reindex pill, and additional editor/desktop regressions.

**Graph inspector fixes and loading state (@@LaneC):** Graph inspector bugs, a loading state while the graph computed, and the overlay scope wipe.

**Release contract:** A repeatable document that codified the version contract and the cut process, so future releases could follow a known procedure.

## Rounds and waves

Phase 11 ran as a single round with a continuation pass. @@Architect dispatched three lanes in parallel from the start. Per-lane git worktrees isolated code changes; coordination documents (event channels, per-author journals) lived in the main checkout. @@Architect serialized every merge and re-gated after each, and @@Alex provided one standing gate per lane before final close.

Round close: all four areas were declared done or accounted for in handoff notes before the phase closed. Part of the bug bundle and some graph polish carried forward to phase 12 as explicit deferred items.

## Team and coordination

Agent handles this phase were positional, not named. See ../agents/README.md for the agent roster. Only @@Architect resolves to a contact card; the lane handles have none.

```
handle       role this phase                           card
-----------  ----------------------------------------  ---------------
@@Architect  plan, dispatch, merge serialization,      architect.md
             re-gating, round-close retrospective
@@LaneA      drive streaming spine (backend +          (no card)
             progressive frontend hydration)
@@LaneB      editor + desktop bug bundle               (no card)
@@LaneC      graph inspector + loading state +         (no card)
             overlay scope wipe
@@Alex       human owner; one standing gate per lane   (human owner)
```

Coordination scheme: append-only directional event channels (`event-<from>-<to>.md`) under a `raw/coordination/` subtree, combined with per-author journals, all edited in the main checkout. Code changes lived in per-lane git worktrees. @@Architect serialized every merge back to main and re-ran the gate after each merge before letting the next lane land. This is the model phase 12 carried forward and refined.

This phase differs from phases 7-9 in two respects: lane handles are generic positional labels rather than the named roster, and the worktrees isolate code while coordination documents stay centralized.

## What shipped, tried, and undone

**Shipped:**
- Drive streaming spine: the drive listing now streams progressively; large drives no longer block first paint or freeze the file tree.
- Editor and desktop bug bundle: the trailing-slash directory reject, the idle terminal garbling, the stuck reindex pill, and associated regressions were resolved.
- Graph inspector fixes: bugs in the inspector panel were corrected, a loading state was added while the graph computed, and the overlay scope wipe landed.
- Release contract document: codified the version contract and repeatable cut process for future releases.

**Tried then deferred:**
- Part of the editor/desktop bug bundle was not fully resolved and carried forward to phase 12.
- Some graph polish items also carried to phase 12.

**Deliberately not done:**
- No version tag was cut this phase; the release contract was documentation groundwork, not an actual release.

## Retrospective

**Highlights:**
- The drive streaming spine was the headline delivery: a structural change that directly improved perceived performance for large drives. Progressive hydration meant the UI responded immediately even before the full listing arrived.
- The release contract gave the project a written cut process for the first time, reducing ad-hoc decision-making at release points.
- Per-lane worktrees with architect-serialized merges kept the main branch clean throughout a parallel three-lane run.

**Lowlights / contention:**
- The bug bundle was broader than a single lane could close in one round. Some items slipped to phase 12, which means the initial scoping underestimated the tail work.
- Positional lane handles without contact cards make the journal harder to trace back to specific contributors. When the coordination model matured in later phases, named handles with cards became standard.

**Constructive feedback / lessons:**
- Even when using positional handles for flexibility, create a minimal contact card so that later graph traversal and retrospective attribution can resolve the handle. A one-line card is better than none.
- Bug bundles benefit from explicit priority triage upfront. Marking items as "must-close this phase" vs. "nice-to-have" before dispatch prevents the common outcome where the high-priority items land but the tail gets deferred again.
- Keep the release-contract document current as the cut process evolves; a stale contract is worse than none because it creates false confidence. Assign an owner to update it at each version bump.
- The architect-serialized merge model works well at three lanes. The round-trip overhead per lane grows linearly, so this model should be reviewed if lane count climbs past four or five.

## Notes

**Terminology drift:** In phase-11 journals, "drive" refers to the chan workspace root directory on disk (also written as "chan drive" or "drive listing"). This became "workspace" as naming stabilized in phase 12 onward. "chan-drive" was the crate name before the chan-drive -> chan-workspace rename.

The raw working material (per-author journals, task files, coordination event-channel logs, the round-1 bug list with its original screenshot notes) is preserved in git history under docs/journals/phase-11/; that tree was removed from the working tree in the phase-15 docs cleanup.
