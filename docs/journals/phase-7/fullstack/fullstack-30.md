# fullstack-30: focus color is Hybrid-wide + pane hamburger menu reorder

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-19

## Goal

Two changes to the pane focus-color feature and the
pane hamburger menu:

1. **Focus border color is a Hybrid-wide setting**, not
   per-pane. All panes in the same Hybrid window share
   one focus-color value; setting it from any pane's
   hamburger menu changes the color for the whole
   window's active-pane border.
2. **Reorder the hamburger menu** to put navigation
   first and structure last, with focus color on top
   since it's now the window-wide setting.

## Relevant links

* @@Alex's chat note 2026-05-19 05:10 BST.
* Predecessors: [./fullstack-6.md](./fullstack-6.md)
  (per-pane focus color), [./fullstack-21.md](./fullstack-21.md)
  (current hamburger menu order).

## Acceptance criteria

### Focus color: Hybrid-wide

* Drop per-pane storage of the focus-color value.
* Add a window-level focus-color setting persisted
  alongside the existing per-window state keyed by
  `w=<window-label>`.
* Default = blue.
* All panes' active-border CSS reads from the
  window-level value; changing it from any pane's
  hamburger updates all panes immediately.
* Migration: any panes that had per-pane colors saved
  before this change can lose them on next load (drop
  silently; this is a small UX state, not user
  content).

### Pane hamburger menu sequence

New order, top to bottom:

```
Focus border color  (with blue / green / pink sub-options)
─────────────────────
Next pane           (Cmd+])
Previous pane       (Cmd+[)
─────────────────────
Split right
Split down
Close pane
```

* No `Split left`, no `Split up` (per `fullstack-21`).
* Focus border color stays as a submenu / inline
  swatches — render it however you want as long as the
  three options are visible at the same level.

### Loaded-pane vs empty-pane right-click

* Unchanged from `fullstack-28`:
  * Empty pane right-click = welcome menu (Files /
    Search / Graph / Terminal + Split right/down +
    Settings).
  * Loaded pane right-click = Reload + Toggle Web
    Inspector.
* Hamburger is the same on both (this task's new
  sequence) since hamburger doesn't depend on
  loaded/empty state.

## Out of scope

* Doc-tab right-click menu (stays).
* Rich-prompt right-click menu (stays).
* Pane right-click menu (stays per `fullstack-21`/`-28`).

## How to start

1. `web/src/state/tabs.svelte.ts` (or wherever the per-
   pane focus-color slot landed in `fullstack-6`) —
   move the field from the pane object into the
   window-level state.
2. Update the persistence shape (URL hash + session.json).
3. CSS variable for the active-pane border now reads
   from the window-level value.
4. `web/src/components/Pane.svelte` — reorder the
   hamburger menu items per the spec.
5. Regression test: changing color from any pane
   updates the border on the focused pane regardless
   of which pane initiated the change.

## Hand-off

Standard. Pre-push gate green. Coordinate with
@@WebtestA for the walkthrough (focus-color flip
across multi-pane layouts). Ping via
`alex/event-fullstack-architect.md`.
