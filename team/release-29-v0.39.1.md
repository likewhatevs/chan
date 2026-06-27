# Phase 29 - devserver + chan-desktop hardening

Status: code+docs complete, gated green, merged to `origin/main`, and **released as v0.39.0** (2026-06-18), with the **v0.39.1** patch (2026-06-18) fixing three issues from the connect-flow smoke. `make pre-push` green over the committed state; the Linux devserver smoked in lima, the lock and FD work cross-checked there. The chan-desktop reconnect / window-lifecycle paths were smoked on @@Alex's hardware and surfaced end-to-end gaps carried into the next round.
Span: 2026-06-18.
Tags: #devserver #chan-desktop #lock #fd-leak #terminal-persistence #unserve #self-upgrade #control-terminal #graph #ipc

A hardening round on the `chan devserver` + chan-desktop surface that phase-28 introduced: workspace lifecycle (on/off, unserve, remove), writer-lock correctness, the long-running-devserver file-descriptor leak, and standalone-terminal persistence at the launcher. Scoped from @@Alex's smoke of the v0.38.0 build - he wedged his own repo "locked" with no live process, the devserver climbed to EMFILE, and standalone terminals did not survive a launcher reattach. Ran as a five-member team (@@Lead + @@Desktop / @@Devserver / @@Core / @@CI); the reliability + persistence core (W5/W6/W7/W10) was the heart.

## Roadmap (the asks)

`dev/old/v0.39.0/plan.md` is the design of record (ten workstreams):

- **W1 - Control / standalone terminal redesign** - a true singleton control terminal, terminal-only, no auto-hide or floating spawn button. (@@Desktop.)
- **W2 - Failing-script survey + stuck-connecting + empty window** - a clean retry/edit/abandon path. (@@Desktop; server audit @@Devserver.)
- **W3 - Empty devserver (zero workspaces) loads** on connect and across a restart. (@@Devserver + @@Desktop.)
- **W4 - Devserver workspace on/off** (unload without forget), persisted across restart. (@@Devserver + @@Desktop.)
- **W5 - `chan unserve` + `chan remove` + desktop close→unserve** over the control socket. (@@Core; desktop wiring @@Desktop.)
- **W6 - Lock correctness:** record the holder (pid/path/start), reclaim only a provably-dead holder. (@@Core, syseng; Windows builds @@CI.)
- **W7 - Devserver file-descriptor exhaustion (EMFILE)** - investigate + fix. (@@Devserver lead, @@Core support.)
- **W8 - Self-upgrade download progress** - terminal text + chan-desktop UI. (@@Core + @@Desktop.)
- **W9 - chan-llm MCP server on Windows** - named-pipe transport feasibility (low-pri investigation). (@@CI.)
- **W10 - Standalone terminal persistence at the launcher** - terminal windows/tabs persist like workspaces. (@@Devserver + @@Desktop.)

## What shipped

**Lock correctness (W6):**
- **`839bd189`** - the writer lock records the holder's pid, path, and start time, and a contender reclaims it only from a provably-dead holder instead of failing. Fixes rapid Open/On/Off clicking wedging a workspace as "locked" with no live process.
- **`d35f233d`** - `is_free()` probes writer-lock state without taking it (the close→reopen handoff).

**FD leak (W7):**
- **`46ed8fb5`** - drop the redundant tantivy commit-watcher (a second inotify watcher per workspace) so a long-running multi-workspace devserver's descriptor count stays bounded across mount/unmount and reconnect churn.

**Workspace lifecycle - on/off + unserve + remove (W4/W5):**
- **`8a9363e1` / `e133bae4`** - devserver workspace on/off API (unmount without forget); release the flock before `close_workspace` returns so on→off→on round-trips.
- **`bd44035f` / `9a19aac5`** - the `ControlRequest::Unserve` transport and `chan unserve` + unserve-before-remove over the control socket.
- **`c13e9669` / `47dd71fc`** - `chan remove` drops the whole `~/.chan/workspaces/<key>/` metadata directory (so it never fails "workspace locked" on a live serve), with a cross-process teardown integration test.

**Control terminal + connect flow (W1/W2/W3):**
- **`29a00d3d`** - the control-terminal redesign (singleton, terminal-only, no flashing), devserver workspace on/off in the launcher, and the update-progress UI, landed as one slice.
- **`a9f83c2e` / `806fcf79` / `06602da5`** - fail the connect fast on control-script exit; bound the connecting-screen retry loop; expose the control-script PTY exit for the scrape. An empty devserver now loads on connect and across a restart.

**Self-upgrade progress (W8):**
- **`471b94f3`** - a text download-progress meter (percent, size, elapsed, ETA) for `chan upgrade`; the chan-desktop progress bar landed with `29a00d3d`.

**Standalone terminal persistence (W10):**
- **`e99df14b` / `c6688271` / `a58b505c`** - persist a devserver's standalone terminals and their pane/tab layout across a restart, and re-surface them in chan-desktop; **`cbf3568a`** re-creates persisted devserver terminals on auto-reconnect; **`948107df`** adds terminal forget (`DELETE /api/devserver/terminals/{prefix}`).

**Graph:**
- **`f13e6598` / `20392b8b`** - in a directory/file scope, every file node anchors to its folder spine, so cross-tree link/mention/tag targets no longer render loose.

**v0.39.1 patch (connect-flow smoke):**
- **`00a68680`** - the connect flow's first terminal is created as a first-class persisted per-tenant terminal, fixing the `HTTP 415` on connect and Cmd+Shift+N on a focused devserver terminal; `3905a967` reverts the bodyless-POST workaround in favor of the labeled body, guarded by `943c011a` / `bd7f4a58` tests.
- **`190c36ca`** - the control terminal surfaces the abandon/edit/retry dialog on every close or exit while connecting (Ctrl-C, Ctrl-W, or the close button), and abandon resets the launcher to "Connect".

## Verification

- **Scoped own-gates** per change (fmt + clippy `-D warnings` + tests; `make web-check`), plus the full-tree `make pre-push` over the committed state.
- **Lima devserver:** the FD-leak fix verified by classifying `/proc/<pid>/fd` by type across mount/unmount + reconnect churn (the `w7-*-fd.sh` harnesses); the lock-steal exercised against a `kill -9`'d serve.
- **Desktop:** the on/off toggle, the control-terminal redesign, the connect-flow patch, and the standalone-terminal reattach were smoked on @@Alex's hardware.

## Deferred / follow-ups

- **W9 (chan-llm MCP on Windows)** stayed a low-priority investigation and did not ship this round.
- **Standalone-terminal persistence + reconnect (W10)** shipped the right intent but the next round's smoke proved the implementation broke end to end: reconnect respawned shells instead of re-attaching, window discard-vs-bury was inferred rather than marked, and the connected-phase control-terminal dialog never fired. Those, plus the devserver-serves-the-host-library model and the leaked-handle Windows lock-steal, were taken up next.
