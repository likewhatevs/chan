# Phase 12 - drive-to-workspace rename and graph/FB carryover

Status: closed (v0.16.0 cut was handed to a lane as a post-close carryover)
Span: 2026-05-27, one day (estimate; only two git commits bookend the round)
Versions: none cut during the round; v0.16.0 prep was carryover to phase 13
Tags: #refactor #graph #cli #release #desktop #bugfixes

## Roadmap (the asks)

Three primary tracks, assembled from a carryover backlog spanning phases 10 and 11, plus three ad-hoc request bundles from @@Alex:

**Graph and File Browser (lane A):** The overlay/scope-concept wipe -- the "big one" -- that makes graph scope equal to a filesystem directory and deletes the load-bearing overlay state machine. Additionally: GI-10 (drive node pinned at the bottom, spine grows upward), a graph loading state, and File Browser per-instance expansion state.

**Drive-to-workspace rename (lane B):** Rename the `chan-drive` crate to `chan-workspace` and all "drive" terminology to "workspace" across code, comments, and documentation. Lane B was cast as a scoping architect first, expected to surface the hard decisions before any codemod: the "team workspace" name collision, user-facing CLI and config key changes, the `drive.chan.app` tunnel domain, uniffi binding names, and sequencing constraints.

**Frontend cosmetics and keyboard shortcuts (lane C):** @@Alex-driven ad-hoc work across three request bundles: Rich Prompt and Drafts bugs, editor and terminal issues, and the full cross-platform shortcut policy. A fourth track -- CI framing and release prep (lane D) -- and a shortcuts-gap-only pass (lane E) were added mid-round when scope expanded.

## Rounds and waves

Phase 12 ran as a single round on 2026-05-27. There was no formal wave breakdown; lanes ran in parallel with sequenced merge windows managed by the architect.

The key sequencing constraint: lanes A, C, D, and E all touch `web/src` while lane B's codemod touches nearly everything in the repo. Lane B scoped first and produced a per-chunk codemod plan. The codemod landed in a freeze window gated on the other lanes reaching web/src quiescence, so the atomic wire-plus-frontend flip did not collide with live frontend edits. Lanes reported "ready to merge: branch@sha" and the architect serialized every merge, re-gated the combined tree, and owned the codemod-window timing decision.

The whole coordination doc tree stayed untracked as the live bus throughout the round and was committed once at close.

## Team and coordination

Lanes are positional phase-12 handles. Only @@Architect resolves to a named agent card; all other handles are phase-local roles. See ../agents/README.md for the agent roster.

```
handle        role this phase                              card
-----------   -----------------------------------------   ----------------
@@Architect   orchestrate, serialize merges, re-gate,     architect.md
              own the codemod-window decision
@@LaneA       graph + File Browser carryover              (no card)
@@LaneB       scoping architect for the rename            (no card)
@@LaneC       @@Alex ad-hoc frontend / cosmetics          (no card)
@@LaneD       CI + release (added mid-round)              (no card)
@@LaneE       cross-platform shortcuts (added mid-round)  (no card)
@@Alex        human owner; drives lane C; rules scope     (human owner)
```

Coordination scheme: per-lane git worktrees held source code only. All coordination docs -- plans, journals, event channels -- were edited in the main checkout by absolute path, keeping the bus shared, conflict-free, and append-only. Channels were directional `event-<from>-<to>.md` files created on first use, including cross-lane seams. The architect re-gated and re-smoked the combined tree after every merge rather than trusting the per-lane gate alone.

## What shipped, tried, and undone

**Shipped:**
- Overlay/scope-concept wipe landed across a set of merge slices managed by @@Architect; graph scope is now a filesystem directory, the overlay state machine is deleted.
- GI-10 (spine grows upward from the drive/workspace root node) and the graph loading state.
- File Browser per-instance expansion state.
- The drive-to-workspace rename in sequenced chunks: a free "workspace" reservation, the crate and type rename with wire literals held stable, then an atomic wire-plus-frontend flip, a full tunnel rename, a docs sweep, and a web/marketing pass. End state: zero code drive-residue, with only `drive.chan.app` and third-party cloud product names preserved by design.
- Lane C ad-hoc bug fixes (Rich Prompt, Drafts, editor, terminal issues).
- Lane D CI framing correction and release prep.
- Lane E shortcut-gap fixes (audited first; roughly 80 percent of the policy was already implemented, so only the gaps shipped).

**Tried, then corrected:**
- Lane A's authorized subagents were not used; the wipe slices were too tightly coupled on shared files, so lane A ran in-session sub-slices instead.
- Chunk-2 of the rename was reported gate-and-smoke green but rejected on re-audit (a frontend config key mismatch and a desktop compile break). Re-squashed with both fixed, then merged.
- The `/api/graph` scope variant was renamed before the client caught up, breaking the whole-workspace graph view. Hotfixed by pinning the variant with a serde rename attribute; the real client flip rode the atomic chunk.
- The architect modeled the tunnel surface as "preserve"; @@Alex corrected it ("I explicitly wanted it removed") and the full tunnel rename shipped.

**Deliberately not done / deferred:**
- v0.16.0 version bump, changelog, the first origin push (which fires CI and the release workflow), and repointing the upgrade channel were handed to a lane as a post-close carryover rather than rushed in at close.
- Two terminal fixes were merged unverified by design: a WKWebView-only render glitch that @@Alex elected to skip verifying given the pre-release status.
- Remaining graph backlog items and the Linux-desktop backlog carried forward.
- A tunnel cloud redeploy (the wire changed) was a carryover dependency.

## Retrospective

**Highlights:**
- The rename landed clean and complete as a hard break with no migration path, which is the right call for a pre-release project. The scoping-first model -- scope doc before any codemod -- surfaced the hard decisions (three meanings of "drive", the wire-decoupling that made the codemod splittable) before edits began.
- Every gate-blind defect was caught by the architect re-gate, re-smoke, and leftover audit before any release, not by the lane gate.
- Five lanes self-coordinated via peer-to-peer quiescence checks across 56 commits without a merge collision.
- Lane E's audit-first approach is worth generalizing: when a policy task arrives, measure coverage before writing new code.

**Lowlights / contention:**
- The gate-blind wire-rename class hit approximately five times: stale Tauri permission names, a handoff serde variant, the `/api/graph` scope variant, a config-key mismatch, and a desktop compile break. The cargo, vitest, and svelte-check gates are all blind to serde tags, IPC strings, route variants, and desktop compile paths. Each miss cost a reject-and-redo cycle.
- Several "complete" signals arrived ahead of the committed, smoked state, forcing reject-and-redo round-trips.
- Multiple late scope additions stretched the round's tail after the main lanes had quiesced.

**Constructive feedback / lessons:**
- Lane B (the rename): the scope analysis was standout quality and the wire-decoupling insight made the whole codemod tractable. The misses were in the per-chunk gate: wire fields, desktop compile, cross-service wire, and a runtime smoke must be part of the gate before reporting ready. "Ready to merge" should mean committed and smoked, not cargo-green.
- @@Alex: decisive on load-bearing calls (WASD collision, tunnel scope correction). Staging the full addenda set before round open and sanity-checking nits against a fresh build would shorten the tail.
- Architect self-note: the re-gate/re-smoke/leftover-audit pattern caught every gate-blind defect this phase, but ad-hoc. Codify a standing rename-audit checklist: wire strings, serde tags, route variants, IPC and Tauri permissions, desktop compile, cross-service wire, runtime smoke. Apply it at every rename-class task, not just when defects surface.

## Notes

**Terminology drift:** `chan-drive` became `chan-workspace` (crate name, Rust types, CLI, config keys, and documentation). The term "drive" in the sense of a chan workspace directory was replaced with "workspace" everywhere in code. Preserved by design: the `drive.chan.app` hostname (cloud surface) and third-party cloud product names that use "drive". The word "drive" still appears in comments or docs that reference those external services.

The raw working material (per-lane journals, task and request files, coordination event logs, the codemod plan) is preserved in git history under `docs/journals/phase-12/`; that tree was removed from the working tree in the phase-15 docs cleanup.
