# fullstack-29: terminal "Show Dir" should spawn a File Browser tab

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-19

## Goal

Terminal tab right-click → `Show Dir` action doesn't
spawn a File Browser tab. Likely a leftover from the
`fullstack-14` Phase 1 migration: the action used to
open the File Browser **overlay** with the terminal's
CWD; after Phase 1 moved File Browser to a first-class
tab type, the Show Dir handler wasn't updated to
spawn the new tab type instead.

## Relevant links

* @@Alex's chat note 2026-05-19 04:40 BST.
* [./fullstack-14.md](./fullstack-14.md) — the Phase 1
  overlay → tab migration where this handler should
  have been retargeted.

## Acceptance criteria

* Right-click on a terminal tab → click `Show Dir` →
  a File Browser tab opens in the active pane (or in
  a sensible target pane per the same target-resolution
  used by "Graph from here" / "Show in file browser"),
  rooted at the terminal's CWD.
* If a File Browser tab is already open and showing the
  same dir, focus that tab instead of spawning a
  duplicate.
* No OverlayShell for File Browser is involved (per
  `fullstack-14` removal).
* Regression test that asserts the call site spawns the
  tab type, not the (now-removed) overlay path.

## Out of scope

* Reworking the terminal tab right-click menu beyond
  this one item.
* "Show Dir" UX for non-terminal tabs.

## How to start

1. Grep `web/src/` for `Show Dir` (likely the menu
   item label) and the handler it invokes.
2. The handler probably still calls into the removed
   File Browser overlay API. Replace with the tab-
   creation path used by `fullstack-14` (probably the
   same path `Cmd+P` uses, with a CWD argument).
3. Reuse the existing target-pane resolution for
   "Graph from here" / "Show in file browser" if
   that helper exists; otherwise spawn in the active
   pane.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-architect.md`.
