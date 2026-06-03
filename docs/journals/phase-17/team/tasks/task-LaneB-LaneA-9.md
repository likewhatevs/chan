# task-LaneB-LaneA-9: rich prompt A (shell chord) + B (Tab list indent) DONE

From: @@LaneB  To: @@LaneA  Re: task-LaneA-LaneB-8 (post-round)

## Result: both fixed + Chrome-verified. Client-only, confined to
   RichPrompt.svelte + its test. NO server change, NO @@LaneC list.ts change.

## A. Submit chord on a SHELL (the `7;9;13~` garbage)

Root cause (as you noted): the rich prompt submitted with NO agent ->
chan-server defaulted to the claude modifyOtherKeys CSI, which a shell can't
read -> `echo ...7;9;13~` left at the prompt, never run.

Fix: `submitAgent()` derives the chord from THIS terminal's negotiated keyboard
protocol (`tab.keyboardProtocol`), then passes the agent name on the prompt
frame (reusing the shared AGENT_SUBMIT_CHORDS map - no new map, no server
change):
- `xtermModifyOtherKeys > 0` -> "claude" (the CSI).
- kitty flags > 0 -> "codex" (B8 bracketed-paste wrap).
- neither (a plain shell, or gemini) -> "gemini" = a bare CR, so the command
  runs.
I used the negotiated protocol you pointed at (tabs.svelte.ts keyboardProtocol)
rather than a declared agent (TerminalTab has no agent field). Client-only.

## B. Tab indents lists instead of escaping to the browser

The rich-prompt CodeMirror had Mod-Enter/Enter/Backspace but no Tab, so Tab hit
the browser's focus nav. Fix in the keymap:
- `Tab`  -> `indentListItem(v) || indentMore(v)`
- Shift  -> `outdentListItem(v) || indentLess(v)`
`indentListItem`/`outdentListItem` are IMPORTED from @@LaneC's
`editor/commands/list.ts` (import only - I did NOT change list.ts);
`indentMore`/`indentLess` from `@codemirror/commands` are the off-list fallback
so Tab NEVER escapes. Lists still continue/renumber on Enter
(insertNewlineContinueMarkup, unchanged).

## Files changed (mine only)

  web/src/components/RichPrompt.svelte           blob 427323402fbae59aee24474d2affd425a7e0fe6b
  web/src/components/richPromptComponent.test.ts blob 0617b6e0c90b649450b51d0ed6e0f9d9b6a5f04e

(No server / chan-server prompt-handler change was needed - the fix is the SPA
choosing the right agent, so I did not touch @@LaneD's terminal.rs.)

## Own-gate (scoped) - GREEN

  npx vitest rich-prompt tests   PASS (16)
  npm test (full vitest)         PASS (1665)
  npm run check (svelte-check)   0 ERRORS (tree clean post round-2)
  npm run build                  OK

## Empirical (Chrome, fresh binary :8796)

- A: rich prompt over a SHELL terminal, `echo hello_rp_shell` + Cmd+Enter ->
  RAN it (output `hello_rp_shell`, no `7;9;13~`); composer cleared. (Claude path
  unchanged: modifyOtherKeys -> "claude" chord, as B1 verified.)
- B: typed `- alpha` <Enter> `beta` <Tab> -> `  - beta` (INDENTED), focus STAYED
  in the prompt (no browser escape); <Shift+Tab> -> outdented back to `- beta`.
Torn down (server by PID, chan remove, rm temp; no broad pkill).

## Status

Both done. Post-round, so this is uncommitted on top of 92fdf17e for your
commit. Nothing pushed. That clears @@Alex's two rich-prompt follow-ups.
