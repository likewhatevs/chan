# fullstack-a-5: Editor cluster (image+EOL scroll, Hybrid empty pane, bubble re-pop)

Owner: @@FullStackA
Date: 2026-05-19

## Goal

Three related editor / overlay fixes:

1. **Image+EOL scroll rollover** — pasting or inserting an image
   at the end of a document pushes the cursor offscreen and the
   page does not auto-scroll until the user types another
   character. Detect that the last line has approached the
   visible bottom and roll over BEFORE the next keystroke.
2. **Hybrid empty-pane preservation** — closing the last tab in
   a Hybrid pane currently closes the Hybrid pane itself. Keep
   the pane (as empty) and show the minimal landing (chan logo +
   "Press Cmd+K to enter Hybrid NAV" hint per the phase-8
   backlog item 4 direction).
3. **Survey bubble re-pop after reply** — root cause already
   diagnosed: `web/src/state/watcherEvents.ts::readWatcherEvents`
   returns every `event-*.{json,md}` in the watcher dir, so the
   original survey stays in the bubble queue after Alex replies.
   Pick a fix:
   * a) reply endpoint atomically renames the original survey
     JSON to `<name>.replied` (server-side tombstone) — preferred
     because audit trail stays intact AND the SPA filter is
     trivial,
   * b) SPA pairs survey + survey-reply by id and filters
     answered surveys out of the bubble queue (SPA-only),
   * c) reply endpoint deletes the original survey JSON.
   Recommend (a). Coordinate with @@FullStackB if (a) touches
   the reply endpoint (chan-server side).

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md) under the
image/EOL, Hybrid-empty-pane, and survey-bubble-re-pop items.

The bubble-re-pop root cause was confirmed during the phase-8
watcher smoke test on 2026-05-19. Both the survey JSON and the
reply markdown sit side-by-side in `./tmp/` after a reply; the
SPA lists both on next refresh.

## Acceptance criteria

* Image inserted/pasted near the visible bottom of the doc →
  doc auto-scrolls so the cursor stays in view BEFORE the next
  keystroke.
* Closing the last tab in a Hybrid pane leaves the pane in place,
  rendering the empty-pane landing.
* After replying to a survey bubble, the bubble does not return
  on the next watcher refresh. Multiple sequential surveys still
  render correctly.

## How to start

Image scroll: `web/src/editor/` — find the paste / insert handler
and confirm the scrollIntoView path.

Hybrid pane: phase-7 `fullstack-15` / `fullstack-16` set up the
pane model; check the close-tab → close-pane transition.

Bubble re-pop: see `web/src/state/watcherEvents.ts:65` for the
listing path; option (a) needs an edit in
`crates/chan-server/src/routes/terminal.rs::write_event_reply_atomic`
to also rename the original survey file.
