# Phase 28 - chan devserver + killing the default workspace

Status: code+docs complete, gated green, merged to `origin/main`, and **released as v0.38.0** (2026-06-17), with the **v0.38.1** patch (2026-06-18) closing the round's smoke fallout. `make pre-push` green over the committed state at each step; `cargo xwin` (`x86_64-pc-windows-msvc`, `-D warnings`) green for the touched crates. The Linux devserver was smoked in a lima VM end to end; the macOS `--launchd` path on Alex's hardware.
Span: 2026-06-16 → 2026-06-18.
Tags: #devserver #workspace-lifecycle #systemd #launchd #desktop #terminal #boot-perf #editor #ipc #handoff

The first round after the phase-27 Windows-smoke work (released as v0.37.0). Two large threads plus boot-performance work: (1) introduce **`chan devserver`** - one headless process that hosts many workspaces behind a single port, with `chan serve PATH` registering into it and chan-desktop connecting to list, open, and run terminals against it; and (2) **kill the default workspace** end to end on the CLI and server (chan-desktop had already dropped it in v0.37.0), so a workspace is strictly opt-in. Boot-performance work unblocked the SPA on a cold open of a very large repo. Ran as a six-member team (Lead + Boot / Devserver / Desktop / Core / CI); round 1 attacked the devserver + lifecycle threads in parallel, round 2 landed `--systemd`/`--launchd`, the reconnect lifecycle, and the smoke patch (v0.38.1).

## Roadmap (the asks)

`dev/old/v0.38.0/plan.md` is the design of record (six workstreams):

- **1 - Kill the default workspace.** `chan serve` with no path errors instead of falling back to `~/Documents/Chan`; drop the per-machine default-workspace setting and the Dashboard field. (Core + the CLI line Devserver.)
- **2 - Fast boot + lazy loading for very large repos.** Unlock the SPA on a cold boot of a huge repo instead of blocking on the first index build.
- **3 - `chan devserver`** - a headless multi-workspace aggregator on one port; `chan serve PATH` registers into it and exits; `--systemd` (Linux) supervises it as a user service. (Devserver.)
- **4 - chan-desktop ↔ devserver integration.** Connect to a devserver, list its workspaces in their own launcher group, run standalone terminals on it, and survive reconnect/restart with token rotation. (Desktop.)
- **5 - `cs team load --script` reliability** (low priority). (Core.)
- **6 - Parallel Windows builds** (low priority). (CI.)

## What shipped

**Kill the default workspace (1):**
- **`038a1ca4` / `0739b919` / `f1b2075c`** - `chan serve` requires an explicit workspace path (the CLI, the chan-workspace require-path refactor, and the Dashboard's default-workspace-root config removal); `2dcc804c` snapshot-forms the require-path messages.

**Fast boot (2):**
- **`65864d9d` / `06f18b6b` / `b59c94ed`** - unlock the SPA on cold boot of a huge repo: take the report scan off the boot URL path, log cold-boot timing (listener / watcher / ready). `afa26b2c` builds the search index shallow-first; `144ff358` shows "still indexing" for an empty search during a cold build.

**`chan devserver` (3):**
- **`d2ba5880` / `51740634`** - the headless multi-workspace devserver and the `chan serve` → devserver registration.
- **`8a714afa` / `c7fedaed`** - `chan devserver --systemd` (Linux user service + journald); only enable-linger when it is actually off.
- **`e29d6691` / `7a672dbb` / `c117a051`** - terminal-tenant infrastructure: run a command on a tenant PTY, close a tenant by prefix (reaping its PTYs), expose a tenant's scrollback for token-scraping.
- **`f61194a6`** - unify graceful shutdown across `serve` and `devserver` (SIGINT/SIGTERM with a hard deadline; durable config write; `--port 0` reports the bound port).

**chan-desktop ↔ devserver (4):**
- **`02501628` / `98a0f910` / `70ad7618`** - the devserver launcher slice (config, persisted commands, New→Devserver form, grouping); connect and list workspaces; run a connect script in a control terminal.
- **`b09850ee` / `d0044de1` / `f015496f`** - edit/forget for devservers and their workspaces; tear down a devserver's window set on disconnect/forget; group hidden devserver windows in the Window menu.
- **`d2bf81d8` / `9fcd3a50` / `503209c4`** - scrape the devserver token from the control terminal and reconnect across a restart (token rotation); the `CHAN_DEVSERVER_TOKEN=` marker.
- **`06fc2672`** - share one terminal tenant per devserver for new-window + tab drag-and-drop; **`07b3d089` / `cd424ea1`** complete the Tauri ACL audit for the devserver commands.
- **`6347390d` / `1b2b7ee2`** - editor: land the image-paste caret on the image's line; backspace near an inline image no longer deletes it.

**Dependencies + docs:**
- **`0bf62006` / `69279452`** - patch npm advisories in web + gateway; upgrade gateway vite 5→8.
- **`ed852c2e` / `b02d7c36` / `8f89b26f`** - devserver + WorkspaceHost architecture docs and the new manual Devserver page.

**v0.38.1 patch (round-2 smoke fallout):**
- **`6752bb01`** - `chan devserver --launchd` (macOS): supervise under a per-user launchd LaunchAgent (`app.chan.devserver`), the macOS counterpart to `--systemd`.
- **`de4bdeea`** - editor: opening a CRLF Markdown file no longer freezes the editor in a reactive render loop (compare/write against CodeMirror's LF-normalized document).
- **`f856f895` / `748ba563`** - `chan devserver --systemd` surfaces the bearer token even when the user cannot read the journal (emit the `CHAN_DEVSERVER_TOKEN=` marker directly from the persisted config), and keeps supervising when journal streaming stops.

## Verification

- **Scoped own-gates** per change (fmt + clippy `-D warnings` + tests; `make web-check` for frontend), plus the authoritative full-tree `make pre-push` over the committed state (the `gate-r4/r5/r6` logs under `dev/old/v0.38.0/`).
- **Linux devserver E2E in lima:** `dev/old/v0.38.0/devserver-verify.sh` exercises `chan serve` → devserver registration, the management API, and the systemd unit lifecycle.
- **macOS:** the `--launchd` agent load/re-attach and the browser smoke of the desktop devserver launcher on Alex's hardware.

## Deferred / follow-ups

- Full reconnect/persistence of standalone terminals at the launcher scope was scoped but carried into the next round (the v0.39.0 W10 work).
- Devserver lock correctness under rapid open/on/off churn, and the long-running-devserver file-descriptor leak, surfaced here and were taken up next round.
