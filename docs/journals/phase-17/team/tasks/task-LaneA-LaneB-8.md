# task-LaneA-LaneB-8: rich prompt - submit chord for non-agents + list editing

From: @@LaneA  To: @@LaneB  (post-round; @@Alex reported live)

Two rich-prompt items @@Alex hit.

## A. Submit chord is wrong on a SHELL terminal (sends the Claude chord)

@@Alex hit Cmd+Enter in the rich prompt over a plain SHELL terminal and got
`echo hello world7;9;13~` left at the prompt (NOT run): the literal tail of the
Claude chord `\x1b[27;9;13~`.

Root cause (grounded): RichPrompt.submit() calls `sendPromptToTerminal(tab.id,
text)` with NO agent; TerminalTab.sendPrompt's `prompt` frame omits `agent`, and
per your own comment (TerminalTab.svelte:870) "omitted defaults to claude
server-side". So a shell terminal gets the Claude modifyOtherKeys CSI, which it
can't read -> garbage + the command never submits.

Fix: the rich prompt must submit with the chord that matches THIS terminal, not
the claude default:
- shell / no agent -> a plain CR (`\r`) so the command runs.
- claude -> the modifyOtherKeys CSI (current).
- codex -> the bracketed-paste + CR (B8); gemini -> CR.
Determine the terminal's submit mode (its declared agent, OR the negotiated
keyboard protocol you already track - modifyOtherKeys/kitty in tabs.svelte.ts
~296/3408 - a shell negotiates neither). Pass it through sendPromptToTerminal ->
the prompt frame, OR detect a non-negotiated shell server-side and use CR
instead of defaulting to claude. If the fix is in the chan-server prompt handler
(@@LaneD's crate), keep it localized + flag it; you own the prompt contract.
Reuse submitMode.ts AGENT_SUBMIT_CHORDS (B8) - do not invent a new chord map.

## B. List editing in the rich prompt (Tab indent / bullets / numbers)

@@Alex: "proper support for lists in the rich prompt - numbered, hyphenated,
unordered bullet lists. Today when I press Tab to indent a list item it moves
focus OUT of the window (the browser's default Tab), not the editor's."

The rich-prompt CodeMirror has Mod-Enter=submit + Enter=insertNewlineContinueMarkup
but no Tab handling, so Tab escapes to the browser. Add list editing:
- Tab -> indent the current list item; Shift+Tab -> outdent (the editor behavior,
  preventDefault so it never reaches the browser).
- Numbered / hyphen / bullet lists continue + renumber on Enter (you already have
  continue-markup; extend to indent).
The main editor's list commands live in web/src/editor/commands/list.ts
(@@LaneC's editor lane - it just fixed Tab/Shift-Tab there for R2-2). PREFER
reusing that extension/commands in the rich-prompt keymap rather than a parallel
impl. list.ts is @@LaneC's file: if you need to import/share it, that's fine
(import is safe); if you need to CHANGE list.ts, STOP and route through @@LaneA
(I'll coordinate with @@LaneC, who is on a graph bug right now).

## Gate + report

- make web-check + svelte-check + npm run build (+ cargo if you touch the server
  prompt handler).
- Browser-smoke: (A) Cmd+Enter in the rich prompt over a SHELL runs the command
  (no garbage); over a claude agent still submits. (B) Tab indents a list item
  in the rich prompt (focus stays), Shift+Tab outdents, bullets + numbers
  continue.
- Cut task-LaneB-LaneA-N + poke @@LaneA.
