# task Lead -> LaneF (2): Wave-1 accepted + 3 decisions

Strong Wave-1 work - 18 consolidated phase docs in one template, the playbook,
and a clean keep/cut + scrub plan with nothing deleted. Accepted. Decisions:

## #3 orchestration/ -> KEEP (confirmed)
Keep orchestration/ (README + atomic-writes + mcp-discovery + spawn-protocol).
It is the chan integration-contracts set and the deliberate blueprint for the
planned watcher-driven automation - load-bearing reference for new agents.
Decisive; no escalation.

## #2 pub-site-release -> KEEP as its own doc under docs/phases/ (confirmed)
Leave it where you put it. It is real, deletable work that must survive the
Wave-3 journals deletion, and an own-doc preserves its essence cleanly. Link it
from docs/phases/README.md (you already do). Decisive.

## #1 per-agent skills/ subdirs -> provisional CUT, pending an @@Alex confirm
I agree with your lean (cut: canonical source is dotfiles; not "learn from past
phases" material; aligns with @@Alex's "minimum set" intent). BUT it reverses a
prior self-containment decision, so I'm escalating it to @@Alex rather than
owning the reversal. It's a Wave-3 concern and does NOT block you, so I'm
BATCHING it into the survey I'll raise when Wave-1 frontend lands (sequenced
with the Wave-2 smoke-client question), not firing a tiny standalone survey now.
- PLAN on cut (stage the "## Skills" link scrubs in your Wave-3 scrub list), but
  do NOT delete the skills/ subdirs until I relay @@Alex's confirm at Wave 3.
- If @@Alex says keep, you drop that one scrub; everything else in your Wave-3
  plan is already approved.

## Everything else in your keep/cut + scrub plan: APPROVED
- Cut (Wave 3): the 8 redirect-only cards + the stale bootstrap.md snapshot.
- Scrub (Wave 3): desktect.md journal links, README "Historical handles" map,
  + chan-workspace/embeddings.rs, chan-server/routes/graph.rs, pages.yml path
  mentions. Good catch folding bootstrap.md's durable essence into playbook.md.

## Hold
Continue holding for my explicit Wave-3 go before ANY rm/git rm. Wave 3 also
folds phase-17 + phase-18 into docs/phases AFTER I confirm the round is
committed (phase-18 is the live bus + gate worktree depends on it). I'll poke
you to start Wave 3. Nothing for you to do until then.
