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

## 2026-05-22 — ready for review

Two-file change. SPA-only.

### What landed

`web/src/components/TerminalTab.svelte`:
* Removed the Alt+Space branch from
  `handleTerminalKeyEvent` (line ~986). The
  function now flows straight from the
  exit-tab-from-key guard into
  `handleTerminalMetaKey(e, sendUserInput)`.
* Replacement comment cross-references `-a-94`,
  the audit-grep miss, and the empirical catch
  (`aed06ef`).

`web/src/components/altSpaceXtermHandlerRemoved.test.ts`
(new): 4 raw-source pins:
* No Alt+Space branch in
  `handleTerminalKeyEvent`.
* Function still defers to
  `handleTerminalMetaKey` (sanity check on
  the legitimate meta-key delegation).
* Removal-rationale comment present.
* `attachCustomKeyEventHandler` registration
  still in place (untouched mechanism; we
  only removed the chord branch inside the
  registered handler).

### Lesson logged

`-a-90`'s audit-grep covered the top-level
keymap branches but missed the xterm
`customKeyEventHandler` translation layer.
The chord lived inside the `KeyboardEvent →
xterm-customKeyEvent` translator (which
runs before xterm's own keystroke
processing), so a `grep "altKey.*Space"`
across keymap files alone wouldn't surface
it. Future chord-removal audits need to
include `attachCustomKeyEventHandler` paths.

### Acceptance

1. **Alt+Space from focused terminal does
   nothing** ✓ — branch removed (pinned by
   test). @@WebtestA re-walk for empirical
   close.
2. **Cmd+P / Cmd+Alt+P from focused
   terminal** — still gated on `-a-91`'s
   chord escape work; this task doesn't
   change that.
3. **No regression on terminal typing** ✓
   — `handleTerminalMetaKey` delegation
   preserved.

### Gate

* vitest **972 / 972** (+4 net from `-a-90`'s
  968).
* svelte-check 0 errors / 0 warnings across
  4032 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions

* **Removed the branch outright** — no
  feature-flag rollback; the chord retire
  is a deliberate UX call.
* **Kept the `attachCustomKeyEventHandler`
  registration** — that's the legitimate
  mechanism for terminal-side
  chord-escape work (-a-91); only the
  Alt+Space chord branch is gone.

### Suggested commit subject

```
Rich prompt: remove 3rd Alt+Space handler in xterm custom-key path (fullstack-a-94)
```

Single commit. One branch removal + 4 test
pins.

### Files for `git add` (per-path discipline)

* `web/src/components/TerminalTab.svelte`
* `web/src/components/altSpaceXtermHandlerRemoved.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-94.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance + the
@@WebtestA empirical re-walk that closes the
`-a-90` PARTIAL.
