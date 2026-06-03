# task-LaneB-LaneA-2: B1 DONE - rich prompt per-terminal + data-loss fix

From: @@LaneB  To: @@LaneA  Re: task-LaneA-LaneB-2 (Wave-2 B1)

## Result: data-loss bug FIXED + per-terminal scoping + resize + z-order, all
   empirically verified in Chrome (2 panes / 2 terminals).

## @@Alex's live data-loss bug - root cause + fix

Root cause: RichPrompt.submit() routed via `sendPromptToActiveTerminal` (the
FOCUSED pane's active tab), but the bubble was shown WINDOW-GLOBAL (a
`richPrompt.visible` bool x TerminalTab `active`, which is per-pane). So with
two panes the bubble showed on EVERY pane's active terminal, focus landed on
the last one, and typing into a bubble that wasn't the focused terminal cleared
the text while it routed to the wrong terminal / nowhere visible.

Fix (in-lane):
1. Per-terminal visibility - richPrompt.svelte.ts keyed by tab id (`byTab`
   Record), not a global bool. cmd+shift+p (App.svelte) resolves the focused
   terminal via `activeTerminalTab()` and toggles ONLY it; NO-OP when the
   focused tab is not a terminal. TerminalTab renders on
   `active && isRichPromptVisible(tab.id)`; menu/Escape/close are per-tab. Two
   terminals can show their own bubble independently.
2. Routing/data-loss - submit() routes to its OWN tab
   `sendPromptToTerminal(tab.id, text)` and reaps the composer ONLY if the
   prompt frame went out to that terminal's OPEN socket (failed send keeps the
   text). Removed dead `sendPromptToActiveTerminal` (no caller; pre-release).
3. Resize-top - a top grab handle drags the bubble up to the terminal top
   (mirrors the 12px bottom inset; capped at parent height - 24). Per-prompt.
4. Survey z-order - BubbleOverlay (position:fixed, z:39000, App root) paints
   above the bubble (z:20 inside terminal-tab z:2). Verified, no change needed.

## Files changed (my lane)

  web/src/state/richPrompt.svelte.ts        blob f2d44dc30de6df35f992089238f5cb1223c7add7
  web/src/App.svelte                        blob 9bd45115a23f6ad0be1de678ed1d53f8037b0dce
  web/src/components/TerminalTab.svelte     blob 39e75ed43c981fa45fef84a08f003e2e0f2d2d11
  web/src/components/RichPrompt.svelte      blob 06da00e3641bd46a70e5a16dd431986567873403
  web/src/state/tabs.svelte.ts              blob f802b46fa6bdfbf26de057b2ce3664188d2e4bec
  web/src/components/richPromptComponent.test.ts      blob 1c638ce21cefe841181de7975fc7460977d15a78
  web/src/components/richPromptTerminalWiring.test.ts blob 00c039254645638a7f2900924afd15af5758d39d

tabs.svelte.ts touch is ONLY the prompt-sink region (~1428, removed the dead
sender) - far from @@LaneC's saveDraft (~2085) and my own pane region (~2353+),
.ts interleave-safe.

## Own-gate (scoped) - GREEN

  npm test (full vitest)        PASS (1646 / 167 files)
  npm run check (svelte-check)  0 ERRORS
  npm run build                 OK

Note: 1 svelte-check WARNING at RichPrompt.svelte root div
(a11y_no_noninteractive_element_interactions, role="group" + Escape onkeydown).
It is PRE-EXISTING (showed at 188:1 before I touched the file) and svelte-ignore
does not suppress it. exit 0 = green. Left as-is to keep the diff focused.

## Empirical smoke (Chrome / Blink, fresh binary, 2 panes / 2 terminals)

All PASS:
- cmd+shift+p opens the bubble on the FOCUSED pane's terminal ONLY (Terminal-2
  had no bubble while Terminal-1's was open) - the headline "shows on all
  terminals" bug is fixed.
- focus + cursor land IN the prompt on show.
- cmd+shift+p again hides it (toggle).
- two terminals show their own bubble simultaneously (independent per-terminal).
- type + cmd+enter -> text lands in the bubble's OWN terminal; composer clears
  after a successful send (the data-loss fix).
- resize-top: drag the top grip up -> bubble grows toward the terminal top,
  caps ~12px below it (mirrors the bottom inset); per-prompt (other unchanged).
- survey raised over the terminal with the tall focused prompt renders ABOVE it
  (centered modal + backdrop; focused-prompt case does not invert).
- File Browser active in the focused pane: cmd+shift+p is a NO-OP; and the
  terminal's bubble hides when its tab goes inactive (active-prop reactivity).
Server + Chrome tab torn down (killed by PID, chan remove, rm temp). I did NOT
broad-pkill; @@LaneD's b11test server (port 8810) was untouched.

## DECISION for you (server-side, per the task's STOP+route)

@@Alex also asked for a loader + cancel "do not reap until the prompt is
CONFIRMED to have reached the terminal". Assessment: the WS `prompt` frame is
fire-and-forget. `sendPrompt`->`send` returns true iff the WS is OPEN and the
frame was written to the socket - that confirms DELIVERY to the correct
terminal's socket, but NOT server-side enqueue. A TRUE confirm with a visible
loader/cancel window needs the chan-server prompt handler to ACK the frame
(e.g. a `prompt-ack` reply) - that region is shared with @@LaneD, so I stopped
and am routing it to you.

Important: the in-lane fix ALREADY prevents @@Alex's reported data loss
(correct routing + reap-only-on-delivery). The loader/cancel ack is an
ADDITIVE robustness enhancement. Options for you to sequence:
  (a) Ship B1 as-is now (data loss fixed); cut a follow-up task for the
      server-side prompt-ack + client loader/cancel (me + @@LaneD).
  (b) Hold B1's loader/cancel sub-item until you sequence the chan-server
      prompt-handler change with @@LaneD's B5 (same crate).
My recommendation: (a) - the data loss is the urgent part and it is fixed +
verified; the ack is a clean follow-up.

## Status

B4 is my last Wave-2 item (HOLD per your earlier poke - B4 + @@LaneD's B5 share
the chan-server crate; you sequence). Holding for your B4 dispatch.
