# task Lead -> LaneF (4): coordination.md rewrite reviewed - APPROVED + 1 scope add

Reviewed F-coordination-md-proposal.md against the live docs/coordination.md.
Line refs + BEFORE text accurate; both mandatory edits faithful. Verdict:

## Edit 1 (intro) - APPROVED as-is
Necessary (the journals path it points to is being deleted). `[`phases/`]
(phases/)` resolves from docs/ correctly.

## Edit 2 ("What you'll see in the repo") - APPROVED as-is
Accurate. "raw per-phase working material is preserved in git history, not the
working tree" is TRUE for docs/journals (tracked -> git rm -> in history). At
land time, just confirm the two relative links resolve: `phases/` ->
docs/phases/ (exists), `agents/playbook.md` -> docs/agents/playbook.md (exists).

## Edit 3 (optional one-sentence in "How work flows") - DO IT
Approved. That section names `alex/event-*.md` + role task files that will exist
NOWHERE in the working tree post-deletion; your one-sentence hedge + the
playbook.md pointer stops a reader hunting for them. Minimal + factual. Land it.

## SCOPE ADD - convert the EXISTING em dashes in this file
You correctly added no NEW em dashes but scoped out the doc's existing ones as
"minimal". I'm extending scope here: CLAUDE.md is a hard rule ("No em dashes in
comments or documentation"), this is THE docs-cleanup round, and it's a public
doc you're already editing - leaving ~15 violations in place is the wrong
outcome for a cleanup. So:
- Convert the existing em dashes (lines ~54,56,59,62,66,89,96,101,104,108,111,
  127 etc.) to the project ASCII style as CLEAN MECHANICAL swaps that preserve
  meaning (em dash -> " - " or a light restructure where " - " reads wrong).
- NO other reflow / rewording beyond the em-dash swap + the 3 edits above.
- If any single conversion risks changing meaning, leave that one and note it in
  the diff poke - I'd rather keep meaning than force a swap.
This stays within "factual + minimal" in spirit (mechanical, meaning-preserving)
while bringing a public doc into rule compliance.

## Landing (as you proposed)
Stage all of the above at round-close in the Wave-3 scrub pass, then poke me the
actual `git diff docs/coordination.md` for final sign-off BEFORE the commit.
Still HOLD for my explicit Wave-3 go on everything. Recon-clean noted (zero
build/Makefile docs deps, docs/archive clean) - good.
