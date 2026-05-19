# systacean-15: activity indicator regression — investigate + fix

Owner: @@Systacean (lead — substrate is yours; hand to @@FullStack if root cause is SPA-side)
Cut by: @@Architect
Date: 2026-05-19

## Goal

`systacean-13` (`1694041`) shipped the chan-server PTY
activity-tracking substrate, and `Pane.svelte:887-893`
renders the `.dirty.activity` marker conditionally on
`t.terminalActivity`. But the marker doesn't fire end-
to-end. @@WebtestA's repro:

* Produce output in an unfocused NoiseGen pane.
* `.dirty.activity` span query returns false at 3s and
  4.5s sample points.
* `t.terminalActivity` never flips to true.

Backend writes are correct (per the substrate); frontend
render code exists. Wire between them is broken.

## Relevant links

* @@WebtestA's webtest-a-7 item 7 verdict (PARTIAL)
  with the repro detail.
* `systacean-13` commit `1694041`.
* `fullstack-13/20` (focus event seam likely shares
  with watcher state changes).

## Acceptance criteria

* Root cause identified. Two main candidates per
  @@WebtestA's diagnosis:
  1. **SPA-side focus/blur emission** isn't firing when
     a terminal tab focus changes. chan-server expects
     a small `focus` WS frame on tab focus.
  2. **chan-server activity-frame ingestion** in the
     SPA isn't flipping the `terminalActivity` flag on
     the tab state model.
* Fix lands on whichever side. If it's SPA-side, hand
  off to @@FullStack for the actual change after
  diagnosis.
* @@WebtestA's repro recipe runs green: produce output
  in an unfocused terminal → `.dirty.activity` marker
  appears within 1s → focus the tab → marker clears
  immediately.
* Regression test (backend-side if it's the wire, or
  frontend-side if it's the state model).

## Out of scope

* Sound or other notification surfaces.
* Activity indicator on non-terminal tab kinds (editor /
  graph / file browser). Spec is terminal-only.

## Side observation to verify

* @@WebtestA noticed a `Focused` checkbox at the bottom
  of the terminal tab right-click menu — possibly a
  manual override that gates auto-tracking. Confirm
  whether this is intentional UI (drop if not, document
  if yes). Could explain why auto-tracking isn't firing
  if the checkbox defaults to unset.

## How to start

1. Bring up a terminal in chrome MCP at 8801 (@@WebtestA's
   server is up). Open DevTools.
2. Focus + blur the terminal tab; watch the WebSocket
   network panel. Does a `focus` frame go out from SPA to
   chan-server? If not, the seam is SPA-side emission.
3. If the SPA does send focus frames, watch for the
   reverse: chan-server should send `activity` frames
   when output lands on an unfocused PTY. If those frames
   arrive in the WS panel but `t.terminalActivity` stays
   false, the SPA ingestion is the seam.
4. Check the `Focused` checkbox observation — locate the
   menu item in the codebase and decide intent.
5. Propose fix in an append before implementing; tag
   @@Architect.

## Hand-off

Standard. Pre-push gate green. Coordinate with
@@FullStack if root cause is SPA-side. @@WebtestA
re-runs item 7 after the fix lands. Ping via
`alex/event-systacean-architect.md`.

## 2026-05-19 06:07 BST - diagnosis and proposed fix

Root cause is SPA-side focus semantics, not the chan-server
activity-frame substrate.

`TerminalTab` currently treats its `active` prop as both:

* visible active tab inside the pane, and
* terminal PTY focus for `focus` WS frames and activity clearing.

That is wrong in split-pane layouts. The active terminal tab in an
unfocused pane is still visible and still has `active=true`, so it
sends `{"type":"focus","focused":true}` and suppresses incoming
`activity` frames with `!active && bytes_since_focus > 0`. Output in
that pane therefore never flips `t.terminalActivity`.

The `Focused` checkbox observation appears to be this state leak in
the terminal tab menu / status surface, not intentional manual
tracking.

Proposed fix for @@Architect:

* Keep `active` meaning "selected tab within its pane" so rendering
  and visibility stay unchanged.
* Add a separate `focused` prop to `TerminalTab`, derived from
  `pane.activeTabId === t.id && layout.activePaneId === pane.id`.
* Use `focused` for terminal WS focus frames, activity clearing, and
  `term.focus()` calls.
* Use `!focused` when ingesting `session.bytes_since_focus` and
  `activity` frames.
* Add a focused frontend regression test that an active tab in an
  unfocused pane marks activity when an activity frame arrives, and
  clears when the pane/tab becomes focused.
