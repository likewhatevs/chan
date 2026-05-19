# fullstack-45: editor list mode triggers on first "- "

Owner: @@FullStackB
Cut by: @@Architect
Date: 2026-05-19

## Goal

When the user types `- ` (dash + space) at the start
of a line, the editor enters list mode immediately.
Today it waits for one more character before
recognizing the list. One less keystroke per list.

@@Alex 2026-05-19 12:45 BST: "if there's no reason,
let's change to how i'm describing now". Audit the
existing trigger first — if there's a real reason for
the delay (e.g. to disambiguate from non-list dash
usage), surface it back; otherwise drop the delay.

## Acceptance criteria

* Typing `- ` at the start of a line (or at the start
  of a list-item continuation) triggers list-mode
  rendering immediately, before any further keystroke.
* `* ` and `+ ` (other markdown bullet syntaxes) get
  the same treatment for consistency, if they're
  currently delayed similarly.
* Ordered lists: `1. ` should also trigger
  immediately if the same delay was present.
* No regression: typing `- ` mid-line (not at line
  start) does NOT trigger list mode.
* Existing list-command behaviors (continue on Enter,
  outdent on Shift-Tab, etc.) keep working.

## Audit before fixing

* Why was there a delay? Check git history for the
  list-trigger logic in
  `web/src/editor/commands/list.ts` (or sibling).
  If the delay is intentional (debounce against
  paste, disambiguate against em-dash, etc.),
  surface that finding back instead of just
  removing.

## Out of scope

* Indent / outdent flow (already correct).
* Image-paste-in-list (already fixed in
  `fullstack-4`).
* Task list (`[ ]`) shortcuts.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-b-architect.md`.
