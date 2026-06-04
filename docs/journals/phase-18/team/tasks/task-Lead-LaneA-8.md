# task Lead -> LaneA (8): HOLD the task-4 EOL commit - fold it INTO the task-7 cleanup

Your task-4 EOL fix is good work and the ROOT CAUSE is the key: a large negative
text-indent (hanging-indent) makes CM6 `posAtCoords` mis-resolve clicks on
deeply-indented rows, and the listCaretGuard prefix clamp then snaps to
text-start. You even tried a pure-geometry rewrite and it regressed near-start
clicks. THAT is exactly @@Alex's point - the list GEOMETRY is what makes
CM6 click-mapping fragile, so every case needs another guard branch.

## Reconcile with task-7 (read it if you haven't - it SUPERSEDES the snap approach)
task-Lead-LaneA-7 is @@Alex's direct steer: "cleanup not scaffolding." Your
task-4 fix ADDS another listCaretGuard branch (the EOL pin) - that is more
scaffolding, the opposite of the directive. Also note: task-4 only handles the
EOL (past-text) click; @@Alex's task-7 bug is clicking IN THE TEXT of a nested
bullet -> start, which your EOL branch does NOT fix.

## So: do NOT commit task-4 standalone. Fold it into the cleanup.
- Goal (task-7): real positioned markers + saner list geometry so CM6 resolves
  clicks NATIVELY, letting you REMOVE the listCaretGuard scaffolding (BOTH the
  prefix clamp AND your new EOL branch), not add to it. Your negative-text-indent
  root cause is the lever: if the hanging-indent geometry is what breaks
  posAtCoords, fixing/avoiding it is the cleanup.
- Keep your EOL diagnosis + tests as INPUT; the unified result should make
  click-in-text + EOL-click + arrow all work at depth 1 AND 2 for bullet/hyphen/
  ordered with MINIMAL (ideally zero) bullet-specific guard code.
- IF you find the CM6 posAtCoords quirk is genuinely unavoidable even with clean
  markers + saner geometry (i.e. the wrap hanging-indent is required and CM6
  just mis-resolves it), THEN a single minimal guard is justified - but STOP and
  FLAG that to me with the evidence; it becomes a @@Alex tradeoff (native-feel vs
  the wrap alignment), and I survey him. Do not silently keep the guards.

## Process
- Your list.ts/list.test.ts EOL work stays in your working tree as a starting
  point; continue from there into the cleanup. I commit the UNIFIED cleanup, not
  the standalone EOL branch.
- Report the unified result in a task-LaneA-Lead-5.md (what you REMOVED, the
  final geometry/marker approach, the depth-1/2 x 3-list-type click/arrow smoke
  matrix), poke me. This is the real close-out of the editor lane.
