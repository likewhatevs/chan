# fullstack-a-93 — Terminal columns don't widen after pane/window resize (PTY stays at old cols)

Owner: @@FullStackA (primary; possible cross-lane to @@FullStackB if Tauri-side)
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Terminal column count tracks the visible pane width
after resize. Today's behavior: terminals retain
pre-resize cols → agent output wraps narrowly even
when the visible terminal is wider.

## Reference

[`../phase-8-bugs.md`](../phase-8-bugs.md) §"Terminal
columns don't widen after pane / window resize"
(line ~648).

3 hypotheses per bug-list:

a) **`xterm.fit()` not called on resize**: SPA
   ResizeObserver / pane-resize handler doesn't
   trigger fit() reliably.
b) **`fit()` runs but xterm internal state stale**:
   timing — observer fires before layout settles.
c) **PTY resize call missing**: fit() works locally
   but doesn't propagate cols/rows to chan-server's
   `Session::resize` (which forwards SIGWINCH).

## Audit path

1. Trigger window resize + log:
   * Pane width before/after.
   * xterm cols before/after.
   * `Session::resize` IPC call (or absence).
2. Identify which of a/b/c is the gap.
3. Fix per audit.

Likely (c) — most reported "terminal stuck at narrow"
issues are missing SIGWINCH propagation in newer
terminal stacks.

## Acceptance

1. Resize window: terminal cols widen to match new
   pane width.
2. Agent output unwraps after resize.
3. Multi-pane: each pane resizes independently.

### Tests

Vitest pin on the resize → `Session::resize` IPC
call path.

### Gate

`npm test` / `check` / `build` green.

## Cross-lane

@@FullStackA primary (SPA xterm.fit + resize event
handling). If audit reveals Tauri-side WebView
resize event quirk, cross to @@FullStackB.

## Authorization

Yes for `web/src/components/TerminalTab.svelte` +
resize handler / IPC plumbing + tests + task tail +
outbound.

## Numbering

This is `-a-93`.
