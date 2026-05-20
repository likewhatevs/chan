# fullstack-b-13: Shell/agent submit-mode toggle + survey-reply echo consumer

Owner: @@FullStackB
Date: 2026-05-20

## Goal

Today the rich prompt's Cmd+Enter submit + the survey-reply
echo path both write text to the underlying terminal's PTY
ending with a literal Enter (`\n`). For a shell, Enter
submits — fine. For an agent running inside the terminal
(Claude Code / codex / gemini), Enter inserts a newline into
the agent's input draft; only Cmd+Enter submits the message.
Reply text ends up wedged in the agent's input draft,
unsubmitted. @@Alex's verbatim ask: `poke<cmd+enter>` not
`poke<enter>`.

Three deliverables:

1. **Per-prompt shell/agent submit-mode toggle** — small
   icon button in the rich-prompt header toolbar (matches
   the `fullstack-a-24` floating-pill toolbar pattern).
   States: "Shell" (default; today's behaviour) and "Agent".
   Persists per-prompt-session as a new SerTab field
   (suggest `rpsm?: "shell" | "agent"`; conditional spread
   on serialize; absence = shell default).
2. **Chord-encoding research + decision** — figure out what
   byte sequence agents (specifically Claude Code; cross-
   check codex / gemini) accept as "submit." Likely
   candidates: xterm modifier-other-keys
   `\x1b[27;9;13~`, raw `\x0d` (CR-only, no LF), or some
   bracketed-paste-mode terminator. Pin the choice in the
   task tail with a one-liner reproducer (e.g. echo bytes
   to a Claude Code session via `printf`, see what
   submits).
3. **Two consumer sites consume the toggle**:
   * **Rich-prompt Cmd+Enter submit path**:
     `submitRichPrompt` / `sendUserInput` (in
     `TerminalTab.svelte`). Shell mode → trailing byte is
     `\n` as today (or empty if buffer already ends in
     `\n`). Agent mode → trailing chord per the encoding
     research above.
   * **Survey-reply echo path**: the SPA emits a literal
     "poke" string + Enter into the PTY when the user
     clicks a survey-reply option. Find the call site
     (grep for the poke string OR the place that writes
     after `writeSurveyReply`); apply the same toggle.

## Background

Bug entries in [`../phase-8-bugs.md`](../phase-8-bugs.md):
* "Survey-reply echoes to the terminal as `poke<Enter>`;
  breaks agents that need `poke<Cmd+Enter>`"
* (Same root family as item C in the rich-prompt session
  evolution work — see
  [`../architect/rich-prompt-session-evolution.md`](../architect/rich-prompt-session-evolution.md)).

Today's PTY-write path:
* Rich prompt submit → `submit()` → `onSubmit(buffer)` →
  `submitRichPrompt(source)` → `sendUserInput(source)` →
  WebSocket frame `{type: "input", data}` →
  chan-server `routes/terminal.rs` → PTY.
* No explicit trailing-newline append in JS; the buffer's
  own trailing characters dictate what arrives. Whatever
  produces `poke<Enter>` for survey replies is a separate
  emission path; find it.

Encoding research notes:
* Claude Code's terminal listens for a "submit" chord. The
  exact byte sequence depends on the terminal's keybinding
  interpretation. Test reproducer: `printf 'pwd\x1b[27;9;13~' > /dev/<pty>`
  (or send via the chan-server WS path); see if Claude Code
  submits.
* If `\x1b[27;9;13~` doesn't work, try alternative xterm
  modifier encodings (CSI 13 ; 5 u for CR+Ctrl, etc.) and
  raw `\x0d`.
* Document the chosen encoding inline in the toggle's
  implementation with a comment citing the test result.

## Acceptance criteria

* Rich-prompt header toolbar has a clearly-labelled
  "Shell" / "Agent" toggle (icon + state-reflecting label
  acceptable). Default "Shell."
* Toggle state persists per-prompt-session (SerTab); empty
  / unset case round-trips identically to today's SerTab.
* In shell mode: today's behaviour preserved byte-for-byte
  in both consumer sites (rich-prompt submit + survey-reply
  echo).
* In agent mode: rich-prompt Cmd+Enter sends the buffer +
  the agent-submit chord. Tested against a live Claude
  Code session; the buffer arrives as a single submitted
  message in Claude Code's input.
* In agent mode: clicking a survey-reply option sends
  `poke<agent-chord>` (or whatever the current reply
  string is) instead of `poke<Enter>`. Tested against a
  live Claude Code session.
* Encoding choice documented inline with a one-line
  reproducer and a citation to the source.
* `vitest` green for the toggle-state SerTab round-trip +
  the consumer wiring. End-to-end PTY behaviour can't be
  fully unit-tested; @@WebtestB verifies on lane-B.

## How to start

1. Spin up a test server with a terminal running Claude
   Code (or any agent with a Cmd+Enter submit chord).
2. Empirically nail down the chord encoding — drop bytes
   into the PTY via `sendUserInput` from the browser
   console and see which one triggers a submit in Claude
   Code's input box.
3. Pin the encoding. Then wire the toggle.
4. Find the survey-reply echo call site (the "poke" string
   emission). Most likely lives near the
   `writeSurveyReply` consumer in BubbleOverlay or
   watcherEvents — grep `"poke"` first.
5. Test both consumers end-to-end against a live agent
   session.

## Coordination

* Pairs with [`fullstack-a-28`](../fullstack-a/fullstack-a-28.md)
  (BubbleOverlay regression cluster). -a-28 owns the
  rendering/dismissal side of the bubble; this task owns
  the PTY-write side. The survey-reply call site that
  emits "poke<Enter>" today might live inside the
  BubbleOverlay code path; coordinate at task-cut if the
  two tasks need to touch the same file. Recommended split:
  -a-28 changes WHAT triggers the reply (dismissal); -b-13
  changes WHAT bytes hit the PTY in response.
* @@WebtestB verifies on lane-B against a live Claude Code
  session in a chan terminal.
