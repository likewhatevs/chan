# task-LaneA-LaneC-3: R2-2 - list paste-link indent bug

From: @@LaneA  To: @@LaneC  Wave: round-2

Round-1 done - excellent (B2/B6/B9 all smoked). Round-2 now.

## @@Alex's bug (round-2/draft.md + images)

"pasting link indents the list" (docs/journals/phase-17/round-2/image-1.png) and
"cmds+shift+tab makes it worse" (image-2.png). Pasting a URL/link into a list
item wrongly INDENTS the list; cmd+shift+tab (outdent) then makes it worse
instead of fixing it.

VIEW image-1.png + image-2.png before starting.

## Scope

Editor list + paste handling - your "editor & graph" lane. Likely the editor's
clipboard/paste path (a CodeMirror paste handler / the list-aware markdown
input logic) and/or the indent/outdent (Tab / Shift-Tab) handling for list
items. Reproduce first (paste a link into a nested list; then shift+tab), then
fix:
  1. Pasting a link must NOT change the list indentation.
  2. Shift+Tab must outdent correctly (not deepen the corruption).

Files: start in your owned editor set (blocks.ts / Wysiwyg.svelte) + wherever the
paste + Tab/Shift-Tab list handling lives. If the fix pulls you into a file
NOT in your owned list (e.g. a separate paste/keymap extension), STOP and route
through @@LaneA (like B9/GraphCanvas) - I'll authorize if it's clearly editor.

## Gate

- make web-check + svelte-check + npm run build.
- Browser-smoke (Svelte-5 reactivity is runtime-only): paste a link into a list
  item at a few nesting levels -> indentation unchanged; shift+tab outdents
  cleanly. rust-embed: npm build before cargo build; smoke the served bundle.

## Report

Cut task-LaneC-LaneA-3 (repro + root cause + fix + own-gate-green + pathspec
shas) + poke @@LaneA.
