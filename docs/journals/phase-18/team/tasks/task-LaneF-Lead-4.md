# task LaneF -> Lead (4): coordination.md + 2 sweeps committed; re-staged-waiting

All safe/reversible Wave-3 work is committed. Re-staged-and-waiting for your
FINAL go on the phase-18 fold-in + ALL deletions.

## Committed this pass (pathspec-clean, on main, NOT pushed)

- d5886380  docs(coordination): rewrite for the docs/phases layout + ASCII
  typography (your sign-off; commit 3).
- 948faed1  docs(agents): ASCII em-dash sweep of kept cards + phase-8 roster
  row (flags 3 + 4, greenlit). Em-dash sweep was mechanical " - " only, no
  rewording. Added a "Desktop + CI lanes (phase 8)" roster section for the
  kept desktect/desktacean/desktest/ci cards.

Full Wave-3-so-far set (4 commits):
  74909e64 consolidation (docs/phases + playbook)
  2e372a93 scrubs + skills delink
  d5886380 coordination.md
  948faed1 em-dash sweep + phase-8 roster row

Each verified with git show --stat. A peer commit (0408db30, @@LaneB graph)
interleaved in the log; my commits are intact and isolated.

## Verify

- Doc gate GREEN: 0 em dashes and 0 docs/journals markdown links across all
  KEPT docs (docs/phases/*, coordination.md, playbook.md, the kept agent
  cards, orchestration/*).
- cargo check -p chan-workspace -p chan-server was green after the .rs
  comment edits (last pass).
- Working tree clean of my files. Only @@LaneA's bullet-cleanup WIP
  (list.ts, list.test.ts) and the live docs/journals/phase-18 bus remain
  uncommitted; I never staged them.

## HELD for your FINAL go (ready, one pass when you poke)

- phase-18 fold-in -> docs/phases/phase-18.md (from distilled essence) +
  README index entry, AFTER you confirm the round is committed.
- Deletions in order: .claude + .codex (rm -rf, untracked), docs/archive
  (git rm), the 8 redirect cards (git rm), bootstrap.md (git rm), skills/
  subdirs (git rm), docs/journals LAST (git rm; removes the team bus + my
  working briefs F-*.md).
- Final verify: re-grep tree for docs/journals + cut-card links (expect zero
  outside git history + the intentional graph.rs synthetic test fixtures);
  confirm docs/phases/README.md links resolve; doc gate.

Nothing else for me until your final-go poke.
