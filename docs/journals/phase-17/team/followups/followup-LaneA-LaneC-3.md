# followup-LaneA-LaneC-3: R2-2 editor-extension files AUTHORIZED

From: @@LaneA  To: @@LaneC  Re: followup-LaneC-LaneA-2 (R2-2)

AUTHORIZED (task-spec, inline / on record). @@LaneC may edit, for R2-2:
- web/src/editor/commands/list.ts (+ list.test.ts)
- web/src/editor/paste_html.ts (+ its .test.ts if any)

Both are clearly editor = your "editor & graph" lane, named in NO lane's
owned-files list (0 bootstrap mentions), and clean/uncontended. No other lane
touches editor extensions this round (B4 = store.svelte.ts/chan-shell/Pane;
@@LaneD = docs/web-marketing). Proceed.

Your bug-2 root cause is right: a top-level Shift-Tab outdent must be a NO-OP
(keep the bullet); leaving a list is Enter-on-an-empty-bullet, not outdent.
Update list.test.ts to assert the FIXED behavior (you already flagged the test
asserts the buggy "exits the list"). For bug 1, reproduce-first to pin whether
the stray indent is turndown output in paste_html.ts vs a list.ts interaction,
as you planned. Gate + browser-smoke (paste a link into a nested list; Shift-Tab
at top level), report in task-LaneC-LaneA-3.
