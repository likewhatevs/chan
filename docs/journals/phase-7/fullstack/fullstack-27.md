# fullstack-27: pre-flight bubble render seam

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-19

## Goal

Pre-flight events from `systacean-12`'s control channel
land on disk (chan-server writes them into the
orchestrator session's watcher dir — @@WebtestA confirmed
the file exists in `events/`), but the bubble overlay
doesn't render them as a survey. @@WebtestA's
`webtest-a-7` item 4 PARTIAL.

`fullstack-20` shipped the pre-flight rendering branch
in `BubbleOverlay.svelte` per spec. Either the SPA
event reader isn't picking up `type: pre-flight` files,
the parse skips them, or the render branch isn't taking
for some reason. Diagnose + fix.

## Relevant links

* @@WebtestA's `webtest-a-7` item 4 detail.
* Predecessor frontend: [./fullstack-20.md](./fullstack-20.md).
* Predecessor backend: [../systacean/systacean-12.md](../systacean/systacean-12.md).

## Acceptance criteria

* @@WebtestA's repro recipe runs green: spawn an agent
  whose first output matches a pre-flight signal →
  pre-flight bubble renders with the 1/2/3 options
  (open terminal, kill spawn, retry now) → spinner +
  elapsed counter visible → pick option 2 (kill) closes
  the spawn.
* Items 4, 5, 6 in `webtest-a-7` flip to PASS.
* If the fix is in the event parser, ensure the
  `pre-flight` type doesn't accidentally fall into the
  unknown-type drop path (`fullstack-17` polish item).
* Regression test for the pre-flight render path.

## Likely seams to check

* `web/src/state/watcherEvents.ts:parseWatcherEvent` —
  after `fullstack-17`'s "drop unknown types" change,
  the allow-list was `survey`, `survey-reply`, `poke`.
  Confirm `pre-flight` is also in the allow-list (or
  add it).
* `web/src/components/BubbleOverlay.svelte` — the
  pre-flight render branch and its data wiring.
* `web/src/state/store.svelte.ts` (or wherever the
  event-file polling/reading lives) — confirm the
  pre-flight file gets picked up on watcher events.

## Out of scope

* Backend (chan-server) pre-flight detection — already
  works per @@WebtestA's verification that the event
  file lands.
* Activity counter sensitivity — separate task
  (`systacean-16`).

## How to start

1. Reproduce: 8801 server up at @@WebtestA's URL.
   Spawn `bash -c 'echo please log in; sleep 60'`
   against the orchestrating watcher. Confirm the
   event file lands in `events/`.
2. Open DevTools. Watch the SPA's watcher event reader.
   Confirm the file is read.
3. Trace whether the parse returns null (unknown-type
   drop) or returns a valid `pre-flight` event but the
   bubble doesn't render.

## Hand-off

Standard. Pre-push gate green. Ping @@WebtestA when
landed for item-4 re-test. Ping via
`alex/event-fullstack-architect.md`.

## 2026-05-19 06:39 BST — hand-off

`fullstack-27` is committed and pushed on `main`.

Commit:

* `ebb347b` Read pre-flight watcher files (fullstack-27)

Gate run: `npm run test -- watcherEvents BubbleOverlay`,
`npm run check`, `npm run build`, and
`bash -lc 'ulimit -n 4096; scripts/pre-push'`.

Notes: SPA watcher ingestion now accepts chan-server
`pre-flight-*.md/json` files as well as `event-*`, so the existing
pre-flight parser and BubbleOverlay render path receive the emitted spawn
pre-flight events.
