# fullstack-18: simplify bubble overlay to TUI density

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-18

## Goal

Rework the bubble overlay from `fullstack-13` to TUI-style
density. Current shape has too much UI weight for what's
fundamentally a "press 1, 2, or 3" interaction: Submit
button, Scope dropdown, Skip/not now button, standing
option as its own separated row, stack vs tray picker per-
bubble — none of these survive the "answer-in-one-keystroke"
discipline @@Alex wants.

This task supersedes the survey-rendering portion of
`fullstack-13`. The backend `systacean-9` schema stays
unchanged — only the frontend rendering simplifies.

## Relevant links

* @@Alex's direction in chat 2026-05-18 22:50 BST and
  follow-up 22:55 BST (multi-topic tabs).
* Predecessor: [./fullstack-13.md](./fullstack-13.md).
* Survey schema (unchanged):
  [../architect/journal.md](../architect/journal.md)
  "Round 2 capacity proposal" entry.

## Acceptance criteria

### Single-topic survey

* The bubble renders the question text and a row of
  numbered buttons `1`, `2`, `3`, … one per option.
* Clicking a numbered button replies immediately. No
  Submit step.
* Pressing the matching number key on the keyboard
  while the rich prompt is open also replies immediately.
* Esc on the bubble (or closing the rich prompt /
  dismissing the bubble) is the implicit "skip / not
  now". No dedicated Skip button.
* The standing option ("Check my comments first") is
  rendered as the next numbered button after the real
  options. Producer still distinguishes it at the schema
  level (`standing_options` field); the UI just appends
  it to the option list.

### Multi-topic (4×3) survey

* Horizontal topic-tab strip across the top of the
  bubble. Each tab shows the topic header / short
  question text.
* Tab content area shows the focused topic's options
  vertically as numbered buttons, one per row, each
  with its label.
* Default focused tab = first.
* Press `1`/`2`/`3` while a tab is focused → answers
  that topic's question.
* After answering, focus auto-advances to the next
  unanswered tab.
* Tab / Right arrow = next tab; Shift+Tab / Left arrow
  = previous tab. User can revisit and revise an answer
  before all tabs are filled.
* When every tab has an answer, the reply commits
  automatically and the bubble closes. No Submit button.
* Standing options (if present): rendered as a footer
  row below the tab content area, visible regardless of
  which tab is focused. Numbered continuing from the
  current tab's options (e.g. if the current tab has 3
  options, standing option is `4`).
* Esc anywhere = skip the whole survey. Partial answers
  are discarded.

### Scope grant

* Drop the scope-grant selector from the UI for v1.
* Always reply with `scope_grant: "one-shot"`.
* Future upgrade gestures (modifier + number for
  topic-session / topic-phase) are out of scope.

### Stack vs tray

* Move the stack vs tray picker out of the per-bubble
  chrome and into the preferences surface (Settings
  dialog or rich-prompt menu). The pill in the bubble
  header goes away.
* Default stays whatever the preference is set to;
  preference plumbing from `fullstack-13` is reused as-is.

### Keyboard scoping

* Number keys (`1`–`9`) route to the focused bubble's
  current tab while the rich prompt is open AND no
  text input / textarea inside the prompt has keyboard
  focus.
* If multiple bubbles are visible (stack mode), the
  topmost / most recent one receives the keystrokes.
  Tab between bubbles with `Cmd+Down` / `Cmd+Up` (or
  similar; pick something that doesn't conflict).

## Out of scope

* Backend schema changes — `standing_options` and
  `scope` fields stay where they are.
* Multi-topic future upgrade gestures for scope.
* Markdown-rendered survey questions (plain text + links
  is enough).

## How to start

1. Strip `BubbleOverlay.svelte` of the Submit / Scope /
   Skip / standing-option-as-its-own-section chrome.
2. Render single-topic surveys as a question + numbered
   option row.
3. Add the horizontal topic-tab strip for multi-topic
   surveys; reuse existing tab-style primitives where
   possible.
4. Keyboard handler: capture number keys when the rich
   prompt is open + no text-input focus + a bubble is
   visible. Route to focused bubble's focused tab.
5. Move the stack/tray pill to prefs.

## Hand-off

Standard. Pre-push gate green. Coordinate with @@WebtestA
on the substrate walkthrough — items 7-10 in
[../webtest-a/webtest-a-6.md](../webtest-a/webtest-a-6.md)
will need re-running against the simplified UI. Ping via
`alex/event-fullstack-architect.md`.

## 2026-05-18 19:58 BST — implementation ready

Implemented the TUI-density bubble overlay:

* Single-topic surveys render question text plus numbered option buttons;
  click or number key replies immediately.
* Multi-topic surveys render a horizontal topic strip, vertical numbered
  options for the focused topic, auto-advance after each answer, and auto-
  commit once all topics are answered.
* Standing options append into the same numbered list; reply scope is always
  `one-shot`.
* Esc skips by writing a one-shot reply with no answers.
* Stack/tray controls are removed from bubble chrome; the rich-prompt
  context menu now exposes Bubble stack / Bubble tray.

Verification:

* `npm run test -- BubbleOverlay watcherEvents TerminalRichPrompt`
* `npm run check`
* `npm run build`
* `scripts/pre-push`
