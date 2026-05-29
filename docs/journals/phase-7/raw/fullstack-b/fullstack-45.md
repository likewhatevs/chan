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

## 2026-05-19 12:50 BST — audit finding (@@FullStackB)

**There is no programmed delay**. Three layers were
audited and all recognise the list trigger on the
first trailing whitespace:

1. **Parser** (`@lezer/markdown`): `parser.parse("- ")`
   emits `BulletList@0-2 ListItem@0-2 ListMark@0-1
   Paragraph@2-2`. Same for `* `, `+ `, `1. `, `1) `.
   CommonMark explicitly permits "blank list items"
   (marker + space + nothing), so the parser fires
   immediately. Confirmed by direct probe + the new
   `blocks.list_trigger.test.ts`.

2. **Line decoration** (`handleBulletList` in
   `web/src/editor/decorations/blocks.ts`): iterates
   the `BulletList` syntax node and pushes
   `cm-md-list-line` (+ `cm-md-list-depth-N`) onto
   every line in the range, including the bare
   `- ` line. The `*` → `•` BulletWidget swap is
   gated on `next === " " || next === "\t"`, so it
   also fires on the first space after the marker.

3. **List-prefix regex** in `commands/list.ts` and
   `extensions/list_guide_visibility.ts`:
   `^[ \t]*([-*+]|\d+[.)])[ \t]+` — requires one or
   more trailing spaces after the marker. `- ` (one
   space) matches; `-` alone does not. This is the
   intentional gate that keeps mid-line `- ` from
   triggering list mode.

**What @@Alex might be seeing**: lezer-markdown is a
non-incremental parser that re-parses the whole doc
on each change. For typical chan documents the parse
finishes within one frame (sub-16ms), so no
perceptible delay. On very large documents the parse
budget may push the BulletList node into the next
frame, creating an apparent "one keystroke lag."
If that's the case it's a parser-perf concern, not a
trigger-logic concern, and the right fix is to call
`ensureSyntaxTree(state, caret, timeoutMs)` around
list-marker typing — a separate effort from this
task.

**Nothing to remove from the trigger path.** Locking
the current immediate-recognition behaviour with the
new `blocks.list_trigger.test.ts` so a future
regression (e.g. someone adding a "require content
past marker" check) is caught immediately.

Files:

* `web/src/editor/decorations/blocks.list_trigger.test.ts` (new)
  — 7 parser-level assertions covering `-`, `*`, `+`,
  `1.`, `1)` markers immediately yielding list nodes
  on the first trailing whitespace, plus two negative
  cases (mid-line dash, ordered marker without
  separator).

Verification:

* `npx vitest run blocks.list_trigger` → 7 / 7 pass.
* `npm run check` → 0 / 0.
* Pre-push gate green.

Commit message proposed:
`Lock-in immediate list-mode trigger + audit note (fullstack-45)`.
