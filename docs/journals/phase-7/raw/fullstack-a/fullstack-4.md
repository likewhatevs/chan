# fullstack-4: list + image interaction bugs

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-18

## Goal

Fix three editor bugs around list editing and image paste.
None of these block other work; they're hot keystrokes
landing on every author every day.

## Relevant links

* [../request.md](../request.md) Bugfixes (shift-tab bullet,
  image-paste bullet, "typing on a list" bullet).
* Repro images: `../image-8.png`, `../image-9.png` (cursor
  jumping before list marker).

## Acceptance criteria

### B1: shift-tab outside a list

* Pressing Shift+Tab when the cursor is NOT inside a list
  must be a no-op (or the editor-internal "decrease indent"
  on the current block where that makes sense), and must NOT
  shift focus to the pane hamburger / surrounding chrome.
* When the cursor IS inside a list, Shift+Tab de-indents the
  current item by one level. Repeated Shift+Tab continues to
  de-indent until the item exits the list (becomes a plain
  paragraph). Further Shift+Tab is a no-op (does not steal
  focus).

### B2: image paste inside a list

* Pasting an image while the cursor is on a list bullet
  inserts the image inline followed by a single trailing
  space. Do NOT push the cursor to BOL of the next line.
* If the user presses Enter *without* using that trailing
  space (i.e., they type Enter immediately after the paste),
  the trailing space is retracted so no trailing whitespace
  lands in the saved file.
* If the user types one or more characters after the paste,
  the trailing space is "consumed" / kept depending on what
  they typed — the goal is no trailing whitespace in the
  final saved document.

### B13: typing on a list moves cursor before marker

* Typing inside a list item leaves the cursor exactly where
  it was; the cursor must not jump before the list marker
  (or to any other position). Verify in bullet, ordered, and
  task lists. Repro images attached.

## Out of scope

* Trailing-whitespace tooling in the Find menu — that's
  fullstack-3's "Remove trailing whitespace" action.

## How to start

1. Identify the keybinding handlers for Tab / Shift-Tab; the
   focus-steal is almost certainly a missing preventDefault
   or a wrong handler precedence with the pane hamburger.
2. The image-paste handler likely terminates with a newline
   insert; replace with a single space + retract-on-Enter
   handler.
3. Cursor-jump-on-list-typing is likely a stale selection
   model after marker re-render.

## Hand-off

Same as fullstack-1.

## 2026-05-18 17:05 BST — Specialist review requested

Implemented the `fullstack-4` list + image interaction fixes.

Files changed:

* `web/src/editor/Wysiwyg.svelte`
* `web/src/editor/bubbles/image_drop.ts`
* `web/src/editor/commands/list.ts`
* `web/src/editor/commands/list.test.ts`

Behavior:

* Shift+Tab is now editor-local: indented list items outdent one level,
  top-level list items lose their marker and become plain paragraphs, and
  non-list lines consume Shift+Tab as a no-op instead of moving focus to pane
  chrome.
* Pasted / dropped images on list lines insert inline with one trailing space
  instead of forcing a newline.
* Pressing Enter immediately after an inline pasted image retracts that unused
  trailing space before list continuation runs.
* Left-clicks that land inside a list marker prefix clamp the caret to the
  start of list content, preventing typing before the marker.

Verification:

* `npm run test -- list` from `web/`
* `npm run check` from `web/`
* `npm run build` from `web/` (passes with existing chunk-size /
  ineffective-dynamic-import warnings)
* `scripts/pre-push`

Known gaps:

* No manual browser / Chan.app walkthrough yet.
