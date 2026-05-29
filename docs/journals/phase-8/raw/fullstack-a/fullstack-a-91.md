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

## 2026-05-22 — ready for review

Three-file change. SPA-only. Both lanes
bundled (registry + TerminalTab consult).

### What landed

`web/src/state/shortcuts.ts`:
* New optional `escapeTerminal?: boolean`
  field on the `Shortcut` type. Doc-comment
  explains the contract +
  `attachCustomKeyEventHandler` semantics.
* Flagged App-group chords: `app.settings.toggle`,
  `app.terminal.richPrompt`, `app.files.toggle`,
  `app.graph.toggle`, `app.terminal.toggle`,
  `app.window.reload`, `app.draft.new`.
* New `chordFromEvent(e)` helper — normalises
  a raw `KeyboardEvent` into the registry's
  chord shape (`Mod+P`, `Cmd+Alt+P`, etc.).
  Modifier-only / printable-key events return
  null.
* New `shouldEscapeTerminal(e)` helper —
  consults the registry for `escapeTerminal:
  true` entries, matches via
  `canonicalChordTokens` so `Mod+P` (event on
  Mac) === `Cmd+P` (registry literal) +
  `Mod+Alt+P` === `Cmd+Alt+P` on Mac. Returns
  false on Linux/Windows for `Cmd+...` entries
  since most keyboards don't have a Cmd key
  there.

`web/src/components/TerminalTab.svelte`:
* `chordFor` import extended to include
  `shouldEscapeTerminal`.
* `handleTerminalKeyEvent` (xterm
  `customKeyEventHandler` callback at line
  ~1001) now calls `shouldEscapeTerminal(e)`;
  on true returns `false` so xterm leaves the
  event alone + App.svelte's window-level
  keymap handles it.
* Inline rationale comment cites `-a-91`,
  the registry contract, and the consequence
  of NOT escaping (chord gets swallowed by
  xterm + written to PTY as escape sequence).

`web/src/state/chordEscapeRegistry.test.ts`
(new): 15 raw-source + behavioural pins:
* Shortcut type carries `escapeTerminal?`.
* 7 expected App-group entries are flagged.
* Non-flagged entries (tab close, Hybrid NAV
  entry chord) default to undefined.
* `chordFromEvent` normalises Mac chord
  shapes; modifier-only + plain-key events
  return null.
* `shouldEscapeTerminal` matches Cmd+Alt+P
  (web Mac), Cmd+R, Cmd+Shift+M, Cmd+,
  (all escape).
* Plain alphabet keys + Ctrl+D (tab.close,
  not flagged) DO NOT escape.
* TerminalTab consults the helper + the
  rationale comment is present.

### Acceptance

1. **Cmd+P from focused terminal opens rich
   prompt** ✓ — chord-escape registry
   matches; xterm returns false; App.svelte
   handles. @@WebtestA empirical walk for
   confirmation.
2. **Cmd+R reloads window** ✓ —
   `app.window.reload` flagged.
3. **Cmd+Shift+M opens graph** ✓ —
   `app.graph.toggle` flagged.
4. **Non-registered chords (typing)** work
   unchanged ✓ — `shouldEscapeTerminal`
   returns false for printable keys.

### Gate

* vitest **994 / 994** (+22 net from
  `-a-94`'s 972 — +15 new pins in
  chordEscapeRegistry.test.ts plus the
  full-suite running cleanly under
  --no-isolate).
* svelte-check 0 errors / 0 warnings across
  4033 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions

* **Bundled both lanes** in one commit — the
  registry shape extension + the
  TerminalTab consultation are tightly
  coupled. Splitting would leave a
  half-shape live in HEAD. @@FullStackB
  doesn't need to pair separately for this
  one; lane boundary was advisory.
* **Cross-platform `Cmd`/`Mod` aliasing**
  via the token-set comparison — registry
  uses `Mod+P` for native + `Cmd+Alt+P` for
  web Mac fallback. On Mac both expand to
  the Cmd key; the matcher treats them as
  equivalent.
* **Web-platform Cmd+P doesn't escape** —
  browser owns it for the print dialog;
  the SPA never sees it. Web Mac users use
  Cmd+Alt+P (which DOES escape per the
  flagged entry).
* **Did NOT flag tab navigation chords** —
  `app.tab.next/prev/close/jump` aren't
  flagged. Those go through different
  dispatch paths (Ctrl+D for close,
  Alt+Shift+[/] for prev/next, Ctrl+Alt+1..9
  for jump) that don't conflict with xterm's
  Alt-prefixed escape sequences enough to
  warrant an escape gate. If empirical
  walks surface a regression, add per-entry.

### Suggested commit subject

```
Terminal: chord-escape registry — global App chords bubble out of xterm focus (fullstack-a-91)
```

Single commit. Registry shape + matcher +
TerminalTab consultation + 15 test pins.

### Files for `git add` (per-path discipline)

* `web/src/state/shortcuts.ts`
* `web/src/components/TerminalTab.svelte`
* `web/src/state/chordEscapeRegistry.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-91.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance + the
@@WebtestA empirical walk that confirms
global chords surface from focused-terminal
context.
