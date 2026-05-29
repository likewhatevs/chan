# @@LaneC request - Phase 14 (the architect)

You are @@LaneC / the `/architect`. Two workstreams with different
timing:

- **C2 (concurrent)** - reorganise `docs/journals/` into the project's
  second brain. Touches only `docs/journals/`, so it runs alongside
  @@LaneA and @@LaneB with no collisions.
- **C1 (closing wave)** - the round-2 `/architect` pass over the
  FRONTEND comments, docs, and user-facing copy. Runs AFTER @@LaneA and
  @@LaneB merge (it edits the same code they are rewriting).

## Recover context (read in order)

- `/Users/fiorix/dev/github.com/fiorix/chan/CLAUDE.md` (writing rules)
- `docs/journals/phase-14/roadmap-round-2.md` (the `/architect` brief for C1)
- `docs/journals/README.md` (the journals index; currently stale, stops at phase-7)
- a sampling of `docs/journals/phase-*/` (the raw material for C2)

## C2: docs/journals as the second brain (concurrent)

Goal: `docs/journals/` becomes the project's second brain - the context
of how we got here and what we tried, did, undid, and how long it took.
Per phase, produce a full report. Suggested shape (one report per phase
dir):

- **Initial asks** - the source request(s) that opened the phase.
- **Team + profiles** - who was involved (the lanes/agents) and their
  roles; link the contact cards under `docs/agents/`.
- **Duration** - an estimate of how long the sessions ran.
- **Highlights / lowlights** - what went well, what did not.
- **Constructive feedback** - what to do differently next time.
- **What shipped / tried / undone** - the outcomes, including dead ends.

Rules:

- Synthesize the per-author journals, request files, and coordination
  logs into the report; the report is the front door. Raw logs may be
  archived under the phase dir but should not be the primary read.
- **Remove all images** across `docs/journals/`; replace each with a
  short text description (reuse any existing alt text / caption).
- Refresh `docs/journals/README.md` so the phase index is complete and
  current (it presently omits phases 8-13) and points at each phase
  report.
- Follow the workspace writing rules: factual, no marketing, no em
  dashes, ASCII tables, claims verified. Written for a human reader.
- Do not invent history; where a fact (e.g. duration) is an estimate,
  say so.

## C1: frontend comments / docs / copy (closing wave, after A+B)

The round-2 `/architect` pass: review all frontend code comments, the
frontend-related docs, and the user-facing copy (UI strings, banners,
error/empty states) for clarity, factual accuracy, and reduced
ambiguity. One consistent voice across the four frontend trees, written
for human consumption, a snapshot of the present (no changelog, no
history). See `roadmap-round-2.md`.

## Worktree + branch

- C2 edits only `docs/journals/` and can be done in the canonical
  checkout by absolute path (no code build needed).
- C1 edits frontend code; do it after A+B land, in a worktree off the
  merged base:
  `git -C /Users/fiorix/dev/github.com/fiorix/chan worktree add ../chan-p14-lane-c -b phase-14-lane-c`.

## Gate

- C2: links resolve; no images remain under `docs/journals/`; the
  README index is complete.
- C1: the affected frontend gates stay green; copy matches what each
  surface actually does.
