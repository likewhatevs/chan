# systacean-11: chan-server seam for survey-reply atomic writes

Owner: @@Systacean
Cut by: @@Architect
Date: 2026-05-18

## Goal

Land a chan-server-internal write seam for survey-reply
event files. The SPA can't go through `chan_drive::Drive::
write_text` for this channel because chan-drive's
editable-text gate rejects the `.tmp` staging file the
atomic-write contract requires:

> reply failed: path is not editable text:
> events/.event-reply-s1-mpbk3dio.tmp

Watcher event files are machine-to-machine traffic, not
user content — the gate is correct for editor saves; it's
the wrong gate for this surface. Cleanest fix is a new
chan-server endpoint that does the temp+rename atomically
server-side, bypassing the drive write API entirely.

## Relevant links

* Bug in @@WebtestA's `webtest-a-6` item 7 PARTIAL verdict
  (event-webtest-a-architect.md, 2026-05-18 ~22:50 BST
  appendix).
* Related contract: `systacean-9` watcher reads + temp+
  rename writer-side rule from the request.md engineering
  addendum.
* SPA-side switch is `fullstack-19` (cut alongside).

## Acceptance criteria

* New endpoint: `POST /api/terminal/<session>/event-reply`
  with JSON body matching the survey-reply schema (`id`,
  `type`, `from`, `to`, `answers`, `scope_grant`, `note`).
* chan-server writes the JSON to the session's currently-
  configured watch directory using `tokio::fs` (NOT
  `chan_drive`): write to a `.tmp` in the same dir, then
  rename to `event-reply-<id>.md`. Same atomicity contract
  as the writer-side rule.
* If no watcher is currently attached to that session,
  return `409 Conflict` with a clear message.
* If the body fails schema validation (missing `id`,
  unknown `type` other than `survey-reply`), return
  `400`.
* Returns `204 No Content` on success.
* Tests:
  * Write succeeds → file exists, content is the JSON.
  * Concurrent calls don't corrupt the destination
    (atomic semantics).
  * Tmp file is cleaned up on success and on failure.
  * Endpoint refuses when no watcher attached.

## Why not extend chan-drive

* The editable-text gate exists for a reason: user
  content is sandboxed + extension-filtered to keep the
  drive a coherent text-notes store. Loosening it to
  accept `.tmp` files weakens that contract.
* Event files are infrastructure-internal, not user
  content. They live in a watched dir which may even
  be outside the drive root — chan-drive shouldn't be
  involved in writes either way.
* Keeps the drive boundary clean (per CLAUDE.md
  "Drive is the boundary" principle).

## Out of scope

* Producer-side writes (agents writing surveys into the
  watch dir). Those happen from the agent's terminal
  via shell/python `mv` and don't go through this
  endpoint.
* Changing `chan_drive::Drive`'s editable-text gate.
* The SPA call switch — that's `fullstack-19`.

## How to start

1. New route in `crates/chan-server/src/routes/terminal.rs`
   or a sibling file. Reuse the watcher-session lookup
   from `systacean-9`.
2. Resolve the active watch dir from the session state.
3. Write the JSON atomically via `tokio::fs::write` to
   a `.tmp` (in the same dir) + `tokio::fs::rename` to
   the final `.md`. fsnotify-self-loop concern doesn't
   apply because the SPA does NOT have a watcher on its
   own reply outbox (replies go to the producer's inbox).
4. Surface a test that confirms the write path works
   with the in-process fsnotify watcher (drop a survey
   in, get a reply out via this endpoint, watcher
   dispatches the reply to the target).

## Hand-off

Standard. Pre-push gate green. Coordinate with @@FullStack
on the endpoint shape before the SPA switch lands. Ping
via `alex/event-systacean-architect.md`.

## 2026-05-18 20:46 BST - ready to land

Implemented the chan-server event-reply seam:

* `POST /api/terminal/:session/event-reply` accepts the locked
  `survey-reply` shape and maps JSON/schema rejections to `400`.
* The endpoint resolves the session's active watcher dir, returns
  `409 Conflict` when no watcher is attached, and writes JSON via
  `tokio::fs` temp-file + same-dir rename to
  `event-reply-<id>.md`.
* `TerminalSessions` now tracks the watcher directory alongside
  the watcher handle and clears it with the watcher/session
  lifecycle.
* Tests cover successful writes, concurrent atomic writes, temp
  cleanup on success/failure, no-watcher refusal, required text
  validation, and watcher-dir lifecycle tracking.

Verification:

* `cargo test -p chan-server event_reply --no-default-features`
* `cargo test -p chan-server --no-default-features`
* `cargo clippy -p chan-server --all-targets --no-default-features -- -D warnings`
* `scripts/pre-push`
