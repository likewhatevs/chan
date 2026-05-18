# systacean-8: PTY scrollback retention on browser reload (B19)

Owner: @@Systacean
Cut by: @@Architect
Date: 2026-05-18

## Goal

After a browser reload, the terminal pane re-attaches to
the live PTY correctly (per @@WebtestB's 17:30 BST
verification on post-recycle main) — input works, the same
PID is alive, background jobs survive. The single
remaining gap is **scrollback retention**: the xterm.js
buffer in the SPA shows only the fresh prompt after
reload; everything previously rendered is gone, even though
the PTY itself still has the same output stream behind it.

Fix the visual record so a reload doesn't wipe the
on-screen history.

## Relevant links

* @@WebtestB's verification trail:
  [../webtest-b/webtest-b-1.md](../webtest-b/webtest-b-1.md)
  (look for the 17:30 BST B14 / B19 re-verification
  section).
* The PTY session machinery: `crates/chan-server/src/`
  (terminal routes and the PTY broker). The frontend side:
  `web/src/components/Terminal*.svelte` and xterm.js wiring.

## Acceptance criteria

* Refreshing the browser tab on an active terminal session
  shows the prior on-screen scrollback after re-attach,
  not just an empty prompt. The buffer can be capped
  (suggested: a configurable line count, e.g. 10k lines)
  so we don't grow unbounded.
* Implementation choice (your call):
  1. **Server-side ring buffer**: chan-server keeps the
     last N lines of PTY output per session. On
     re-attach, the WebSocket replays them before joining
     the live stream. Simple, central, but adds memory.
  2. **Client-side persistence**: the SPA stashes the
     xterm.js buffer into `sessionStorage` /
     `IndexedDB` keyed by the per-tab session id and
     restores on reload. Avoids server memory but doesn't
     survive a chan-server restart.
  3. Hybrid: server ring buffer is the source of truth
     when chan-server is alive; client cache is a fast
     path for re-render before the WebSocket re-attach
     completes.
* No regression in the live PTY stream (writes still
  arrive, input still routes, BCAST mute still works).
* Cap is configurable in `ServeConfig` or a sibling
  setting; default sensible (10k lines or so).

## Out of scope

* Persistence across chan-server restarts (full session
  resurrection — separate concern).
* Replaying ANSI animations / cursor moves exactly; line-
  granular replay is fine.

## How to start

1. Locate the WebSocket terminal route in
   `crates/chan-server/src/routes/terminal.rs` (or `ws.rs`
   if the terminal stream lives there). Inspect the
   per-session state on the server.
2. If you go the ring-buffer route, add a bounded
   `VecDeque<String>` (or byte-buffer) per session and
   prepend the contents to the re-attach response.
3. If you go the client-side route, pair with @@FullStack
   on the SPA-side restore path.
4. Coordinate with @@WebtestB for the re-attach repro;
   they have the working test recipe.

## Hand-off

Standard. Pre-push gate green. @@WebtestB walkthrough
after landing.

## @@Systacean proposal — 2026-05-18 18:45 BST

Diagnosis:

* `chan-server` already has the server-side replay ring for PTY
  output: `TerminalConfig::ring_bytes` defaults to 1 MiB and
  `AttachHandle.replay` sends retained bytes before the live
  stream.
* The browser reload gap is in the SPA attach cursor. Terminal
  tabs persist both `terminalSessionId` and `lastSeq` (`tseq`) in
  per-window `session.json`.
* On browser reload, xterm starts with an empty buffer, but the
  restored tab reconnects with `since=<old lastSeq>`. The server
  correctly replays only bytes after that offset, so the old
  visible scrollback is skipped.

Patch direction:

* Keep the server ring as the source of truth.
* Keep `lastSeq` in live tab memory for same-page WebSocket
  reconnects, so reconnect does not duplicate already-rendered
  output.
* Stop serializing `tseq` into the per-window session layout and
  ignore legacy saved `tseq` on restore/hydration. Browser reloads
  still retain `terminalSessionId`, but reconnect with no cursor,
  causing the server to replay the retained ring into the fresh
  xterm buffer.

## @@Systacean update — 2026-05-18 18:43 BST

Patch implemented as proposed. The server-side ring remains unchanged;
the SPA now treats terminal sequence offsets as live reconnect state,
not reload-persisted session state.

Verification:

* `cd web && npm run test -- src/state/tabs.test.ts src/terminal/session.test.ts`
  — passed, 41 tests
* `cd web && npm run check` — passed, 0 diagnostics
* `cd web && npm run test` — passed, 25 files / 228 tests
* `scripts/pre-push` — first run hit an unrelated transient
  `Too many open files` in a chan-drive test; immediate rerun passed
