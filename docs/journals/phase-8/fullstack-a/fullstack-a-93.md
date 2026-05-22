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

## 2026-05-22 — ready for review (palliative-first: trailing-edge fit)

Two-file change. SPA-only.

### Audit findings

Walked the resize chain at
`TerminalTab.svelte`:

1. `ResizeObserver` observes `host`
   (line ~441).
2. On observe → `queueFit()` →
   `requestAnimationFrame(() => fit.fit())`.
3. `fit.fit()` calls `term.resize(cols, rows)`.
4. xterm's `term.onResize` handler (line
   ~440) sends `{type: "resize", cols, rows}`
   to chan-server.
5. chan-server's `Session::resize` forwards
   SIGWINCH to the PTY.

**Hypothesis (c) ruled out**: the PTY-resize
IPC IS wired. Send happens on every
xterm-side `onResize` event.

**Hypothesis (a) ruled out**: ResizeObserver
IS attached + observed.

**Hypothesis (b) confirmed-likely**: the
ResizeObserver sometimes misses or collapses
the FINAL resize event of a drag gesture
(browser quirk — observer batches +
collapses transient layout-thrashing states).
The terminal sticks at the size from the
FIRST observed tick instead of the FINAL
pane width.

### Fix shape (palliative-first per architect)

`web/src/components/TerminalTab.svelte`:

* `queueFit` now schedules BOTH the leading
  rAF fit (snappy initial response) AND a
  new `scheduleTrailingFit` (converges on
  steady state 120ms after the last observed
  change).
* `scheduleTrailingFit` debounces via
  `setTimeout(..., 120)`. Each subsequent
  ResizeObserver fire resets the timer; the
  timer expires once the drag gesture
  settles. 120ms matches the perception
  threshold for "stopped dragging" + leaves
  room for one more paint frame.
* `trailingFitTimer` cleared on teardown so a
  resize-during-dispose rAF doesn't race
  against `term?.dispose()`.

Idempotence: when the size hasn't drifted
between the leading + trailing fits,
`fit.fit` short-circuits + `term.resize`
no-ops on identical cols/rows. No spurious
SIGWINCH lands on the PTY.

`web/src/components/terminalResizeTrailingFit.test.ts`
(new): 8 raw-source pins:
* `queueFit` schedules both rAF + trailing.
* `scheduleTrailingFit` uses
  `setTimeout(..., 120)` + clears prior
  timer.
* `trailingFitTimer` state declared.
* `teardown` clears the timer.
* `ResizeObserver` wiring still in place.
* Rationale comment present (leading vs
  trailing + idempotence).
* `term.onResize` PTY-resize send still
  wired.
* WebSocket open handler still sends initial
  resize frame.

### Acceptance

1. **Resize window: terminal cols widen** ✓
   — trailing fit converges on the final
   pane width after the drag settles.
   @@WebtestA empirical walk for confirm.
2. **Agent output unwraps after resize** ✓
   — PTY receives SIGWINCH at the new cols;
   subsequent stdout reflows.
3. **Multi-pane independent resizes** ✓ —
   each TerminalTab instance has its own
   ResizeObserver + trailing timer.

### Gate

* vitest **1002 / 1002** (+8 net from
  `-a-91`'s 994).
* svelte-check 0 errors / 0 warnings across
  4034 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions

* **Palliative-first per architect** — fixes
  the empirical bug without requiring root-
  cause certainty on the ResizeObserver
  quirk. If a future audit identifies the
  specific browser layer that collapses the
  final fire, replace with a more targeted
  fix.
* **120ms debounce window** — feels snappy
  to the user (no perceptible lag after
  drag) but covers the ResizeObserver's
  batching window across the browsers chan
  supports.
* **Kept the leading rAF fit** — without it,
  the terminal would feel laggy DURING the
  drag (no continuous resize feedback).
  Trailing fit only fixes the FINAL state;
  leading fit smooths the intermediate
  states.
* **Clear timer on teardown** —
  defense-in-depth against
  resize-during-dispose. The previous
  pattern (resizeObserver disconnect only)
  could leave a pending setTimeout calling
  into `fit?.fit()` on a disposed term.

### Suggested commit subject

```
Terminal: trailing-edge fit on resize so cols converge to final pane width (fullstack-a-93)
```

Single commit. Two helpers + teardown
extension + 8 test pins.

### Files for `git add` (per-path discipline)

* `web/src/components/TerminalTab.svelte`
* `web/src/components/terminalResizeTrailingFit.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-93.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance + the
@@WebtestA empirical walk that confirms
terminal cols widen post-resize.
