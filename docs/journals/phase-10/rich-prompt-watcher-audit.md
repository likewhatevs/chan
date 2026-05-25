# Phase 10 Rich Prompt Watcher Audit

Date: 2026-05-25.

Scope: Rich Prompt workspace lifecycle, terminal event watcher, event-file
ingest, event reply, `$CHAN_TAB_NAME` routing, UI polling, broadcast fanout,
and the current Phase 10 pending-work state.

## Code Paths Audited

- Workspace routes:
  `crates/chan-server/src/routes/rich_prompts.rs`
- Rich Prompt metadata:
  `crates/chan-drive/src/rich_prompts.rs`
- Terminal watcher and dispatch:
  `crates/chan-server/src/event_watcher.rs`
  `crates/chan-server/src/terminal_sessions.rs`
  `crates/chan-server/src/routes/terminal.rs`
- Frontend state and UI:
  `web/src/components/TerminalTab.svelte`
  `web/src/components/TerminalRichPrompt.svelte`
  `web/src/state/watcherEvents.ts`
  `web/src/state/tabs.svelte.ts`
- Team and spawn docs:
  `docs/templates/team-process/bootstrap.md.tpl`
  `docs/templates/team-process/orchestration/spawn-protocol.md.tpl`

## End-To-End Flow

1. The UI opens a Rich Prompt on a terminal tab.
2. `TerminalTab.svelte` creates a Rich Prompt workspace once a terminal session
   exists.
3. `POST /api/rich-prompts` creates `Drafts/rich-prompt[-N]/` metadata under
   the drive drafts root.
4. The workspace contains:
   - `draft.md`
   - `.chan-rich-prompt-active`
   - `spool/process.md`
   - `spool/events/`
   - `spool/journals/`
   - `spool/tasks/`
5. The server attaches the terminal-scoped watcher to the workspace's absolute
   `spool/events/` path.
6. Producers write event files into `spool/events/`.
7. The backend watcher parses matching files into `AgentEvent` and dispatches
   by `event.to`.
8. Dispatch emits a `SessionEvent::AgentEventEcho` to the target session's
   terminal WebSocket.
9. The frontend decodes `agent_event_echo`, calls `sendUserInput`, writes to
   the target PTY, and optionally fans out to the same browser window's
   broadcast group.
10. Separately, the UI polls
    `GET /api/terminal/:session/watcher/events` every 5 seconds to render
    survey, reply, poke, and pre-flight bubbles.

## Workspace Lifecycle

- `chan-drive` owns the metadata structure and safety checks.
- Creation writes the active marker last so a partial workspace is not treated
  as active during preflight.
- Inspection rejects missing required files, symlinks, special files, and
  unsafe nested entries.
- Submission archives the prompt into `prompt-N.md`, increments
  `submission_sequence`, and resets `draft.md` to empty.
- Closing discards the workspace to draft metadata trash, clears the watcher,
  and closes the terminal session when a session id is supplied.
- Restore rehydrates `workspaceName` and watcher UI state from per-window
  session storage, but it does not reattach a detached watcher to an existing
  workspace.

## Event-File Contract

Backend watcher ingest accepts only final paths delivered by `notify` for:

- create events;
- rename/name-modify events, using the final path when present.

Accepted filenames:

- `event-<id>.md`
- `event-<id>.json`
- `pre-flight-<id>.md`
- `pre-flight-<id>.json`

Hidden files, directories, non-matching names, wrong extensions, and missing
ids are ignored. Matching files are rejected or dropped when:

- the path is a symlink or special file;
- the file is larger than 1 MiB;
- the file cannot be read;
- JSON parsing fails;
- the event id was already seen by that watcher handle.

Required JSON fields:

- `id`
- `type`
- `from`
- `to`

Backend dispatch understands these event types:

- `survey`
- `survey-reply`
- `poke`

The frontend polling parser also accepts `pre-flight`. Current effect:
pre-flight files are visible to BubbleOverlay via polling, but they are not
injected into a PTY by backend dispatch because the Rust `AgentEventType` does
not include a pre-flight variant.

Extra fields read by backend dispatch:

- `path`
- `heading`

When both are present, dispatch sends a rich poke template that includes the
task anchor. Otherwise it sends bare `poke`.

Extra fields read by the frontend:

- `topic`
- `questions`
- `standing_options`
- `scope`
- `session`
- `tab_label`
- `note`

## Routing Semantics

Routing is based on the server's stored terminal session name, not on a live
shell lookup.

- Terminal spawn receives `tab_name` from the UI.
- `Session::spawn` stores that value in `Session.tab_name`.
- The PTY environment gets `CHAN_TAB_NAME=<tab_name>` at spawn time.
- `dispatch_agent_event` resolves `event.to` against stored
  `Session.tab_name`.
- The matcher normalizes both sides by:
  - trimming;
  - stripping a leading `@@`;
  - removing ASCII whitespace;
  - removing `-`;
  - removing `_`;
  - lowercasing.
- If exactly one session matches, dispatch emits `AgentEventEcho` to that
  session.
- If no session matches, the event is dropped and `watcher_dropped_events`
  increments.
- If more than one session matches, the event is dropped as ambiguous and
  `watcher_dropped_events` increments.

`event.from` is not used to route incoming injection. It is used as display
context and as the reply target when the UI writes `survey-reply` files.

## Rename Behavior

Live terminal rename changes the frontend tab title only.

- The running shell's `$CHAN_TAB_NAME` does not change.
- The server's stored `Session.tab_name` does not change.
- Event routing keeps using the old spawn-time name.
- The UI detects this mismatch through `terminalEnvTabNameStale` and warns:
  `$CHAN_TAB_NAME` stays at the old value until restart.
- Restarting a controlled terminal sends the current tab title back to the
  server so the new PTY receives the new `CHAN_TAB_NAME`.

Breakage modes:

- A producer sends `to` using the renamed UI title before restart. The event
  will not match the target session.
- A user creates two terminals whose names normalize to the same value, such
  as `@@Agent`, `Agent`, and `A-gent`. Matching becomes ambiguous and events
  are dropped.
- A team member overrides `CHAN_TAB_NAME` manually. The shell identity may
  differ from the server's `Session.tab_name`, depending on how the terminal
  was spawned.

## Watcher Polling And Replies

The UI does not rely only on backend dispatch. Each terminal tab with watcher
state polls `GET /api/terminal/:session/watcher/events` every 5 seconds.

The route reads the session's active `watcher_dir` directly with `std::fs`.
It bypasses the drive sandbox by design because watcher files are
infrastructure traffic, not user content.

Polling behavior:

- server filters filenames with the same event/pre-flight convention;
- server returns raw `{path, content}` pairs;
- frontend parses JSON and drops unknown shapes;
- visible events replace `tab.watcher.events`;
- new ids set the unread dot if the Rich Prompt is closed.

Reply behavior:

- BubbleOverlay calls `writeSurveyReply`.
- The server writes `event-reply-<sanitized-id>.md` atomically into the same
  watcher dir.
- The reply file uses `from: @@Alex` and `to: event.from`.
- Reply files are visible to polling and also match the backend filename
  prefix, but `survey-reply` dispatch still only sends a poke-style echo to
  the resolved `to` terminal.

## Broadcast Path

Backend dispatch does not write directly to the PTY. It broadcasts
`AgentEventEcho` on the target session.

Frontend handling:

- decode base64 payload;
- call `sendUserInput(payload)`;
- send a normal terminal WS input frame to the target PTY;
- call `broadcastTerminalInput(tab, payload)`.

Broadcast fanout is browser-window scoped:

- target ids are resolved only through the current JS window's layout;
- missing sink ids are skipped silently;
- no server-side bus fans input across windows.

This means an injected event writes to itself first, then to selected peers in
the current window when broadcast mode is enabled.

## Failure Modes

Watcher attach and lifecycle:

- Missing terminal session on workspace create returns 404 before creating the
  workspace.
- Watcher start errors mark the Rich Prompt phase broken and return a failed
  watcher view.
- A session close clears the watcher handle and watcher dir.
- After serve restart, restored watcher UI state usually points at no live
  server watcher. The first poll returns conflict/not attached and the UI
  clears watcher state.
- Existing Rich Prompt workspaces are inspected on restore, but detached
  watchers are not automatically reattached.

File ingest:

- Existing event files present before watcher attach are not dispatched by the
  backend watcher.
- Producers that write directly to the final filename can race the watcher and
  produce partial-read or parse failures.
- Producers that update a final event file by modify-only writes can be
  missed because dispatch accepts create and rename/name-modify events only.
- Duplicate event ids are silently skipped after debug logging.
- Bad JSON in a matching filename increments `watcher_dropped_events` but does
  not mark watcher status failed.
- Unknown event types are warned and ignored without incrementing dropped
  events.

Routing:

- No matching `event.to` increments dropped events and logs a warning.
- Ambiguous normalized names drop the event.
- Live tab rename does not update server routing until restart.
- `$CHAN_TAB_NAME` is a spawn-time identity signal, not a mutable route key.

Delivery:

- `AgentEventEcho` uses a broadcast channel and terminal WebSocket receiver.
  If no receiver is attached during the event, the echo can be lost.
- Broadcast fanout reaches only mounted terminals in the same browser window.
- Malformed base64 in an echo frame fails soft in the UI.

UI state:

- Polling can show event files that backend dispatch ignored, especially
  pre-flight files or old files present before attach.
- `seenIds` is replaced by the current file set on every poll. If a producer
  deletes and later recreates an event id, unread behavior depends on whether
  that id remained in the prior poll set.
- Closing a Rich Prompt with a missing or stale session id can leave the
  server-side terminal already gone while the workspace discard still runs.

## Phase 10 Pending Implementation Snapshot

Source docs checked:

- `docs/journals/phase-10/track-a-round-2-handoff.md`
- `docs/journals/phase-10/roadmap-track-a.md`
- `docs/journals/phase-10/roadmap-track-b.md`
- `docs/journals/phase-10/roadmap-track-c.md`
- `docs/journals/phase-10/track-a-handoff-from-track-b-logo-and-docs.md`
- `docs/journals/phase-10/track-c-next-agent-handoff.md`

Current Track A state:

- Track A immediate sequence items 1 through 20 are recorded complete in
  `roadmap-track-a.md`.
- The Track A round-2 handoff still lists native desktop drag-out manual smoke
  as ready to run, but `roadmap-track-a.md` later records that round-2 macOS
  Finder drag-out smoke passed on 2026-05-25.
- Remaining Track A implementation or validation items:
  - CLI-to-desktop handoff design remains deferred until selected.
  - Linux desktop launch remains deferred until selected.
  - Linux native drag-out remains unsmoked and depends on Linux desktop
    launch viability.
  - Release validation remains deferred until selected.

Track A items handed off from Track B:

- Tauri app icon regeneration is pending.
- Desktop docs and config audit is pending, including stale Linux/Windows
  wording, old `/dl/latest` language, updater URL shape, and installer
  contract comments.

Current Track B state:

- Public site, manual source, install split, release link validation, and
  release workflow shape are implemented.
- Pending Track B follow-ups:
  - Run `npm run verify:release` without skip flags after the next
    `chan-v*` tag includes the manual bundle and the repository is public.
  - Update `docs/manual/` and generated public manual after the latest
    streaming open, relationship loading, graph streaming, and inspector
    transfer behavior is final.
  - Run `cd web-marketing && npm run check` after those manual edits.

Current Track C state:

- `roadmap-track-c.md` is marked complete.
- `track-c-next-agent-handoff.md` closeout says no Track C-owned code, docs,
  browser-smoke, or teardown item remains.
- Rich Prompt browser validation, Spawn agents preflight, rapid edit
  validation, streaming UI intake, inspector transfer, and terminal pane
  switching were all closed by Track C on 2026-05-25.

## Recommended Follow-Up Ordering

1. Decide whether Rich Prompt watcher reattach on restored workspace should be
   implemented. This is the clearest functional gap found in this audit.
2. Decide whether backend dispatch should include an explicit pre-flight event
   variant or keep pre-flight as UI-polled only.
3. Decide whether event delivery needs replay for `AgentEventEcho`, or whether
   lost delivery during detached WS remains acceptable.
4. Resume Phase 10 Track A round-2 by selecting one of:
   - CLI-to-desktop handoff design;
   - Linux desktop launch plus Linux drag-out smoke;
   - release validation.
5. Separately schedule the Track B to Track A desktop icon and desktop
   docs/config audit tasks.
