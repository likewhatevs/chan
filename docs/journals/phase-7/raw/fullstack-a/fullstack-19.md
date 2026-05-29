# fullstack-19: switch survey-reply write to chan-server endpoint

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-18

## Goal

Switch the SPA's survey-reply write path from the current
drive-API-mediated atomic write (which hits chan-drive's
editable-text gate and fails on the `.tmp` staging file)
to the new chan-server endpoint added in `systacean-11`.

Unblocks `webtest-a-6` item 7 PARTIAL.

## Relevant links

* Bug: @@WebtestA's `webtest-a-6` item 7 PARTIAL verdict
  (red banner: `reply failed: path is not editable
  text: events/.event-reply-s1-mpbk3dio.tmp`).
* Backend partner: [../systacean/systacean-11.md](../systacean/systacean-11.md).
* Predecessor: [./fullstack-13.md](./fullstack-13.md)
  (the original reply atomic-write).

## Acceptance criteria

* The SPA's survey-reply write calls
  `POST /api/terminal/<session>/event-reply` with the
  reply JSON body. Stop writing the file directly via the
  drive API.
* Success (`204`) → bubble closes, no error banner.
* Error responses (`400`, `409`) → bubble shows a
  meaningful error (don't leak the raw response shape).
* Skip / Esc still writes a one-shot reply with no
  answers (just goes through the same endpoint with an
  empty `answers` array).
* Existing reply unit tests update to mock the new
  endpoint shape.

## Out of scope

* Backend endpoint implementation — that's
  `systacean-11`.
* Other event types (poke, survey from-SPA) — they don't
  exist yet from the SPA side.

## How to start

1. Wait for `systacean-11` to land on main, OR coordinate
   with @@Systacean to coordinate the API shape (they'll
   commit first; you land the SPA switch right after).
2. Find the current reply write in
   `web/src/components/BubbleOverlay.svelte` (or wherever
   the reply path lives after `fullstack-18`'s
   simplification).
3. Replace the drive-write path with a `fetch` to the
   new endpoint, using the same per-launch bearer token
   already used by other API calls.
4. Update tests.

## Hand-off

Standard. Pre-push gate green. Ping @@WebtestA when
landed so they can re-run item 7. Ping via
`alex/event-fullstack-architect.md`.

## 2026-05-18 20:14 BST — implemented, blocked on backend lane

Implemented the SPA switch locally:

* Added `api.writeTerminalEventReply(sessionId, body)` for
  `POST /api/terminal/<session>/event-reply`.
* `BubbleOverlay.svelte` now receives the terminal session id and sends
  replies through the endpoint instead of writing `.tmp` files through
  the drive API.
* Skip / Esc still sends an empty `answers` array with
  `scope_grant: "one-shot"`.
* Reply error messages map watcher-missing / invalid-reply responses to
  user-facing text.
* Updated `BubbleOverlay` and `watcherEvents` tests to mock the endpoint.

Verification:

* `npm run test -- BubbleOverlay watcherEvents`
* `npm run check`
* `npm run build`

Blocked:

* `scripts/pre-push` currently fails before the web checks on the dirty
  backend-lane `crates/chan-server/src/terminal_sessions.rs` work:
  `E0597: session does not live long enough`.
* The endpoint is not yet visible on `main`; latest commit at this note
  is `1cd4ef2` (`Reattach terminal PTY by window and tab`). Hold the
  FullStack commit until `systacean-11` lands / the backend gate is
  green, unless @@Architect directs a coordinated landing.

## 2026-05-18 20:33 BST — backend landed, gate green

`systacean-11` landed the terminal reply endpoint in `530e30f`
(`Add terminal event-reply writer`). Re-ran the full gate after the
backend lane merged; `scripts/pre-push` is green.
