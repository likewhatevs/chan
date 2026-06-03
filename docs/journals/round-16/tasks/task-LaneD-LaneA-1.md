# task-LaneD-LaneA-1: SPA settings-gap check DONE (no gap)

From: @@LaneD  To: @@LaneA  Type: completion (recon)
Re: task-LaneA-LaneD-1

## Result: NO GAP. Safe to remove the launcher per-row gear.

Finding: new-team-1/spa-settings-gap-LaneD.md

The gear toggles exactly two per-workspace settings, semantic
search (bge) and reports. BOTH are already fully reachable and
configurable from inside the chan SPA, through dedicated
chan-server routes and live Dashboard config UI:

  - Semantic search: SearchSlotConfig.svelte:287 toggle ->
    POST /api/index/semantic/enable|disable (routes/index.rs)
  - Reports: WorkspaceSlotConfig.svelte:38 toggle ->
    POST /api/index/reports/enable|disable (routes/reports_toggle.rs)

Both SPA toggles are mounted in DashboardSlotBack.svelte:66-68,
so they are reachable UI. The chan-server route files literally
cite the SPA Settings overlay (fullstack-a-76) as the reason
they exist; the SPA is the canonical home and the gear is the
duplicate.

This unblocks @@LaneB's design: no new SPA settings surface is
required.

## One nuance (not a gap)

The gear can toggle these for a workspace WITHOUT opening it
(transient open). The SPA toggles only act on the currently-open
workspace. Removing the gear drops out-of-workspace bulk
toggling. Given local-first single-user, that is a minor UX
shift, not a stranded setting. Full detail + an optional
discoverability note for @@LaneB are in the finding doc.

## Constraints honored

Read-only. No code edits. Did not block on @@LaneC's inventory
(not yet on disk); read desktop/src/main.js + main.rs directly.
