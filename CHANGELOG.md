# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [v0.13.0] - 2026-05-23

Phase-8 closing release. Public-flip pre-flight docs landed, screensaver themes shipped, terminal renderer regression fixed, chan-server async-blocking cleanup, plus a number of UI polish and bug fixes.

### Added

- Apache 2.0 `LICENSE` at the repo root.
- `CONTRIBUTING.md` with build/test/PR submission instructions plus the architectural ground rules (drive boundary, single binary, local-first, MCP-only, pinned toolchain).
- `CODE_OF_CONDUCT.md` adapted from Contributor Covenant 2.1.
- `SECURITY.md` with private disclosure policy, 90-day window, and chan-drive sandbox identified as the primary security boundary.
- `.github/ISSUE_TEMPLATE/bug_report.md` and `feature_request.md`, plus `PULL_REQUEST_TEMPLATE.md` with the pre-push gate checklist.
- `docs/coordination.md` explaining the multi-agent development pattern visible in the journals to outside readers.
- `CHANGELOG.md` (this file) seeded with v0.6.23 through v0.13.0 entries.
- Screensaver visual themes: Matrix rain (default) and code-drawn Castaway pixel-art scene with eight animation states (idle / wave / sit / sleep / drink / walk / fish / ship).
- Settings theme picker for screensaver (Matrix or Castaway), persisted per drive.
- Screensaver `prefers-reduced-motion` handling: Matrix rain drops to once-per-second refresh instead of full animation.
- Right-click menu footer rows across Terminal, File Browser, Graph, and Editor: Settings (toggle), Reopen Closed Tab, and Close.

### Changed

- Terminal and editor tab header clicks now focus the tab content: terminals are ready to type immediately, editors are ready to edit.
- Hybrid pane hamburger menu cleaned up to match the addendum-a spec: stale "Light mode" and "Flip pane" entries removed (pane flip is now per-tab-kind via the tab's settings).
- Editor right-click menu reordered: name row now leads, page width slider follows after the first separator.
- Terminal right-click menu: redundant separator after the name row removed.
- Screensaver inactivity timeout bounded to `[10s, 3600s]`; the chan-server PATCH endpoint rejects out-of-range writes with `400 Bad Request`.
- chan-desktop bundle metadata bumped to track v0.13.0 release artifacts.
- chan-desktop updater public key rotated to the production identity.

### Fixed

- Terminal WebGL renderer glyph atlas corruption under animated SGR sequences (per-character substitution when colored text and animations hit the renderer simultaneously). Detects styled output, coalesces a texture-atlas refresh on the next animation frame, keeps WebGL enabled.
- File Browser: Drafts subtree refreshes correctly after `Cmd+N` creates a new draft. `refreshTreeForPath` now climbs to the nearest loaded ancestor instead of no-oping when the immediate parent of the new file is not yet loaded. Fixes the symptom where Drafts looked empty after creating a draft, Graph tabs stalled, and `Cmd+N` appeared to do nothing.
- chan-server: GET `/api/files` and `/api/files/<path>` no longer block the async runtime. Sync filesystem work moved behind `tokio::task::spawn_blocking`. Resolves the 10s timeouts on small-file reads observed under indexer / watcher contention.
- chan-server: twelve additional route handlers (`fs_graph`, `terminal`, `fonts`, `index`, `graph`, `report`, `search`, `inspector`, `attachments`, `contacts`, `drive`) plus `static_assets` audited and moved to `spawn_blocking` or `tokio::fs` so request-path filesystem and graph work no longer starves Tokio workers.
- chan-server: terminal watcher event listing now caps individual event files at 1 MiB before reading them, preventing memory pressure on stray large files in attached watcher directories.
- chan serve: bind-port errors now name the requested listen address (e.g. `127.0.0.1:8787`) instead of returning a generic message.
- chan-tunnel-client and chan-tunnel-server: removed twelve confirmed-unused dependency edges; `cargo machete` clean.
- chan-tunnel-server: integration-test `reqwest` `stream` feature now explicit, no longer relying on feature unification.
- Repo history audited (gitleaks) for the open-source flip: three pre-release loopback bearer-token entries found, all triaged as acceptable (stopped local services), no purge required.

### Removed

- chan-tunnel-client and chan-tunnel-server: twelve unused dependencies (`anyhow`, `async-trait`, `bytes`, `http-body`, `http-body-util`, `pin-project-lite`, `serde`, `serde_json`, `tower`, and friends across the two crates).

## [v0.12.0] - 2026-05-23

### Added

- Drafts metadata workspace: New Draft creates `Drafts/<name>/draft.md`
  inside chan metadata, outside the drive root.
- Drafts now appear in the file browser, inspector, graph, rich prompt
  history, drive read/write/list/stat primitives, BM25 indexer, and watcher
  flow.
- Team workspace bootstrap: team config, watcher load, per-cell pane
  assignment, worker spawn, identity prompt staging, lead PTY rename, and
  restart.
- Team APIs for create, duplicate, load, unload, loaded state, and config.
- Split-pane real-estate grid with per-cell team member assignment.
- Screensaver with per-drive enable state, timeout, PIN storage, and manual
  lock chord.
- Settings Features section for chan-reports and BGE semantic-search toggles.
- Mention completion merges contacts and the mention corpus.
- Cross-platform release pipeline for Linux CLI packages and signed,
  notarized macOS chan-desktop DMGs.

### Changed

- Hybrid Nav moved to transactional staging for T/O/P/G/E operations.
- Right-click menus were rebuilt for Terminal, File Browser, and Editor.
- Carousel moved into the Infographics tab; the welcome pane is now a static
  spawn grid.
- First boot now opens with a docked file browser on the left.
- chan-desktop defaults to native monospace fonts per OS and supports optional
  Source Code Pro download.

### Fixed

- Rich Prompt cursor and placeholder now share the same row.
- Hang-recovery banner persists unsaved editor content and restores it after
  reload.
- Terminal resize now converges columns to final pane width.
- Toasts auto-dismiss across success and info surfaces.
- Silent axum 0.7 path-param mismatch on team routes was fixed.
- PTY soft-wrap test refactor removed cross-lane drift seen in release smoke.

### Removed

- Legacy Alt+Space rich-prompt chord.

## [v0.11.2] - 2026-05-21

### Added

- First signed and notarized chan-desktop release path.
- Tag-triggered signed + notarized chan-desktop workflow.
- Bundled `chan` binary fallback for chan-desktop, with PATH-first lookup and
  version match.
- Local `make app-notarized` path using notarytool Keychain profile.

### Changed

- chan-desktop signing identity rotated to the release Developer ID identity.

### Fixed

- Missing-file panel no longer falsely appears while the file still exists.
- Re-open action and suggest-reopen flow restored for legitimate moved files.
- Source-mode editor no longer auto-continues markdown lists.
- Wysiwyg ordered lists render dotted outline numbering.
- Copied-path notification auto-dismisses.
- Pre-flight spinner no longer sticks at `0:00`.
- Submit-mode toolbar toggle persists across reload.
- Shell-mode tooltip copy no longer claims to append a trailing newline.
- File-browser expand/collapse state persists across tab switches.
- Spawn chord always creates a new file-browser tab.
- chan-desktop Reload and Open Inspector menu items work.
- Browser-style zoom works in chan-desktop and persists per window.

## [v0.11.1] - 2026-05-20

### Added

- Per-prompt page-width slider for Rich Prompt.
- Shell/agent submit-mode toolbar toggle and Claude Code submit chord path.
- Graph ancestor breadcrumb navigation.
- Inline file rename band above the page-width cap.

### Changed

- Rich Prompt bubble overlay, collapse/expand spacing, terminal broadcast
  selector, chord surface, and graph-from-here defaults were polished.
- chan-desktop window title now shows the drive path.

### Fixed

- BubbleOverlay regression cluster around filtering, dismissal, and flicker.
- Collapse/expand dead-space recompute.
- Wysiwyg paste now keeps markdown unescaped.
- Event watcher silently skips non-matching filenames.
- CLI no-default-features build is clean.

## [v0.11.0] - 2026-05-19

### Changed

- Workspace, web, and Tauri desktop metadata bumped to 0.11.0 for the phase-7
  wrap.
- Cargo.lock and package-lock metadata refreshed for the release boundary.

### Fixed

- Release verification passed: release build, release tests, web build,
  pre-push gate, and `chan --version`.

## [v0.10.1] - 2026-05-18

### Changed

- Version metadata bumped to 0.10.1.

## [v0.9.0] - 2026-05-17

### Added

- chan-native persistent PTY sessions with byte-offset ring, idle prune,
  alt-screen-aware reattach, and winsize handling.
- Terminal environment now exposes only the `CHAN_` MCP namespace, with a
  per-tab MCP env toggle.
- VCS-aware indexing for git/hg checkouts.
- Search aggression budget and `chan serve --search-aggression`.
- xterm.js Alt-key word motions.

### Changed

- chan-llm became MCP-only.
- chan-desktop windows key editor sessions per window; browser tabs use
  per-tab session storage.

### Fixed

- Confirm-on-close for dirty editor tabs and live terminal tabs.
- Editor caret restore uses nearest scrolling.

### Removed

- In-app Agent overlay.
- chan-drive assistant blob API.

## [v0.8.1] - 2026-05-14

### Changed

- Maintenance release for the pre-release track.

## [v0.7.1] - 2026-05-11

### Changed

- Maintenance release for the pre-release track.

## [v0.6.23] - 2026-05-11

### Changed

- Final v0.6.x maintenance release before the v0.7 line.

[Unreleased]: https://github.com/fiorix/chan/compare/chan-v0.13.0...HEAD
[v0.13.0]: https://github.com/fiorix/chan/compare/chan-v0.12.0...chan-v0.13.0
[v0.12.0]: https://github.com/fiorix/chan/compare/chan-v0.11.2...chan-v0.12.0
[v0.11.2]: https://github.com/fiorix/chan/compare/chan-v0.11.1...chan-v0.11.2
[v0.11.1]: https://github.com/fiorix/chan/compare/v0.11.0...chan-v0.11.1
[v0.11.0]: https://github.com/fiorix/chan/compare/v0.10.1...v0.11.0
[v0.10.1]: https://github.com/fiorix/chan/compare/v0.9.0...v0.10.1
[v0.9.0]: https://github.com/fiorix/chan/compare/v0.8.1...v0.9.0
[v0.8.1]: https://github.com/fiorix/chan/compare/v0.7.1...v0.8.1
[v0.7.1]: https://github.com/fiorix/chan/compare/v0.6.23...v0.7.1
[v0.6.23]: https://github.com/fiorix/chan/releases/tag/v0.6.23
