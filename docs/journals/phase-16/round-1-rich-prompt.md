# Rich Prompt -- design brief (@@Lead, from @@Host)

A RETURNING feature: a floating markdown input ("Rich Prompt") over the
terminal, now wired to submit through the `cs terminal write` server-side
queue (the always-on per-session FIFO @@LaneA is building) instead of
whatever it used before. @@Host fully specified the UX; this brief records the
spec + the backend seam + the lane split + the architect-side calls.
Lanes post their own design-first implementation briefs before coding.

## Spec (settled by @@Host -- do NOT re-litigate)

- A floating markdown input bubble over the BOTTOM of the active terminal.
- It FLOATS: inset from the terminal edges (does NOT touch them), rounded
  corners, no buttons. The only chrome is a label: "submit with cmd+enter".
- Markdown-first, LIGHTWEIGHT (not the full Wysiwyg). Free markdown editing.
- ENTER = newline (keep editing). CMD+ENTER = submit to the queue + clear.
- Toggle: Cmd+Shift+P (show/hide). CONFIRMED FREE (App.svelte onWindowKey is
  the central dispatcher; no Cmd+Shift+P binding, no command palette).
- Terminal right-click menu gets a "Show/Hide Rich Prompt" entry showing the
  Cmd+Shift+P shortcut.
- Submit target = the ACTIVE terminal session, via the SAME queue the CLI
  `cs terminal write` feeds, so CLI pokes + bubble prompts share one FIFO and
  auto-submit after each other.

## Grounded seams (from the read-only scout)

- Central keybinding dispatcher: `web/src/App.svelte` `onWindowKey`
  (~395-761), hardcoded conditions. Cmd+Shift+M/R/[/] taken; Cmd+Shift+P FREE.
- Terminal input today: SPA `TerminalTab.svelte:853` `sendInput` sends
  `{ type: "input", data }` over the terminal WS (`/api/terminal/ws`); server
  `routes/terminal.rs` `ClientFrame::Input` (~98-109, handled ~501) ->
  `session.send_input`. NOTE: that path is the RAW keystroke path -- it must
  NOT be what the bubble uses (that would bypass the queue).
- Active terminal: `web/src/state/tabs.svelte.ts` `activeTerminalTab()`
  (~1018) via `activePane().activeTabId`.
- No lightweight CM factory exists. Hand-assemble from the Source.svelte
  extension set: markdown syntax + history + a minimal keymap; OMIT the
  Wysiwyg decorations/widgets/bubbles. (find.test.ts ~21-25 shows a minimal
  CM6 EditorState/View bootstrap.)

## The backend seam (the unifying decision)

The per-session write queue (@@LaneA) must accept TWO producers feeding ONE
FIFO per session, with ONE drain:
1. the control socket (`cs terminal write`) -- the CLI path (already in scope);
2. the terminal WS route -- the Rich Prompt bubble (NEW).

Both enqueue; the single drain delivers in order, appending the submit chord
when the target agent is idle. This is exactly @@Host's "messages always
enqueue properly and submit after each other" regardless of source.

### NEW WS frame (the @@LaneA <-> @@LaneB contract)

Add a `ClientFrame` variant to `routes/terminal.rs` (pin the wire string with
serde rename), proposed:

```
// SPA -> server, over the existing terminal WS
{ "type": "prompt", "data": "<markdown text>", "agent": "claude" }
```

- On receipt, the server ENQUEUES `data` into THIS session's write queue (the
  same FIFO the control socket feeds) -- it does NOT call `send_input`
  directly. The drain delivers it + the submit chord when the agent is idle.
- `agent` is OPTIONAL; it picks the submit chord (claude = the Cmd+Enter CSI,
  codex/gemini = CR). DEFAULT = claude (the round's primary agent) when
  absent. The SPA passes the active terminal's agent if it knows the launch
  command; else omits it and the server defaults.
- Response / receipt: the queued# can ride the existing terminal WS server
  frames (or be fire-and-forget for v1 -- the bubble just clears on submit;
  the queue position is not surfaced in the UI for v1). @@LaneA's call.

This makes the queue producer-agnostic. @@LaneA owns the server frame +
the enqueue plumbing; @@LaneB consumes the frame from the bubble.

## Lane split

- **@@LaneA (backend, extends the in-flight queue):** make the per-session
  queue WS-reachable -- add the `prompt` ClientFrame to `routes/terminal.rs`
  that enqueues into the session queue (NOT send_input), default-claude chord.
  The queue stays a single per-session FIFO with two producers (control socket
  + WS) and one drain. Fold into the cs-write-queue build; one extra producer,
  not a second queue. Post the final frame shape so @@LaneB can wire to it.
- **@@LaneB (frontend, the Rich Prompt component):** the whole bubble --
  `RichPrompt.svelte` (floating, inset, rounded, the "submit with cmd+enter"
  label, lightweight hand-assembled markdown CM6: syntax + history + a keymap
  where Enter=newline and Cmd+Enter=submit), the Cmd+Shift+P toggle in
  App.svelte onWindowKey, the show/hide state, the terminal right-click
  "Show/Hide Rich Prompt" entry, and the submit -> WS `prompt` frame wiring
  (targets activeTerminalTab(); reuse TerminalTab's ws send, do NOT use the
  raw `input` frame). Browser-smoke the toggle, the float position over the
  terminal bottom, Enter-vs-Cmd+Enter, and an end-to-end submit once @@LaneA's
  frame lands.
- **@@LaneD:** UNCHANGED -- stays on the blocklist FB-settings UI.

## Architect-side calls (made by @@Lead; @@Host can veto)

- Default submit chord = claude when the frame omits `agent`. (Primary agent;
  the bubble works out of the box for the common case.)
- v1 does NOT surface the queue position in the bubble UI (fire-and-submit +
  clear). Queue# stays a CLI-side receipt for now.
- v1 lightweight editor = markdown syntax + history + the submit keymap ONLY.
  No wiki `[[` picker, no @today/@date macros, no widgets inside the bubble.
  (Can grow later; keep v1 truly lightweight per @@Host.)

## Sequencing

@@LaneB can build the component + toggle + context menu + lightweight editor
in PARALLEL against the frame contract above; the end-to-end submit lights up
once @@LaneA's `prompt` frame + queue land. @@LaneA's queue is the dependency
for E2E validation, not for @@LaneB starting. Both post design-first
implementation briefs before coding, per the round's discipline.
