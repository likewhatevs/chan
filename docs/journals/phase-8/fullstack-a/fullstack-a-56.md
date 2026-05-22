# fullstack-a-56 — Cmd+P 3-state contract + depth-slider shallow-scope discoverability (bundled UX polish)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Two small UX papercuts bundled in one commit:

1. **Cmd+P 3-state contract** — fix the rich-prompt chord
   so it follows the active tab + supports toggle-to-hide
   on re-press.
2. **Depth slider discoverability on shallow scopes** —
   visual cue when slider max=1 already reveals
   everything forward-reachable.

## Reference

* Cmd+P bug: [`../phase-8-bugs.md`](../phase-8-bugs.md)
  "Cmd+P (rich prompt) doesn't honor the active-tab + doesn't toggle-to-hide on re-press"
  — full 3-state contract + audit-confirmed root causes
  + fix shape (single function rewrite of `showOrSpawnRichPromptInFocusedPane`)
  + 3 test pins.
* Depth slider observation: from `webtest-a-6` verdict
  (`cf383d8`) — "slider max can be misleading for shallow
  scopes (no visual cue that depth=1 already reveals
  everything forward-reachable)". Small visual cue (disable
  the slider OR show "[max]" label OR similar) when scope's
  depth-cap is hit at value 1.

## Acceptance

### Cmd+P 3-state

* Current tab IS a terminal + prompt NOT showing → open
  prompt on current terminal.
* Current tab IS a terminal + prompt IS showing → HIDE
  the prompt (toggle off).
* Current tab is NOT a terminal → spawn a fresh terminal
  + open prompt.
* 3 vitest pins per the bug body.

### Depth-slider discoverability

* When the scope's depth-cap is 1 (e.g. a single-file
  graph that has only one forward hop), the slider
  shows a clear visual that depth-1 already reveals
  the full scope. Implementer picks the cue — could be
  a `[max]` suffix label, disabling the slider when
  max=1, or a one-line caption.

### Gate

* `npm test -- --run` green.
* `npm run check` 0e/0w.
* `npm run build` clean.

## Coordination

* @@FullStackA lane. SPA-only.
* Atomic-audit-commit discipline per memory rule.

## Authorization

**Yes** for `web/src/state/tabs.svelte.ts`,
`web/src/components/GraphPanel.svelte`, the relevant test
files, plus the task tail + outbound.

## Numbering

`-a-55` is highest cut. This is `-a-56`.
