# fullstack-a-4: Rich prompt cluster (cursor focus, overlay-bottom, spawn-agent dialog)

Owner: @@FullStackA
Date: 2026-05-19

## Goal

Four related rich-prompt UX fixes:

1. **Cursor focus on rich-prompt open** — if no bubbles, focus
   the prompt input; if bubbles present, focus the survey area
   (so numbered keystrokes reply immediately).
2. **Cursor stays in rich prompt after Cmd+Enter** — today it
   drops out; per @@Alex it should remain so consecutive prompts
   are fluid.
3. **Rich-prompt overlay obscures bottom of terminal** — push
   the terminal up (or resize cleanly) so the last rendered
   terminal line is still visible above the overlay.
4. **'Spawn agent' from rich prompt** — clicking the button
   currently dims the screen but no dialog appears. Restore the
   spawn dialog (or wire it for the first time if it never
   landed).

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md) under
"Rich prompt", "spawn agent", and the overlay/cursor items.

`spawn-agent` ties into phase-7 wave-B fullstack-20 (spawn from
rich prompt + pre-flight survey). Confirm whether that work
landed; if it shipped but the dialog regressed, this is a
fix; if it never landed, this is the wire-up cut.

## Acceptance criteria

* Opening the rich prompt with no bubbles → caret in the prompt
  input.
* Opening with at least one bubble → caret in the survey area;
  pressing `1`/`2`/`3` etc. replies the focused bubble.
* After dismissing all bubbles → caret returns to the prompt
  input.
* Cmd+Enter submits the prompt and leaves the caret in the rich
  prompt area for the next entry.
* Rich-prompt overlay no longer paints over the bottom terminal
  line. Resize is acceptable; covering is not.
* `Spawn agent` button opens an actual dialog (pre-flight survey
  + agent profile selector).

## How to start

Likely files:
* `web/src/components/TerminalRichPrompt.svelte`
* `web/src/components/BubbleOverlay.svelte`
* SPA terminal layout for the resize/push behaviour.
