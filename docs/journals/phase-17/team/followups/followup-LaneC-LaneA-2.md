# followup-LaneC-LaneA-2: R2-2 needs auth for 2 editor-extension files

From: @@LaneC (phase-17)  To: @@LaneA  Re: task-LaneA-LaneC-3 (R2-2)

Viewed image-1/image-2. Two symptoms:
- image-1: pasting a link into a bullet list adds an EXTRA indent level (the
  pasted item renders as a nested bullet).
- image-2: cmd+shift+tab (outdent) on a top-level item STRIPS THE BULLET
  entirely (item drops to col 0, no longer a list item) - "makes it worse".

## Root cause (confirmed by Read) + the lane-boundary ask

The fix lives in editor-extension files that are NOT in my owned list
(blocks.ts / Wysiwyg.svelte). Per the task's STOP rule, requesting auth (you
said "I'll authorize if it's clearly editor" - both are web/src/editor/):

1. **web/src/editor/commands/list.ts** - BUG 2 confirmed here. `shiftListLines`
   outdent branch (lines 147-163): if the line has leading spaces it strips up
   to 2; ELSE (a top-level item, no indent) it strips the ENTIRE prefix
   (marker + spaces + task box), ejecting the item from the list. That is the
   image-2 "lost bullet". Fix: a top-level outdent should be a NO-OP (keep the
   bullet); exiting a list is Enter-on-blank-bullet, not Shift-Tab.
   NOTE: web/src/editor/commands/list.test.ts currently ASSERTS the buggy
   "exits the list" behavior (~lines 141-145) - I'll update it to the fixed
   behavior.

2. **web/src/editor/paste_html.ts** - BUG 1 likely here. Copying a URL from a
   browser puts text/html on the clipboard (an <a> tag), so htmlPasteHandler
   runs turndown + inserts. The handler itself just dispatches the converted
   md, so the extra indent must come from the turndown output (list/indent
   structure or leading whitespace) or a list-line interaction. I will
   REPRODUCE empirically (paste a real link into a nested list) to pin the
   exact mechanism before editing; the fix may be in paste_html.ts (strip a
   stray leading indent/marker from a single-link paste) and/or list.ts.

The Tab/Shift-Tab keymap WIRING is in Wysiwyg.svelte (already mine); the
COMMANDS are in commands/list.ts. No other lane lists these files; both clean.

Ask: authorize web/src/editor/commands/list.ts + web/src/editor/paste_html.ts
(+ their .test.ts) for R2-2. I'll reproduce-first, fix both, gate + browser-
smoke, and report in task-LaneC-LaneA-3.
