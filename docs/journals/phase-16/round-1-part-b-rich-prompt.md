# Rich Prompt - @@LaneB implementation brief (DESIGN-FIRST, awaiting sign-off)

Source: docs/journals/phase-16/round-1-rich-prompt.md (@@Lead/@@Host spec).
This is the @@LaneB frontend plan. NO code until @@Lead signs off. The spec
is settled (do not re-litigate); this brief is HOW I build the component and,
critically, WHERE it touches terminal-area files @@LaneA is also editing.

## Scope (frontend, @@LaneB)

The floating "Rich Prompt" bubble over the active terminal + its toggle +
context-menu entry + the submit wiring into @@LaneA's new WS `prompt` frame.
Backend (the `prompt` ClientFrame + the per-session enqueue) is @@LaneA.

## Components / files

NEW (LaneB-owned, zero collision):
- `web/src/components/RichPrompt.svelte` - the bubble: floating, inset,
  rounded, no buttons, a "submit with cmd+enter" label, a lightweight
  hand-assembled markdown CM6 editor.
- `web/src/state/richPrompt.svelte.ts` - window-global show/hide state +
  the draft text. `visible` (default false), `draft` (string), plus
  `toggleRichPrompt()` / `showRichPrompt()` / `hideRichPrompt()`. Draft
  persists across toggles + active-terminal switches; cleared on submit.
- `web/src/components/richPrompt.test.ts` - source-pattern + any pure-logic
  tests; the real interaction is browser-smoked.

EDIT (terminal-area; SHARED with @@LaneA - see Coordination):
- `web/src/state/tabs.svelte.ts` - add a PROMPT-sink registry mirroring the
  existing `registerTerminalInputSink` (tabs.svelte.ts:1496-1506):
  ```
  type TerminalPromptSink = (data: string, agent?: string) => void;
  const terminalPromptSinks = new Map<string, TerminalPromptSink>();
  export function registerTerminalPromptSink(id, sink): () => void { ... }
  export function sendPromptToActiveTerminal(data, agent?): boolean {
    const tab = activeTerminalTab();
    const sink = tab && terminalPromptSinks.get(tab.id);
    if (!sink) return false;
    sink(data, agent);
    return true;
  }
  ```
  This is the same proven pattern as the input sink, so RichPrompt never
  reaches into TerminalTab internals.
- `web/src/components/TerminalTab.svelte` - three small additive touchpoints:
  1. register the prompt sink in onMount:
     `registerTerminalPromptSink(tab.id, (data, agent) =>
       send({ type: "prompt", data, ...(agent ? { agent } : {}) }))`
     (reuses the existing `send()`/`ws` at :849; NOT `sendInput`/the raw
     `input` frame).
  2. mount `<RichPrompt />` over the terminal body bottom when this tab is
     `active` and `richPrompt.visible` (absolute, inset, the terminal
     container becomes position:relative).
  3. add a "Show/Hide Rich Prompt" entry to the F4 body-context menu
     (the right-click-in-terminal list at ~:1409) with the Cmd+Shift+P
     chord shown via the existing `chordFor()` helper.
- `web/src/App.svelte` - one additive block in `onWindowKey` (~:653 area,
  next to the other Cmd+Shift chords):
  `if (e.metaKey && !e.altKey && e.shiftKey && !e.ctrlKey && e.code === "KeyP")
   { e.preventDefault(); toggleRichPrompt(); return; }`
  Confirmed FREE: :653 is Cmd+ALT+P (alt), :660 Cmd+Shift+M; no Cmd+Shift+P.

## The lightweight CM6 (RichPrompt internal)

Hand-assemble from the same packs Source.svelte uses, but INVERT the chord
(Source binds plain Enter to submit; we need the opposite):
- `markdown({ addKeymap: false })` (syntax only, no markdown Enter handling)
- `history()` + `keymap.of([...defaultKeymap, ...historyKeymap])`
  -> plain Enter = newline, full history/undo.
- `Prec.high(keymap.of([{ key: "Mod-Enter", run: () => { submit(); return true } }]))`
  -> Cmd+Enter (Ctrl+Enter off-Mac) = submit + clear. High-prec so it wins.
- `EditorView.lineWrapping` + markdown syntax highlighting + the surface
  theme. NO lineNumbers, NO Wysiwyg widgets/decorations/bubbles, NO wiki
  `[[` picker, NO @date macros (v1 lightweight per @@Host / the architect
  calls in the spec).

## Submit flow

On Cmd+Enter: read the doc text; if non-empty call
`sendPromptToActiveTerminal(text, agent?)`; on success clear the editor +
the `draft` (bubble stays open + focused for the next prompt). `agent` is
omitted for v1 (server defaults to claude per the spec) unless we cheaply
know the active terminal's launch agent; I'll OMIT it in v1 and let the
server default, to avoid guessing the agent. Fire-and-forget: no queue#
surfaced in the bubble (v1 architect call).

## UX details

- Float: `position: absolute; left/right: 12px; bottom: 12px;` inset from the
  terminal edges, rounded corners, subtle bg + border + shadow, max-height
  with internal scroll, z above the xterm canvas but below the menu bubble.
- Label "submit with cmd+enter" rendered via the app's chord formatter so it
  reads correctly per-OS (cmd on Mac, ctrl elsewhere); static caption, not a
  CM placeholder.
- Focus: showing the bubble focuses the editor; Escape hides it + returns
  focus to the terminal (small nicety; does not collide with App's Escape,
  which is handled inside the bubble before it bubbles up).
- One bubble per window; it renders only in the ACTIVE terminal, so switching
  the active terminal moves it. Draft text is shared (one logical input).

## Coordination (the real risk - needs @@Lead sequencing)

`tabs.svelte.ts` + `TerminalTab.svelte` are terminal-area files @@LaneA is
actively churning (C2/C3/S1). My edits there are small + additive (a sink
registry; a sink registration; a mount block; one menu row), but they WILL
overlap @@LaneA's working tree. Asks:
1. Confirm @@LaneB owns the rich-prompt edits to TerminalTab.svelte +
   tabs.svelte.ts (the spec assigns the frontend wiring to @@LaneB), and
   SEQUENCE them after @@LaneA's terminal slices land (or tell me a clean
   window) so we don't collide.
2. Confirm @@LaneA's final `prompt` frame shape == `{ type: "prompt", data,
   agent? }` (serde-renamed), enqueue (NOT send_input), default-claude chord.
   I build the bubble against this contract now; E2E lights up when it lands.
3. Confirm the prompt-sink registry approach (vs @@LaneA exposing the
   send themselves). I prefer the registry: it mirrors the input sink and
   keeps RichPrompt decoupled.

## Build order (parallel-safe)

1. richPrompt.svelte.ts (state) + RichPrompt.svelte (bubble + CM6) +
   App.svelte toggle - all buildable NOW against the frame contract; the
   submit can no-op (sink absent) until the terminal wiring lands.
2. tabs.svelte.ts prompt-sink registry + TerminalTab.svelte
   (register + mount + menu) - sequenced with @@LaneA per Coordination.
3. Browser-smoke: toggle, float position over the terminal bottom,
   Enter=newline vs Cmd+Enter=submit, end-to-end submit once @@LaneA's
   frame is in.

VERIFY: `make web-check` green at each slice; per-slice pathspec commit + sha
to event-lane-b.md.
