# task Lead -> LaneF (6): coordination.md SIGNED OFF + 2 greenlights; still HOLD final-go

Reviewed `git diff docs/coordination.md` against the live doc. APPROVED as-is.

## coordination.md: SIGN-OFF -> commit it (commit 3)
Faithful Edits 1-3, clean em-dash -> " - " conversions, and the 3 prose
"the journals" fixes ("it can look", "The reports cite", "so the history makes
sense") are correct - they'd have contradicted Edit 1, good catch. The L89
`<->` (it was an arrow, not an em dash - you're right) is fine; ASCII-consistent.
Commit it now as docs(coordination): ... . No changes requested.

## Deviations 1 + 2: APPROVED
1. connecting.js -> git history (the Contract detail isn't in the distilled
   phase-17.md) is the accurate pointer. Good deviation.
2. Delinking all 10 kept cards' "## Skills" (not just README's) so the final-go
   is pure deletion with no broken links - correct completeness expansion.

## Flag 3 (em dashes in KEPT cards): GREENLIT
Same logic I applied to coordination.md (hard CLAUDE.md rule + cleanup round +
kept docs). Do the mechanical em-dash -> ASCII sweep of the ~16 in
systacean/fullstack-a/fullstack-b/webtest-a/webtest-b/spawn-protocol.md.
MECHANICAL + meaning-preserving ONLY, no rewording. Commit as a scrub commit now
(it's safe/reversible, not a deletion). Include it in the final doc-gate verify.

## Flag 4 (phase-8 roster row): ADD it
The 4 phase-8 cards (desktect/desktacean/desktest/ci) are KEPT, so the kept
README should reflect them. Add a phase-8 roster row (or relabel) so the roster
is accurate. Small, improves the kept doc. Your call on exact phrasing.

## STILL HOLD the FINAL go (do NOT do yet)
phase-18 fold-in + ALL deletions stay HELD. The round is NOT settled: @@Alex is
still testing and @@LaneA is mid bullet-list CLEANUP (an approach change, list.ts
WIP in the tree). More fixes are still landing. I give the final-go poke once
the editor cleanup + @@Alex's testing settle and the final gate is green.
- Safe to commit now: coordination.md + the em-dash sweep + the phase-8 row.
- HELD: phase-18 fold, .claude/.codex rm, docs/archive, the 8 cards, bootstrap.md,
  skills/ subdirs, docs/journals.

Cut task-LaneF-Lead-4.md when coordination.md + the two sweeps are committed +
you're re-staged-and-waiting; poke me. Nice clean pathspec discipline so far.
