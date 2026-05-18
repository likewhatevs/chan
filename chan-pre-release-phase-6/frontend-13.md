# frontend-13: terminal keymap — Cmd/Ctrl+Enter and adjacent chords

Owner: @@Frontend
Status: REVIEW

## Goal

Close the modifier-Enter chord gap in the embedded terminal so
claude / codex / similar TUIs receive the right key bytes. Today
only Shift+Enter is mapped
(`web/src/terminal/keymap.ts`); Cmd+Enter (macOS) and
Ctrl+Enter (Linux/Windows) fall through to plain Enter, which
breaks the "submit / newline / send" gestures those CLIs use.

Alex needs this working before they can dig back into the live
test service for end-to-end verification.

## Scope

* Add Cmd+Enter (macOS) and Ctrl+Enter (other platforms) chords
  next to the existing Shift+Enter handler.
* Match the byte sequence the modern Anthropic / OpenAI CLIs
  expect. Most CLIs read the modifier-Enter as a distinct key
  via the kitty / fixterms enhanced-keyboard protocol that
  [backsystacean-1](./backsystacean-1.md) already enabled.
  Use the corresponding CSI-u encoding:
  * `Ctrl+Enter` → `\x1b[13;5u`
  * `Cmd+Enter` (interpreted as Meta on macOS through
    `macOptionIsMeta`-style handling) → `\x1b[13;9u` if the
    CLI consumes Meta; otherwise fall back to the
    Ctrl+Enter byte sequence on macOS as well so a single
    chord works cross-CLI.
  * Confirm against a live CLI on @@WebtestA's service before
    commit; if the byte sequence needs adjusting, prefer the
    one the user-typed CLI actually responds to.
* Audit `etc` chords while in there:
  * Alt+Enter — distinct on some CLIs; consider mapping if
    used.
  * Shift+Tab — readline backward-completion; check whether
    today's keymap forwards it correctly.
* Add a regression test in
  `web/src/terminal/keymap.test.ts` for each new chord.

## Out of scope

* Configurable bindings (deferred; phase 6 ships fixed
  mappings).
* xterm.js-level chord rebinding for non-Enter combinations
  outside of those listed.

## Relevant links

* Original ask: [request.md](./request.md) bugs / nits section
  (Shift+Enter was the original ask; Cmd+Enter is the wrap
  follow-up Alex flagged).
* Existing keymap: `web/src/terminal/keymap.ts`.
* Tests: `web/src/terminal/keymap.test.ts`.
* CSI-u reference: kitty keyboard protocol / xterm modifyOtherKeys.

## Acceptance criteria

* Cmd+Enter on macOS and Ctrl+Enter on Linux/Windows reach the
  shell / TUI as a distinct key, not as plain Enter.
* Plain Enter and Shift+Enter behavior unchanged.
* Regression tests cover the new byte sequences.
* `npm --prefix web run check`, `npm --prefix web test -- --run`,
  `npm --prefix web run build` green.

## Review and hardening

* @@WebtestA live verification with a real CLI (claude / codex)
  on the rebuilt test service before commit.

## Progress notes

* 2026-05-18: Added CSI-u mappings in `web/src/terminal/keymap.ts`:
  Shift+Enter remains `\x1b[13;2u`, Ctrl+Enter now sends
  `\x1b[13;5u`, and Cmd/Meta+Enter sends `\x1b[13;9u`.
* 2026-05-18: Left Alt+Enter and Shift+Tab to xterm default
  handling; no verified CLI binding in this lane required changing
  them.
* 2026-05-18: Added keymap regression coverage for Ctrl+Enter,
  Cmd/Meta+Enter, combined modifier rejection, and Alt+Enter
  fallthrough.

## Completion notes

Ready for review. Validation:

* `npm --prefix web run check`
* `npm --prefix web test -- --run`
* `npm --prefix web run build` (passes with existing Vite chunk-size,
  ineffective dynamic import, and plugin timing warnings)
