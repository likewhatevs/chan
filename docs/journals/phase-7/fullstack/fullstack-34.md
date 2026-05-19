# fullstack-34: pane chrome refinements + wobble + close-all-tabs/close-pane split

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-19

## Goal

Three small pane-level UX refinements from @@Alex's
click-around session 2026-05-19 05:30 BST. They're
unrelated but all live in the pane chrome / hamburger
menu, so bundle them.

## Acceptance criteria

### Pane chrome — floating shade + rounded + border space

* Add a small amount of padding/border space around each
  pane so panes don't butt directly against each other
  or against the workspace edge.
* Slightly round the pane corners (subtle radius —
  ~4-6px feel, not visually loud).
* Each pane gets a slight shadow / shade behind it so
  it reads as "floating" over the workspace background.
  * **Dark mode** → shadow is white-ish (subtle white
    glow / soft white shadow).
  * **Light mode** → shadow is black-ish (standard soft
    drop-shadow).
* The pane focus-border color (from `fullstack-30`) sits
  on top of this base chrome; nothing about that
  changes.

### Wobble on split / delete / pane-move

* When a pane is split (creating two), both the source
  pane and the new sibling do the existing CSS hover-
  wobble animation once.
* When a pane is closed (sibling absorbs the freed
  space), the absorbing pane wobbles.
* When focus moves between panes via Cmd+K mode +
  WASD/arrow-swap (`fullstack-16`), the panes involved
  wobble.
* Reuse the existing hover-wobble keyframes — no new
  animation. Just trigger them on these events.
* Wobble is single-fire (not looping); duration matches
  the hover wobble.

### Hamburger menu: split "Close all tabs" from "Close pane"

* Today the hamburger menu has a single combined "close
  all tabs and pane" action (per current `Close pane`).
* Split into two items:
  * **Close all tabs** — closes every tab in the pane
    but keeps the pane itself (welcome state shows
    again).
  * **Close pane** — closes the pane (and its tabs).
    Sibling absorbs the space.
* Both items render in the hamburger; sequence stays
  as `fullstack-30` shipped, with "Close all tabs"
  inserted right above "Close pane":
  ```
  Focus border color
  ─────────────────────
  Next pane / Previous pane
  ─────────────────────
  Split right / Split down
  Close all tabs           ← new
  Close pane
  ```

## Out of scope

* The carousel widget on empty panes (separate task,
  `fullstack-35`).
* Graph-from-here behavior (separate task,
  `fullstack-32`).
* Animation primitives beyond reusing hover-wobble.

## How to start

1. Pane chrome: `Pane.svelte` (or its stylesheet).
   Add the padding + border-radius + shadow rules.
   Use CSS variables tied to the theme so dark/light
   mode swap is automatic.
2. Wobble: grep for the existing hover-wobble
   keyframes. Trigger them via a small `wobbleOnce`
   helper that adds a class for one animation cycle
   then removes it. Wire into the split / close /
   pane-move code paths.
3. Hamburger items: add the new `Close all tabs`
   action — clears `pane.tabs` array but keeps the
   pane node in the tree. `Close pane` remains
   as-is.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-architect.md`.
