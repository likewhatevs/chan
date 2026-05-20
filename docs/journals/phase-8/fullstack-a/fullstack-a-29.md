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
