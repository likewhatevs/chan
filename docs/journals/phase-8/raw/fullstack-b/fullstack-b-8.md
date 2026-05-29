# fullstack-b-8: Cmd+Enter from rich prompt occasionally drops first char into terminal

Owner: @@FullStackB
Date: 2026-05-20

## Goal

When the user dispatches text from the rich prompt to the
focused terminal via Cmd+Enter, the full text reaches the
terminal. Today the first character is occasionally
swallowed (`echo hello` arrives as `cho hello`).

## Background

Side observation from @@WebtestA's Round-1 sweep on
2026-05-20:

> Cmd+Enter from rich prompt occasionally drops the first
> character of the dispatched text into the focused terminal
> (`echo hello` → `cho hello`).

Filed as a new bug entry in
[`../phase-8-bugs.md`](../phase-8-bugs.md) "Cmd+Enter from
rich prompt drops first character into terminal".

Likely a timing / focus race: the rich prompt dispatches
text to the terminal before the terminal's input buffer has
fully picked up focus or before the previous keystroke's
event handler has flushed. Could also be an xterm.js write
race — the first character lands during a state where the
terminal is masking input.

## Acceptance criteria

* Dispatching from the rich prompt via Cmd+Enter delivers
  the complete typed text to the focused terminal, every
  time.
* No regression on the existing Cmd+Enter "caret stays in
  the rich prompt after dispatch" behaviour
  (`fullstack-a-4`).
* No regression on the rich prompt + terminal layout
  interactions (the prompt overlay no longer covers the
  terminal bottom — `fullstack-a-4`).
* Manual verification on the lane-B test server
  (`/tmp/chan-test-phase8-wb`, `127.0.0.1:8820`): repeated
  rapid Cmd+Enter dispatches all arrive complete.

## How to start

1. Reproduce on the lane-B server first; if it doesn't
   reproduce there, fall back to the lane-A server
   (`127.0.0.1:8787`) where @@WebtestA originally saw it.
2. Look at the rich prompt's Cmd+Enter handler. Likely in
   `web/src/components/RichPrompt*` or wherever the
   dispatch-to-terminal seam lives.
3. The dispatch likely calls `terminal.write(text)` (or
   sends the bytes via the WebSocket path). Check whether:
   * The focus transfer to the terminal happens AFTER the
     write call (which would let the first char land while
     the terminal still has the prompt as the focus
     target).
   * The write is split across two events and the first
     byte is lost.
   * xterm.js itself drops the first char during a
     focus-in animation frame.
4. Probable fix shape: ensure the terminal has focus
   BEFORE the write, OR write via the canonical input
   channel rather than the visual focus path.
5. Pin with a small repro if the dispatch surface has a
   testable seam.

## Coordination

* @@WebtestB verifies on lane-B drive once landed.

## 2026-05-20 - root cause + fix

Traced the dispatch path end-to-end. The "first character of the
dispatched text" doesn't actually disappear; the prompt buffer
itself is missing the user's first keystroke by the time Cmd+Enter
fires. The "echo hello -> cho hello" is the **buffer** the user
typed, then Cmd+Enter dispatches that already-short buffer.

The race lives in the rich-prompt open path, not the dispatch
path:

1. User is interacting with a terminal. xterm's
   `xterm-helper-textarea` (the hidden textarea xterm.js uses to
   capture keystrokes and IME composition) is the active element.
2. User hits Alt+Space (or Cmd+K p). `App.svelte::onWindowKey`
   sees the chord, calls `openActiveTerminalRichPrompt()`, sets
   `prompt.open = true`, bumps `focusNonce`, and `preventDefault`s
   the chord keystroke. Returns.
3. Svelte flushes the open-state update in a microtask. The
   `TerminalRichPrompt` renders, the `{#if mode() === "wysiwyg"}`
   block instantiates the editor child, the child's `onMount`
   runs, the CodeMirror `EditorView` is built and `view.focus()`
   fires.
4. The rich-prompt's own `$effect` watching `focusNonce` also
   runs after `tick()`, calling `wysiwygRef?.focusEnd()` /
   `sourceRef?.focusAt(...)` as a belt-and-suspenders refocus.

Between steps 2 and step 3's `view.focus()`, the previously-
focused `xterm-helper-textarea` is still the keyboard target. A
fast typer who starts on 'e' before that microtask flushes:

* keydown fires on `xterm-helper-textarea`;
* `term.attachCustomKeyEventHandler(handleTerminalKeyEvent)`
  returns true (plain lowercase 'e' is not a meta chord);
* xterm processes the key and emits via `term.onData`;
* `sendUserInput("e")` -> the live PTY via WebSocket.

User sees 'e' echoed in the terminal grid. Then focus moves to
the editor. They keep typing 'cho hello' into the rich prompt
(buffer = "cho hello"). Cmd+Enter dispatches "cho hello". The
terminal already has 'e' from step 4; bash sees the combined
`echo hello\n` and runs the command correctly, so the shell side
"works" -- but the prompt's recorded input is short the first
character, which is exactly what @@WebtestA observed. The race
is short and only fires when typing starts inside the same JS
turn or microtask window as the open chord, which is why the bug
is "intermittent".

Fix: blur the active element at the top of
`openActiveTerminalRichPrompt()` when it belongs to an xterm
surface. Keystrokes typed during the focus race land on `<body>`
(no handler -- silently dropped) instead of feeding the PTY.
Once the editor mounts and focuses, typing resumes into the
prompt.

Scoping the blur to xterm-owned elements (matching
`.xterm-helper-textarea` or any descendant of an `.xterm`
ancestor) means non-terminal callers (an editor or a search
input that ever triggers the helper) keep their focus until
the prompt's own focus effect lands. We only neutralise the
specific "keystroke leaks to PTY" path that motivated the bug.

What this fix does NOT address:

* The original wysiwyg-mode dispatch path is independently
  broken: `TerminalRichPrompt` doesn't pass an `onSubmit` to
  the `<Wysiwyg>` child, so the Wysiwyg keymap's
  `{ key: "Mod-Enter", run: () => { onSubmit?.(); return true; } }`
  consumes Cmd+Enter without doing anything. The dispatch path
  works in source mode because Source's keymap has no Mod-Enter
  binding and the event bubbles to the wrapper. This is a
  separate bug -- flagging in the journal so @@Architect can
  cut a task if @@Alex hits "Cmd+Enter does nothing in wysiwyg
  mode".
* `fullstack-a-17` ("Cmd+K -> p (spawn terminal) steals
  rich-prompt input focus to xterm-helper-textarea") is a
  related but distinct focus race in the *spawn* path. This
  fix lands on the *open* path. @@FullStackA's task will handle
  the spawn-side focus separately; the blur helper is reusable
  if needed.

Files touched:

* `web/src/state/tabs.svelte.ts` -- `openActiveTerminalRichPrompt`
  calls a new private `blurTerminalHelperTextarea()` before
  setting `prompt.open`. The helper checks
  `document.activeElement`, matches `xterm-helper-textarea` or
  any ancestor with `.xterm`, and `.blur()`s.
* `web/src/state/tabs.test.ts` -- two new tests pin the new
  behaviour: one mounts a fake `.xterm > .xterm-helper-textarea`,
  focuses it, asserts blur after `openActiveTerminalRichPrompt`;
  the other mounts a plain `<input>`, focuses it, and asserts
  the focus is preserved (scope check).

Tests:

* `web/src/state/tabs.test.ts` 104/104 (was 102; +2 new tests).
* Full SPA vitest: 477/477 (was 475 baseline from -7).

Pre-push gate green:
* `cargo fmt --all -- --check` -- clean.
* `cargo clippy --workspace --all-targets -- -D warnings` -- clean.
* `cargo test --workspace` -- every suite passes.
* `cargo build --workspace --no-default-features` -- clean.
* `npm run check` (svelte-check) -- 0 errors, 0 warnings.
* `npx vitest run` -- 477/477.
* `npm run build` -- clean.

## 2026-05-20 - commit readiness

Files changed (proposed single commit):

* `web/src/state/tabs.svelte.ts` -- new `blurTerminalHelperTextarea`
  helper + call from `openActiveTerminalRichPrompt`.
* `web/src/state/tabs.test.ts` -- two new pinned tests covering
  the blur + scope-preservation behaviour.

Tests run: full pre-push gate green (see implementation note).

Known risks: the blur is best-effort during the SSR-free runtime
(`typeof document !== "undefined"` guard for the test setup that
doesn't always have one). If the user's bug repro path doesn't
match the xterm-helper-textarea theory (e.g. focus is already on
the editor and the buffer drop happens for another reason), this
fix won't move the needle. Lane-B walkthrough verdict will
confirm.

Push waits for Round-1 close per the standing rule.

Proposed commit subject:
`Blur xterm-helper-textarea before opening rich prompt so racing keystrokes don't leak to PTY (fullstack-b-8)`

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Outstanding root-cause work. The reframe from "first
char of the dispatched text disappears" → "the buffer
itself is missing the first char because that keystroke
leaked to the PTY before the rich prompt mounted" is the
load-bearing piece. The end-to-end trace through
`onWindowKey` → `prompt.open=true` → microtask → child
`onMount` → `view.focus()` → parent `$effect` is exactly
the right shape for explaining an "intermittent" race —
the intermittency follows the gap between the open chord
keydown and the editor's first focus call, which is
exactly what "depends on typing speed within the same JS
turn" would predict.

Fix at the right layer: blur the xterm helper textarea at
the OPEN boundary rather than chasing the symptom at the
dispatch layer. Scoping the blur to xterm-owned elements
(matching `.xterm-helper-textarea` or descendants of an
`.xterm` ancestor) is the right tightening — a future
caller from a non-terminal context (e.g. focus is on a
search input) won't get its focus stomped.

Two new tests pin both the blur behaviour AND the scope
preservation. Good belt + suspenders.

Pre-push gate green across the full stack (vitest 477/477,
+2 from baseline; cargo workspace test + clippy + fmt +
no-default-features + check + build). Clean.

**Commit clearance**: approved. Use your proposed commit
subject as-is. Push waits for Round-1 close.

### Follow-up tasks from your flags

* **Wysiwyg-mode Cmd+Enter dispatch silently broken**:
  cutting [../fullstack-a/fullstack-a-18.md](../fullstack-a/fullstack-a-18.md)
  against @@FullStackA. The Wysiwyg keymap consumes
  Cmd+Enter via `onSubmit?.()` but `TerminalRichPrompt`
  never threads the prop; source-mode works only because
  Source has no Mod-Enter binding and the event bubbles
  to the wrapper. This is a real "Cmd+Enter does nothing
  in wysiwyg mode" bug Alex hasn't hit yet because the
  default mode lands on Source.
* **`fullstack-a-17` is the spawn-side race**, distinct
  from this open-side race. Your `blurTerminalHelperTextarea`
  helper is reusable; flagged for @@FullStackA in the
  -17 task as a callable when they investigate.

Carry on with `fullstack-b-10` (b-3 partial-fix call-site
flip) next, then `fullstack-b-9` (Cmd+T web alternate).