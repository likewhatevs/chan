# @@Frontend task 4: terminal tab reattach + persistent session id

Owner: @@Frontend
Status: REVIEW
Depends on: [systacean-5](./systacean-5.md) wire contract agreed (or
in parallel; coordinate the contract before either lane writes
behaviour).
Source: [architect-tmux-1](./architect-tmux-1.md), Option 4
(confirmed by Alex).
Coordinates with: [backend-2](./backend-2.md) (per-window session
blob; we store the new field there).

## Goal

Make terminal tabs survive window reloads by persisting a
`terminal_session_id` per tab and reattaching to the existing
chan-server PTY session on reload, replaying ring scrollback against
the last `seq` seen.

## Wire contract (Frontend ↔ Systacean)

Mirror of the [systacean-5](./systacean-5.md) draft. Reconcile the
two task files before either side ships behaviour. Highlights from
the client side:

* WebSocket URL on attach: `/api/terminal/ws?session=<id>&since=<seq>&cols=<n>&rows=<n>&tab_name=<...>`.
  * `session` omitted on fresh tab creation. Server returns the new
    id in the first control frame (`{type:"session", id, seq,
    missed_bytes}`).
  * `since` is the last seq we processed for that session. Use 0 on
    first attach with a known id.
* Output frames stay binary (xterm.js write).
* Control frames are JSON (text frames). The client must handle at
  least:
  * `{type:"session", id, seq, missed_bytes}` — store id (if new
    session), update `last_seq` baseline, surface a "missed N bytes"
    banner when `missed_bytes > 0`.
  * `{type:"resize_other", cols, rows}` — another attachee resized;
    call xterm.js `fit()` or update `cols`/`rows` to match.
  * `{type:"closed", reason}` — server closed the session. Surface a
    one-line message per reason (`idle`, `drive`, `shutdown`,
    `explicit`, `capped`, `error`), clear the persisted
    `terminal_session_id`, do not auto-reconnect.
* Client explicit close frame on user-driven tab close / restart:
  `{type:"close"}`. Browser reload / component teardown does **not**
  send this frame; it only detaches the WebSocket.
* `last_seq` is a byte offset. Client sets it from the `session.seq`
  control frame, then advances it by the byte length of each binary
  output frame it writes to xterm.

## Scope

* `web/src/state/tabs.svelte.ts`: extend the terminal tab descriptor
  with `terminal_session_id?: string` and `last_seq?: number`. Both
  ride along in the per-window session blob ([backend-2](./backend-2.md))
  so they survive reload.
* `web/src/components/TerminalTab.svelte`:
  * On mount, decide create-vs-attach from the stored id. Pass
    `session` + `since` + initial `cols/rows` + `tab_name` on the
    WebSocket URL.
  * Update `last_seq` on every output frame (cheap counter; persist
    on a debounce so the session blob doesn't get hammered).
  * Handle the three control frame kinds above.
  * On user-driven close (kill the tab), send the explicit close
    control frame, then clear the stored id.
  * On server-side close, show the reason inline and offer a "Start
    new session" button that drops the id and creates a fresh one.
* Reload UX: when the tab reattaches, briefly show "Resuming" + the
  `missed_bytes` banner if any, then the live stream takes over.
* Resize: keep the existing resize control frame path; respect
  `resize_other` from the server so two attachees stay in sync.

## Acceptance criteria

* Closing and reopening the browser tab on the same window does not
  kill the shell; reattach replays scrollback.
* chan-desktop reload of a single drive window keeps the terminal
  tab's shell process alive (validated end to end by
  [webtest-1](./webtest-1.md) follow-up).
* Two windows attached to the same `terminal_session_id` mirror IO
  and stay in winsize sync.
* When the server times the session out, the UI surfaces "session
  ended (idle)" and clears the stored id.
* `npm --prefix web run check`, `npm --prefix web test -- --run`,
  `npm --prefix web run build` all green.

## Test expectations

* Unit / component tests for the tab descriptor extension (new
  fields persist + deserialise).
* A small mocked-WebSocket test verifying the create vs attach
  branch picks the right query shape.
* The end-to-end smoke (reload survives, two attaches sync, idle
  closes cleanly) belongs to @@Webtest A and gets a webtest-N
  follow-up task when the build is ready.

## Hardening expectations

* Be conservative about persisting `last_seq` — debounce so a noisy
  shell doesn't write the session blob hundreds of times per second.
  Order-of-once-per-second is fine.
* If the WebSocket fails before the first `session` control frame
  arrives, drop the stored id and let the user manually retry.
  Do not auto-spin reconnects.

## Progress

* 2026-05-17 @@Frontend started the task after update check.
* Extended `TerminalTab` state with `terminalSessionId` / `lastSeq`.
  These fields are included only in the per-window session layout payload,
  not in the shareable URL hash.
* Added `terminalWsPath()` to build fresh vs. reattach WebSocket URLs:
  fresh tabs omit `session` / `since`; restored tabs send
  `session=<id>&since=<lastSeq>`.
* `TerminalTab.svelte` now handles `session`, `resize_other`, and `closed`
  control frames, tracks `lastSeq` from binary output byte lengths, shows
  missed scrollback / close-reason UI, and offers "Start new session" after
  server-side close.
* User-driven tab close / restart sends the explicit `{type:"close"}` control
  frame and clears the persisted terminal session id. Reload/unmount still
  just detaches.
* Added tests for terminal descriptor persistence and WebSocket query shape.

## Completion notes

* Wire-contract revisions for [systacean-5](./systacean-5.md):
  * client explicit close frame is `{type:"close"}`;
  * `last_seq` is treated as a byte offset and advanced by binary output
    frame byte length after the `session.seq` baseline.
* Verification:
  * `npm --prefix web run check`
  * `npm --prefix web test -- --run`
  * `npm --prefix web run build`
* Build completed with existing Vite chunk-size / ineffective dynamic-import
  warnings, but no errors.
* End-to-end reload / multi-attach validation still waits for
  [systacean-5](./systacean-5.md)'s server registry implementation.
