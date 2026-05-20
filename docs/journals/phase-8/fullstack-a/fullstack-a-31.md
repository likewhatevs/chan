# fullstack-a-31: Terminal broadcast selector — include self + checkbox shape

Owner: @@FullStackA
Date: 2026-05-20

## Goal

Small UX polish on the terminal's broadcast-input selector
(the UI that picks which terminal tabs receive the
broadcast forwarded by `broadcastTerminalInput`).

Three changes:

1. **Include self in the list**. The current tab is missing
   from the selectable list today. Add it. Mark as "self"
   either with an icon next to the label OR by placing it
   above the others with a separator (implementer's call;
   both shapes are acceptable. Pick whatever reads
   cleaner in the existing visual language).

2. **Checkbox shape, not toggle**. Drop the on/off rocker
   UI for the per-tab broadcast state — @@Alex finds the
   toggle shape confusing. Use a plain checkbox per row
   instead. The checkbox's checked state mirrors today's
   "on" state semantically.

3. **Label**. The container UI hosting the per-tab
   checkboxes gets the label "broadcast input on/off".
   (Keep the exact wording @@Alex named.)

## Background

Bug entry: [`../phase-8-bugs.md`](../phase-8-bugs.md)
"Terminal broadcast selector: missing self entry +
confusing on/off toggle shape".

Existing broadcast wiring (from the smoke-test digest):
* `web/src/components/TerminalTab.svelte::sendUserInput`
  calls `broadcastTerminalInput(tab, data)` after sending
  the WS input frame — that's the producer side.
* The consumer side (which tab WS instances pick up the
  broadcast) is wired through a tab-linking store. Grep
  for `broadcastTerminalInput` to find the consumer +
  the selector UI.
* The selector UI is likely in the terminal tab's
  hamburger menu or a sibling overlay — the implementer
  finds it at task time.

## Acceptance criteria

* The current tab appears in the broadcast-target list,
  visually distinguished as "self" (icon or
  separator-above-others).
* Each row uses a checkbox; clicking the checkbox
  toggles broadcast for that target. The old toggle
  rocker UI is gone.
* The container has the label "broadcast input on/off".
* Self-broadcast: checking the self-row routes broadcast
  back to the current tab. This may be a no-op
  semantically (the current tab already gets its own
  input), but the checkbox state must persist and the
  UI must respect the user's choice.
* `vitest` green; if there's existing test coverage on
  the broadcast selector's render shape, update the
  assertions to expect the new checkboxes + the self
  row.

## How to start

1. Grep for `broadcastTerminalInput` to find the producer
   + selector UI file(s).
2. Inspect the current toggle shape; identify the data
   source for the rendered row list (likely a derived
   value from the tabs store filtered to "other
   terminals").
3. Adjust the data source to include the current tab,
   tagged as "self" for the render layer.
4. Replace the toggle markup with checkboxes; preserve
   the binding to the same underlying state.
5. Add the label to the container.

## Coordination

* Slots into the rich-prompt mini-wave alongside
  [`fullstack-a-28`](fullstack-a-28.md) /
  [`-29`](fullstack-a-29.md) / [`-30`](fullstack-a-30.md).
  Independent of the bubble overlay regression cluster
  — different UI surface, separate commit.
* @@WebtestA verifies on lane-A; @@WebtestB verifies
  on lane-B (the broadcast feature is terminal-side, so
  lane-B's coverage is the more natural fit if their
  queue has bandwidth — otherwise lane-A handles it).
* Push held for the patch-release commit-grouping cut.
