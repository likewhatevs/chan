# task LaneF -> Lead (3): Wave-3 PARTIAL done; staged-and-waiting on deletions

The safe/reversible Wave-3 work is committed. HOLDING for your FINAL go on
the phase-18 fold-in + ALL deletions, and for your coordination.md sign-off
before I commit that one.

## Committed (pathspec-clean, on main, NOT pushed)

- 74909e64  docs(phases): consolidate journals into docs/phases + playbook
  (docs/phases/* = phase-1..17 + pub-site-release + README, plus
  docs/agents/playbook.md). 20 files. phase-17.md = 294 lines, synthesized
  by a subagent (it reconstructed the v0.25.0 wave-2 from git log since the
  retrospective only covered the early SPA work).
- 2e372a93  docs: repoint stale docs/journals references; delink agent skills
  (17 files: embeddings.rs, graph.rs, pages.yml, connecting.js, CHANGELOG.md
  one line, docs/agents README + 9 cards + desktect + atomic-writes).

Verified each with git show --stat. EXCLUDED from both: docs/journals/ (held),
docs/coordination.md (sign-off pending), and @@LaneA's in-flight WIP
(GraphPanel.svelte, list.ts, list.test.ts) - never staged them.
cargo check -p chan-workspace -p chan-server = GREEN (the .rs edits are
comment-only).

## coordination.md - staged, awaiting your sign-off (guardrail 3)

Edited + staged (uncommitted). Diff + a changelog of exactly what changed is
in docs/journals/phase-18/team/F-coordination-md-diff.md; live view is
`git diff docs/coordination.md`. On your go I commit it as commit 3
(docs(coordination): ...). Flagged in that file: I also fixed 3 prose
"the journals" refs that would contradict Edit 1, and converted the L89 `↔`
(you listed line 89; it was an arrow, not an em dash) to `<->`. Revert any
before commit if you disagree.

## Deviations + completeness expansions to note

1. connecting.js (scrub item 6): you said -> docs/phases/phase-17.md, but the
   distilled phase-17.md has NO "Contract" section (that was raw round-2
   detail). I pointed it to git history instead (accurate). Flagging the
   deviation.
2. Per-card skills delink: the enumerated item 5 named only README's "##
   Skills". Since skills/ cut is confirmed, I delinked ALL 10 kept cards'
   "## Skills" sections too (kept the descriptions, dropped the dead links),
   so the final-go is pure deletion with no leftover broken links. Same kind
   of completeness expansion as the journal-scrub list.

## Two flags (your call; not blocking)

3. Em dashes in KEPT docs/agents cards: ~16 remain, OUT of your
   coordination.md-only em-dash scope: systacean.md 3, fullstack-a.md 4,
   fullstack-b.md 3, webtest-a.md 1, webtest-b.md 1, orchestration/
   spawn-protocol.md 4. Same hard-rule + same docs-cleanup-round logic you
   applied to coordination.md. I can fold a mechanical sweep into the
   final-go pass if you want kept docs rule-compliant. Say the word.
4. docs/agents/README.md "Active roster (phase 7)" lists only the 6
   chan-core agents, not the 4 phase-8 cards I kept (desktect/desktacean/
   desktest/ci). Left as-is (it is a labeled phase-7 snapshot). Optional:
   add a phase-8 roster row. Flagging.

## HOLDING for your FINAL go (staged, ready, will do in one pass)

- phase-18 fold-in -> docs/phases/phase-18.md (from distilled essence) +
  README index entry, AFTER you confirm the round is committed.
- coordination.md commit (after your sign-off).
- Deletions in order: .claude + .codex (rm -rf, untracked), docs/archive
  (git rm), the 8 redirect cards (git rm), bootstrap.md (git rm), skills/
  subdirs (git rm), docs/journals LAST (git rm; also removes the team bus +
  my working briefs).
- (optional, if you greenlight #3) em-dash sweep of the kept cards.
- Final verify: re-grep tree for docs/journals + cut-card links (expect zero
  outside git history + the intentional graph.rs test fixtures); confirm
  docs/phases/README.md links resolve; doc gate.

Nothing else for me until your final-go poke.
