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

## 2026-05-19 — implementation note

Three independent edits, all SPA-side:

1. **Image+EOL scroll rollover** — the paste / drop handlers in
   `web/src/editor/bubbles/image_drop.ts` dispatched the insert
   without a `scrollIntoView: true` flag, so pasting at the
   bottom moved the caret off-screen until the next keystroke.
   Added `scrollIntoView: true` to the dispatch so CodeMirror
   scrolls the new selection into view immediately on insert.
   Affects both the `paste` and `drop` paths (they share
   `uploadAndInsertAll`).

2. **Hybrid empty-pane preservation** — `closeTabAsync` in
   `web/src/state/tabs.svelte.ts` auto-called
   `collapseEmptyPane(p.id)` when a non-root pane went to zero
   tabs. Dropped that block: the pane now stays standing and
   renders the empty-pane landing (chan logo + Cmd+K hint).
   Explicit `closePane` is still the way to dismiss a Hybrid
   pane on purpose. Test: new `tab close confirmation > closing
   the last tab in a Hybrid pane leaves the pane in place`
   asserts the survivor leaf still exists with `tabs.length: 0`
   and `activeTabId: null` after closing the last tab.

3. **Survey bubble re-pop after reply** — picked option (b)
   from the task spec (SPA-only) since the chan-server reply
   endpoint already writes a sibling `event-reply-{id}.md` with
   `type: "survey-reply"` and `id` matching the original
   survey. `BubbleOverlay`'s `visibleEvents` derive now builds a
   `Set` of replied ids from the survey-reply rows and filters
   the original surveys against it. The original survey JSON
   stays on disk (audit trail), but the bubble queue stops
   re-rendering it on each watcher poll. Test:
   `BubbleOverlay > survey paired with a sibling survey-reply
   is filtered out of the bubble queue` mounts a watcher with
   one answered + one fresh survey and confirms only the fresh
   one renders.
   Chose (b) over (a) because the server endpoint already
   leaves a clean pair-by-id record and (b) is purely SPA — no
   coordination needed with @@FullStackB or chan-server side.

Files touched:

* `web/src/editor/bubbles/image_drop.ts` — `scrollIntoView`.
* `web/src/state/tabs.svelte.ts` — drop auto-collapse on last
  tab close.
* `web/src/state/tabs.test.ts` — new last-tab-stay test.
* `web/src/components/BubbleOverlay.svelte` — `visibleEvents`
  filters surveys with matching replies.
* `web/src/components/BubbleOverlay.test.ts` — new
  reply-pairing test.

Pre-push gate (SPA portion): vitest 452/452 green
(+2 new tests over the prior 450); `npm run check` 0 errors /
0 warnings; `npm run build` clean.

## 2026-05-19 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Three independent fixes, all small and well-targeted. Option (b)
for the bubble re-pop (SPA-only Set-of-replied-ids filter) is
the right call — the server already leaves a clean pair-by-id
record on disk so audit trail stays intact and no cross-lane
coordination needed. The auto-collapse drop on last-tab close
matches the spec; phase-8 backlog item 4's empty-pane landing
will paint into that survivor leaf.

**Commit clearance**: approved. Suggested subject:

```
Editor: image+EOL scroll, empty Hybrid pane preserved, bubble re-pop filter (fullstack-a-5)
```

Push waits for Round-1 close. Pick up `fullstack-a-6` next
(Cmd+K F search focus).
