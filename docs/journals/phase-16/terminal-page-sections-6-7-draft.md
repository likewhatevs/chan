# terminal.md sections 6+7 - resume-ready draft (DEFERRED post-break)

Status: terminal.md sections 1-5 are COMMITTED (0970c902). Sections 6 (Rich
Prompt) + 7 (write queue) + a Team Work note are DEFERRED to after @@Host's
break (finalize against the settled code). Nothing is uncommitted/lost. The
model below is VERIFIED against shipped source (not the stale specs), so the
post-break finalization is: paste these sections into docs/manual/terminal.md
(after Pokes; Team Work after Survey), do the ONE open verification, run
web-marketing `npm run check`, pathspec-commit.

## Verified grounding (shipped source, not the specs)

Rich Prompt (the spec round-1-rich-prompt.md is SUPERSEDED - it describes the
original fire-and-clear CM6 bubble; the shipped model is Drafts-backed):
- web/src/state/richPrompt.svelte.ts: window-global visible flag; one bubble
  per window, renders only in the ACTIVE terminal; text NOT held here.
- web/src/components/RichPrompt.svelte: floating markdown bubble; Enter=newline,
  Cmd+Enter=submit (Mod-Enter); label "submit with cmd+enter" (mac) /
  "submit with ctrl+enter". Text + pasted images live in a per-terminal
  Drafts/<name>/draft.md (tab.richPromptDraftPath); image paste reuses the
  editor's imageDropHandlers -> writes into the same draft folder + inserts
  ![](path), no base64 (any claude/codex/gemini agent reads the file).
  submit() [:85-96]: send via sendPromptToActiveTerminal(text) -> clear the
  draft TEXT (folder+media KEPT) -> flushWrite; empty/whitespace swallowed;
  failed send keeps text. ensureDraft() creates the draft lazily, seeds empty.
- Delete-on-close VERIFIED (not just a comment): TerminalTab.svelte:1085
  `void api.discardDraft(tab.richPromptDraftPath)`.
- Toggle: Cmd+Shift+P (App.svelte onWindowKey) + terminal right-click
  "Show/Hide Rich Prompt". Submit path: terminal WS `prompt` frame ->
  per-session write queue (tabs.svelte.ts sendPromptToActiveTerminal).

Write queue (cs-write-queue-design.md final section + terminal_sessions.rs):
- ALWAYS-ON per-session bounded FIFO. WRITE_QUEUE_CAP=100 (terminal_sessions
  .rs:34); dropped at cap; dropped on session recycle (close/restart).
- Single signal = agent OUTPUT quiescence (last_output_at); QUIET_MS=800
  (:38). Drainer tick 150ms (:45). Post-submit await-generation-start settle,
  GEN_START_CAP_MS=2000 (:43). Two producers (control socket cs-write + WS
  prompt frame), one drain; delivers next only when the agent is idle, each
  with its submit chord -> chained messages auto-submit one after another.
  Does NOT detect a paused half-typed compose buffer (the queue owning the
  input path is what keeps that case rare). Queue merged 3d6d144e.

## Draft prose (paste into terminal.md when finalizing)

### ## Rich Prompt   (place after "## Pokes")

Rich Prompt is a floating markdown input over the bottom of the active
terminal. Toggle it with Cmd+Shift+P (or the terminal right-click
"Show/Hide Rich Prompt" entry); one bubble per window follows whichever
terminal is active. Type markdown freely: Enter inserts a newline, and
Cmd+Enter submits.

Each terminal's bubble is backed by a real draft on disk, a
`Drafts/<name>/draft.md` in the workspace, so the prompt text is an ordinary
file. Pasting an image works like the editor: the image is written into the
same draft folder and referenced with `![](path)`, so an agent reads the
picture as a file (no base64), whichever agent it is. Submitting clears the
draft text but keeps the folder and any pasted media, so the agent can still
read them; the whole draft folder is discarded when the terminal closes.

Submit sends the text to the active terminal's agent through the same write
queue the CLI uses, so a prompt and a `cs terminal write` poke share one
ordered path.

### ## The write queue   (place after Rich Prompt)

Every write to a terminal session, from `cs terminal write` or from Rich
Prompt, goes through a per-session FIFO queue. The queue serializes
deliveries so chained messages never interleave into one compose buffer and
submit one after another: the drainer delivers the next queued message only
once the target agent has gone idle (its previous turn's output has
quiesced), each with the right submit chord. A free target drains
immediately; a busy one enqueues and drains as it frees.

The queue holds up to 100 messages per session and is dropped when the
session is recycled (restart or close). It detects that the agent is
generating, not that it has a half-typed but unsubmitted compose buffer, so
a message can still land mid-buffer in the rare case where text was left
typed and paused; routing all input through the queue is what keeps that
case rare.

### ## Team Work   (place after "## Survey") -- ONE OPEN VERIFICATION

> VERIFY before final: the "no special composer" claim depends on @@LaneB's
> in-flight delete of the old in-terminal Team Work bubble. Confirm the
> bubble/composer is actually gone in the final code (grep web/src for the
> removed component) before shipping this sentence. @@Lead will signal when
> the delete lands.

Team Work runs a set of agents as a team across terminal tabs. The lead is
an ordinary terminal, with the same Rich Prompt and survey as any other and
no special composer. You bootstrap a team from it with Cmd+P (the team
setup/load dialog) or `cs terminal team new|load`, which writes or reads the
team's `config.toml` and the generated bootstrap. The team then coordinates
through the same `cs terminal` tools (pokes, survey) this page covers.

## Finalization checklist (post-break)

1. Re-read this file; spot-recheck the source still matches (queue consts,
   RichPrompt submit, discardDraft-on-close) in case anything moved.
2. Verify the Team Work "no special composer" claim against the landed
   bubble-delete (@@LaneB); adjust the sentence if the shape differs.
3. Paste the three sections into docs/manual/terminal.md in the noted spots.
4. web-marketing `npm run check` green; pathspec-commit docs/manual/terminal.md;
   post the sha.
