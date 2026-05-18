# systacean-9: Round 2 substrate — fsnotify watcher + event ingestion

Owner: @@Systacean
Cut by: @@Architect
Date: 2026-05-18

## Goal

Land the backend substrate for Round 2 feature streams F1
(survey protocol) and F2 (notification bubbles): a
chan-server fsnotify watcher tied to a terminal session
that reads typed event files, dispatches them, and writes
`poke\n` to the matching agent's PTY when an event targets
them. This is the engine; the bubble UI in `fullstack-13`
is the consumer.

## Relevant links

* Round 2 capacity proposal + survey schema:
  [../architect/journal.md](../architect/journal.md)
  ("2026-05-18 21:00 BST — Round 2 capacity proposal").
* Original requirements:
  [../request.md](../request.md) — "How I envision the pokes
  working" section + engineering addendum.

## Acceptance criteria

### Watcher lifecycle

* A terminal session can attach an fsnotify watcher to a
  user-chosen directory via a new HTTP API
  (`POST /api/terminal/<session>/watcher`,
  `DELETE /api/terminal/<session>/watcher`). Body: target
  directory path (drive-relative or absolute).
* The watcher is owned by the terminal session. On
  terminal close / restart / exit, the watcher drops.
  Not re-created automatically.
* One watcher per terminal session. Setting a new watcher
  replaces the old one.

### Event ingestion

* Watcher subscribes to `Create` and `Rename` (final-name)
  events on the target dir. Other events ignored.
* On event fire, read the file once. Parse as JSON. If
  parse fails OR required fields missing, log a warning
  and move on. **No retry, no multi-read.** The writer is
  responsible for atomic temp+rename writes.
* Required fields: `id`, `type`, `from`, `to`. Optional:
  `topic`, `questions`, `standing_options`, `scope`,
  `answers`, `scope_grant`, `note`.
* `type` values handled in this task:
  `survey`, `survey-reply`, `poke`. Unknown types log +
  ignore (allows forward-compat).

### Dispatch

* Resolve `to` (`@@SomeAgent`) to a target terminal tab.
  Tab lookup uses the existing tab name registry — match
  by display name first, fall back to env
  `$CHAN_TAB_NAME` if the agent's tab was named via
  `chan open`.
* If a matching tab is found: write `poke\n` to its PTY.
  That's it for this task. The bubble overlay UI reads
  the event file content separately when the user opens
  the rich prompt.
* If no match: log + drop. Surface a counter in
  `/health` or similar so we can see drops in dev.
* `TODO` markers in the dispatch path for the
  `/clear` / `/effort` / `/fast` automation @@Alex
  mentioned. Don't implement.

### No self-loops

* chan-server must never write into a watched directory
  in response to an event from that directory. Default
  posture is structural separation: the response is
  always a PTY write, never a disk write.
* If you absolutely need to write within a watched dir
  later (out of scope for this task), the existing
  `crates/chan-server/src/self_writes.rs` module is the
  notify-suppression seam. Document the contract in a
  comment block on the watcher module.

### Tests

* Unit tests on the watcher: temp+rename a JSON event in
  a tmp dir, assert the watcher dispatches once.
* Unit test on dispatch: synthetic event with known `to`
  resolves to a mocked PTY writer.
* Property-style test for parse robustness: malformed
  JSON, missing fields, unknown `type` — all should not
  crash the watcher.

## Out of scope

* Bubble overlay UI (that's `fullstack-13`).
* Survey reply handling beyond writing the file out — the
  reply event is just another `survey-reply` JSON that
  the producer agent reads from its own outbox.
* HTTP agent control channel for spawning (wave-B,
  separate task).
* `/clear` / `/effort` / `/fast` exotic automation. Mark
  TODO only.

## How to start

1. Decide on the fsnotify crate. `notify` is already in
   the workspace dep tree (chan-drive uses it). Reuse it.
2. New module:
   `crates/chan-server/src/event_watcher.rs`. State:
   per-session watcher handles in `AppState`. Add HTTP
   routes under `crates/chan-server/src/routes/terminal.rs`
   or a new `routes/watcher.rs` if cleaner.
3. Event schema: derive a small enum + struct set with
   `serde`. Match the spec in the architect journal.
4. Wire dispatch through the existing per-tab PTY writer
   path. The terminal tab registry already exists for
   the `chan open` work; reuse `CHAN_TAB_NAME` lookup.
5. Sanity check: spin up a fresh `chan serve`, attach a
   watcher to `/tmp/test-events/`, drop a JSON via
   `python -c "import os; os.replace('/tmp/test-events/.tmp', '/tmp/test-events/event-1.md')"`,
   confirm the targeted tab gets `poke\n` typed in.

## Hand-off

Standard. Pre-push gate green. Coordinate with @@FullStack
on the HTTP API shape (they need to call the watcher-set
endpoint from the bubble dialog). Ping via
`alex/event-systacean-architect.md` when ready for review.
