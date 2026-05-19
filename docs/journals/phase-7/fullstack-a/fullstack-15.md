# fullstack-15: Phase 2 substrate — Hybrid binary-tree pane model

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-18

## Goal

Land the Phase 2 "Hybrid" pane substrate per @@Alex's
ui-exploration.md Phase 2 section. The shape is a binary
tree of splits, not a grid. One viewport, one tree,
panes as leaves. Tabs stay scoped per-pane like today.
This task is **substrate only** — no Cmd+K transactional
pane mode yet (that's `fullstack-16`).

Desktop-first per @@Alex's call. Don't bother with
web/browser shortcut conflicts; central shortcut config
absorbs cross-platform.

## Relevant links

* @@Alex's design:
  [../ui-exploration.md](../ui-exploration.md) — Phase 2
  section ("Model" subsection authoritative).
* Inspiration references in the same doc: Hyprland tiling
  feel, WinBox for early thinking.

## Acceptance criteria

### Tree model

* Binary tree where every interior node is a split
  (horizontal or vertical) with exactly two children.
  Leaves are panes. Splits nest arbitrarily.
* Drag a divider between siblings: redistributes the
  ratio between exactly those two siblings, never
  reflows the rest of the tree.
* Resize clamps to a sensible minimum (panes can't
  collapse to zero).

### Detach tab → new pane

* Drag a tab out of its pane's tab bar. Drop on another
  pane's body shows a drop indicator at the edge being
  approached (left / right / top / bottom).
* On drop, the target leaf splits in the direction of
  the drop edge. The dragged tab becomes the content of
  the newly-created sibling pane.
* Drag a tab into another pane's tab bar: tab transfers
  to that pane's tab list. If the source pane was on its
  last tab, the source pane collapses (its sibling
  absorbs the freed space).

### Persistence

* Tree shape, split ratios, per-pane tab order, and
  focused pane all survive reload and reopen.
* Persist alongside the existing per-window state keyed
  by `w=<label>`.
* Reload mid-layout brings back the same tree.

### Constraints

* **No floating windows.** No minimise. Tiling-first.
* **Resize is local.** Only two siblings interact on a
  divider drag. No row/column spans, no global recalc.
* The model collapses "make another Hybrid" into "add
  another pane to the tile tree". One viewport, one
  tree.

## Out of scope

* Cmd+K transactional pane mode + WASD/arrows/resize
  keybinds (that's `fullstack-16`).
* Pane-mode visual chrome (overlay fx, focused-pane
  brighter border, pane-mode pill in status bar).
* Multi-window / multi-viewport — single Hybrid.

## How to start

1. Inventory current pane layout in
   `web/src/state/tabs.svelte.ts` — there's already a
   `LeafNode` type per the imports @@FullStack-6
   showed. Confirm whether the existing layout is
   already a binary tree or some other shape.
2. If it's already binary-tree-shaped: this task is
   mostly drag-and-detach UX + persistence completeness.
   If it's grid-or-other-shaped: build the tree model
   first, migrate the existing layout state.
3. Drop edge detection: hit-test which quadrant of the
   target leaf the pointer is in; use that to determine
   split direction.
4. Persistence: serialize the tree to JSON, deserialize
   on hydrate. Keep the format versioned for future
   migrations.

## Hand-off

Standard. Pre-push gate green. Coordinate with @@WebtestA
on the drag-detach + persistence walkthrough; @@WebtestB
on terminal-pane-detach interactions. Ping via
`alex/event-fullstack-architect.md`.

## 2026-05-18 20:58 BST — implementation

The existing layout model was already a binary split tree with local
divider ratios and URL/session persistence, so this task focused on
the missing drag-detach substrate:

* Added `detachTabToPaneEdge(fromPane, tab, targetPane, edge)` to move
  a dragged tab into a newly-created sibling pane.
* Body drops choose the nearest target edge (`left`, `right`, `top`,
  `bottom`) and split only that target leaf.
* Tab-bar drops still merge into the target tab list; body-edge drops
  split into a new pane.
* Source panes collapse when their last tab is detached into another
  pane.
* Added body-edge drop highlighting so the split destination is visible
  before release.
* Added focused tests for same-pane detach and last-tab source collapse.

Verification so far:

* `npm run test -- tabs`
* `npm run check`
* `npm run build`

Next: full gate, then commit if green.

## 2026-05-18 20:59 BST — commit readiness

Ready to land.

Changed files:

* `web/src/state/tabs.svelte.ts`
* `web/src/components/Pane.svelte`
* `web/src/state/tabs.test.ts`
* `docs/journals/phase-7/fullstack-a/fullstack-15.md`

Verification:

* `npm run test -- tabs`
* `npm run check`
* `npm run build`
* `scripts/pre-push`

Known risk: drag edge detection is pointer-distance based and should
get a Webtest walkthrough for terminal tabs and nested splits.

Proposed commit: `Add pane body tab detach substrate (fullstack-15)`.
