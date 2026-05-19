# fullstack-25: separate `focused` from `active` on terminal tabs (activity indicator fix)

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-19

## Goal

Land the SPA-side fix for the activity-indicator
regression diagnosed by @@Systacean in
[../systacean/systacean-15.md](../systacean/systacean-15.md)
"2026-05-19 06:07 BST" appendix.

Root cause: `TerminalTab` conflates `active` (selected
tab inside its pane) with PTY focus (selected tab in
the focused pane of the workspace). In split-pane
layouts the active tab of an unfocused pane still has
`active=true`, so it emits `focus: true` WS frames and
suppresses incoming `activity` frames with `!active &&
bytes_since_focus > 0`. Output in that pane never flips
`t.terminalActivity`.

## Relevant links

* Diagnosis + proposed fix:
  [../systacean/systacean-15.md](../systacean/systacean-15.md)
  "2026-05-19 06:07 BST - diagnosis and proposed fix".
* @@WebtestA's original repro in `webtest-a-7` item 7.

## Acceptance criteria

* `active` stays meaning "selected tab within its
  pane". No render / visibility changes.
* New `focused` prop on `TerminalTab` derived from
  `pane.activeTabId === t.id && layout.activePaneId === pane.id`.
  True only for the active tab of the focused pane.
* `focused` drives:
  * Outbound `{"type":"focus","focused":true|false}` WS
    frames.
  * Activity-clearing logic (`bytes_since_focus = 0`).
  * `term.focus()` calls (xterm input focus).
* `!focused` drives ingestion of `session.bytes_since_focus`
  and incoming `activity` frames — i.e., the
  `terminalActivity` flag flips true when an activity
  frame arrives for a non-focused tab.
* @@WebtestA's repro runs green: produce output in an
  unfocused-pane terminal → `.dirty.activity` marker
  appears within 1s → focus that pane's tab → marker
  clears.
* Regression test asserting:
  * Active tab in unfocused pane + incoming activity
    frame → `terminalActivity` true.
  * Pane/tab becomes focused → `terminalActivity`
    clears.

## Side cleanup

* Remove the `Focused` checkbox at the bottom of the
  terminal tab right-click menu (@@WebtestA flagged it
  as a likely state leak — @@Systacean's diagnosis
  confirms it's not intentional manual tracking, just a
  surface of the broken state model).

## Out of scope

* Backend changes (the substrate is correct).
* Activity indicator on non-terminal tab kinds.
* Cross-pane focus rules beyond what the active-pane
  layout state already provides.

## How to start

1. `web/src/components/Pane.svelte` — locate
   `TerminalTab` usage. Compute `focused` from
   `pane.activeTabId === t.id && layout.activePaneId
   === pane.id` and pass it down.
2. `web/src/components/TerminalTab.svelte` — accept
   `focused`. Replace `active` references in WS focus
   emit + activity-clear + `term.focus()` paths with
   `focused`.
3. Activity-frame ingestion path: gate the flip on
   `!focused`, not `!active`.
4. Drop the `Focused` checkbox menu entry.
5. Run @@WebtestA's repro recipe locally before
   ping.

## Hand-off

Standard. Pre-push gate green. Coordinate with
@@WebtestA for the re-test once landed. Ping via
`alex/event-fullstack-architect.md`.
