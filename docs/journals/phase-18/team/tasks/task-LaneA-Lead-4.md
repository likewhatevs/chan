# task LaneA -> Lead (4): BUG fix - click at EOL of nested bullet -> line START

Fixed + re-smoked. Editor-domain, append commit (list.ts + list.test.ts only).

## Root cause (reproduced + probed in Chrome, not trusted to hypothesis)
List lines carry a large negative `text-indent` (the hanging-indent for wrap
alignment). On a deeply-indented row, CodeMirror's `posAtCoords` mis-resolves a
far-right (EOL blank-space) click INTO the marker PREFIX. The pre-existing
`listCaretGuard` prefix clamp then snapped that to the text START - the
"beginning of the line" cursor @@Alex saw. Depth-0's smaller indent doesn't
mis-resolve, so it was correct.
- Confirmed it's CM6 geometry, not the DOM: the browser's `caretRangeFromPoint`
  at the EOL point returns the correct text-END; CM6's `posAtCoords` is what's
  off under the negative text-indent.
- NOT my item-1 caret-snap: that (`listAwareArrowDown/Up`) is keyboard-only.
  The click snap is the pre-existing `listCaretGuard`, which happens to use the
  same `clampListCaretPosition`. So @@Alex's bug was a latent click-path issue
  the clamp masked into "text-start", surfaced on nested rows.

## Fix (web/src/editor/commands/list.ts)
- Prepend an EOL branch to `listCaretGuard`: resolve the row via the nearest
  position, and when the pointer sits in the trailing blank space past the
  text end (clickX > `coordsAtPos(line.to)`.right, on that row), pin the caret
  to `line.to`. Runs BEFORE the prefix clamp so a mis-resolved EOL click can't
  be snapped to text-start.
- Kept the original precise prefix clamp untouched (marker / indent / near-
  start clicks still go to text-start). I first tried a pure-geometry rewrite
  but it regressed near-start clicks (CM6 mapped them into the marker); the
  prepend-branch hybrid keeps both behaviors.
- Extracted pure `isListEolClick(...)` for unit testing; added single-click /
  modifier / gutter guards so shift-extend, word-select and the fold chevron
  fall through.

## Gate + smoke
- Own-gate GREEN: svelte-check 0 ERRORS (1 WARNING = pre-existing RichPrompt
  a11y, not mine); vitest 68/68 across my 4 editor test files
  (blocks/list/wikiLinkTargets/external_links); npm run build OK.
- Browser smoke (fresh binary, this is a runtime pointer-mapping bug):
  - Synthetic EOL sweep: bullet / hyphen / ordered x depth 0 / 1 / 2 (8 cases)
    ALL land at LINE END.
  - REAL computer clicks: nested-bullet EOL -> offset 45 (line END; was 33 =
    start), marker-zone -> offset 33 (text START, preserved). Both confirmed.

## Pathspec (append commit)
- base HEAD: 2e372a93
- files: web/src/editor/commands/list.ts, web/src/editor/commands/list.test.ts
- `git diff -- <those 2> | git hash-object --stdin` = f20252fe9c8f9414f6101fb010a54613b9ba82a2
- The other dirty files in the tree (docs/coordination.md, GraphPanel.svelte)
  are peers' WIP, not mine.

Journal: docs/journals/phase-18/team/journals/journal-LaneA.md
Bug fixed, gate-green, smoked at depth 1 AND 2 across bullet/hyphen/ordered.
Ready for your append commit. @@Alex may surface more while poking - queue them.
