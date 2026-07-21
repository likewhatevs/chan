# Complete The Control-Terminal Reconnect Fix Across The Wake Path

Status: accepted scope for v0.74.0. A confirmed defect with a live user report, and it is the unfinished part of the fix shipped in `a61d5748` (v0.70.2), not a new bug class and not a v0.71 regression.

## Problem

On macOS, waking the machine re-runs a devserver's connect script inside its control terminal, so the user is presented with an ssh passphrase or password prompt they did not ask for, and answering it accomplishes nothing.

The trigger is the one member of the terminal reconnect kit that carries no control-terminal guard. `recyclePtySocketAfterWake()` at `web/packages/workspace-app/src/components/TerminalTab.svelte:603-607` is, in full, `if (ws && ws.readyState === WebSocket.OPEN) void connect();`. It is installed from the wall-clock wake detector at `web/packages/workspace-app/src/components/TerminalTab.svelte:583-586`, which calls the shared detector in `web/packages/workspace-app/src/wakeGap.ts:36-52` (probe 2000 ms, wake threshold 6000 ms, `web/packages/workspace-app/src/wakeGap.ts:30-31`). That detector exists precisely because macOS WKWebView fires no focus, pageshow or visibilitychange across a sleep, as its own header states at `web/packages/workspace-app/src/wakeGap.ts:1-9`.

Every other member of the same kit returns early on `ui.terminalControl`: the read deadline at `web/packages/workspace-app/src/components/TerminalTab.svelte:817-819`, the connect deadline at `web/packages/workspace-app/src/components/TerminalTab.svelte:836-841`, the heartbeat ping at `web/packages/workspace-app/src/components/TerminalTab.svelte:939-942`, and the onclose redial at `web/packages/workspace-app/src/components/TerminalTab.svelte:1161`. The wake recycler is the only one that was missed.

Those guards are what make the control terminal reachable here. With no heartbeat and no read deadline, its socket is never force-closed, so across a sleep it is exactly the half-open-but-OPEN socket that `recyclePtySocketAfterWake` acts on.

`connect()` re-dials `/terminal/ws` carrying the tab's stale session id. The ssh PTY died during the sleep, so the session is gone server-side: `get_or_create_for_ws` fails to attach (`crates/chan-server/src/routes/terminal.rs:598-604` into `crates/chan-library/src/terminal_sessions.rs:1104-1141`) and falls through to `self.create(opts)` at `crates/chan-library/src/terminal_sessions.rs:1139`. `create` fills an empty command from the tenant default at `crates/chan-library/src/terminal_sessions.rs:964-976` (`opts.command = default;` at line 971). For a control tenant that default is the devserver connect script: `spawn_control_terminal_window` at `desktop/src-tauri/src/serve.rs:485-495` passes the script into `EmbeddedServer::open_terminal_with_command` at `desktop/src-tauri/src/embedded.rs:403-418`, which mounts a per-control tenant via `Host::open_terminal_session_with_command` at `crates/chan-library/src/host.rs:970`, documented as the tenant default at `crates/chan-library/src/host.rs:957-961`.

There is no retry loop; the guards from `a61d5748` still hold, so this is one re-run per wake. But that single re-run is an orphan. Roughly a second after the PTY exit, `spawn_control_terminal_exit_watcher` (`desktop/src-tauri/src/main.rs:2024-2110`) takes the unhealthy-exit branch and calls `remove_devserver_windows` (`desktop/src-tauri/src/main.rs:1573`) then `mark_devserver_control_exited` (`desktop/src-tauri/src/main.rs:1713-1751`), which drops the connection record and retires the watcher. Nothing is left polling for a re-emitted token, so a user who types the passphrase gains nothing at all.

`reconnect_devserver` (`desktop/src-tauri/src/main.rs:3038`) and `reconnect_devserver_for_window` (`desktop/src-tauri/src/main.rs:4166`) are not involved. The latter is reachable only from the DisconnectOverlay Reconnect button: `web/packages/workspace-app/src/api/desktop.ts:255` is called only from `web/packages/workspace-app/src/components/DisconnectOverlay.svelte:58`. Recorded here so the next reader does not re-investigate them.

Interim workaround for users on a shipped build: after a wake, close the control terminal window and use Connect from the launcher. Do not answer the passphrase prompt the wake produces; it belongs to an orphaned run.

## Desired contract

One behavior change, with the lifecycle boundary pinned explicitly.

1. `recyclePtySocketAfterWake` returns early on `ui.terminalControl`, matching the four siblings at `web/packages/workspace-app/src/components/TerminalTab.svelte:817`, `:836`, `:939` and `:1161`, with a comment stating the same reason those carry: the control terminal is a single-shot local runner owned by the desktop exit watcher.

There is no control-terminal session replacement. If the connect script or its PTY exits after establishing a persistent transport such as `ssh -N`, `spawn_control_terminal_exit_watcher` owns the result: mark the devserver connection down, close its dependent workspace windows, and retain the dead control terminal when it carries a failure reason. Do not redial its WebSocket, mint a replacement session, clear or reuse its viewport, or rerun its tenant default command.

Only a new, explicit user Connect may run the script again. The existing dead-control-terminal gate remains part of that flow: the user first closes the retained terminal after reading its output, then Connect creates a fresh control terminal with a fresh session and viewport. This is a new run, not recovery or replacement of the dead one.

## Acceptance

Every check below names the mutation that must turn it red; the standing rule is that a new check proven only green is not a check.

1. Guard, unit level. Extend `web/packages/workspace-app/src/components/terminalHeartbeatReconnect.test.ts` (the existing venue for this kit, jsdom mount plus `TerminalTab.svelte?raw` source pins) with a control-terminal tab that reaches OPEN, then drive the wake path and assert no second `TestWebSocket` is constructed. This single assertion proves there is no replacement session, viewport reset, or script rerun because none can occur without the second dial. Red mutation: delete the `if (ui.terminalControl) return;` line from `recyclePtySocketAfterWake` and the test must fail on a second socket appearing.

2. Guard, non-regression for ordinary terminals. In the same file, the same wake on a non-control tab must still dial a second socket carrying the prior session id. Red mutation: change the new guard to an unconditional `return` and the test must fail.

3. Existing desktop lifecycle coverage remains load-bearing. `control_script_clean_exit_reaps_and_failed_exit_keeps_terminal` in `desktop/src-tauri/src/main.rs` continues to prove that a persistent control-script exit marks the connection down, closes dependent windows, retains failure output when appropriate, and blocks a plain Connect until the user closes that terminal. Add a source pin that the exit watcher contains no reconnect or control-terminal spawn call. Red mutation: call either path from the watcher and the pin must fail.

4. Whole-repo gate: `make pre-push` (fmt, clippy, cargo test, no-default-features build, gateway build, `npm run check` and the full vitest in `web/packages/workspace-app`). Run vitest on a detached worktree, not a shared tree.

5. Live proof, owner-only, on real hardware. Connect a devserver through an SSH control script from Chan Desktop on macOS, sleep the machine past the 6000 ms wake threshold, and wake it. If SSH survived, the connection stays up without a new passphrase prompt, session id, viewport clear, or re-run banner. If SSH died, the launcher marks the devserver down and retains the dead control terminal for diagnosis; it does not create or restart anything. Closing that terminal and explicitly pressing Connect produces one fresh control terminal and one script run. A workspace terminal in the same window still recovers its socket and accepts typed keys. This cannot be run here: this host has no display server and no macOS, so neither the WKWebView wake behavior nor a real system sleep is reproducible locally. The jsdom tests above prove the guard, not the platform behavior.

## Boundaries

Do not widen the reconnect kit's control-terminal guards into a single shared predicate in this item; four call sites plus the new fifth read clearly and each carries its own reason. A consolidation, if wanted, is separate work with its own review.

Do not change the exit watcher (`desktop/src-tauri/src/main.rs:2024-2110`) or the disconnect and reconnect commands (`desktop/src-tauri/src/main.rs:3038`, `desktop/src-tauri/src/main.rs:4166`). The watcher already owns script death and marks the connection down; once the wake no longer re-dials, there is no orphan to manage.

Do not change the id-changed xterm reset, clear the control terminal, or change `get_or_create_for_ws` and generic terminal replacement behavior in this item. Those paths are not reached after the control wake guard and widening the fix would change ordinary terminal recovery.

Do not add a control-tenant flag to `chan-library`. Control terminals are excluded at the desktop SPA boundary where `ui.terminalControl` already exists and where the other four reconnect guards already live.
