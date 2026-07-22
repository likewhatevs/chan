# Rotate The Devserver Bearer Token, And Keep It Out Of Snapshotted Scrollback

> Status: shipped in [v0.74.0](../../release/release-v0.74.0.md).

Status: accepted scope for v0.74.0. Two related credential-handling findings in the devserver token path, surfaced while diagnosing an unrelated control-terminal defect. Hardening, not an incident: both require an attacker who already has a foothold that would be serious on its own.

## Problem

### The token is minted once and never rotates

`run_devserver` loads the persisted config and mints a token only when the stored one is empty: `crates/chan-server/src/devserver.rs:649-653`. That value is copied into `DevserverState.token` (`crates/chan-server/src/devserver.rs:709`) and written back through `DevserverStore::save`, which writes `~/.chan/devserver/config.json` atomically and chmods it `0600` before the rename (`crates/chan-server/src/devserver.rs:122-176`, path resolved at `crates/chan-server/src/devserver.rs:194-200` through `chan_workspace::paths::config_dir`, so `CHAN_HOME` relocates it). The struct's own doc states the intent: the token is "minted once and reused so a reconnecting client keeps working across restarts" (`crates/chan-server/src/devserver.rs:101-108`).

Grepping `devserver_token` across `crates/` and `desktop/` returns exactly one assignment outside tests, the mint-if-empty at `crates/chan-server/src/devserver.rs:650-651`. There is no rotation anywhere: no timer, no age field in `PersistedConfig` (`crates/chan-server/src/devserver.rs:105-121`), and no CLI verb that clears or re-mints it (`crates/chan/src/help.rs:238` documents the file only as "0600, bearer token"). The only way to rotate today is for a human to delete or edit that JSON file and restart. Existing tests pin the current behaviour deliberately: `crates/chan/tests/devserver_resilience.rs:1000-1003` asserts `token1 == token2` across a SIGKILL and restart.

The consequence, stated plainly: a token printed in a terminal months ago still authenticates the devserver management API today. It survives every restart, every `chan devserver --restart`, and every `--service=systemd --join` re-attach.

Three code paths broadcast it. The foreground start prints both the launcher URL with the token in the query string and the machine marker: `println!("chan devserver: listening on http://{local_addr}/?t={token}")` at `crates/chan-server/src/devserver.rs:853` and `println!("{DEVSERVER_TOKEN_MARKER}{token}")` at `crates/chan-server/src/devserver.rs:860`, the marker constant being `CHAN_DEVSERVER_TOKEN=` (`crates/chan-server/src/devserver.rs:207`). The supervised paths re-emit the marker from the persisted file so a journal-follow still carries it: `crates/chan/src/lib.rs:3930-3940`, called from `crates/chan/src/lib.rs:3665`, `:3696`, `:3755`, `:4227`, `:4249`, `:4297` and from `crates/chan/src/devserver_daemon.rs:83` and `:248`.

Two comments in the desktop assert a rotation that does not happen. `desktop/src-tauri/src/devserver.rs:29-32` justifies keeping `DevserverConn` in memory because "the bearer token rotates with the devserver", and `desktop/src-tauri/src/devserver.rs:1066-1068` says "a recycled or rotated devserver is handled by construction". The client-side handling is real and correct; the server-side rotation those comments describe does not exist.

### Control-terminal scrollback carrying the token is snapshotted to disk

The desktop scrapes the token out of the control terminal's PTY output. `scrape_token` (`desktop/src-tauri/src/devserver.rs:1074-1083`) takes the LAST occurrence of the shared marker via `rmatch_indices` and reads the url-safe run after it. Taking the last match is deliberate, so a script re-run or a devserver restart wins over an older line, and the test at `desktop/src-tauri/src/devserver.rs:2489-2508` pins exactly that. The design therefore depends on the marker staying in that terminal's scrollback across restarts.

The SPA persists that scrollback. `captureSnapshot` in `web/packages/workspace-app/src/components/TerminalTab.svelte:1264-1284` serializes the screen plus up to `SNAPSHOT_SCROLLBACK_LINES` (1000, `web/packages/workspace-app/src/terminal/snapshotCache.ts:47`) into `writeTerminalSnapshot`, and it is registered on both `pagehide` and `beforeunload` at `web/packages/workspace-app/src/components/TerminalTab.svelte:1291-1298`. `writeTerminalSnapshot` puts the ANSI dump into `localStorage` under `chan:term-snapshot:<sessionId>` (`web/packages/workspace-app/src/terminal/snapshotCache.ts:31`, `:87-96`), with a 3-day TTL (`web/packages/workspace-app/src/terminal/snapshotCache.ts:34`). The brief's key shape and pagehide description are both correct as written.

There is no control-window exclusion. The SPA already knows what a control window is: `isControlTerminalWindow()` reads `?kind=control` (`web/packages/workspace-app/src/state/store.svelte.ts:307-309`), it is surfaced as `ui.terminalControl` (`web/packages/workspace-app/src/state/store.svelte.ts:278`), and it already gates behaviour at `web/packages/workspace-app/src/components/TerminalTab.svelte:820` and `:840` and in `windowModeAllowsCommand` (`web/packages/workspace-app/src/state/windowMode.ts:60-67`). `captureSnapshot` consults none of it. The desktop opens the control terminal with that exact marker: `spawn_control_terminal_window` at `desktop/src-tauri/src/serve.rs:485-511`, with the `kind=control` URL contract noted at `desktop/src-tauri/src/serve.rs:1420`.

The only removal paths are incidental. `clearTerminalSnapshot` runs when the server sends a `closed` frame for that session (`web/packages/workspace-app/src/components/TerminalTab.svelte:1069-1072`), and `pruneTerminalSnapshots` evicts on TTL and total-size cap only (`web/packages/workspace-app/src/terminal/snapshotCache.ts:152-175`). A control terminal that is simply closed with the app, or whose window is torn down without a `closed` frame, leaves a snapshot on disk in the WebView's `localStorage` store containing the literal `CHAN_DEVSERVER_TOKEN=<token>` line, for up to three days, refreshed on every hide.

### Blast radius

Be precise about what the token buys and from where.

What it authorizes: `require_bearer` (`crates/chan-server/src/devserver.rs:1369-1389`) gates every `/api/devserver/*` route except `info` and `health`, comparing in constant time via `bytes_eq` (`crates/chan-server/src/devserver.rs:1394-1404`). The gated set is `GET/POST /api/devserver/workspaces`, `DELETE`/`POST /api/devserver/workspaces/{*prefix}`, `GET /api/devserver/windows`, and `POST /api/devserver/systemd-fdstore/prepare` (`crates/chan-server/src/devserver.rs:1041-1060`). The same token gates the launcher bundle's `/api/library/*` data routes (`crates/chan-server/src/devserver.rs:1068-1083`), whose watch WebSocket also accepts it as `?t=` (pinned at `crates/chan-server/src/devserver.rs:2979`).

That set is not narrow. `handle_open` mounts an arbitrary absolute path supplied in the request body (`crates/chan-server/src/devserver.rs:1257-1265`), and `GET /api/devserver/workspaces` returns each mounted workspace's per-tenant bearer token in the clear (`crates/chan-server/src/devserver_api.rs:88-89`, populated at `crates/chan-server/src/devserver.rs:317-321`). A tenant token is the full workspace credential, which includes the terminal surface, so possession of the devserver token is effectively code execution as the devserver's user.

Over what transport, and from where: there is no TLS. The default bind is loopback (`DEFAULT_DEVSERVER_BIND = IpAddr::V4(Ipv4Addr::LOCALHOST)`, `crates/chan/src/lib.rs:100-102`), and binding wider prints an explicit warning that names `ssh -L` as the supported remote path (`crates/chan/src/lib.rs:3054-3064`). The desktop's own module doc describes the intended topology the same way (`desktop/src-tauri/src/devserver.rs:1-6`, `:35-37`). So the normal deployment is: loopback only, reached remotely through an `ssh -L` forward the user established.

What an attacker would already need: either code execution on the box as some user who can reach the loopback port, or access to a machine with a live `ssh -L` forward into it, or an operator who bound non-loopback against the warning. Note also that both the config file (`0600`) and the WebView `localStorage` store are owned by the same user the devserver runs as, so a same-user attacker already has the token from the config file and gains nothing from the snapshot. The snapshot finding matters for the cases where a file leaves that boundary while the config file does not: a home-directory backup or sync, an application-data copy handed to support, a screen recording or screenshot of the control terminal, a pasted log. The rotation finding is what turns any such one-time leak into a permanent one.

## Desired contract

### Rotation

The devserver gains a way to invalidate its bearer token, and every first-class client re-derives the new one without human help.

What already survives rotation, verified:

- The desktop remote path. `DevserverConn` is memory-only by design (`desktop/src-tauri/src/devserver.rs:29-33`), and the connect path re-scrapes on every connect and script re-run (`desktop/src-tauri/src/devserver.rs:1063-1083`). Re-registering a conn is treated as a fresh registration (`desktop/src-tauri/src/devserver.rs:105-113`). The two comments claiming rotation become true rather than aspirational.
- The desktop local path. `read_local_token` reads `~/.chan/devserver/config.json` on each call (`desktop/src-tauri/src/devserver.rs:1032-1049`), so it picks up a rotated value on the next read.
- The CLI management verbs. They resolve through `persisted_devserver_token` per invocation (`crates/chan-server/src/devserver.rs:214-218`, used at `crates/chan/src/lib.rs:3791` and via `resolve_devserver_token` at `crates/chan/src/lib.rs:3900-3940`), so `cs` and `chan devserver` verbs follow the file.
- An `ssh -L` forward. It is transport only and carries no credential, so an already-established forward keeps working; only the bearer presented over it changes.

What breaks and must be named in the migration:

- Any browser tab opened at the printed `/?t=<old-token>` URL (`crates/chan-server/src/devserver.rs:853`). It must be reopened at the newly printed URL. This is the one unavoidable human step.
- Any long-lived process holding a scraped token that does not re-scrape. The marker re-emit on start (`crates/chan-server/src/devserver.rs:860`) and on `--join` re-attach (`crates/chan/src/lib.rs:3930-3940`) is the distribution channel; a rotation that does not re-emit the marker is not a valid rotation.
- `crates/chan/tests/devserver_resilience.rs:1000-1003`, which asserts token stability across restart. Whatever contract this item picks, that assertion must be restated to match it, deliberately and visibly, not deleted.

The recommended shape, because it keeps the documented "a reconnecting client keeps working across restarts" property (`crates/chan-server/src/devserver.rs:101-103`) while removing the "forever" part:

1. An explicit operator verb that re-mints the token, persists it, and prints the new marker plus the new `/?t=` URL. This is the response to a suspected leak and it is the piece with no substitute today.
2. An age field in `PersistedConfig` recording when the token was minted, and rotation on cold start once the token is older than a fixed maximum. Restart-stability is preserved inside the window; a devserver that has been up or restarting on the same token for longer than the maximum rotates and re-emits.

If only one of the two lands, land the verb.

### Snapshot

Small and local. A control window must never write a scrollback snapshot:

- `captureSnapshot` returns early for a control window, using the same `ui.terminalControl` signal as `web/packages/workspace-app/src/components/TerminalTab.svelte:820` and `:840`. Put the decision in a pure exported predicate next to `windowModeAllowsCommand` (`web/packages/workspace-app/src/state/windowMode.ts:60-67`) so it is unit-testable without mounting the component.
- Any snapshot already on disk for a control session is removed rather than left to the 3-day TTL: call `clearTerminalSnapshot` (`web/packages/workspace-app/src/terminal/snapshotCache.ts:139-142`) on control-window teardown, and have the load-time sweep in `pruneTerminalSnapshots` drop control-session entries unconditionally so tokens written by the current build are cleaned up on the first run of the fixed build.

The reattach cost is nil in practice: the control terminal is a single-shot local runner with one PTY, and losing its snapshot only means a full replay from the server ring.

## Acceptance

Every check below names the mutation that must make it red, per the standing rule recorded at `team/release/release-v0.73.0.md:37`.

1. Rotation, server side. A test in the `crates/chan-server/src/devserver.rs` store tests (alongside `crates/chan-server/src/devserver.rs:1620-1730`) asserting that the rotation entry point replaces the persisted `devserver_token` with a different value, keeps the file at `0600`, and preserves `library_id` and `port`. Red mutation: make the rotation entry point return the existing token without writing. Run: `cargo test -p chan-server devserver`.
2. Rotation, end to end. A test in `crates/chan/tests/devserver_resilience.rs` in the shape of `devserver_sigkill_releases_flock_and_survives_config` (`crates/chan/tests/devserver_resilience.rs:977-1010`): rotate, restart, and assert the old token now gets 401 from `require_bearer` while the newly emitted marker's token gets 200, and that the persisted workspace set still re-mounts. The existing `token1 == token2` assertion at `:1000-1003` is restated in the same commit to describe the chosen within-window stability. Red mutation: leave `require_bearer` reading a cached copy of the pre-rotation token, so the stale bearer still passes. Run: `cargo test -p chan --test devserver_resilience`.
3. Marker re-emit. A test asserting the rotation path prints `CHAN_DEVSERVER_TOKEN=<new>` (and the `/?t=` URL) so the control terminal can re-scrape. Red mutation: rotate without printing; the desktop would then be stranded on a dead token, which is precisely the failure the test must catch.
4. Desktop scrape across rotation. Extend the `scrape_token` tests at `desktop/src-tauri/src/devserver.rs:2489-2508` with an input carrying an old marker, a restart banner, and a new marker, asserting the new one wins. Red mutation: change `rmatch_indices` to `match_indices` at `desktop/src-tauri/src/devserver.rs:1075`. Run: `cargo test -p chan-desktop devserver`.
5. Control windows write no snapshot. A vitest unit test on the extracted predicate in `web/packages/workspace-app/src/state/windowMode.ts`, plus a source-pin test in the shape of `web/packages/workspace-app/src/components/confirmCloseDispatch.test.ts:42-49` asserting `captureSnapshot` actually consults it. Red mutation: have the predicate return true for `terminalControl: true`; the unit test reds. Second red mutation: delete the guard call from `captureSnapshot`; the source-pin test reds. Run: `npm run check` and the full `npm run test` in `web/packages/workspace-app`.
6. Existing snapshots are cleaned. A vitest test in `web/packages/workspace-app/src/terminal/snapshotCache.test.ts` seeding a `chan:term-snapshot:` entry for a control session and asserting the load-time sweep removes it. Red mutation: make the new sweep branch a no-op; the seeded key survives.
7. No token text in any snapshot. A vitest test asserting that a snapshot value containing the literal `CHAN_DEVSERVER_TOKEN=` is never written for a control session. This is the check that states the actual security property rather than a proxy for it. Red mutation: any of the guards above removed.
8. Owner-only, real hardware. Three things cannot be proven in an agent sandbox and must be hand-smoked by the owner on a machine with a display server and a real desktop build: that a live desktop control terminal window leaves no `chan:term-snapshot:` key containing the marker in the WebKit `localStorage` store after a hide, close and relaunch cycle; that a desktop connected to a devserver survives a rotation by re-scraping without a reconnect prompt; and that a rotation on a remote box reached over `ssh -L` leaves the forward intact and only requires reopening the browser tab. The owner also inspects the existing WebView store for pre-fix snapshots carrying the marker, and rotates once after the fix ships so any already-leaked token is dead.
9. Full gate. `make pre-push` green, including `cargo fmt --check`, clippy, and the web checks.

## Boundaries

In scope: `crates/chan-server/src/devserver.rs` (the `PersistedConfig` shape, the mint/rotate path, the marker emit), the marker emit and management-verb token resolution in `crates/chan/src/lib.rs` and `crates/chan/src/devserver_daemon.rs`, the two stale rotation comments in `desktop/src-tauri/src/devserver.rs:29-33` and `:1063-1068`, `crates/chan/src/help.rs:238` if a verb is added, and the SPA snapshot guard across `web/packages/workspace-app/src/components/TerminalTab.svelte`, `web/packages/workspace-app/src/terminal/snapshotCache.ts` and `web/packages/workspace-app/src/state/windowMode.ts`.

Out of scope, and each for a reason:

- The per-workspace tenant tokens (`crates/chan-server/src/devserver_api.rs:88-89`) and the standalone serve token at `<paths.tokens>/token` (`crates/chan-server/src/auth.rs:42-56`), which has the same never-rotates shape for the same stated reason. Tenant tokens are already re-minted on every mount (`crates/chan-server/src/devserver.rs:344-348`); the serve token is a separate surface and a separate item.
- The `/?t=<token>` URL form at `crates/chan-server/src/devserver.rs:853`. Putting a bearer in a query string is its own question, and the launcher SPA bootstrap depends on it (`crates/chan-server/src/devserver.rs:1068-1071`); changing it here would widen this item into a launcher-auth redesign.
- Gateway PATs, tunnel origin assertions, and the `TunnelOrigin` read-only downgrade (`crates/chan-server/src/devserver.rs:1084-1100`).
- The editor buffer and caret-index localStorage stores, and the snapshot size, TTL and eviction budgets (`web/packages/workspace-app/src/terminal/snapshotCache.ts:22-47`).
- TLS for a non-loopback bind. The warning at `crates/chan/src/lib.rs:3054-3064` stands as the current answer.

## Corrections to the brief

- The brief's suggested anchor `desktop/src-tauri/src/devserver.rs:1062-1083` for `scrape_token` is close but slightly off: the doc comment starts at `:1062` and the function body is `:1074-1083`, with `rmatch_indices` at `:1075`.
- The brief describes the desktop as learning the token only by scraping. There is a second path: on a local-loopback connection the desktop reads it straight out of the `0600` config file via `read_local_token` (`desktop/src-tauri/src/devserver.rs:1032-1049`). It matters for the migration, because that path needs no re-scrape at all.
- Everything else in the brief checks out: the config path, the mint-once-never-rotate behaviour, the marker and its last-match scrape, the `chan:term-snapshot:<sessionId>` key shape, and the absence of any control-window exclusion.
