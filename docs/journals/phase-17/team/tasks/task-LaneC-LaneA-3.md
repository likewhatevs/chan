# task-LaneC-LaneA-3: R2-2 list paste-link indent bug - DONE

From: @@LaneC (phase-17)  To: @@LaneA  Re: task-LaneA-LaneC-3 (R2-2)

Both bugs fixed, gate GREEN. Edited only the 2 authorized editor-extension
files (+ their tests). No commit/push (your round-close call).

## Repro + root cause

- **Bug 2 (cmd+shift+tab "makes it worse", image-2):** confirmed by Read.
  web/src/editor/commands/list.ts `shiftListLines` outdent branch: when a line
  has leading spaces it strips up to 2; ELSE (a top-level item, no indent) it
  stripped the ENTIRE prefix (marker + spaces + task box), ejecting the item
  from the list (the "lost bullet" -> col 0).
- **Bug 1 (pasting a link indents the list, image-1):** root cause pinned
  EMPIRICALLY with an htmlToMarkdown() probe (browser nav to the test server
  was permission-denied, so I verified the conversion step directly). Copying a
  link out of a list / web page puts text/html `<ul><li><a>...` on the
  clipboard; turndown converts it to `"-   [url](url)"` (a leading bullet
  marker + 3 spaces). paste_html.ts htmlPasteHandler inserts that verbatim, so
  pasting onto an existing `- ` bullet yields `- -   [url]` -> a stray NESTED
  bullet. A bare anchor converts to inline `[url](url)` (no marker) - no indent,
  which is why only the list-item-link case bites.

## Fix

- **Bug 2 - list.ts:** a top-level Shift-Tab outdent is now a NO-OP (skip the
  prefix-strip when there are no leading spaces). It still returns true so
  Shift-Tab never escapes the editor; the bullet is preserved. Leaving a list
  stays Enter-on-an-empty-bullet (unchanged). Indented items still strip one
  level (unchanged).
- **Bug 1 - paste_html.ts:** new `dedentListPaste(state, pos, md)` - when the
  paste target line IS a list item, strip a leading list marker from the FIRST
  pasted line (via parseListPrefix imported from commands/list) so the content
  flows into the current bullet as a SIBLING, not a nested item. First line
  only, so a genuine multi-item paste keeps its later bullets; a bare-anchor
  paste (no marker) and a paste into a non-list line pass through unchanged.

## Tests

- list.test.ts: flipped the 2 tests that ASSERTED the buggy "exits the list"
  behavior to assert the bullet is preserved (no-op). These mount a REAL
  EditorView in jsdom + dispatch the command, so bug 2 is runtime-verified, not
  just regex.
- paste_html.test.ts: 5 new `dedentListPaste` unit tests (strip into bullet /
  into task item / first-line-only / bare-anchor passthrough / non-list-line
  passthrough).

## Own-gate: GREEN

make web-check: svelte-check 0 ERRORS (1 pre-existing a11y WARNING in
RichPrompt.svelte = @@LaneB's, not mine - the earlier mcpEnv errors are gone,
your fix landed); vitest 1661 passed / 168 files; build OK. Scoped vitest on my
2 test files: 38 passed.

## Empirical-verification note (please read)

Live in-browser smoke was BLOCKED: the browser nav to my throwaway test server
was permission-denied, and per harness guidance I did not retry it. Mitigation:
both fixes are DETERMINISTIC CM6 transforms (not Svelte $state reactivity, so
the "static gate misses runtime" risk that bit B6/B9 does not apply here); bug 2
is verified through a real EditorView in jsdom; bug 1's transform is unit-tested
and its root cause empirically confirmed via the turndown probe. Residual risk
is the DOM paste-event -> handler wiring (simple, unchanged except the dedent of
the insert string). Suggest a 30-second manual confirm by @@Alex when convenient:
paste a copied link into a nested list (no extra indent) + Shift-Tab a top-level
bullet (stays a bullet). I'll fix immediately if it misbehaves.

## Pathspec shas (uncommitted WIP; HEAD 03bb91f8)

  cdb4ae0f...  web/src/editor/commands/list.ts
  5296e594...  web/src/editor/commands/list.test.ts
  fa7b618f...  web/src/editor/paste_html.ts
  98158296...  web/src/editor/paste_html.test.ts

Ready for the round-2 commit on your signal.
