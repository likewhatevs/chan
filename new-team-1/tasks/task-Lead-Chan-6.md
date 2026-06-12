# task-Lead-Chan-6 — two review findings; fold into your straggler pass

From: @@Lead. To: @@Chan. Source: @@ChanGateway's review (full
receipts in new-team-1/tasks/task-ChanGateway-Lead-4.md — verdict on
all your web commits AND your guard: ACCEPT; these are the only
findings, both minor).

## F-W1: SERVE_LONG_ABOUT Rich Prompt row mislabels on Linux

The new "Show/Hide Rich Prompt — Cmd+Shift+P" row carries no note,
so under the table's "(Cmd = Ctrl on Linux / Windows)" header a
Linux user reads Ctrl+Shift+P — which the handler deliberately
ignores (physical-Cmd-only, as you registered it). The closeEmpty
row shows the generator renders `note:` fields: add one to the
terminal.richPrompt registry entry ("physical Cmd on every
platform" or similar) and regenerate via shortcuts-table.mjs. Same
mislabel class you fixed in the right-click menu — this is its
help-text sibling.

## F-W2: codemod scar in a test name

fileBrowserUploadDrop.test.ts:30 — "upload progress can workspace
status" is a drive→workspace codemod scar (was "...can drive
status..."). Rename to something that parses.

Both ride your task-5 own-gate (full vitest per your own lesson).
Note for morale: the reviewer probed your escaping with
newline-bearing filenames adversarially and found nothing — the
guard work held up under hostile review.
