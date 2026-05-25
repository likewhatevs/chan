# Phase 10 Roadmap Track A: Desktop Merge and Carryover Closure

Status: in progress.

Track A makes the desktop merge the first dependency-bearing item, then
turns the phase 8 and phase 9 carryovers into explicit work instead of
leaving them as intent in prior docs.

## Objectives

- Embed `chan-server` in `chan-desktop` for normal local drives.
- Preserve `chan serve` as a standalone CLI/server path.
- Close the desktop-native gaps that did not materialize in phase 9.
- Sweep the highest-risk phase 9 validation, docs, config, and release
  hygiene gaps into named tasks.
- Own native desktop File Browser drag-out/download support if browser-only
  drag-out is not sufficient.

## 1. Desktop server merge

Current state:

- `chan-server` now exposes a multi-drive host API for in-process desktop
  use.
- `chan-desktop` starts normal local drives through the embedded host.
- Local child-process serving has been removed from desktop. There is no
  sidecar fallback mode.
- The embedded desktop tunnel server remains inbound attach plumbing.
- Desktop can persist and open explicit outbound URL attachments for
  already-running `chan serve` instances.
- On a fresh desktop launch with empty chan metadata, desktop creates
  `Documents/Chan`, seeds it from `docs/manual/`, registers it, and
  opens it through the embedded local server.
- Existing desktop users with registered drives but no default drive
  get a non-destructive prompt to choose an existing drive or create
  `Documents/Chan`.
- If the registered default `Documents/Chan` drive is missing,
  desktop requires a factory-reset confirmation before clearing chan
  metadata and recreating the seeded default drive.

Target state:

- `chan-server` exposes a multi-drive host API that can open, close,
  route, and isolate multiple drives in one process.
- `chan-desktop` links that host API and starts local drives in-process by
  default. This is the only local desktop serve mode.
- `chan serve <path>` remains as a compatibility wrapper around the same
  runtime.
- External `chan serve` processes remain valid as standalone servers and
  explicit attach endpoints.

Implementation notes:

- Keep edit, terminal, search, watch, session, and WebSocket state scoped by
  drive.
- Avoid direct filesystem access outside `chan-drive`.
- Keep request routing explicit enough that accidental cross-drive state
  bleed is testable.
- Audit async server paths while changing the host shape. No synchronous
  filesystem, graph, report, search, or inspector work should block the
  tokio runtime.

## 2. Desktop-native carryovers

Linux desktop launch blocker:

- Current Linux desktop app is unusable for at least one manual tester.
- Symptoms:
  - App opens only a white window with no rendered content.
  - Menus are `Edit` and `Window`; no `File` menu is present.
  - `Window -> Drives` opens another empty white window with the same menu.
- Expected:
  - Linux desktop launches the drive/onboarding surface.
  - Menus expose the expected desktop actions for the current platform.
  - Opening the drives window shows the drive list or first-run flow, not a
    blank duplicate window.
- Treat this as a launch-blocking desktop smoke failure before broader
  desktop merge work is called done.

Bidirectional discovery:

- Add same-user desktop discovery so a bare local `chan serve <path>` can
  hand off to an already-running desktop instance.
- Use Unix-domain socket discovery on Unix. Use the equivalent same-user IPC
  on Windows when Windows desktop support returns.
- If desktop accepts the handoff, open the drive in the existing desktop
  window and make the CLI exit cleanly.
- If desktop is missing, incompatible, or declines, fall back to standalone
  `chan serve` behavior.
- Add a no-handoff escape hatch. Explicit host, port, tunnel, or other
  standalone flags should preserve standalone behavior.

Attach modes:

- Keep local embedded drives as the default desktop mode.
- Preserve attached inbound mode through the existing tunnel listener path.
- Attached outbound mode lets desktop open an already-running `chan serve`
  URL as a non-owned remote drive.
- Document the three modes: local embedded, attached inbound, attached
  outbound.
- Outbound attach should accept token-bearing URLs as pasted. The desktop
  owns only the window, not the remote process, token lifecycle, or server
  shutdown.
- Inbound attach should continue to support local testing with a command such
  as `chan serve /tmp/foo --tunnel-url=http://127.0.0.1:9999` against a
  desktop listener on `127.0.0.1:9999`.

Default `Chan` drive:

- On first desktop launch with fresh metadata, create a default drive named
  `Chan`. Done.
- Store the drive under the platform Documents location:
  - macOS: `~/Documents/Chan`
  - Linux: XDG Documents directory, falling back to `~/Documents/Chan`
  - Windows: `Documents\Chan`
- Seed the drive with the full `docs/manual/` tree embedded at build time.
  Done for fresh metadata.
- For existing users with metadata but no default `Chan` drive, prompt to
  designate an existing drive or create a new one. Do not wipe existing
  metadata during migration. Done.
- If the registered default `Chan` drive is missing on launch, enter a
  factory-reset confirmation flow before wiping chan metadata. Done.

Desktop File Browser drag-out/download:

- Track C owns browser File Browser drop-to-upload and the web status-bar
  progress/cancel flow.
- Track A owns native desktop integration when dragging files or directories
  from File Browser out to the OS desktop or file manager.
- File drag-out and the right-click Download action preserve the original
  basename and bytes through a token-bearing `/api/files/<path>?download=1`
  URL. Done for browser-level drag-out and context-menu download in
  `d11eef5`.
- Directory drag-out should preserve the directory tree. If the platform or
  webview cannot expose a live directory drag payload, stage an archive export
  with clear naming. Browser-level directory drag-out and right-click Download
  use the same `download=1` URL and return a `.tar` archive. Done in
  `d11eef5`.
- Use an OS-native drag payload or a temporary desktop export provider from
  the Tauri layer.
- Do not add direct desktop filesystem reads of drive content. Route export
  data through the embedded server or another chan-drive-backed boundary.
- Clean temporary exports after the drag lifecycle finishes, or through a
  bounded cleanup pass if the platform does not provide a reliable completion
  callback.
- Cancelled drags should not leave user-visible temporary files behind.
- Verify browser-only drag-out first. Keep this task scoped to gaps that need
  native desktop help.

## 3. Manual, docs, and site

- Create `docs/manual/` as the canonical user manual source.
- Make the desktop first-launch seed consume the same manual source. Done.
- Publish the manual through the static site. Done: `web-marketing` renders
  `docs/manual/` under `/manual/`.
- Add CI for manual and site builds. Done: `pages.yml` runs the marketing
  site check when `docs/manual/**` or `web-marketing/**` changes.
- Manual/site local gate verified on 2026-05-25 with
  `cd web-marketing && npm run check`.
- Update stale desktop docs to describe the Tauri webview, embedded server,
  and attach modes. Done for `desktop/README.md`, `desktop/CLAUDE.md`,
  `desktop/Makefile`, and `desktop/src-tauri/src/serve.rs`.
- Keep design docs factual. Remove old claims that desktop is only a thin
  shell around an external browser and per-drive `chan serve` child process.

## 4. Phase 9 hardening carryovers

Rich Prompt:

- Validate non-empty CodeMirror prompt submit in a browser environment that
  can type into CodeMirror.
- Verify archive contents, clear-on-submit behavior, and the edited-during-
  submit race.
- Validate clipboard-dependent Spawn agents preflight.

Server and editor consistency:

- Run low-file-descriptor stress under `ulimit -n 256` with many terminals
  and active indexing. Done on 2026-05-25 with an isolated `/tmp`
  config/home: `chan serve` stayed healthy at a 256 fd soft limit,
  created 32 live `sleep 45` terminals, rejected 3 extras at the
  configured session cap, rebuilt a 600-note drive, and settled back to
  idle with 600 indexed docs.
- Reproduce rapid-edit stale editor/index races.
- Decide whether background search and indexing need further throttling
  beyond current file-descriptor budgets and terminal admission. Decision
  on 2026-05-25: no new fd throttle for this wave. Existing index worker
  budgets plus terminal fd admission held under the 256 fd smoke above.
  Revisit only if the rapid-edit stale-index repro shows queue churn rather
  than descriptor pressure.
- Audit remaining direct sync calls from async server paths. Done on
  2026-05-25: drive info/warnings, config PATCH saves, reports
  state/update, screensaver state/update/verify, and team watcher attach
  now route lazy drive/config filesystem work through `spawn_blocking`.
  Search, graph, files, reports, metadata, inspector, drafts, rich prompt,
  contacts, storage reset, and semantic endpoints were already wrapped.
  Startup-only config loads remain outside request hot paths.

Product contracts:

- Codify `[[` as a file/path link picker, not global content search.
  Done: UI uses `/api/link-targets` and server tests pin that body text
  does not match link-target results.
- Add endpoint and UI tests for that contract. Done.
- Fix or close the File Browser duplicate-key smoke failure. Closed:
  FileTree already dedupes repeated paths and explicit directory entries
  that duplicate placeholder parents; `fileTreeDuplicatePaths.test.ts`
  pins both guards.

Config cleanup:

- Remove or deprecate stale `ServerConfig.reports.enabled`. Done:
  removed from server config and SPA now uses per-drive reports endpoints.
- Keep drive-scoped report config as the source of truth.
- Update config reference docs after the code path is simplified. Done.

## 5. Release and CI hygiene

Standalone `chan` binary:

- Keep `.github/workflows/release.yml` as the standalone binary release
  workflow.
- Keep it separate from `.github/workflows/release-desktop.yml`.
- Accepted release matrix:
  - Linux x86_64
  - Linux aarch64
  - macOS aarch64
- Do not add macOS x86_64 support.
- Verify that tag pushes still attach standalone binary artifacts plus
  `VERSION` and `SHA256SUMS` to the GitHub release.

Desktop release:

- Keep desktop app packaging, signing, notarization, and installer artifacts
  in `release-desktop.yml`.
- Do not make Track A's binary release work depend on desktop packaging.
- Release workflow comment audit done on 2026-05-25: standalone
  `release.yml` and desktop `release-desktop.yml` both trigger on
  `chan-v*`, stay separate, and desktop docs call the bundled `chan`
  binary a helper, not a local serving fallback.

Operational release checks:

- Verify current desktop artifact launch.
- Verify current standalone `chan` artifact launch.
- Verify docs, changelog, release notes, and issue tracker links before the
  phase 10 release cut.

## Interfaces

- `chan-server` adds a multi-drive host API.
- Existing single-drive `serve()` behavior remains available for the CLI.
- `chan-desktop` state stores embedded local-drive handles plus explicit
  external attachments.
- CLI handoff sends drive path, requested action, client version, and
  capability set.
- Desktop replies with accepted/opened or declined/standalone reason.
- Desktop outbound attach accepts a full server URL with bearer token.
  Token persistence is out of scope unless separately approved.

## Test plan

- Rust gates:
  - `cargo test`
  - `cargo test -p chan-server`
  - `cargo test -p chan`
  - `cargo test -p chan-drive`
  - `cargo clippy --all-targets -- -D warnings`
- Desktop gates:
  - Build the Tauri app.
  - Open two embedded local drives.
  - Edit both drives.
  - Run terminals in both drives.
  - Verify no cross-drive state bleed.
  - Drag a File Browser file to the desktop and verify name and bytes.
  - Drag a File Browser directory to the desktop and verify tree or archive
    contents.
  - Cancel a File Browser drag-out and verify temporary export cleanup.
  - Repeat drag-out smoke on macOS and Linux where desktop builds are
    available.
- CLI compatibility:
- Standalone `chan serve <path>` with no desktop running.
- Handoff with desktop running.
- No-handoff standalone path.
- Explicit host, port, and tunnel behavior.
- Attach matrix:
  - Local embedded drive.
  - Attached inbound tunnel drive.
  - Attached outbound URL drive.
  - Remote disconnect and reconnect.
  - Invalid token.
  - Version mismatch.
- First-launch matrix:
  - Fresh metadata.
  - Existing metadata without `Chan`.
  - Missing registered default drive.
  - Manual seed present.
  - Reset confirmation path.
- Web and product regressions:
  - Rich Prompt submit, archive, clear, clipboard, and Spawn agents preflight.
  - File Browser duplicate-key case.
  - `[[` picker contract.
  - Rapid-edit stale index repro.
- Release gates:
  - Manual and site build.
  - Desktop artifact launch.
  - Standalone `chan` artifact launch.
  - `VERSION` and `SHA256SUMS` release artifacts.
  - Docs/config consistency.

## Assumptions and non-goals

- Track A is a full carryover sweep, with desktop merge first.
- Embedded `chan-server` is the target architecture.
- Desktop local serving is embedded-only. Child-process local serving is not
  a desktop fallback mode.
- `~/.chan` remains the shared chan metadata root.
- The default `Chan` drive is user content and lives under Documents.
- The full manual is embedded and seeded.
- `[[` remains a link/file picker. Global Search remains the content and
  graph search surface.
- No macOS x86_64 release target is required for phase 10.

## Immediate sequence

1. Commit the embedded-local desktop merge and documentation cleanup. Done:
   `04e9a83`.
2. Implement outbound URL attach in chan-desktop. Done.
3. Implement fresh first-launch default `Chan` drive plus manual seed. Done.
4. Implement existing-user default-drive prompt. Done.
5. Remove stale server-wide reports config. Done.
6. Implement missing-default factory-reset confirmation. Done.
7. Browser-level File Browser drag-out and right-click Download. Done:
   `d11eef5`.
8. Refresh stale desktop embedded-server docs. Done.
9. Verify manual/site local gate. Done.
10. Refresh release workflow comments for embedded desktop mode. Done.
11. Audit async server sync-I/O boundaries. Done.
12. Low-file-descriptor stress with terminals and active indexing. Done.
13. Decide background indexing throttle scope. Done: no new fd throttle
    after the low-FD smoke.
14. Current next remains open for selection. CLI handoff is deferred until
   its design checkpoint.
