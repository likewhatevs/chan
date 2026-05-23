# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

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

[Unreleased]: https://github.com/fiorix/chan/compare/chan-v0.12.0...HEAD
[v0.12.0]: https://github.com/fiorix/chan/compare/chan-v0.11.2...chan-v0.12.0
[v0.11.2]: https://github.com/fiorix/chan/compare/chan-v0.11.1...chan-v0.11.2
[v0.11.1]: https://github.com/fiorix/chan/compare/v0.11.0...chan-v0.11.1
[v0.11.0]: https://github.com/fiorix/chan/compare/v0.10.1...v0.11.0
[v0.10.1]: https://github.com/fiorix/chan/compare/v0.9.0...v0.10.1
[v0.9.0]: https://github.com/fiorix/chan/compare/v0.8.1...v0.9.0
[v0.8.1]: https://github.com/fiorix/chan/compare/v0.7.1...v0.8.1
[v0.7.1]: https://github.com/fiorix/chan/compare/v0.6.23...v0.7.1
[v0.6.23]: https://github.com/fiorix/chan/releases/tag/v0.6.23
