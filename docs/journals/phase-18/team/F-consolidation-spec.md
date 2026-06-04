# Phase consolidation spec (LaneF, phase-18 round-1)

Shared brief for the per-phase consolidation subagents. Each subagent
owns exactly one `docs/phases/phase-N.md` and writes nothing else.

## Goal

Consolidate a phase's development journals into a single
`docs/phases/phase-N.md`: the phase's roadmap, rounds, waves, and
retrospective. Capture the ESSENCE so a NEW agent joining the project
learns from prior execution: what was asked, how the team coordinated,
what shipped, and (most important) the successes and mistakes worth
carrying forward. This is a distillation, NOT a raw dump and NOT a
verbatim copy.

The raw journals tree (`docs/journals/`) is being DELETED at the close
of this round. `docs/phases/phase-N.md` is its permanent replacement, so
it must stand on its own.

## Output template

Use this exact section order. Drop a section only if it genuinely does
not apply (e.g. a sparse phase); never pad.

```
# Phase N - <short title>

Status: <closed | sparse | ...>
Span: <dates, estimate basis if uncertain>
Versions: <vX.Y.Z cut this phase, if any; else "none">
Tags: <#hashtags carried from the source>

## Roadmap (the asks)

What @@Alex requested, in essence. Group by area. For multi-round
phases, note which round each ask belongs to.

## Rounds and waves

How the work was structured. Single-round phases: say so in a line or
two. Multi-round phases: one short subsection per round (the ask, the
wave breakdown, the version cut). Name the concrete outcomes, not the
chatter.

## Team and coordination

Who worked it (handles/roles) and the coordination SCHEME that phase ran
on (flat task files / per-author journals + event channels / per-lane
worktrees / isolated gate worktree / cs-terminal team bus). The scheme
evolved over the project; record what THIS phase used. Reference the
agent roster as `../agents/README.md` and use `@@handle` in prose. Do
NOT deep-link individual agent cards (some are being trimmed).

## What shipped, tried, and undone

Shipped (the durable outcomes, with version tags where known). Tried
then corrected (the course-changes). Deliberately not done / deferred
(recorded risks).

## Retrospective

The learning payload. Highlights, lowlights / contention, and
constructive feedback / lessons. Keep the lessons that generalize: the
ones a future agent should internalize. This is the highest-value
section; do not compress it to nothing.

## Notes

Terminology drift (e.g. chan-drive -> chan-workspace, folder ->
directory, Rich Prompt -> Team Work), and a one-line pointer that the
raw working material lives in git history under
`docs/journals/phase-N/`.
```

## Hard rules

- Text only. NO embedded images. If a screenshot was load-bearing,
  describe what it showed in one bracketed sentence (the existing
  journals already do this). Do not copy `.png` files.
- NO links into `docs/journals/**` (that tree is being deleted). Point
  to git history in prose instead. Agent references go to
  `../agents/README.md` only.
- Writing rules (project CLAUDE.md): NO em dashes anywhere. Tables are
  pure ASCII, target 80 columns. Factual, no marketing language.
  Explain WHY, not just WHAT.
- Preserve the `Tags:` hashtags and any terminology-drift notes from the
  source; a new reader needs them to map old names to current ones.
- Length: a worked phase lands roughly 120-260 lines. A sparse phase
  (phase 4) stays short and says why. Quality of the retrospective beats
  length.

## Self-identify

Your prompt names your phase number N and your source. Write ONLY
`docs/phases/phase-N.md`. Do not touch any other file. Report back the
path you wrote, its line count, and a 2-3 line summary of what you
captured.
