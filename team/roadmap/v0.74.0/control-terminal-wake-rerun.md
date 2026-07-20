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

A second, independent defect from the same commit is in scope because it is the visible half of the same wake. When a session frame's id differs from the tab's prior id, the SPA writes a mode reset into the same live xterm at `web/packages/workspace-app/src/components/TerminalTab.svelte:1003-1004`, ending in `\x1b[?1049l`. In xterm.js, DECRST 1049 runs `activateNormalBuffer()` then `restoreCursor()`, and with no matching 1049 SET both saved coordinates are 0, so the cursor homes to the top-left without erasing anything. The replacement session then paints over the first rows of the dead one and the previous session's output stays visible below it. This is not control-terminal specific; any tab whose session is replaced shows it.

Clearing the viewport is safe, and this must be said plainly so the fix is not scoped down to cosmetics out of misplaced caution: the devserver token scraper reads the server-side raw ring, not the client's xterm. `scrape_token` (`desktop/src-tauri/src/devserver.rs:1062-1083`) is fed from `desktop/src-tauri/src/main.rs:1903` via `EmbeddedServer::read_control_terminal_output` (`desktop/src-tauri/src/embedded.rs:424-425`), which reads `Host::terminal_tenant_scrollback` (`crates/chan-library/src/host.rs:1911`). A client-side clear never reaches it.

Interim workaround for users on a shipped build: after a wake, close the control terminal window and use Connect from the launcher, or Reconnect in the workspace window's DisconnectOverlay. Do not answer the passphrase prompt the wake produces; it belongs to an orphaned run.

## Desired contract

Three changes, in order of how load-bearing they are.

1. `recyclePtySocketAfterWake` returns early on `ui.terminalControl`, matching the four siblings at `web/packages/workspace-app/src/components/TerminalTab.svelte:817`, `:836`, `:939` and `:1161`, with a comment stating the same reason those carry: the control terminal is a single-shot local runner owned by the desktop exit watcher.

2. The id-changed reset at `web/packages/workspace-app/src/components/TerminalTab.svelte:1003-1004` erases as well as homing, so a replacement session starts on a clean viewport rather than overpainting the dead one. The reset keeps its current gate (`frame.id !== priorId`) so a same-id live resume still leaves a running program's modes and screen untouched.

3. A server-side belt: when a `/terminal/ws` dial names an explicit session id that the server no longer has, the freshly created session does not inherit the tenant default command. Today `get_or_create_for_ws` discards the distinction between "no id, open me a session" and "resume this id" before reaching `create` (`crates/chan-library/src/terminal_sessions.rs:1104-1141`), so a stale-id re-dial from any client is indistinguishable from a first open. The belt is narrow by construction: an ordinary workspace tenant has no default command, so only a single-purpose tenant (control, devserver connect) changes behavior, and for that tenant re-running the script on a resume attempt is never what the caller meant.

The belt is what stops the class rather than this one instance: any future path that re-dials with a stale id, in the SPA or elsewhere, cannot re-run a connect script by accident.

## Acceptance

Every check below names the mutation that must turn it red; the standing rule is that a new check proven only green is not a check.

1. Guard, unit level. Extend `web/packages/workspace-app/src/components/terminalHeartbeatReconnect.test.ts` (the existing venue for this kit, jsdom mount plus `TerminalTab.svelte?raw` source pins) with a control-terminal tab that reaches OPEN, then drive the wake path and assert no second `TestWebSocket` is constructed. Red mutation: delete the `if (ui.terminalControl) return;` line from `recyclePtySocketAfterWake` and the test must fail on a second socket appearing.

2. Guard, non-regression for ordinary terminals. In the same file, the same wake on a non-control tab must still dial a second socket carrying the prior session id. Red mutation: change the new guard to an unconditional `return` and the test must fail.

3. 1049 clear. Assert on a mounted tab that a session frame with a new id writes a sequence that erases the viewport, not only `\x1b[?1049l`. Red mutation: revert the write at `web/packages/workspace-app/src/components/TerminalTab.svelte:1004` to its current bytes and the test must fail. A same-id frame writing nothing stays asserted; red mutation for that half is dropping the `frame.id !== priorId` gate.

4. Server belt. A Rust unit test beside the existing registry tests in `crates/chan-library/src/terminal_sessions.rs` (the `get_or_create_for_ws` cases around `crates/chan-library/src/terminal_sessions.rs:5244-5430`): set a tenant default command, call `get_or_create_for_ws` with an id that was never minted, and assert the resulting session does not run the default command and does not emit the tenant-default announce banner. Red mutation: remove the stale-id condition so the fall-through calls `create` with the default still applied, and the test must fail. A companion case, `get_or_create_for_ws` with `id: None` on the same tenant, must still receive the default; red mutation: apply the suppression unconditionally.

5. Whole-repo gate: `make pre-push` (fmt, clippy, cargo test, no-default-features build, gateway build, `npm run check` and the full vitest in `web/packages/workspace-app`). Run vitest on a detached worktree, not a shared tree.

6. Live proof, owner-only, on real hardware. Connect a devserver from Chan Desktop on macOS, sleep the machine past the 6000 ms wake threshold, wake it, and observe that the control terminal shows no new passphrase prompt and no re-run banner, and that a workspace terminal in the same window still recovers its socket and accepts typed keys. This cannot be run here: this host has no display server and no macOS, so neither the WKWebView wake behavior nor a real system sleep is reproducible locally. The jsdom tests above prove the guard, not the platform behavior.

## Boundaries

Do not widen the reconnect kit's control-terminal guards into a single shared predicate in this item; four call sites plus the new fifth read clearly and each carries its own reason. A consolidation, if wanted, is separate work with its own review.

Do not change the exit watcher (`desktop/src-tauri/src/main.rs:2024-2110`) or the disconnect and reconnect commands (`desktop/src-tauri/src/main.rs:3038`, `desktop/src-tauri/src/main.rs:4166`). The orphaned re-run is described above as consequence, not as a repair target: once the wake no longer re-dials, there is no orphan to manage.

Do not touch the token scraper or its feed (`desktop/src-tauri/src/devserver.rs:1062-1083`, `desktop/src-tauri/src/embedded.rs:424-425`, `crates/chan-library/src/host.rs:1911`). They are named here only to establish that the viewport clear is safe.

Do not make the server belt depend on tenant kind. There is no control-tenant flag on the registry, and adding one to serve this fix would put desktop product knowledge inside `chan-library`'s session registry. The condition is "an explicit session id the server does not have", which is well-defined for every caller.
