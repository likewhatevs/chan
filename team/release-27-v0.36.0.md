# Phase 27 - Opt-in workspace lifecycle + the v0.36.0 Windows smoke fixes

Status: code+docs complete, gated green, merged to `origin/main`; **not yet released** (the version bump + tag are deferred to the release cut, treated as a continuation of the same in-flight version). `make pre-push` green and `cargo xwin` (`x86_64-pc-windows-msvc`, `-D warnings`) green for the touched crates at each step. The Windows *runtime* is being smoked on @@Alex's hardware off the `release-desktop.yml` dry-run NSIS artifact: `cs` working is confirmed (W4); the named-pipe hand-off, the markdown-load fix (W5), and the cross-window tab-DnD guard are pending his pass.
Span: 2026-06-15 → 2026-06-16.
Tags: #desktop #windows #workspace-lifecycle #launcher #ipc #named-pipe #handoff #terminal #drag-and-drop #packaging

Scoped directly from @@Alex's real-hardware smoke of the phase-26 v0.36.0 Windows build. Two threads: (1) a deliberate **opt-in workspace lifecycle** cleanup of chan-desktop (no default workspace; standalone-terminal boot; config consolidation; persisted on/off; Remote-inbound removal), and (2) the **Windows bugs the v0.36.0 smoke surfaced** — first three known up front (W1-W3), then more found live during this round's smoke (W4-W5), plus a cross-window tab-DnD bug and the `chan serve` hand-off gap. Round 1 ran as the `new-team-5` team (@@Lead + lanes, the `cs terminal team` tooling); after a mid-round session crash the remaining Windows-smoke fixes were landed solo by @@Lead in direct collaboration with @@Alex.

## Roadmap (the asks)

`dev/phase-27/plan.md` is the design of record. Opt-in lifecycle:

- **B1** - remove the default workspace end to end: no `~/Documents/Chan`, no seeded manual, and no first-run create/choose/factory-reset prompt.
- **B2** - boot into a standalone terminal window + an empty launcher (a workspace is now strictly opt-in).
- **B3** - consolidate desktop config under `~/.chan/desktop/config.json` (off the OS app-support dir; keeps the CLI registry pure).
- **B4** - persist which workspaces are on across restart and re-serve them on the next boot (the boot matrix).
- **B5** - drop the separate manual tarball from releases.
- **B6** - remove Remote **inbound** from chan-desktop entirely (shed the embedded inbound tunnel listener; the standalone gateway `chan-tunnel-server` crate stays); rename the remaining outbound mode's label to "Remote".

Windows v0.36.0 fixes:

- **W1** - in-app terminal hang on first open (blocking Git BASH resolve on the async runtime).
- **W2** - `chan` / `cs` not on PATH from the desktop install.
- **W3** - missing Settings chord label in the tab right-click menus.

## What shipped

**Opt-in lifecycle (B1-B6):**
- **`6c270343` / `fb40e5fa`** - B1: removed the default-workspace prompt machinery on both sides (the four `default_workspace_*` Tauri commands + their `default_workspace.rs` backing + the frontend `maybePromptDefaultWorkspace` / dialogs / `invoke('default_workspace_*')`), plus the two serve.rs guardrail tests that asserted the old prompt.
- **`6e2815b2`** - B4 + B2 (rust): persist `enabled_workspaces` and re-serve them on boot (the §3.2 boot matrix); a fresh launch with no workspaces opens the empty launcher + a standalone terminal window.
- **`ffac2c0c`** - B3: desktop config moved to `~/.chan/desktop/config.json`.
- **`0f9d5e65` / `aabc08b4`** - B6: removed the embedded Remote **inbound** tunnel listener (backend) and the inbound tab/renderers/state/`invoke('tunnel_*')` (frontend); renamed the outbound mode's label to "Remote"; the launcher boots to the empty state. The separate `gateway/` workspace's `chan-tunnel-server` is untouched.
- **`647ee32f`** - B5: releases no longer build/ship the manual tarball.
- **`5829c05e` / `14531eaf`** - docs: `desktop/design.md` refreshed to the opt-in lifecycle (§3.0-§3.2, config home, the boot matrix, the "Remote" label); inbound-listening prose dropped from current-state docs.

**Windows smoke fixes (W1-W5, hand-off, tab-DnD):**
- **`767b1616`** - W1 + W2 (server half): prime the Git BASH discovery cache off the async request path so the first terminal create doesn't freeze the SPA; prepend the chan bin dir to the spawned shell's PATH.
- **`a280935e`** - W2 (desktop half): write extensionless POSIX `chan`/`cs` shims beside the `.cmd` shims (Git BASH ignores `PATHEXT`), and broadcast `WM_SETTINGCHANGE` after the per-user PATH write so a fresh shell resolves them without a logout.
- **`4824994d`** - W3: show the Settings chord (`Ctrl+,`) in the terminal-tab and editor-tab right-click menus.
- **`506e4853`** - W4 (found in this round's smoke): a release `chan-desktop.exe` is GUI-subsystem (no console), so the `chan`/`cs` CLI dispatch wrote stdout to a null handle and "returned empty". Attach to the parent console (`AttachConsole(ATTACH_PARENT_PROCESS)`) and bind any null std handle to `CONOUT$`/`CONIN$`, preserving `> out.txt` redirection.
- **`28ea2ffe`** - W5 (found in this round's smoke): a markdown file spun on "loading" forever with the byte count already complete — `readFileStream` only finished on HTTP body EOF, which WebView2 doesn't reliably surface to fetch's `ReadableStream`. Complete on the server's explicit ndjson `done` event instead (then `reader.cancel()`); keeps the EOF/flush path as fallback.
- **`2f7018d4`** - Windows `chan serve` hand-off (found in this round's smoke): the CLI↔desktop hand-off channel (`chan-server/src/handoff.rs`) was Unix-domain-socket only, so a Windows `chan serve <path>` couldn't reach a running desktop and fell back to a standalone browser server, leaving the workspace registered-but-off in the launcher. Implemented the hand-off over a named pipe (per-user `\\.\pipe\chan-desktop-<user>`), mirroring the proven control_socket.rs seam; un-gated the desktop's hand-off listener + `open_workspace_from_handoff` to Windows.
- **`6065df5c`** - cross-window tab drag-and-drop guard: tabs can no longer be dragged between a standalone terminal window and a workspace window, or between different workspaces (refused at dragover with a no-drop cursor and at drop with the source tab kept). Each window stamps a drag "scope" (its `?w=` label minus the per-window seq) as a drag MIME type; same-scope (intra-window, same-workspace-multi-window, terminal↔terminal) moves still work.
- **`ef7cf335`** - chan-server Windows-target clippy cleanup (a doc-list continuation + a detached `spawn_blocking` handle) surfaced by running `cargo xwin clippy` on the W1 code.

## Verification

- **Scoped own-gates** per change (fmt + clippy `-D warnings` + tests; `make web-check` for frontend), plus the authoritative full-tree `make pre-push` green over the committed state.
- **Windows cross-compile + lint:** `cargo xwin clippy --target x86_64-pc-windows-msvc -p chan-server -p chan-desktop` under `-D warnings`, green (lib+bin) — the real check that the named-pipe hand-off, the console-attach FFI, and every cfg arm compile and lint clean for Windows.
- **macOS browser smoke (Chrome, real `chan serve`):** the W5 file-load fix loads single- and multi-chunk files fully (first+last markers, no truncation), clears the spinner, and stays editable; the tab-DnD guard was exercised end-to-end via synthetic DnD against the real handlers (intra-window reorder works; a foreign-scope cross-window drop is rejected with 0 tabs added; a same-workspace cross-window drop still adopts the tab).
- **Empirically pending @@Alex (Windows hardware):** the named-pipe hand-off, W5, and tab-DnD on a real desktop (cross-window OS-drag and WebView2 behavior can't be driven from the Mac/Chrome harness). `cs`/`chan` printing (W4) is confirmed working from @@Alex's smoke; the rest ride the `release-desktop.yml` dry-run NSIS artifact.

## Deferred / follow-ups

- **Windows workspace-open hang (under investigation):** opening a workspace on Windows can hang the UI and block document loads even after the hand-off fix lands — suspected event loop or semantic indexing (semantic search was enabled). Its own investigation; not part of this round.
- **Windows `chan serve` with no running desktop:** still serves standalone (today's behavior); auto-launching chan-desktop on Windows (a `current_exe()` relaunch with `ARGV0` cleared) is deferred.
- **`chan upgrade` over the Windows hand-off:** deferred (no Windows updater feed this phase).
- **CHANGELOG gap:** v0.35.0 and v0.36.0 shipped without CHANGELOG entries; this round's notes sit under `[Unreleased]`. Backfilling the two missing releases is a separate cleanup.
