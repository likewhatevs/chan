# fullstack-21: swap pane menus back + trim splits

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-19

## Goal

Revise `fullstack-6`'s pane menu placement. @@Alex's
final intent (clarified after living with the
implementation):

* **Right-click on the pane** → Reload + Toggle web
  inspector. The original placement before `fullstack-6`.
* **Pane hamburger menu** → Structural actions: Split
  right, Split down, Close, Next pane (`Cmd+]`), Previous
  pane (`Cmd+[`), Focus-border color.

Also drop **Split left** and **Split up** entirely;
@@Alex only asked for right + down. Pane-to-pane
navigation in horizontal/vertical directions is the
already-bound `Cmd+[` / `Cmd+]` shortcut, not a split.

## Relevant links

* [../request.md](../request.md) — "Pane menu
  reorganization" bullet with the 2026-05-19 01:45 BST
  revision sub-bullet.
* Predecessor: [./fullstack-6.md](./fullstack-6.md)
  (the original reorg).

## Acceptance criteria

### Right-click on the pane

* Two items only: `Reload` and `Toggle Web Inspector`.
* No structural actions, no focus-color picker.
* Same dismissal behavior as today (Esc / click outside
  / mutually exclusive with hamburger per `fullstack-17`
  polish).

### Pane hamburger menu

* In this order: `Split right`, `Split down`, `Close
  pane`, separator, `Next pane Cmd+]`, `Previous pane
  Cmd+[`, separator, `Focus border color` with the
  blue/green/pink sub-options.
* No `Split left` or `Split up` entries.
* `Cmd+]` / `Cmd+[` shortcuts continue to fire from
  anywhere in the pane regardless of menu visibility
  (they already do; just keep working).

### Out of scope

* Doc-tab right-click menu (stays as `fullstack-6`
  shipped it — close / close others / close all / copy
  path / show in file browser / reopen closed).
* Rich-prompt right-click menu (stays).
* Pane focus-border color UX itself (stays per-pane,
  persisted with layout).
* Removing the split mechanics — `splitPane` still
  supports left/up under the hood for the drag-detach
  substrate (`fullstack-15` body-drop on left/top edge
  uses the same primitives). Only the menu entries get
  removed; programmatic split-left/up stays as the
  drag-detach implementation needs it.

## How to start

1. `web/src/components/Pane.svelte` — locate the
   right-click menu and hamburger menu definitions
   (both landed in `fullstack-6`).
2. Swap the contents per the spec above.
3. Drop the `Split left` and `Split up` menu entries.
4. Update any tests covering the menu contents.
5. Coordinate with @@WebtestA for the re-verification
   pass (their `webtest-a-5` wave-2b verdict on
   `fullstack-6` will need a quick re-check).

## Hand-off

Standard. Pre-push gate green. Insert in @@FullStack's
queue right after `fullstack-20` (the spawn UI) lands.
Ping via `alex/event-fullstack-architect.md`.

## Result

2026-05-19 05:16 BST — Implemented in `web/src/components/Pane.svelte`.
Right-click pane menu is back to `Reload` + `Toggle Web Inspector`
only. Pane hamburger now carries the structural menu in the requested
order: `Split right`, `Split down`, `Close pane`, next/previous pane,
and focus-border colors. Removed the visible split-left/up rows and
their now-unused click wrappers/icons while leaving the underlying
`splitPane` before-direction support untouched for drag-detach.

Verification:

* `npm run check`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

Note: first plain `scripts/pre-push` hit macOS fd limit 256 in
`chan-drive` tests (`Too many open files`). Rerunning the same gate
outside the sandbox with fd limit 4096 passed.
