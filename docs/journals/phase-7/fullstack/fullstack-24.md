# fullstack-24: follow-up affordance is a button, not a link

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-19

## Goal

Promote the "follow up" affordance on bubble surveys
from a text link to an explicit button. @@Alex called
this out 2026-05-19 03:30 BST: the link styling is too
subtle for what's a real third reply state alongside
the numbered option buttons.

The `F` keyboard binding stays as-is — this is purely
the visual treatment.

## Relevant links

* Predecessor: [./fullstack-23.md](./fullstack-23.md)
  — the original TUI vertical layout + follow-up
  introduction.

## Acceptance criteria

* The follow-up affordance under the option list
  renders as a real button (same primitive used for
  the numbered option rows), not a `<a>` / styled-text
  link.
* Visually distinct enough from the numbered options
  to read as a different kind of action — e.g. a
  smaller "ghost button" style, a different prefix
  marker (`[F]` to match the keystroke), or both.
* Position stays subtle: under the options row, not
  crowding the question text or the topic tab strip.
* `F` keystroke binding unchanged.
* `follow_up: true` reply payload unchanged.
* Existing follow-up tests still pass; add one that
  asserts the button (not link) markup.

## Out of scope

* Changing the schema.
* Changing the keystroke.
* Reordering / sorting follow-up bubbles in the tray.

## How to start

* `web/src/components/BubbleOverlay.svelte` — locate
  the follow-up affordance from `fullstack-23` and
  swap the element + styling.
* If you reuse the numbered-option button primitive,
  ensure the `[F]` prefix renders consistently.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-architect.md`.
