# fullstack-a-94 — Alt+Space PARTIAL follow-up: remove the 3rd handler in TerminalTab.handleTerminalKeyEvent

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Remove the third Alt+Space handler in
`TerminalTab.svelte`'s `handleTerminalKeyEvent`
(the xterm `customKeyEventHandler` path). `-a-90`
removed 2 handlers (App.svelte global + TerminalTab
onShellKeydown) but missed this third one — so
Alt+Space STILL opens the rich prompt when fired
from a focused terminal.

## Reference

@@WebtestA's proactive walk (`aed06ef`): empirical
test shows Alt+Space still opens the rich prompt
from a focused terminal. Audit identified the
missed handler at
`TerminalTab.svelte::handleTerminalKeyEvent`.

`-a-90`'s audit-grep caught the 2 keymap-driven
handlers but didn't surface the xterm-handler
path because the chord-detection lives inside the
`KeyboardEvent → xterm-customKeyEvent` translation
layer rather than as a top-level keymap branch.

## Fix shape

1. Audit `TerminalTab.svelte::handleTerminalKeyEvent`
   for the Alt+Space branch (around the xterm
   `customKeyEventHandler` registration).
2. Remove the branch.
3. Update any stale comment that claims the
   chord is live.

## Acceptance

1. **Alt+Space fired from focused terminal does
   nothing** (or whatever browser default fires,
   likely no-op).
2. **Cmd+P / Cmd+Alt+P from focused terminal** —
   gated on `-a-91`'s chord escape work; if that
   hasn't shipped yet, this test stays the same
   as today.
3. **No regression** on terminal typing behavior.

### Tests

Vitest pin asserting the Alt+Space branch is GONE
from `handleTerminalKeyEvent`.

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA. SPA-only.
* Atomic-audit-commit.
* Tiny — ~5-10 LOC removal.

## Authorization

Yes for `web/src/components/TerminalTab.svelte` +
test + task tail + outbound.

## Numbering

This is `-a-94`.

## Out of scope

* `-a-91` chord escape work (separate task).
* Adding new chords.
