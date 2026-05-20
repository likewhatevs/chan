# fullstack-a-29: Rich prompt collapse — terminal-host margin recompute on collapse transition

Owner: @@FullStackA
Date: 2026-05-20

## Goal

When the rich prompt collapses via the `v` chevron from
`fullstack-a-24`, the `.terminal-host` margin-bottom does
not recompute to reflect the new collapsed-pill height. The
terminal stays at its expanded-state height, leaving a tall
empty band between the terminal output and the collapsed
pill (visible in @@Alex's 2026-05-20 screenshot).

Wire the margin-recompute path from `fullstack-a-4` to fire
on the collapse/expand transition introduced by
`fullstack-a-24`. On collapse: `margin-bottom = collapsed-pill-height + 12px`,
xterm.js ResizeObserver re-fits the terminal downward. On
expand: restore the existing `heightPx + 12px` path. Both
transitions feed the same recompute reactor.

## Background

Bug entry: [`../phase-8-bugs.md`](../phase-8-bugs.md) "Rich
prompt collapse / expand chevrons leave dead vertical space
under the terminal".

`fullstack-a-4` introduced the dynamic
`.terminal-host { margin-bottom: heightPx + 12 + "px" }`
binding so the rich prompt's open transition pushes the
terminal up cleanly. `fullstack-a-24` added the
collapse/expand chevron + `prompt.collapsed: boolean` state
but the margin-recompute path was wired to open/close +
height-resize transitions only, not the new collapsed-state
transition. xterm.js's ResizeObserver doesn't re-fire on the
collapse transition because the effective height change isn't
reaching the .terminal-host's margin binding.

Likely single-file scope: `web/src/components/TerminalRichPrompt.svelte`
and / or `web/src/components/TerminalTab.svelte` depending on
where the binding lives. Look for the
`fullstack-a-4`-commented `.terminal-host` margin reactor +
the `fullstack-a-24` collapsed-state plumbing.

## Acceptance criteria

* Collapsing the rich prompt: terminal output grows
  downward; bottom of terminal sits ~12 px above the
  collapsed pill. No dead band.
* Expanding the rich prompt: behaviour unchanged from
  `fullstack-a-4` — terminal shrinks to make room for the
  expanded composer.
* Resizing the expanded prompt vertically: behaviour
  unchanged.
* xterm.js fits cleanly in both directions (no cut-off
  lines, no scroll-state weirdness on the transition).
* `prefers-reduced-motion`: if any transition is animated,
  honour the reduce-motion path.
* `vitest` green; pin the collapsed-state margin formula
  if it's testable in the SerTab / DOM level.

## How to start

1. Reproduce: spin up a test server, open a terminal, open
   the rich prompt (Alt+Space), click the `v` chevron.
   Observe the dead band above the collapsed pill.
2. Read the `fullstack-a-4` margin reactor + the
   `fullstack-a-24` collapsed-state code. The fix is most
   likely "include `prompt.collapsed` in the margin
   reactor's dependency set + branch to the collapsed-pill
   height when true."
3. Verify the collapsed pill's actual rendered height (in
   the floating-pill design from `fullstack-a-24`) — that's
   the constant the recompute path needs.

## Coordination

* Sits in the same rich-prompt surface as
  [`fullstack-a-28`](fullstack-a-28.md) (BubbleOverlay
  regression) — likely no file conflict but coordinate the
  commit ordering so the working tree stays clean.
* Hard gate before the broader rich-prompt session-evolution
  work; the history backlog band needs accurate
  collapsed-state heights.
* @@WebtestA verifies on lane-A once landed.

## 2026-05-20 — implementation note + ready for review

Root cause exactly as the task spec described: the
`fullstack-a-4` margin reactor on `.terminal-host` reads
`tab.richPrompt.heightPx`, which is the user-resized
EXPANDED height. On collapse, CSS `height: auto` shrinks
the visible prompt to header-only (~44 px) but
`heightPx` stays at its old expanded value, so the
reserved-space band over-reserves and leaves the dead
band @@Alex caught.

### Approach

Introduced a non-persisted `measuredHeightPx?: number`
field on `TerminalRichPromptState`. A new `$effect` in
`TerminalRichPrompt.svelte` attaches a `ResizeObserver`
to the prompt's `rootEl`; every layout tick writes
`Math.round(entry.contentRect.height)` into the field.
The terminal-host margin formula in `TerminalTab.svelte`
prefers `measuredHeightPx` over `heightPx`, falling back
to the existing `320` default for the brief mount
window before the first observer tick fires.

ResizeObserver is the right source of truth here: it
fires on the collapse transition AND on the expanded
drag-resize AND on viewport-driven max-height clamps
(the `max-height: calc(100% - 48px)` rule), so the
margin tracks ALL paths uniformly. Drag-resize already
updates `heightPx` synchronously and CSS uses that for
`height:`, so on the expanded path the observer simply
mirrors `heightPx` one frame later — no behaviour drift.

### Why not the alternatives

* **Constant collapsed-pill height (~44 px)**: brittle
  under future header chrome changes (StyleToolbar,
  collapse chevron, future control icons). The
  observer auto-adapts.
* **Mutate `heightPx` directly on collapse**: would
  clobber the user's persisted expanded-height
  preference; expanding would land at the wrong size.
* **Separate `expandedHeightPx` + `collapsedHeightPx`
  fields**: more state to round-trip; the observer is a
  single source of truth that subsumes both.

`measuredHeightPx` is deliberately NOT persisted to
SerTab — it repopulates within one observer tick of
remount, and persisting would introduce stale-on-restore
risk.

### Files touched

* `web/src/state/tabs.svelte.ts`
  * `TerminalRichPromptState`: new `measuredHeightPx?:
    number` field (non-persisted; runtime-only).
* `web/src/components/TerminalRichPrompt.svelte`
  * New `$effect` registers a `ResizeObserver` on the
    prompt's `rootEl`. Bails cleanly when
    `ResizeObserver` is undefined (jsdom test env).
* `web/src/components/TerminalTab.svelte`
  * Margin formula prefers `measuredHeightPx` over
    `heightPx`. Comment refreshed.

### Test pin

Skipped a vitest pin: the collapse formula lives in the
template-side ternary (style attribute), and
`ResizeObserver` isn't available in jsdom — the field
would never populate in unit tests. The behavioral
contract requires a real browser layout pass.
@@WebtestA's lane-A walkthrough is the authoritative
verification per the task body.

The component-level wiring IS exercised by the existing
TerminalRichPrompt + TerminalTab tests (gate
unchanged: 512/512).

### Gate

* `vitest`: 512/512 (no regression).
* `svelte-check`: 0 errors / 0 warnings / 3974 files.
* `npm run build`: clean.
* Rust gate: no Rust changes; not run.

### Suggested commit subject

`Rich prompt: ResizeObserver-driven margin reactor for collapse + drag-resize parity (fullstack-a-29)`

### Cross-lane coordination

No conflicts with `fullstack-b-13` — different files
entirely. -a-29 stays in `tabs.svelte.ts`'s
`TerminalRichPromptState` + `TerminalRichPrompt.svelte`
mount-time effects + `TerminalTab.svelte`'s style
attribute. -b-13 touches header toolbar + chan-server
`terminal_sessions.rs`.
