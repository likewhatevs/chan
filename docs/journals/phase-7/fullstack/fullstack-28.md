# fullstack-28: empty-pane right-click menu — restore the welcome menu

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-19

## Goal

`fullstack-21` swapped the loaded-pane right-click menu
back to `Reload + Toggle Web Inspector`. The swap
applied uniformly — so right-clicking an EMPTY pane
also now shows just `Reload + Toggle Web Inspector`,
which is the wrong menu for that surface. The original
empty-pane right-click was the "open something here"
welcome menu, and that should be restored as a separate
menu shape distinct from the loaded-pane one.

## Relevant links

* @@Alex's chat note 2026-05-19 04:30 BST.
* [./fullstack-21.md](./fullstack-21.md) — swap-back
  that didn't distinguish loaded vs empty panes.

## Acceptance criteria

### Empty pane right-click

A right-click on an empty pane (no tabs open, welcome
state) shows the following menu, in this order:

```
Files
Search
Graph
Terminal
separator
Split right
Split down
separator
Settings
```

* Each open-something item creates a new tab of that
  kind in the current pane (Files = first-class
  FileBrowser tab from `fullstack-14`, Graph = first-
  class Graph tab, Terminal = new terminal, Search =
  the search overlay since Search stays as overlay per
  Phase 1).
* `Split right` / `Split down` reuse the structural
  primitives. (No `Split left` / `Split up` per
  `fullstack-21` decision.)
* `Settings` opens the Settings overlay (unchanged).

### Loaded pane right-click

Stays as `fullstack-21` shipped: `Reload`, `Toggle Web
Inspector`. No change.

### Detection

* "Empty" = pane has zero tabs (the welcome state). As
  soon as the pane has at least one tab, switch to the
  loaded-pane right-click menu.
* Hamburger menu stays the same regardless of
  loaded/empty (structural actions per `fullstack-21`).

## Out of scope

* Pane focus-border color picker — stays in the
  hamburger per `fullstack-21`.
* Doc-tab right-click menu — unchanged from
  `fullstack-6`.

## How to start

* `web/src/components/Pane.svelte` — locate the right-
  click menu definition from `fullstack-21`. Branch
  the menu items on `pane.tabs.length === 0`.
* Wire the welcome-menu items to the same tab-creation
  paths used elsewhere (Cmd+P opens Files, Cmd+Shift+M
  opens Graph, etc. — reuse those handlers).

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-architect.md`.

## 2026-05-19 07:44 BST — hand-off

`fullstack-28` is committed and pushed on `main`.

Commit:

* `06739a9` Restore empty pane context menu (fullstack-28)

Gate run: `npm run test -- Pane`, `npm run check`,
`npm run build`, and `bash -lc 'ulimit -n 4096; scripts/pre-push'`.

Notes: empty panes now right-click to the welcome menu in spec order; loaded panes still right-click to Reload / Toggle Web Inspector.
