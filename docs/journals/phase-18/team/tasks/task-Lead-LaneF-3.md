# task Lead -> LaneF (3): Wave-3 plan RATIFIED + guardrails

Good catch - the first-pass scrub list was incomplete and your hold-time recon
found the 5 misses (shipping code comments + the public coordination.md) before
they could become dead links post-deletion. Complete plan RATIFIED. Still HOLD
for my explicit Wave-3 go (after all lanes land + I gate + commit the code).

## Approved as written
- A. Fold-in 17/18 -> docs/phases + README entries (only AFTER I confirm the
  round is committed; phase-18 from distilled essence, not the raw bus).
- B. All scrubs 1-10 approved. The .rs handling is right: repoint only the
  ILLUSTRATIVE doc-comments in graph.rs, LEAVE the synthetic test-fixture
  literals (L2265-2282, L2970-2981) untouched - those build an in-tempdir fake
  workspace, editing them risks the tests.
- C. Deletions as ordered, docs/journals LAST.
- D. Final verify incl. cargo check -p chan-workspace -p chan-server after the
  .rs comment edits.

## Three guardrails
1. connecting.js (item 6) + the desktop tree: comment-only is fine, but do it
   in Wave 3 AFTER @@LaneE's desktop work is committed - never touch the desktop
   tree while E is mid-flight. (You're holding for Wave 3 anyway, so automatic;
   just don't jump it.)
2. CHANGELOG.md (item 7): scrub ONLY the stale "history is kept in docs/journals"
   line -> docs/phases. Do NOT add/alter the version section; I add the v0.26.0
   entry separately at round-close. Keep your edit to the one stale line.
3. coordination.md (item 11, PUBLIC + a content rewrite, not a path swap):
   APPROVED - it's a required consequence of the approved docs/journals deletion
   (leaving it stale = a public doc lying about repo layout). Keep it factual +
   minimal as you propose. BUT: stage it and poke me the DIFF before the
   round-close commit - I want to eyeball the one content rewrite (public-facing)
   before it lands. The other scrubs are mechanical; this one I review.

## skills/ (item 5 "## Skills" part): still provisional-cut pending @@Alex
Unchanged from task-Lead-LaneF-2.md. I batch it into the next survey; if @@Alex
says keep, you drop that scrub sub-item.

## Hold
No execution until my Wave-3 go poke. Keep doing read-only recon / staging if
useful; flag anything else you find. Nice work.
