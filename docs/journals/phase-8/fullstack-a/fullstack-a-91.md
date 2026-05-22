# fullstack-a-91 — Chord escape registry: terminal lets global chords bubble to App-level

Owner: @@FullStackA (primary; cross-lane to @@FullStackB)
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Add chord-escape registry shape so global chords
(Cmd+P / Cmd+Alt+P / Cmd+R / etc.) bubble out of
terminal focus to App-level handlers instead of
being consumed by xterm.

## Reference

[`../phase-8-bugs.md`](../phase-8-bugs.md) §"Global
chords swallowed by terminal focus" (line ~205).

Root cause: `handleTerminalKeyEvent` (the
`attachCustomKeyEventHandler` callback in
`TerminalTab.svelte`'s xterm) returns true for the
chord; xterm consumes + writes escape sequence to
PTY; App-level keydown never fires.

## Fix shape

Per bug-list framing:

1. **Registry source-of-truth** in `shortcuts.ts`:
   each global chord entry carries an
   `escapeTerminal: boolean` field. Default true
   for all global chords (Cmd+P, Cmd+R, Cmd+Shift+M,
   etc.).
2. **`handleTerminalKeyEvent` consults the registry**:
   for incoming `KeyboardEvent`, derive chord shape,
   look up. If matched + `escapeTerminal: true` →
   return `false` (don't consume; let it bubble).
   Otherwise current behaviour.

## Cross-lane scope

* @@FullStackA: registry shape extension in
  `shortcuts.ts`.
* @@FullStackB: `TerminalTab.svelte`'s xterm
  handler consults the registry.

@@FullStackA primary; can bundle the
TerminalTab.svelte change or scope-poke for
@@FullStackB to pair.

## Acceptance

1. Cmd+P fired from focused terminal opens rich
   prompt (currently fails; xterm consumes).
2. Cmd+R fired from focused terminal reloads
   window.
3. Cmd+Shift+M fired from focused terminal opens
   graph.
4. Non-registered chords (typing in terminal)
   work as today.

### Tests

Vitest pins on the registry shape + the
`handleTerminalKeyEvent` consultation path.

### Gate

`npm test` / `check` / `build` green.

## Authorization

Yes for `web/src/state/shortcuts.ts` +
`web/src/components/TerminalTab.svelte` + tests +
task tail + outbound.

## Numbering

This is `-a-91`.
