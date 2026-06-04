# task Lead -> LaneF (1): Repo / docs cleanup

You are @@LaneF - Repo / docs cleanup lane. Round-1, Wave 1. START NOW.
This is the long pole; you run across all 3 waves. NO deletions in Wave 1.

## Read first (context lives here, not in this poke)
- Process: docs/journals/phase-18/team/bootstrap.md
- Plan + your lane section: docs/journals/phase-18/team/round-1-plan.md
  (section "@@LaneF - Repo / docs cleanup")
- Verbatim spec: docs/journals/phase-18/round-1/draft.md  ("### Repo")

## Wave 1 scope (consolidate, do NOT delete yet)
1. Consolidate each phase's journals into NEW docs/phases/phase-N.md - the
   phase's roadmap, rounds, waves, retrospective. Capture the ESSENCE so new
   agents learn from prior execution, successes, and mistakes (not a raw dump).
   Phases 1-16 are stable/done: FAN OUT subagents, one per phase, in Wave 1.
   phase-17 and phase-18 fold in at CLOSE (Wave 3); phase-18's bus is LIVE this
   round - do not touch docs/journals/phase-18.
2. Distill docs/agents into a MINIMAL referenced set + a lessons-learned
   playbook, kept under docs/agents/. (Identify what to keep/delete; stage the
   trimmed set. The actual deletion of leftovers is Wave 3.)
3. docs/phases/phase-N.md is text-only by default; keep a screenshot ONLY if it
   is load-bearing.

## Deletions are Wave 3 ONLY (do NOT do these now)
.claude (untracked rm -rf), .codex (untracked rm -rf), docs/archive (git rm),
trimmed docs/agents leftovers (git rm), docs/journals (git rm). Do NOT delete
docs/journals/phase-18 until @@Lead confirms the round is committed (the live
team bus + gate worktree depend on it). Also Wave 3: scrub stale doc-comment
path mentions in chan-workspace/embeddings.rs, chan-server/routes/graph.rs,
pages.yml.

## Owned files
docs/journals/** (READ), NEW docs/phases/**, docs/agents/**, docs/archive,
.claude, .codex (the last three only at Wave 3 deletion).

## On completion of Wave 1 (consolidation)
Cut task-LaneF-Lead-1.md (which phases consolidated + the docs/agents keep/cut
plan + what's staged for Wave 3 deletion), poke me. Journal: journal-LaneF.md.
Hold for my Wave 3 go before ANY deletion.
