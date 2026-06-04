# task LaneF -> Lead (2): complete Wave-3 execution plan

Decisions from task-Lead-LaneF-2.md logged (pub-site + orchestration KEEP;
skills/ provisional-cut pending @@Alex; rest approved). Still HOLDING for
your explicit Wave-3 go.

Did read-only Wave-3 recon while holding. Heads-up: the scrub list you
APPROVED in task-LaneF-Lead-1.md was INCOMPLETE. A full-tree sweep found 5
more stale refs to the to-be-deleted docs/journals tree, including in
shipping CODE comments and the PUBLIC coordination.md. This is the
complete, corrected plan; nothing here is executed until your go.

## A. Fold in (only AFTER you confirm the round is committed)

- phase-17 -> docs/phases/phase-17.md (closed phase; same subagent
  synthesis I used for 15/16, source docs/journals/phase-17/).
- phase-18 -> docs/phases/phase-18.md (from the round's DISTILLED essence,
  not the raw bus; written at close).
- Add phase-17 + phase-18 entries to docs/phases/README.md (it already
  says "Phases 17 and 18 fold in when phase 18 closes").

## B. Scrubs (edit KEPT files; cut cards/bootstrap.md vanish so need no scrub)

APPROVED already:
1. crates/chan-workspace/src/index/embeddings.rs L19,L360:
   docs/journals/phase-11/gpu-embed-followup.md -> docs/phases/phase-11.md
2. crates/chan-server/src/routes/graph.rs: repoint the illustrative
   doc-COMMENTS (L552-553, L1605, L2301-2302, L2963-2966) that cite real
   docs/journals example paths. LEAVE the synthetic TEST-FIXTURE literals
   (L2265-2282, L2970-2981); they build an in-tempdir fake workspace, not
   refs to the real tree, and editing them risks the tests.
3. .github/workflows/pages.yml L10: docs/journals/phase-14/addendum-1.md
   example -> docs/phases/ or generic.
4. docs/agents/desktect.md L21,L47,L53: ../journals/phase-8/README.md ->
   ../phases/phase-8.md. (git-history raw/ prose pointers stay.)
5. docs/agents/README.md "Historical handles" table L33-40: links the 8
   cut cards -> convert to a link-free map. "## Skills" section: per the
   @@Alex skills decision.

NEW from recon (were NOT in the approved list):
6. desktop/src/connecting.js L14: docs/journals/phase-17/round-2/
   desktop-connecting-screen.md -> docs/phases/phase-17.md.
7. CHANGELOG.md L23: "that history is kept in `docs/journals`" ->
   docs/phases.
8. docs/agents/orchestration/atomic-writes.md L93 (KEPT file):
   docs/journals/phase-7/architect/journal.md example -> generalize.
9. docs/agents/README.md L55: "crawl `docs/journals/**` for @@{name}" ->
   docs/phases/** (or docs/agents/**).
10. docs/agents/desktect.md L56: drop the bootstrap.md link (bootstrap is
    cut).
11. docs/coordination.md L6,L101-108 (PUBLIC doc): describes the OLD
    journals layout (docs/journals/phase-N/, alex/event-*.md, role task
    files). Needs a small section rewrite of "What you'll see in the repo"
    to point at docs/phases/ + docs/agents/playbook.md and the current
    scheme. Flagging because it is a content edit, not a path swap; I will
    keep it factual and minimal.

## C. Deletions (rm -rf untracked / git rm tracked) - only after go + commit

- rm -rf .claude, .codex (untracked)
- git rm docs/archive
- git rm the 8 redirect cards (backend, frontend, webdev, fullstack,
  syseng, rustacean, backsystacean, webtest .md)
- git rm docs/agents/bootstrap.md
- skills/ subdirs: ONLY if @@Alex confirms cut (provisional). If keep, I
  drop scrub-item 5's "## Skills" part.
- git rm docs/journals LAST (after 17/18 folded + round committed). This
  also removes my working brief
  docs/journals/phase-18/team/F-consolidation-spec.md.

## D. Final verify

- Re-grep tree for `docs/journals` / `../journals` / links to cut cards:
  expect zero outside git history.
- Confirm every docs/phases/README.md link resolves (incl. 17/18).
- Doc gate: em-dash + line-width sweep on new/edited docs.
- cargo check -p chan-workspace -p chan-server after the .rs comment
  edits; desktop build sanity after connecting.js (comment-only, but
  cheap to confirm).

Holding for your go.
