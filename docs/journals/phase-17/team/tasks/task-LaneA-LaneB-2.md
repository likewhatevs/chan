# task-LaneA-LaneB-2: B1 - rich prompt per-terminal + submit-confirm

From: @@LaneA  To: @@LaneB  Wave: 2 (HOLD - start after you report B8;
do NOT interrupt B8 to pick this up)

## Why this is queued, not interrupting

@@Alex just hit a LIVE data-loss bug on the rich prompt (below). It is the
same component as B1, so B1 now carries it. Finish B8 first; this is your
immediate next-up. If @@Alex re-pokes to expedite over B8, @@LaneA will relay.

## @@Alex's live bug (verbatim, 2026-06-02)

"i just tried the rich prompt and when i hit cmd+enter to submit the input
cleared but the text i wrote went nowhere, to no terminal.. i think we should
not clean up the prompt until we can confirm the submission.. we can put a
loader / cancel when in this state, and if cancelled we do not reap the prompt"

### Root cause (grounded read, re-verify)

- RichPrompt.svelte submit() ~85-96: clears the composer (view.dispatch empty)
  whenever sendPromptToActiveTerminal(text) returns true.
- tabs.svelte.ts sendPromptToActiveTerminal ~1433-1449: resolves
  activeTerminalTab() (WINDOW-global active tab, NOT the focused pane's tab -
  the B1 bug) and returns the sink's synchronous result. The sink returns true
  on a local WS-queue write; there is no end-to-end confirmation the FOCUSED
  terminal received it. So the text can clear while landing on the wrong
  terminal or nowhere visible.

## B1 scope (from bootstrap; @@Alex's draft.md bug #1)

1. Per-terminal toggle. cmd+shift+p must raise the rich prompt ONLY on the
   selected tab in the FOCUSED pane. Today it is window-global
   (App.svelte ~659-665 -> richPrompt.svelte.ts visible flag), so it shows on
   all terminals and focus lands on the last terminal, not the focused pane.
   - Do NOTHING if no terminal is selected.
   - On show, put focus + cursor IN the prompt area.
   - App.svelte is YOUR file (the cmd+shift+p handler); richPrompt.svelte.ts is
     yours. Scope visibility to the focused pane's active terminal tab. (You may
     need to carry the focused terminal's tab id into the visibility/state
     instead of a window-global boolean.)
2. Resize the prompt TOP up to the top of the terminal, mirroring the existing
   bottom margin (RichPrompt.svelte ~197-210).
3. Survey bubbles must stack ABOVE the rich prompt. BubbleOverlay z:39000 vs
   RichPrompt z:20 - verify the focused-prompt case does not invert this.

## @@Alex's submit-confirmation requirement (NEW, fold into B1)

- On cmd+enter submit, do NOT reap (clear) the composer until submission is
  CONFIRMED. While in-flight, show a loader + a cancel affordance.
- If the user cancels, KEEP the prompt text (do not reap).
- "Confirmed" should mean the prompt actually reached the intended terminal -
  not merely that a local sink returned true. Assess whether the current WS
  `prompt` frame gives you a usable confirmation (flush/ack) or whether you need
  a lightweight ack on the prompt frame. If confirming requires a server-side
  prompt-frame change (chan-server prompt handler), STOP and route to @@LaneA -
  that region is shared with @@LaneD and needs sequencing.
- Empty/whitespace submit stays swallowed (current behavior).

## Files (your lane)

web/src/App.svelte, web/src/components/RichPrompt.svelte,
web/src/state/richPrompt.svelte.ts, web/src/components/BubbleOverlay.svelte;
tabs.svelte.ts prompt-sink helpers (~1433-1449,
sendPromptToActiveTerminal/sendPromptToTerminal) are terminal-prompt plumbing
in your domain - far from @@LaneC's saveDraft (~2085) and your own pane region
(~2353+); the lead commits the merged tabs.svelte.ts at round close.

## Gate

- make web-check + svelte-check + npm run build.
- Browser-smoke (Svelte-5 reactivity is runtime-only): with TWO panes each
  holding a terminal, focus pane A, cmd+shift+p -> prompt opens on A's terminal
  ONLY, cursor in prompt; type + cmd+enter -> text lands in A's terminal and
  the composer clears only AFTER it is confirmed; cancel mid-flight -> text
  preserved; no-terminal-selected -> shortcut is a no-op; survey bubble shows
  ABOVE the prompt. rust-embed: npm run build before cargo build; smoke the
  SERVED bundle.

## Report

Cut tasks/task-LaneB-LaneA-N.md (summary + own-gate-green + pathspec shas) and
poke @@LaneA. Then B4 is your last Wave-2 item (separate task; B4 + @@LaneD's B5
share the chan-server crate - @@LaneA sequences).
