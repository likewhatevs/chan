# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- **Devserver service status reports the managed command.** `chan devserver --service --status` now shows the command behind the managed service, and `--restart` preserves the bound address and port across the service handoff.
- **Marketing footer and install layout refreshed.** The download/install footer actions are split more clearly, swap order where needed, and fit the mobile layout without crowding.

### Changed

- **Gateway gate/admin/public-host env vars renamed `WORKSPACE_*` -> `DEVSERVER_*`.** The devserver-proxy contract shared by identity, profile, and devserver-proxy is now `DEVSERVER_GATE_SECRET`, `DEVSERVER_ADMIN_TOKEN`, `DEVSERVER_ADMIN_URL`, `DEVSERVER_PUBLIC_SCHEME`, and `DEVSERVER_PUBLIC_PORT` (formerly `WORKSPACE_*`), matching the `devserver.<domain>` hostnames the services already derive. Self-hosters must rename these in their `/etc/chan-gateway/*.env` files (and any orchestration/secrets) before deploying -- the services require the new names. The `configure.sh` generator and the bundled `.env` templates emit the new names; the admin CLI's `CHAN_ADMIN_WORKSPACE_URL` is unchanged.

### Fixed

- **Mermaid zoom stays readable in dark mode.** The diagram zoom view re-renders on a light panel so dark-theme diagrams no longer disappear into a black overlay.
- **List-line selection no longer bleeds into the gutter.** Selecting list items at nested depths keeps the highlight aligned with the text instead of extending past the marker.
- **Cmd+E preserves the editor caret.** Toggling between rendered Markdown and source mode maps the current caret into the target mode instead of jumping away.
- **Rich-prompt image paste sends a bare absolute drafts path.** Pasted images are written to drafts and inserted as the same bare absolute path shown in the prompt and delivered to the terminal, without Markdown image syntax or width hints.
- **Windows serving lookups normalize verbatim paths.** `chan ps`, `chan close`, and related workspace lookup paths handle `\\?\`-prefixed Windows paths consistently.
- **`cs open` focuses a newly created empty file.** Opening a new path from a terminal moves focus into the editor instead of leaving it in the terminal.
- **Graph from here always opens a fresh graph tab.** Repeated file-scoped graph opens no longer reuse or overwrite an existing graph tab unexpectedly.
- **Devserver disconnect and Abandon lifecycle tightened.** A disconnected devserver clears its workspace windows, Retry/Abandon can reach the desktop Abandon path, and the launcher leaves the control terminal for re-run instead of reaping it.

## [v0.55.0] - 2026-06-28

An editor-polish and devserver-hardening round: mermaid diagrams zoom, dev servers show their OS, local workspaces take a display name, wide tables stay readable, pasted image paths resolve from the terminal, plus a batch of editor and Windows fixes.

### Added

- **Mermaid diagrams zoom.** Clicking a rendered mermaid diagram opens a pan-and-zoom view with keyboard control (`+`/`-`/`0`, arrow keys and WASD to pan, wheel to zoom, Escape to close), on both the web app and chan-desktop.
- **Dev servers show their operating system.** A dev server self-reports its OS (and Linux distribution where available); the launcher shows an OS icon on the local machine card and on each remote dev server.
- **Name a local workspace.** Adding a local workspace in the launcher accepts an optional display name, shown in place of the folder name.

### Changed

- **Wide tables stay readable.** A table wider than the editor now scrolls horizontally instead of wrapping every cell character-by-character, in both the editor and the rendered/printed output.
- **Pasted image paths resolve from the terminal's directory.** An image pasted into the rich prompt is delivered as a path relative to the terminal's working directory (an absolute on-disk path when that directory is unknown or outside the workspace), so the receiving agent resolves it; the composer preview still shows the image.

### Fixed

- **Ordered lists renumber on a mid-list insert.** Inserting an item in the middle of a numbered list -- including a loose, blank-line-separated list -- renumbers the rest instead of leaving a duplicate number.
- **List-line selection no longer bleeds into the left margin.** Selecting a list line highlights just the line instead of overflowing past the marker into the margin.
- **The model download reports a clear error behind a broken proxy.** When a proxy environment variable is set but unusable, the dev server's model download fails with an actionable error instead of silently. Standard `HTTP(S)_PROXY` / `ALL_PROXY` / SOCKS proxies already worked; `NO_PROXY` and https-scheme proxies are documented as unsupported for the model download.
- **Windows `chan open` and `chan ps`.** `chan open` on Windows no longer prints the stale-port error toast -- the dev server persists its bound port and the local on-toggle is best-effort -- and `chan ps` resolves a server's PID and kind under the `\\?\` verbatim path prefix.

### Notes

- Self-hosting docs and the Kubernetes manifests now point at the container images published to Docker Hub in v0.54.0; the project's internal dev-log was reorganized into a repo-root `team/` release-history layout.
- Validation: a non-publishing cross-OS dry-run build plus on-device smoke testing of the editor, the launcher OS icon, the model download, and Windows.

## [v0.54.0] - 2026-06-27

A feature round: the chan-desktop launcher reorganized machine-first, container images published from the release, in-place editing of inline-code file links, the ambient status notification moved clear of the terminal prompt, and `chan open` taught to serve where its shell actually runs.

### Added

- **Releases publish container images to Docker Hub.** Alongside the CLI and desktop artifacts, the release now builds and pushes multi-arch (amd64 + arm64) images for `chan` and the three gateway services -- `chan-gateway-identity`, `chan-gateway-profile`, and `chan-gateway-devserver-proxy` -- under the `fiorix` namespace, all public. Each release gets an immutable `X.Y.Z` tag; `latest` tracks the newest GA release only, and prerelease `-rc` tags push immutable images without moving `latest`. The path is exercised on a non-publishing dry-run build that builds every image without a registry.
- **Re-point an inline-code file link in place.** Typing inside an inline `` `path` `` link that resolves to a real workspace file opens a file picker to change its target without leaving the line, re-rendering as a link on commit. (The detect-and-open half shipped in v0.53.0.)

### Changed

- **The chan-desktop launcher is organized machine-first.** The local machine and each dev server are equal top-level blocks. Each block opens its own terminals and lists windows control-terminal-first, then standalone terminals, then per-workspace windows nested inside their workspace; the old flat window feed is gone. Adding a workspace and adding a dev server are now separate actions, the bulk-selection checkboxes reveal on a Select toggle (Gmail-style) with a docked bulk bar, workspace cards lift on hover, and a dev server whose control process disconnects shows an inline "reconnecting" flash instead of a modal.
- **The ambient status notification sits in the top-right.** It moved from the bottom-left, where it overlapped the terminal prompt, to the top-right with its collapse control on the right; transfer notifications now stack downward beneath it. The session-handover and survey overlays are unchanged.
- **`chan open` routes by where its shell is running.** `chan open <path>` now detects whether its shell belongs to chan-desktop or a dev server and serves there by default -- standalone when it can detect neither -- instead of always trying the desktop handoff first. The existing `--standalone` plus the new `--desktop` / `--devserver` force a target; `--devserver` from inside a dev server is refused (no nested dev servers). When a workspace is already held (for example by a local dev server), the standalone path now points you at `--devserver`. This fixes a dev-server shell whose `chan open` opened on chan-desktop instead of the dev server it runs on.

### Notes

- Prerelease `-rc` tags now publish as GitHub prereleases (previously a `-rc` tag published as a full release); the moving `latest` image tag and the GitHub "latest release" stay GA-only.
- Validation: a non-publishing cross-OS dry-run build (which also builds the container images) plus on-device smoke testing of the launcher and the editor.

## [v0.53.1] - 2026-06-27

A patch release: the Windows `chan ps` server-kind column, terminal clipboard copy over OSC 52 in chan-desktop, and a markdown editor link whose label contains brackets.

### Fixed

- **`chan ps` shows the serving process kind on Windows.** The BY column resolved a holder's control socket only as a Unix temp-dir `.sock` file, so on Windows -- where the control socket is a `\\.\pipe\` named pipe -- the probe missed and the column printed the literal word `served`. It now enumerates the named-pipe namespace by pid and shows the real kind, falling back to `-` (never the bare word `served`) when the kind cannot be probed. The same probe restores `chan close` / `chan workspace rm` teardown over the wire on Windows.
- **The terminal honors OSC 52 clipboard copies.** Text an agent copies via the OSC 52 escape (for example Claude Code's copy) now lands in the system clipboard -- through the native clipboard in chan-desktop and `navigator.clipboard` in the browser -- instead of being silently dropped. The query form is a no-op, so clipboard contents are never echoed back to the terminal.
- **A markdown link whose label contains balanced brackets renders as a link.** `[[foo] bar](path)` (and the image form `![[foo] bar](img)`) now render as a clickable link instead of plain text, resolving the v0.53.0 known limitation; an upstream `@lezer/markdown` shortcut-reference rule had been swallowing the outer link, and the inner-bracket escape workaround is no longer needed.

### Notes

- Validated on a non-publishing cross-OS dry-run build plus on-device smoke testing (Windows `chan ps`, desktop OSC 52 copy on Windows and macOS, and the editor link in the browser).

## [v0.53.0] - 2026-06-26

The first feature round since the unification: multi-client session presence, a self-managed cross-platform devserver daemon, terminal scrollback resume on reload, editor cursor persistence and inline-file links, and a regrouped chan-desktop launcher -- plus six rolled-forward v0.52.0-rc2 fixes and a `chan serve` terminology rename.

### Added

- **Session presence: leader and followers.** Multiple browser / chan-desktop / API clients in one workspace now collaborate. The first client to connect is the session leader; `cs session list` shows the participants, the leader, and each one's live / disconnecting / disconnected / gone status. `cs session self --name=` renames you, `cs session handover` requests leadership from the live leader (who gets an accept/reject prompt), and `cs session takeover --force` seizes it; when a leader goes away the longest-connected live participant is promoted automatically.
- **`chan devserver --service` is a self-managed cross-platform daemon.** `--service` takes a backend (`none` picks the best for the OS, or `chan` / `systemd` / `launchd`). The `chan` backend runs a single-instance foreground daemon on Linux, macOS, and Windows -- a pidfile + flock with stale-process takeover, `--status` / `--stop` / `--restart` / `--force`, and a `-v` listing of every related file. Reattaching to an already-running server is a health-check watchdog (it no longer follows journald / launchd logs), and a relocated binary still relaunches.
- **The terminal resumes scrollback on reload.** Instead of replaying the whole server-side ring on every reattach, the client caches a screen snapshot plus a byte cursor in localStorage and asks the server only for the delta since it last saw, guarded by a per-session generation so a restart refreshes cleanly.
- **The editor remembers your cursor per file.** Reopening a file restores the caret and scroll position; a large file streaming in parks the caret at the top until it finishes; the saved position is dropped when the file disappears. An explicit open still lands at the top.
- **Inline code that names a local file becomes a link.** When an inline `` `code` `` span resolves to a real workspace file, it renders as a clickable link you open with Cmd/Ctrl-click.
- **`cs terminal list` traces window -> pane -> tab.** Each terminal shows its owning window, pane, and tab (blank when unknown).

### Changed

- **The chan-desktop launcher is a "Library" tree.** Workspaces and devservers regroup under one tree with per-row controls and a host label you click to copy. On/off spinners settle correctly and resync against the server on a dropped feed or on re-show, with no dangling or out-of-state rows; a devserver's control terminal flashes its EYE button when its process exits.
- **Empty editable files are discarded on close.** Opening a file, clearing it, and closing the tab deletes the empty file instead of saving it.
- **`chan serve` is now `chan open` (a local workspace) and `chan devserver` (the tunnel).** The command was renamed; documentation and messages follow.

### Fixed

- **`chan close` / `chan workspace rm` hands off to a running chan-desktop** so the desktop's view stays in sync.
- **The disconnect/retry overlay** no longer swallows cmd+backtick window cycling, and Abandon disconnects the devserver cleanly.
- **An explicit open lands the cursor at the top** instead of a stale position.
- **The link autocomplete inside `[](url)` offers the link itself first.**
- **The black bar at the bottom of the terminal is gone.**
- **chan-desktop startup restores only the workspaces that are actually mounted** (one closed out-of-band is not resurrected).

### Notes

- Validated by a non-publishing cross-OS dry-run build (Linux / macOS / Windows CLI and desktop, including the macOS sign/notarize path) plus on-device smoke testing.
- Known limitation: a markdown link whose label contains balanced brackets (`[[foo] bar](path)`) renders as plain text (an upstream `@lezer/markdown` limitation); escape the inner brackets as a workaround.

## [v0.52.0] - 2026-06-26

A repository-structure unification — the frontend consolidates into a single `./web` npm workspace, build and deploy tooling moves under `./packaging`, and the crate layer gets a naming, docs, and dependency-hygiene pass — plus a round of window and terminal lifecycle fixes.

### Changed

- **One `./web` npm workspace.** The workspace app, launcher, gateway identity SPA, shared chrome, and marketing site are now members of a single `./web` monorepo (`@chan/{workspace-app,launcher,profile,web-shared,marketing}`) with one lockfile and a shared design system. The embedded bundles and the `/dl` release-download contract are byte-stable.
- **One `./packaging` tree.** Docker, Kubernetes, Linux packaging, desktop packaging, sdme, and gateway packaging consolidate under `./packaging`. Every Makefile target and CI job name is unchanged.
- **Crate hygiene.** Shared dependencies centralize in `[workspace.dependencies]`, app-internal crates are marked `publish = false`, three crates gain a `design.md`, and the product is described consistently as an AI-native IDE.

### Fixed

- **Dead and offline windows are removable again.** `cs window rm` (and clearing an offline devserver row) now routes through the library's authoritative window discard — it drops the persisted registry row, ends the window's terminal sessions, and deletes their saved layout — so a dead window no longer reappears on the next `cs window list` or after a restart. Removing a window that still has live terminal shells is refused unless `--force` is passed, and `cs window rm` no longer blocks on a desktop confirm dialog.
- **`cs window rm` can remove a connected devserver's window from a local terminal**, not only from one of that devserver's own terminals.
- **`cs terminal list` shows each terminal's owning window**, its kind (standalone-terminal / workspace / control / orphaned), and whether that window is alive or offline.
- **A window's titlebar number matches `cs window list`.** Watcher-opened windows now title themselves from the library's persisted ordinal (the `#` column) instead of a desktop-local counter, so the titlebar `Window N` and the registry no longer drift.

### Notes

- The unification restructures sources only — the rust-embed bundle paths and the `/dl` release-download contract are byte-stable. The window and terminal lifecycle items under **Fixed** are the release's only behavior changes.

## [v0.51.0] - 2026-06-25

Windows desktop support graduates from a CI-only artifact to a published download:
the release now ships an (unsigned) Windows desktop installer and a standalone
Windows CLI, the terminal defaults to the user's own shell instead of requiring
Git BASH, and `chan open` integrates with a running devserver over a named pipe.

### Added

- **The Windows desktop installer and CLI are published downloads.** The release
  builds and uploads `Chan_<version>_x64-setup.exe` (NSIS desktop installer) and
  `chan-x86_64-pc-windows-msvc.zip` (standalone CLI), and the install page lists
  both. The installer is **unsigned** for now, so Windows SmartScreen may warn on
  first run; Authenticode signing is tracked for a later release. The Windows build
  is best-effort: a failure does not block the Linux and macOS release.
- **`chan devserver --service`** unifies the previous `--systemd` / `--launchd`
  flags into one cross-platform flag, with a Windows service backend.
- **Windows named-pipe devserver discovery.** `chan open` finds and registers into
  an already-running devserver over a named pipe, matching the unix-socket behavior.
- **The chan-desktop launcher window remembers its size and position** across
  restarts (per monitor, like the editor window) and opens at a more compact default.

### Changed

- **The Windows terminal defaults to the user's shell** (PowerShell / cmd, with a
  `CHAN_SHELL` override) instead of requiring Git BASH; the in-app "install Git for
  Windows" gate is removed.

### Fixed

- **`chan open` hands off to a running chan-desktop** from the bundled console
  `chan.exe`, so opening a path from the CLI focuses the existing window instead of
  starting a second server.
- **chan-server forces process exit on Windows** when the graceful-shutdown deadline
  lapses, so a lingering task can no longer keep the process alive.
- **`cs open <path>` moves the cursor to the opened editor** instead of leaving it in
  the terminal that ran the command.
- **The desktop "Window Hidden" notice mark follows the theme** — a fixed dark logo
  that had become invisible on the dark dialog.

## [v0.50.0] - 2026-06-25

A terminal-interaction, reload-state, and CLI-ergonomics bug-sweep with desktop
window-geometry restore: copy works in full-screen TUIs, htop survives a reload,
files open with a usable caret, pane sizes and inspector widths persist across
reload, per-Hybrid terminal themes stop resetting, `cs terminal survey` gains a
timeout, team setup gains a `--brief`, and chan-desktop restores window size and
position per monitor.

### Added

- **`cs terminal survey --timeout=<secs>`** (default 600). On elapse the survey is
  cancelled and the command exits **124** (GNU `timeout` convention) with an
  elapsed-seconds message on stderr (stdout stays clean for `$(...)` capture) — a
  distinct timed-out outcome, not an inferred dropped connection.
- **`cs terminal team new --brief <file>`** (and a Cmd+P team-dialog field) folds a
  brief verbatim into the generated `bootstrap.md`, so it survives a normal `new`.
- **chan-desktop restores window size and position per monitor.** Each window's
  geometry is captured on hide/close and restored on reopen, keyed by a monitor
  signature with a per-machine LRU; a monitor-layout mismatch restores size only,
  clamped on-screen. Desktop-only; the browser keeps its URL-hash layout restore.
  Known issue: on a secondary/external display, repeated hide/show can
  progressively shrink the window; a fix is tracked for a later release.

### Fixed

- **Copy works in full-screen TUIs.** Holding **Shift** now forces a native terminal
  selection while a TUI holds mouse tracking (e.g. the Claude TUI), so drag-to-select
  and copy work instead of the drag being forwarded to the program.
- **htop arrows and the mouse wheel survive a reload.** After a full SPA reload that
  reattaches to a live PTY, the terminal re-asserts the full private-mode set (DECCKM
  cursor-keys + mouse), not just the alt-screen, so cursor keys and the wheel work
  again.
- **The control-terminal banner** prints the bare command instead of a `running: `
  prefix, so the command's own output begins on the next line.
- **Files open with a usable caret.** A file opened via `cs open` or the File Browser
  (no initial selection) now places the caret at the document start and focuses the
  editor, matching the Draft path.
- **The `system` theme resolves to dark when the OS appearance is undeterminable**
  (e.g. headless linux, where neither prefers-color-scheme query matches), on both
  the app and the launcher.
- **Pane sizes persist across reload, including empty panes** — a divider drag now
  schedules a layout save.
- **File-Browser inspector width persists across reload**, routed through the same
  per-tab state the editor inspector already uses.
- **A per-Hybrid terminal light/dark override no longer resets on reload.** Global
  config writes are now serialized, so a concurrent autosave can no longer clobber a
  just-saved theme override.

## [v0.49.0] - 2026-06-24

A UI-responsiveness, desktop-presentation, and packaging release: the chan-launcher
now drives its on/off and connect spinners from real backend lifecycle state instead
of a fixed optimistic timer, turning a workspace on during boot no longer false-errors,
the desktop "Window Hidden" notice is centered, every local window title shows the home
glyph, `cs upload` works from a tunnel window, and chan plus the gateway services now
ship as container images with Kubernetes manifests.

### Added

- **Container images and Kubernetes manifests.** Multi-stage Dockerfiles for the `chan`
  binary and the gateway services (identity, profile, devserver-proxy) under `docker/`,
  plus `kube/` manifests for the gateway stack (Deployments, Services, ConfigMap, Secret,
  Postgres, and an sdme single-pod variant). Validated under sdme: images build, the
  gateway services answer `/healthz`, and a headless-browser upload lands.
- **`cmd+r` / `ctrl+r` reloads the launcher window** in chan-desktop.

### Changed

- **The launcher drives its spinners from real backend status.** Workspace and devserver
  toggles reflect the backend lifecycle — workspace `stopped | starting | running |
  error` and devserver `disconnected | connecting | connected` — instead of a fixed 45s
  optimistic timer. A toggle spins while its workspace is starting and is disabled
  mid-transition, an errored mount surfaces its reason on the row, and a devserver
  disconnect clears the connect spinner with no manual reload.
- **The "Window Hidden" notice is centered.** chan-desktop replaces the native
  left-aligned alert with a custom centered notice (icon, title, text, and OK button).
- **Every local window title shows the home glyph (🏠).** The desktop-monitor glyph for
  paths outside `$HOME` is gone; all local windows show 🏠 and remote/devserver windows
  keep the globe (🌐).
- **`cs upload` works from a tunnel window.** chan-desktop grants `pick_upload_files` to
  tunnel (devserver) windows, so uploading a file over an ssh tunnel opens the picker
  instead of failing with an ACL error.

### Fixed

- **Turning a workspace on during boot no longer false-errors.** A turn-on for a
  workspace that this chan process is already mounting (or has mounted) is idempotent;
  the "another process is locking the workspace" error now fires only for a genuinely
  foreign lock holder, not for chan's own in-flight mount during boot-restore.

## [v0.48.0] - 2026-06-24

A devserver / launcher window-lifecycle, identity, and presentation release: the
per-library pane focus-border colour now actually persists and reaches every window
of a chan-library (a root-cause fix), same-basename workspaces coexist, the control
terminal echoes the command it runs, a new `CHAN_HOME` isolates a chan instance, and
a batch of presentation + hygiene fixes — several carried over from v0.47.0.

### Added

- **`CHAN_HOME` environment variable.** Point chan at a different home directory —
  config, workspace registry, devserver tree, window/terminal state — without
  changing `$HOME` (e.g. `CHAN_HOME=/tmp/scratch chan …` for a fully isolated
  instance). When it is set, chan-desktop also installs its `chan`/`cs` shims under
  `CHAN_HOME/.local/bin`.
- **The control terminal echoes its command.** A script-based devserver's control
  terminal prints `running: <command>` before it runs, so the connect command is
  visible.

### Changed

- **Devserver windows use a 🌐 globe icon** — in window titles and the launcher feed
  — replacing the old outbox-tray / arrow glyph.
- **The shell is never hardcoded.** Terminals and the macOS PATH-harvest resolve the
  user's configured shell uniformly (`$SHELL` → passwd entry → `/bin/sh`); the old
  `/bin/sh` / `/bin/zsh` fallbacks are gone.
- **Two workspaces with the same folder name can be open at once.** A workspace's
  mount prefix is now `/{name}-{hash}` (a short hash of its canonical path), so
  `foo/notes` and `bar/notes` no longer collide.
- The launcher's *Workspaces* and *Devservers* rows align their labels left, matching
  *Open windows*.

### Fixed

- **Per-library pane focus-border colour now persists and propagates.** Setting a
  pane's focus colour persists for the chan-library, and a newly-opened window (local
  or devserver, terminal or workspace) shows it. Previously the change never
  persisted — the request was misrouted under the window's tenant prefix and 404'd —
  so new windows fell back to the default blue.
- **Pasted rich-prompt images resolve for the receiving agent.** An image pasted into
  the rich prompt is delivered as a workspace-rooted path, so the agent finds it at
  its working directory instead of 404ing.
- **Terminals no longer blank under a full-screen TUI** (e.g. claude code). The
  reattach reply-gating that could stall and drop live cursor/device-status replies
  was removed (at the cost of an occasional historical reply echoing at the prompt).
- A **script-based devserver disconnects immediately** when its control script exits:
  no lingering "connected", the control row leaves the feed, and the re-run / abandon
  prompt appears.
- The launcher's **control-closed survey fires again** — the remote-served launcher
  was missing the `core:event` listen permission.
- Same-name workspaces no longer **crash the launcher** with a duplicate-key error.
- `chan open` on a port a devserver already holds (`:8787`) prints an **actionable
  message** instead of a raw `EADDRINUSE`.
- A **standalone terminal window leaves the feed** when its shell exits while
  detached, instead of lingering as a ghost.
- A devserver's **Control terminal groups under its devserver** in *Open windows*,
  not under a blank header.
- Clicking the **eye on a just-closed window** is a clean no-op — no console errors.

## [v0.47.0] - 2026-06-23

A devserver / launcher lifecycle release: `chan devserver` gains tunnel-only and
supervised-service controls, the devserver control terminal is unified onto
chan-library's window model (fixing several connect/feed bugs at the root),
per-window visibility now persists and is mirrored on connect, and the per-library
focus-border colour propagates live across all windows of a library.

### Added

- **`chan devserver` tunnel-only mode.** When a tunnel token is present, the
  devserver no longer binds a local TCP listener by default (the gateway is the
  surface). `CHAN_DEVSERVER_LISTEN=0/1` overrides; tunnel-off + `LISTEN=0` is a clear
  error. Added `--stop` / `--restart` for supervised (`--launchd` / `--systemd`)
  devservers (`--restart` starts a stopped service).
- **Per-window visibility persists.** A window hidden in one session stays hidden on
  reconnect and across a chan-desktop restart; the launcher mirrors the persisted
  layout instead of re-opening every window.
- **Live per-library focus-border colour.** Setting the focus colour on any pane now
  updates every open window of that chan-library live, and new windows inherit it.

### Changed

- **The devserver control terminal is now a first-class chan-library window** (unified
  onto the window registry instead of a desktop-synthesized record): it appears in the
  launcher's "Open windows" on connect and is reaped when its process exits.
- The "Open windows" panel shows hidden windows inline with an eye toggle (no separate
  section).
- Removed the dead Tauri devserver CRUD commands; the launcher manages devservers over
  HTTP.

### Fixed

- The devserver group / Control terminal now appears on a fresh (zero-window) connect
  and survives a reload — previously missing until a second window was minted.
- Control-terminal process exit surfaces the **re-run / edit / abandon** prompt again,
  flips the devserver to disconnected when it is actually unreachable, and removes the
  closed terminal from the feed.
- A devserver stays connected when its setup-style connect script exits cleanly (a
  benign exit no longer flips it to disconnected).
- New windows no longer come up with the default focus-border colour when a per-library
  colour is set.
- Closed workspace windows no longer re-open on chan-desktop restart.

## [v0.46.0] - 2026-06-23

A launcher-polish and fix release on top of the v0.45.0 desktop release: the
workspace launcher gains unified bulk management for served workspaces, per-window
focus / show-hide controls, in-flight spinners, and a dismissable error banner;
editor and graph navigation are fixed; and desktop upload, native dialogs, the
devserver connection, and the app icon are hardened.

### Added

- **Launcher — per-window Focus and Show/Hide controls.** Each "Open windows" row
  now has a **Focus** button (raise + focus the window, un-hiding it if buried) and
  an **Eye / Eye-off** show-hide toggle, replacing the single click-to-toggle dot.
- **Launcher — in-flight spinners.** Turning a workspace on/off and connecting or
  disconnecting a devserver now show a spinner while the action runs; the spinner
  **survives a launcher reload** and reconciles to the latest state.
- **Launcher — served workspaces are managed like local ones.** A served
  (devserver-mounted) workspace row gets a select checkbox and feeds **one** global
  bulk bar spanning local + served + devserver selections, with an ordered
  cross-kind Remove (forget served → remove devservers → remove local).

### Changed

- **Launcher — the top-level open-terminal button uses the SquareTerminal icon.**
- **Graph — "Open" on a file node opens the editor** (matching the File Browser);
  directory nodes still open the File Browser.
- **App icon — the enso is no longer over-zoomed**, re-rendered with its original
  cream-paper margin (colours unchanged).

### Fixed

- **Editor — a `[[wiki-link]]` to a resolvable note no longer shows a false
  "document not found."** The link target is resolved to its real file before
  opening; genuinely broken links still surface the banner.
- **Editor — reopening a closed File Browser tab (Cmd+Shift+T) restores its
  expanded directories** (and selection, scroll, and workspace toggle).
- **Launcher — the error/warning banner can be dismissed** (an [X] button) without
  reloading.
- **Launcher — `chan open <url>` shows the new devserver immediately**, with no
  manual reload.
- **Desktop — `cs upload` opens a native file picker** on macOS, so uploads work
  from a desktop terminal (the web file input is blocked by WKWebView; download was
  unaffected).
- **Desktop (macOS) — native confirm dialogs honor Return-to-default** — "Quit
  Chan?", Remove window, transfer-in-progress, and update-ready all respond to
  Return on the blue default button.
- **Desktop — the devserver connection no longer leaks file descriptors.** The
  desktop built a fresh HTTP client per poll (~22 leaked connections/minute) until
  the devserver hit its 1024-fd cap and died (~40 min); it now reuses one client.
- **Manual — the intro bullet list renders correctly** (a missing blank line had
  folded the bullets into the preceding paragraph).

## [v0.45.0] - 2026-06-23

The desktop release. It finishes the launcher on the **desktop / WKWebView** surface the v0.44.0 headless
gate couldn't reach, then — across follow-on rounds driven directly by desktop hand-smoke — builds out the
full **devserver-in-the-launcher** experience and hardens the window lifecycle. A connected devserver's
windows, served workspaces, and control terminal now appear in the launcher; the focus-border colour
persists per chan-library (one for the local library, one per devserver); the launcher rows are redesigned
with icon buttons + bulk actions; turning a workspace off preserves its window layout for restore on
turn-on (only Forget purges); and the desktop show/hide, reconnect, and live-terminal-off paths are fixed.
Alongside: desktop auto-update on launch, standalone-terminal `cs upload`/`cs download`, a new app icon,
graph-navigation refinements, a reworked marketing homepage with the docs consolidated into the manual, and
a devserver-reality docs pass.

### Added

- **Desktop auto-update on launch.** chan-desktop checks for an update in the background at startup and
  prompts to install (honors `CHAN_UPDATE_CHECK=0`) — a directly-booted desktop now self-updates instead
  of only updating via an explicit `chan upgrade`.
- **Devserver Connect from the launcher.** The launcher's Connect button now dials a configured devserver
  (runs its connect command in a control terminal and connects), enabled on the desktop surface and inert
  in a plain browser.
- **New-Workspace folder picker.** A native **Browse…** button opens an OS folder dialog to fill the
  workspace path (the text field stays the fallback, and the only path in a browser).
- **Standalone-terminal `cs upload` / `cs download`.** Library-level transfers from a standalone terminal
  (no workspace): cwd-anchored, shell-uid reach, with read/write pre-flight checks that fail fast and
  leave no partial artifact. `<path>` is required (`.` = current dir); a directory downloads as a
  streamed tarball, and a cancelled download leaves nothing behind. Workspace transfers stay bounded to
  the workspace root.
- **Transfer close-guard for connected-devserver windows.** Closing a connected devserver's window
  mid-transfer prompts Keep open vs Cancel — the in-flight signal now rides the windows feed.
- **New desktop app icon** — a black enso on cream paper.
- **Devserver windows + served workspaces in the launcher.** A connected devserver's standalone terminal,
  control terminal, and workspace windows now appear in the launcher's Open-windows (and the native Window
  menu), and its `chan open` workspaces appear in the launcher list — grouped under the devserver, with
  their on/off/Forget routed to it. Built on a devserver-feed source merged into the window feed +
  per-workspace cache, plus disconnect / New-Terminal / open-workspace bridge ops.
- **Control terminal in the launcher.** A connected devserver's control terminal shows **first** in its
  window group (labelled "Control terminal"), with an optional **"Auto-hide control terminal on success"**
  on the connect form so it tucks away once the connection is up.
- **Per-library focus-border colour.** The pane focus-border colour now persists per chan-library — set it
  once and every standalone terminal and workspace window of that library uses it; the local library and
  each devserver each keep their own (file-backed, surviving reconnect/restart). Set from the pane's
  focus-border menu.
- **Launcher row redesign + bulk actions.** Workspace and devserver rows use icon buttons (New window /
  On-Off; New terminal / Edit / Connect-Disconnect), with multi-select bulk **Turn on / Turn off / Remove**.
  Edit opens read-only while a devserver is connected.
- **Turn-off confirm for live terminals.** Turning off a workspace that still has live terminals now prompts
  with the live-terminal count and offers to force it off — for both devserver and local workspaces.

### Changed

- **Launcher live-refresh.** The desktop launcher's workspace list updates live as you `chan open` a
  workspace or turn one on/off — no manual reload.
- **Open-windows rows are show/hide toggles.** Clicking an Open-windows entry shows or hides its window
  (the whole row, not just the dot).
- **Graph "still indexing" state.** While the workspace index is building, the graph tab shows
  "graph temporarily unavailable while indexing the workspace" instead of "no markdown files in this
  workspace yet", and the graph repopulates automatically once indexing finishes.
- **Uploads use the transfer bubble everywhere.** The replace-file upload now reports through the transfer
  bubble; the upload status-bar text is retired (v0.44.0 retired the download bar only).
- **File-browser inspector Open.** Opens odd-extension plaintext files (matching the tree's content-peek)
  instead of offering Download.
- **Tunnel-publish docs corrected to the `chan devserver` reality** across the README, manual, marketing,
  and the tunnel-crate docs; the "anonymous public tunnel" section is removed (publishing is always
  authenticated).
- **Devserver form is Host + Port.** The add/edit devserver dialog takes a host and a port (the URL is
  formed for you) instead of a single URL field; the optional token and connect command stay.
- **Graph shortcut is `Cmd+Shift+M`** (Linux/Windows `Ctrl+Shift+M`) — restored after a mistaken retirement;
  it opens a Graph tab in the current window and shows on the Graph tile. `Cmd+Shift+G` stays Find-previous;
  the hybrid-nav alias is `Mod+. M`.
- **Graph navigation.** "Graph from here" and the inspector's "Open" each open a **new tab** (no in-place
  re-root), and the graph now renders the filesystem skeleton immediately and layers semantic edges in as
  the index settles (instead of showing "unavailable" until the index is ready).
- **README reduced to a minimal pointer** (download from chan.app or build with the Makefile).
- **Marketing homepage reworked and the docs consolidated into the manual** — a leaner home page, with the
  product documentation living under the manual (refreshed screenshots).

### Fixed

- **Launcher on-state on the desktop.** A desktop-served workspace now correctly shows as on (it showed
  "Turn on" despite being served); the launcher resolves a workspace's on-state and its on/off/remove
  actions by the workspace's canonical root, not the slug prefix the desktop never mounted at.
- **Turned-off workspaces no longer leave stale windows in the launcher** — and turn-on restores them.
  Turning a workspace off removes its windows from the launcher but **preserves their layout** (panes/tabs);
  turning it back on restores the same windows (the terminals restart). Only **Forget** purges the layout.
  Holds for both local and devserver workspaces (a devserver workspace's windows no longer resurrect on
  disconnect→reconnect).
- **Devserver window show/hide from the launcher dot no longer hangs.** Hiding a devserver standalone
  terminal, control terminal, or workspace window via its dot updates the dot correctly, and clicking the
  greyed dot **shows it back** (previously it could be hidden but not reopened except via the Window menu).
  The OS close button updates the dot too.
- **Control terminal appears on devserver reconnect** without needing to open a second terminal.
- **Directory download progress no longer shows `NaN%`** — a streamed directory download (no Content-Length)
  renders an indeterminate progress on the desktop, matching the browser.

## [v0.44.0] - 2026-06-22

A round that makes the launcher a true view of the real library on the desktop, finishes the
`chan serve`/`unserve` → `chan open`/`close` verb migration, and turns `cs upload`/`cs download` into a
visible, cancellable, reload-surviving surface. The launcher's registry CRUD — workspaces **and**
devservers — flipped off the in-memory mock onto the live `/api/library/*` client, so the desktop
launcher lists the user's real `~/.chan` workspaces and configured devservers instead of a hardcoded
fake set.

### Added

- **Launcher reflects reality.** The web-launcher registry CRUD flipped from the in-memory mock to the
  live HTTP client; the desktop loopback lists/mutates the real workspaces + devservers.
- **Live devserver registry.** `GET/POST /api/library/devservers` + `PUT/DELETE /:id`, backed by a
  `DevserverRegistry` bridge over the desktop config (token write-only — `has_token` reported, never
  echoed); empty + 404-mutation on the headless/gateway surface.
- **Per-row Open / Turn on.** A workspace row's pill is now **Open** (mint a new workspace window) when on,
  **Turn on** when off; read-only surfaces keep the static pill.
- **Transfer progress bubble for `cs upload`/`cs download`** — a prominent, cancellable surface (reusing
  the download-progress idiom), survives a window reload (in-flight restores as *interrupted*, never a
  frozen bar; download offers Retry, upload Dismiss), with a terminal-style **window close-guard**
  (closing a window mid-transfer prompts hold / cancel).
- **`cs open` + the file browser open any plaintext file.** `cs open {path}` opens any existing plaintext
  file (content peek, not extension) and creates a nonexistent path as plaintext; the file browser peeks
  content before refusing, matching the same gate.

### Changed

- **`chan serve`/`unserve` → `chan open`/`close`** (verbs + polymorphic target: a path opens/serves a local
  workspace with the existing desktop/devserver handoff; a `scheme://host` URL registers a devserver).
- **Devserver form takes one full URL** (scheme included), not Host + Port — the forward hook for the
  devserver-proxy dial; the desktop defaults the port from the scheme.
- **Window-bury notice simplified** (no em dash).

### Fixed

- **Rich-prompt ArrowUp recall** no longer leaves the composer stuck read-only on a queued message (the
  un-grey is folded into the dispatch + focus deferred, matching the delivered path).
- **`chan close --remove` unregisters from a running devserver** (config + overlay + launcher, durable
  across a `persist_state`); a plain `chan close` now persists the workspace's off-state.

## [v0.43.0] - 2026-06-22

A round centred on **one launcher, three surfaces**: the `web-launcher` SPA is served at `/` by the
`chan-library` `WorkspaceHost` root fallback and reached identically on the desktop loopback, a
`chan devserver`, and the gateway-proxied root through the existing transparent proxy — the native
desktop `main.js` launcher was retired. Alongside it, the v0.42.0-reported "indexing stalls" turned out
to be a slow (not broken, not a regression) single-tail-flush cold embed that *looked* frozen; it now
commits progress incrementally and runs faster on macOS. Plus the editor / team / window-close
carryover and `cs upload`/`cs download`.

### Added

- **Web-launcher unification across all three surfaces.** chan-server embeds `web-launcher/dist`
  (`serve_launcher`) + serves `/api/library/{workspaces,windows}`, installed on the `chan-library`
  `WorkspaceHost` root fallback; the desktop loads the same SPA from its embedded loopback. Per-surface
  auth: full workspace mutation on the loopback, read-only over the gateway/tunnel.
- **Gateway "Open whole devserver."** An owner-only `GET /s/:owner` mints an entry token and forwards the
  browser to the devserver root (launcher) through devserver-proxy; the gateway renders nothing.
- **`cs upload` / `cs download`** raise the Inspector upload/download UI from a workspace terminal.
- **Team-setup dialog survives a window reload** (the in-progress config persists).

### Changed

- **Embeddings cold reindex commits incrementally** — progress advances live and partial results are
  searchable mid-run, instead of one tail flush that looked frozen.
- **Apple Accelerate CPU BLAS** for embeddings on macOS (~1.5–2× faster cold reindex; target-gated, no
  Linux/musl impact).
- **Editor source toggle** gated to renderable files (`.md`/`.json`/`.csv`), Ctrl+E on Linux/Windows;
  `web/EDITOR.md` refreshed to the shipped `@today`/`@date` macros.
- **Window-close notice** simplified; **empty-workspace copy** reframed as a project directory + inline
  Open-terminal.

### Notes

- Windows Authenticode signing remains out (certs pending). The launcher devservers-list bridge, grantee
  mutation over the gateway (a signed proxy role header), and the launcher drag-drop folder-add gesture
  are deferred to a future round.

## [v0.42.0] - 2026-06-22

A round centred on **"opening a chan-library behaves identically whether it is local or remote."**
The library now owns the open rules — first open mints exactly one terminal (and never again),
workspace on/off and terminal-window persistence live in one place — so chan-desktop and a headless
`chan devserver` inherit one definition. Alongside it, the chan.app gateway migrated to a
**per-devserver** model: a user's devserver is a first-class entity reached through an
always-authenticated, segment-preserving reverse-proxy over a per-devserver tunnel.

### Added

- **Open a chan-library identically, local or remote.** The first time a library is opened with an
  empty window set it mints exactly one terminal and records that it has done so; close that terminal
  and reopen the library and it comes back with none. This rule now lives in the library itself, so
  the desktop's local library and a connected `chan devserver` behave the same — replacing the
  desktop's per-boot "always a shell" floor and the per-connection bootstrap flag.
- **Per-devserver sharing on chan.app.** A user's devserver is a first-class entity with a stable id;
  the identity dashboard's **Devservers** page manages it and email-based **sharing grants**
  (viewer/editor), and per-workspace share links hand an authenticated browser straight to the
  devserver. (Opening the *whole* devserver as a launcher is deferred — see below.)
- **Library-aware drag-and-drop scope.** Tab and pane drags carry a structured
  `(library_id, container, workspace)` scope, so a terminal or workspace tab only drops within its own
  library and workspace — consistent local and remote.

### Changed

- **The gateway is now a per-devserver, always-authenticated reverse-proxy.** Renamed
  `workspace-proxy → devserver-proxy` and `workspace-gate → devserver-gate`; tunnel registration is
  keyed on the token-resolved `devserver_id`, the tunnel always authenticates, and the proxy forwards
  the full request path unchanged to the devserver's own router (it renders nothing itself).
- **New Terminal and Cmd+Shift+N on a devserver window** mint through the focused window's library — a
  proper library terminal on the shared terminal tenant — instead of a local/legacy isolated terminal.
- **Workspace on/off and terminal-window persistence are unified** into one library-owned shape, so a
  restart comes back serving exactly what was on, local and devserver alike.

### Fixed

- Intra-window pane drag-and-drop, which broke under the new library-aware scope: the scope rode a
  DataTransfer MIME *type* and WebKit mangled the `:` / `|`, so even same-window drops were rejected.
  The scope token is now hex-encoded and byte-stable.
- The rich-prompt composer becoming un-typeable after a queued message drained: the clear now
  re-enables editing in the same transaction and refocuses on a microtask.
- Terminal query-reply garbage (`…R` / `…c`, cursor-position and device-attribute replies) printed at
  the prompt after a Cmd+R reattach: the replay window that suppresses replies to historical queries
  now ends when the replayed ring has drained, not when the `ready` frame arrives.
- Devserver tenant root: `/{slug}/` now serves (trailing slash canonicalized).
- Cross-window tab-drag scope now keys on workspace identity rather than the window label.

### Removed

- The dead per-label devserver terminal subsystem — `POST /api/devserver/terminals` and its handlers,
  `PersistedTerminal` persistence, and the Window-menu terminal-reopen path — superseded by library
  terminals on the shared tenant.
- The tunnel's `public` wire field and the dead per-workspace public-router path; the tunnel is always
  authenticated.

## [v0.41.0] - 2026-06-21

A round centred on the window lifecycle: a single library window registry now owns every window
(local and devserver), and a window watcher reconciles native windows against its live feed — so
windows mint, persist, reconnect, reload, and restore their layout from one source of truth.
On top of that: live cross-window settings sync, dashboard config moved out of the search index,
broader reload-survival, and an async/perf pass.

### Added

- **Live cross-window settings sync.** Changing a setting in one window of a workspace — theme,
  fonts, pane widths, the page-width slider, overlay-maximize — now applies in every other open
  window of that workspace immediately, without a reload. A Settings save broadcasts a
  `config_changed` frame on the workspace's event bus and each window re-reads and reflects it.
- **Web launcher: Gmail-style multi-select + bulk actions.** Select one or more workspace rows to
  reveal a bulk-action bar — Turn On, Turn Off, Delete — that loops the single-workspace op over the
  selection and reports partial failures. Delete is bulk-only behind a confirm; the per-row On/Off
  pill stays the quick single toggle.
- **Web launcher: Open terminal.** A top-bar button that mints a fresh local terminal window.
- `cs terminal close --tab-name <n> | --tab-group <g>`: tear down terminal sessions by name or
  group — the explicit teardown partner to `cs terminal restart` / `new`. Closing a session frees
  its tab name; `--tab-group` tears down a whole group (e.g. a finished team) in one call.
- Confirm-before-off for a workspace with live terminals: turning a workspace off when it still has
  running terminals now prompts ("N terminals still running — turn off anyway?") and only unmounts
  on confirm, instead of silently killing the shells. Enforced server-side so the desktop, `cs`, and
  the launcher all get the guard.

### Changed

- **The window lifecycle is driven by a window watcher against a library window registry.** A single
  per-library registry is the authoritative window set (it mints opaque window ids, assigns
  "Window N" ordinals, composes titles, and persists the set to disk). The desktop opens, closes,
  and restores native windows by reconciling against that set's live feed, for both local windows
  and a connected `chan devserver` — replacing the per-surface imperative open/close paths. Standalone
  terminals are now first-class library windows under the same lifecycle, so they mint, persist, and
  reopen like workspace windows. `cs window list` reads the same set, so `cs`, the launcher, the HTTP
  API, and the desktop never disagree.
- The dashboard / overlay config (screensaver toggle, timeout, theme, pin, and the report /
  semantic-search opt-ins) is no longer stored inside the search index config — it moves to a
  per-workspace `dashboard.toml`, so a search reindex or a vector wipe can no longer reset it.
  Existing workspaces migrate their toggles in place on first open.
- `cs-link-dismissed`, the page-width ratio, and overlay-maximize are now per-library server
  preferences instead of browser-local storage, so they travel with the library and stay consistent
  across clients (and sync live across windows).

### Fixed

- **Reload-survival of the full layout.** A window reloads back to its exact prior state — a
  standalone terminal, a terminal-only or empty-split layout, and a Hybrid pane flip (with its
  per-Hybrid theme) all now persist and restore, where before they reset on reload, off/on, or a
  desktop relaunch. (Terminal panes come back with fresh shells; the layout is preserved.)
- **Transparent re-attach of a restarted terminal.** `cs terminal restart` now re-attaches the tab
  to the relaunched session in place — the shell swaps under a live socket and the tab stays — instead
  of dropping the tab and leaving a live-backend / dead-frontend ghost.
- A killed terminal session is reaped from the registry so it stops appearing in `cs terminal list`
  and frees its tab name, so re-spawning under that name no longer collides and comes up renamed.
- **Rich-prompt queuing.** The composer no longer locks read-only after a submit: it clears and stays
  editable so you can queue messages back to back, ArrowUp recalls the last queued message to edit,
  and Esc dequeues it (or abandons the current draft). A failed send restores the text for retry.
- macOS GUI launch (Finder / Dock / Spotlight) now resolves the user's real interactive shell PATH
  before the embedded server starts, so `~/.local/bin`, Homebrew, and custom dirs are visible — fixing
  the false "create the `cs` alias" card under the restricted launchd PATH. The resolution is bounded
  with a ~3s timeout so a pathological shell rc can't hang app launch.
- Cmd+R (and the devtools / zoom chords) are no longer dead on a devserver window: the desktop
  key-bridge only swallows a keystroke when its IPC is actually present, otherwise the event falls
  through to the SPA's own reload handler.
- The editor hang-recovery buffer is now namespaced per workspace, so two workspaces with a file at
  the same relative path (e.g. `README.md`) can no longer restore one's unsaved content into the other.
- The onboarding nudge ("enable semantic search + reports") now shows only on a workspace's first
  boot — gated on whether the workspace has any indexed content or an optional layer enabled — instead
  of on every boot in a fresh WebView.
- Performance / async hardening: PTY spawn and the `lsof` cwd probes run off the terminal-registry
  lock (and off the async runtime), so a terminal launch or a multi-session `cs term list` no longer
  stalls every other terminal op; preference writes are serialized through one in-flight chain so
  near-simultaneous setting flips can't clobber each other; and a workspace-off no longer blocks the
  desktop runtime waiting on the lock release.

## [v0.40.0] - 2026-06-19

Making the `chan devserver` window + terminal lifecycle actually work end to end — reconnect,
window cleanup, and the file-descriptor leak — plus the devserver serving the host library, a CLI
reorganisation, and the deferred Windows/graph items.

### Added

- `chan ps`: show which registered workspaces are currently being served, and by what — a standalone
  `chan serve`, chan-desktop, or a `chan devserver`.
- Menu-reopen of closed devserver windows: a connected devserver's closed-but-saved windows appear in
  the chan-desktop Window menu and reopen to their live terminal / saved workspace layout.
- The chan-llm MCP server is now reachable on Windows (the bridge runs over the cross-platform
  control-socket transport).
- Windows writer-lock: a contender can now reclaim a lock from a leaked file handle left by a
  provably-dead holder.

### Fixed

- Reconnecting to a `chan devserver` (from chan-desktop or a browser tab) now **re-attaches to the
  live terminal sessions** instead of restarting them: standalone-terminal shells and a workspace's
  terminals come back with their processes still running and scrollback intact — not fresh shells.
- The devserver **file-descriptor leak** (EMFILE on a long-running devserver) is fixed at its root: a
  terminal session now lives exactly as long as its window is *saved*, so a discarded window's
  sessions are reaped immediately and busy detached sessions no longer leak descriptors across
  reconnect churn. (Deeper than the v0.39.0 tantivy-watcher fix, which did not cover a steady devserver.)
- Window cleanup is now explicit: closing a window with ^W / ^D / Ctrl+Shift+W, and empty windows,
  **discard** the window (gone from `cs window list`); only **burying** a window (the OS close button
  while connected, or a window with content) saves and hides it.
- The control-terminal dialog now fires on a **connected-phase exit** — the connect script returning
  on its own or via Ctrl-C — and on Cmd+W while it is still running, not only during connecting.
- `chan devserver` now **serves the host library**: it lists every workspace `chan workspace ls`
  shows (each on/off-able), instead of coming up empty and chan-desktop hanging on "Loading…".
- fs-graph paged-resume pages no longer carry parent-less `contains` edges (an internal correctness
  fix; the paged graph now matches the unpaged one page-for-page).

### Changed

- CLI: registry and content operations are grouped under a `chan workspace <…>` subcommand —
  `chan add` → `chan workspace add`, `chan list` → `chan workspace ls`, `chan remove` →
  `chan workspace rm`, and `index` / `reports` / `search` / `graph` / `status` / `metadata` /
  `contacts` likewise. The top level keeps `serve`, `unserve`, `ps`, `devserver`, `shell`, `config`,
  `upgrade`, and `completions`. (Pre-release: the old flat forms are removed, not aliased.)
- The `chan` tagline is now "an AI-native workspace for your Markdown notes and projects."
- "Forget" on a devserver workspace now removes it from the host library (the same as
  `chan workspace rm`, binning its trash) — one destructive Forget across the CLI, chan-desktop, and
  the devserver, since the host library is the single source of truth.

## [v0.39.1] - 2026-06-18

A patch for three issues found smoke-testing the v0.39.0 `chan devserver` connect flow.

### Fixed

- Connecting to a remote devserver no longer fails with `HTTP 415 Unsupported Media Type`. The
  connect flow's first terminal is now created as a first-class persisted, per-tenant terminal (like
  every other devserver terminal), so it also re-surfaces on reconnect. This also fixes Cmd+Shift+N
  on a focused devserver terminal silently falling back to the launcher.
- The control terminal now surfaces the abandon / edit / retry dialog on every close or exit while
  connecting — Ctrl-C, Ctrl-W, or the close button — not only when the connect script fails. Choosing
  abandon disconnects and resets the launcher back to "Connect" instead of leaving it stuck on
  "connecting".
- Connect-failure error message: the missing period before "Its control terminal is still open …" is
  restored.

## [v0.39.0] - 2026-06-18

A hardening round on the `chan devserver` + chan-desktop surface: workspace lifecycle, lock
correctness, and standalone-terminal persistence.

### Added

- Devserver workspaces now have an on/off toggle: unload a remote workspace (releasing its writer
  lock) without forgetting it, then toggle it back on — from the chan-desktop launcher. The off/on
  state persists across a devserver restart.
- `chan unserve <path>`: tear down a running `chan serve` for a workspace from the command line (the
  CLI counterpart to the desktop on/off), releasing the writer lock so the workspace can be re-served
  or removed.
- `chan remove <path>` now unserves a running serve first, then forgets everything about the
  workspace — index, graph, sessions, tokens, report, registry entry, and the whole
  `~/.chan/workspaces/<key>/` metadata directory — so it never fails with "workspace locked" on a
  live serve.
- Self-upgrade download progress: a text meter (percent, size, elapsed, ETA) in the terminal and a
  progress bar in chan-desktop.
- Standalone terminal persistence at the launcher: a devserver's terminal windows and their pane/tab
  layout come back when chan-desktop reconnects or the devserver restarts — reconnecting to the live
  shells while the devserver is still up, or fresh shells with the saved layout after a restart.
  `cs window list` and the Window menu reflect them.

### Fixed

- Workspace lock correctness: the writer lock now records the holder's pid, path, and start time, and
  a contender reclaims the lock only from a provably-dead holder instead of failing. Fixes rapid
  Open / On / Off clicking in chan-desktop wedging a workspace as "locked" with no live process.
- Devserver file-descriptor leak (EMFILE) on a long-running multi-workspace devserver: the redundant
  tantivy commit-watcher (a second inotify watcher per workspace) is gone, so the descriptor count
  stays bounded across mount/unmount and reconnect churn.
- Control / standalone terminal behaviour in chan-desktop: the control terminal opens and stays open
  on connect (no auto-hide or flashing), is a true singleton (no replicated Terminal 1/2/3), and the
  empty standalone-terminal window no longer shows a flashing floating button.
- Failing connect script: closing a failing control terminal now surfaces a re-run / disconnect
  survey and tears down cleanly instead of leaving the launcher stuck on "connecting" with an empty
  window.
- An empty devserver (zero workspaces) now loads on connect and across a restart.
- Graph: in a directory scope, every file node now anchors to its folder spine, so cross-tree files
  (link / mention / tag targets from elsewhere in the workspace) no longer render loose.

## [v0.38.1] - 2026-06-18

### Added

- `chan devserver --launchd` (macOS): supervise the devserver under a per-user launchd LaunchAgent (`app.chan.devserver`) so it survives the launching shell; re-running re-attaches to the live agent. The macOS counterpart to `--systemd`. It outlives the GUI login session but not a full logout (launchd has no per-user linger without a root LaunchDaemon); stop it with `launchctl bootout gui/$(id -u)/app.chan.devserver`.

### Fixed

- Editor: opening a Markdown file with Windows (CRLF) line endings no longer freezes the editor in a reactive render loop. CodeMirror normalizes the document to LF internally, so the external-value sync now compares and writes against the same normalization; previously a `\r\n` file never matched the live (LF) document, re-dispatching on every reactive pass until Svelte tripped its update-depth guard.
- `chan devserver --systemd`: a fresh start now surfaces the bearer token to the controlling terminal even when the invoking user cannot read the systemd journal (a uid below `SYS_UID_MAX`, or a user outside the `systemd-journal`/`adm` groups) — the supervisor emits the `CHAN_DEVSERVER_TOKEN=` marker directly from the persisted config rather than relying on the journal follow, and keeps supervising (or fails loud) instead of quitting when the journal stream ends.

## [v0.38.0] - 2026-06-17

### Added

- `chan devserver`: one process hosts many workspaces behind a single port. Register workspaces into it with `chan serve PATH` (each registers and exits instead of binding its own port, so one process owns each workspace). chan-desktop connects to a devserver and lists its workspaces in their own launcher group, with a New Terminal button that opens standalone terminals on the devserver.
- `chan devserver --systemd` (Linux): run the devserver under a `chan-devserver.service` systemd user service so it survives the launching shell and logout; re-running re-attaches to the live service. Reach it from chan-desktop at `localhost` via a host-network lima VM or sdme container, or forward it from a remote box with `ssh -L`. A new Devserver page in the manual covers the workflow.

### Changed

- `chan serve` now requires an explicit workspace path. Running it with no path exits with an error asking you to pass one, instead of falling back to a default workspace.
- New workspaces open with no docked file browser — just the empty pane — across the web app, chan-desktop, and devserver workspaces.
- A devserver's launcher section mirrors the local-workspace controls: a single Connect button with an Edit/Forget menu that becomes Disconnect plus a New Terminal button once connected; adding a devserver auto-connects it.
- Per-devserver standalone terminals behave like local ones — Cmd+Shift+N opens another terminal on the same devserver, and terminal tabs drag and drop between that devserver's windows. Control terminals stay isolated from both.
- Connecting to a scripted devserver reads its token from the connect-script's `CHAN_DEVSERVER_TOKEN=` output on every connect (including a `--systemd` re-attach), so reconnecting after a dropped connection or a devserver restart is seamless.

### Fixed

- Editor: pasting an image leaves the cursor just past the image instead of jumping to the next line.
- Editor: backspacing near an inline image no longer deletes the whole image; deletion is directional, matching a normal text editor.
- A failed scripted-devserver connect now offers retry / edit / abandon instead of getting stuck on "Connecting", and closing a control-terminal tab surveys the same way instead of leaving a broken window.
- Disconnecting or forgetting a scripted devserver stops its connect script instead of leaving the process running, and quitting chan-desktop reaps a connected devserver's script.
- Editing a devserver's port and reconnecting works without sticking on "Connecting"; New-workspace dialog validation errors render inside the dialog rather than behind it.
- `chan devserver` shuts down promptly on SIGINT and SIGTERM with a hard deadline (matching `chan serve`) and writes its config durably; `chan devserver --port 0` reports the actual bound port.

### Removed

- The default-workspace concept is gone from the standalone CLI and server too (chan-desktop dropped it in v0.37.0): no `~/Documents/Chan` / `$XDG_DATA_HOME/chan/default` fallback, no per-machine default-workspace setting, and the Dashboard's "Workspaces → Default" field is removed.

## [v0.37.0] - 2026-06-16

### Added

- chan-desktop remembers which workspaces were on and re-serves them on the next launch, so the app comes back up showing what you left running.

### Changed

- A fresh chan-desktop launch no longer creates a default workspace: there is no `~/Documents/Chan` and no seeded manual. The launcher opens empty and a standalone terminal window opens alongside it; add a workspace when you want one.
- chan-desktop configuration now lives under `~/.chan/desktop/config.json`.
- The remote-workspace mode is now labeled simply **Remote**.

### Removed

- The first-run default-workspace prompt (create / choose / factory-reset) is gone end to end.
- Remote **inbound** is removed from chan-desktop entirely (the embedded inbound tunnel listener is gone); only the outbound "Remote" mode remains. The standalone gateway's tunnel server is unaffected.
- Releases no longer ship the separate manual tarball.

### Fixed

- Windows: opening a terminal no longer briefly hangs the app while Git BASH is being discovered — discovery is primed off the async request path.
- Windows: `chan` and `cs` resolve from the desktop install in cmd, PowerShell, and Git BASH, and a freshly-opened shell picks them up without a logout.
- Windows: `chan` / `cs` now actually print their output (for example `chan --version`) when run from a terminal — the desktop binary reattaches to the parent console for the CLI path; output redirection (`> out.txt`) still works.
- Windows: `chan serve <path>` hands the workspace to a running chan-desktop (opening it in a window) instead of starting a standalone browser server and leaving the workspace stuck "off" in the launcher.
- Windows: opening a file in a workspace no longer hangs the whole window while the workspace is still building its index. The graph reader pool no longer stalls behind the first index build (a contended read now fails fast instead of parking), and the reindex paces itself so the editor loads and the window stays responsive; the relationship/graph panels fill in once indexing finishes.
- The Settings shortcut (Ctrl+,) is shown in the terminal-tab and editor-tab right-click menus.
- Tabs can no longer be dragged between a standalone terminal window and a workspace window, or between two different workspaces; such drops are refused. Reordering within a window, and moving a tab between two windows of the same workspace (or two terminal windows), still work.

## [v0.34.0] - 2026-06-14

### Added

- `cs window` manages desktop windows from a terminal. `cs window list` shows each window's real title and kind alongside its status, matching the title bar and the Window menu, and the new verbs drive the desktop: `new` opens a window (another standalone terminal window from a standalone terminal, another window of the workspace from a workspace terminal), `open <id>` focuses or un-hides one, `hide <id>` hides it like the close button, `rm <id>` removes it for good and drops its saved layout (prompting first when it still has running terminals, or `--force` to skip), and `title <id> <title>` sets a custom window title (empty resets it; a title another window already shows is rejected so window names stay unambiguous). The lifecycle verbs need the desktop app.

### Fixed

- `chan serve .` (or any relative path) on macOS could open a workspace on the filesystem root when handed off to a running chan-desktop: the relative path was resolved against the desktop's working directory instead of the terminal's. The serve root is now made absolute before the handoff.

## [v0.33.0] - 2026-06-13

### Added

- The Rich Prompt keeps a submitted message visible until the agent actually consumes it: the text stays in the prompt (read-only) with a "queued" indicator, and the terminal tab shows a queue-depth badge counting pending messages (including teammate pokes). Mirrors the Claude/Codex desktop behavior.
- The graph right-click menu has a Reload item again, between Depth and Copy link to graph, for refetching the graph on demand.
- The survey overlay can be dismissed from the keyboard with X (in addition to Escape and the Dismiss button).
- The desktop launcher's Open button is always enabled: opening a stopped workspace turns it on automatically, and a turn-on failure (for example, the workspace is already open in another process) now shows a dialog explaining why instead of silently flipping the toggle back.

### Fixed

- Switching away from and back to an editor tab no longer shows raw un-decorated markdown until you click, and no longer resets the scroll position. Editor tabs are kept alive across switches, so scroll, caret, undo history, and find state are all preserved.
- Switching to a graph tab no longer reloads and re-lays-out the graph. Graph tabs are kept alive across switches; pan, zoom, and selection survive, and large workspaces no longer pay a reload on every tab focus. On-disk changes still refresh the visible graph, and the new Reload item forces a manual refetch.
- Clicking a terminal tab now lands keyboard focus in the terminal so you can type immediately, matching the keyboard pane-switch shortcut.
- Undo can no longer walk back past a file's initial load to an empty document (which autosave would then have written to disk).

### Changed

- New teams start with broadcast off; enable it per tab when you want a lead terminal to fan keystrokes to the others.
- Buried desktop windows (closed but kept warm in memory) no longer count against the per-workspace window cap, and the Window menu's "Hidden Windows" header shows how many are kept warm.

## [v0.32.0] - 2026-06-12

### Added

- Dropping files from Finder onto a terminal pane types their shell-escaped absolute paths at the cursor, like macOS Terminal (multiple files space-separated). macOS desktop only; remote (tunnel/outbound) windows deliberately excluded.

### Fixed

- Dropping a file anywhere outside the editor on a desktop window no longer navigates the webview into a bare image view with no way back. Drops are now inert on every non-editor, non-terminal surface, in the desktop app and the browser alike; editor image embeds and in-page tab drags are unaffected.
- SVG images embedded in documents render again: the file API served SVG (valid UTF-8 text) as an editor JSON envelope instead of image bytes, so the image widget showed "image not found". Image- and PDF-class reads now return raw bytes with the correct content type.

### Changed

- The macOS bundle identifier is now `app.chan.desktop` (was `com.chanwriter.desktop`). After upgrading, expect a one-time keychain "Always Allow" prompt and a launcher theme reset; workspaces, configuration, and self-update continuity are unaffected.
- Documentation overhaul: README content that duplicated the manual is now pointed into it (serve flags, tunnel walkthrough), every design document was rewritten against current source, and the config reference was trued up field-by-field. Code comments and help text no longer narrate project history; several stale claims (a help text inverting the reports default, docs citing removed commands and wrong env vars) were corrected.
- Internal hygiene: compiler and frontend warnings are at zero across every workspace; several many-parameter functions gained config structs; the last ad-hoc keyboard shortcuts moved into the chord registry (fixing a Linux menu label that displayed a chord the handler ignores).

## [v0.31.1] - 2026-06-12

### Added

- Linux and Windows gained File > Close Window on Ctrl+Shift+W (plain Ctrl+W remains a terminal readline chord): it closes the active tab in a workspace window, cancels a connecting window, and closes other windows natively — the same routing macOS has on Cmd+W.

### Changed

- The About window no longer shows the application menubar on Linux and Windows; the fixed-size dialog is just the About content.

### Fixed

- Quitting (Cmd/Ctrl+Q or the Quit menu) now actually asks for confirmation while windows are open or hidden. The v0.31.0 dialog never appeared on macOS: the system's predefined Quit item exits through a flow the confirmation hook cannot stop, so Quit is now Chan's own menu item that asks before any exit begins.
- Outbound connecting/retry windows are closable again: the close button closes them for real instead of hiding an invisible retry loop, and Cmd+W (macOS), Ctrl+Shift+W (Linux/Windows), and Ctrl+D all cancel the connection attempt from the keyboard.
- Discarding Hybrid Nav staging (Esc) now kills the shell a staged terminal spawned; previously a staged-then-cancelled split left its shell running invisibly until the idle pruner collected it.

## [v0.31.0] - 2026-06-12

### Added

- Closing a desktop window with the OS close button now hides ("buries") it instead of destroying it: terminals keep running, the layout stays warm, and an informational dialog explains the behaviour. Buried windows are listed in a "Hidden Windows" section of the Window menu for reopening; a standalone terminal window with no shells left still closes for real.
- Cmd/Ctrl+Shift+N now reopens the most recently hidden window of the focused window's family before opening a new one, and "New Window" follows the focused connection everywhere: another window of the same local workspace, the same outbound or tunneled remote, or another standalone terminal window.
- Remote windows are reopenable ad hoc: chan-server gained `GET /api/windows` (saved per-window layouts joined with live socket presence), and chan-desktop polls outbound/tunnel connections to offer their reopenable windows in a "Remote Windows" menu section.
- `cs window list` (or `cs w l`) shows every window the server knows about — open (a live event socket is connected) and/or saved (a persisted layout exists). Works in workspaces and standalone terminals.
- Standalone terminal windows now expose the chan control socket: `cs terminal list/write/restart/scrollback`, `cs pane`, `cs terminal survey`, and `cs window list` work inside them, while workspace-only commands (open, graph, dashboard, search, team) refuse with a clear "this is a standalone terminal session" message.
- Quitting Chan Desktop (Cmd+Q or the Quit menu) now asks for confirmation while any window is open or hidden, since quitting stops their terminals and local workspaces. A bare launcher still quits silently.
- A window now reloads itself when the server process behind it restarts (e.g. an outbound `chan serve` was ^C'd and re-run): previously the window sat on a stale view with stuck terminals until a manual reload.

### Changed

- The workspace launcher is a singleton titled "Chan Desktop" (no more "Window N" suffix), and Cmd/Ctrl+Shift+N on it opens a standalone terminal window instead of another launcher.
- The mislabeled "Settings… Cmd+," Window-menu item is gone; Cmd+, (the Hybrid pane flip) is handled by the app itself and keeps working.
- In standalone terminal windows, the Hybrid Nav cheatsheet now shows only terminal-relevant commands; the workspace-only rows (File Browser, Graph, New Draft, Search, docks) no longer render as dead controls.
- `make clean` now also scrubs the gateway workspace (its own cargo target, npm trees, and SPA dist), the desktop extras, and the web build stamp.
- Tab titles get a little fade headroom so short names ("Terminal-1") keep their trailing character legible instead of fading out.
- CI macOS desktop builds select the newest Xcode on the runner so the shipped app gets the modern window chrome (the look follows the SDK the binary was linked against; older CI Xcode produced the legacy opaque title bar).

### Fixed

- Splitting a pane no longer leaves the original terminal showing only its last line until a window reload. Root cause: a remounted terminal kept a replay cursor and skipped the server's scrollback replay; the cursor was removed and every remount (split, swap, drag, move, reload) now replays the full ring.
- Opening a standalone terminal window no longer logs a spurious "503 Service Unavailable" error in the desktop console: `/api/health` now answers on workspace-less tenants (the indexer block is simply null there).
- The dead "p Stage Team Work Terminal" row was removed from the Hybrid Nav cheatsheet; Team Work spawning lives in the lead-only Cmd+P dialog.

## [v0.30.1] - 2026-06-10

### Changed

- The "Set MCP env vars" control moved from the terminal right-click menu into Terminal Settings, where it is a single global toggle (off by default) that applies to newly opened workspace terminals.
- Desktop windows are now numbered in the Window menu — "<workspace> Window 1", "Terminal Window 1", "Chan Desktop Window 1", and so on — with a number reused when a window closes, so duplicate windows are no longer indistinguishable.
- The broadcast-input Select All / Deselect All shortcut now works on Linux and Windows as Ctrl+Shift+I (Cmd+Shift+I on macOS); it previously had no binding outside macOS.
- The install script now also symlinks `cs` to `chan` in the install directory.

### Fixed

- Enabling MCP env vars now actually sets CHAN_MCP_* in newly opened workspace terminals; the toggle had no effect after MCP was made off-by-default. Standalone terminal windows have no workspace and still do not expose MCP.
- Dragging a terminal tab into another window no longer pulls the Chan Desktop launcher to the front when the source window closes — focus stays on the window you dropped into.

## [v0.30.0] - 2026-06-10

### Changed

- The Dashboard carousel now opens on Workspace first, then Search, then About (previously About led).
- The per-workspace config — your default workspace directory and the recent workspaces list — moved off the Workspace dashboard slide and onto that slot's settings. Flip the slide with Cmd+, to reach it, below chan-reports and the metadata archive.
- The workspace inspector's "Notes directories" section is now titled "Workspaces".

### Fixed

- The chan-desktop menu bar no longer shows two "File" menus on macOS.
- Cmd+W works again on the chan-desktop launcher (Workspaces) window, where it closes the window; workspace and terminal windows still close the active tab.
- New terminals reuse the lowest free number: open Terminal-1 and Terminal-2, close Terminal-2, and the next terminal is Terminal-2 again instead of Terminal-3.
- Dragging a terminal to another window keeps its name when nothing clashes, instead of always appending a "-N" suffix. A suffix is added only on a real name conflict, and then the terminal shows the "$CHAN_TAB_NAME stays until restart" notice so you can resync the env.

## [v0.29.0] - 2026-06-10

### Added

- Standalone terminal windows on chan-desktop: File > New Terminal (Cmd+T) opens a window that holds only a terminal, with no workspace. These windows split panes, use Hybrid Nav, keep broadcast + shortcuts, and configure the terminal via the Cmd+, tab flip; Cmd+T adds a tab and Cmd+Shift+N opens another terminal window.
- Broadcast input now spans terminal windows. A terminal's broadcast menu lists same-group terminals in other windows, Select All / Deselect All (Cmd+Shift+I on macOS) applies to the whole group across every window, and every participating terminal shows the broadcast sign in its own window.

### Changed

- Terminal-N numbering is consistent across every window of a tenant: all standalone terminal windows share one sequence, and all windows of a workspace share that workspace's sequence, instead of restarting at 1 in each new window.
- The desktop About window is unified across macOS and Linux and shows the same information as the in-app Dashboard.

### Fixed

- Cross-window broadcast respects group boundaries: a terminal with broadcast turned off no longer receives input broadcast from another window.
- Terminal names are unique across all windows, not just within one window, so renaming or regrouping a terminal can no longer collide with a terminal in another window.
- The desktop update notification shows plain text plus a changelog link instead of rendering the release notes as raw markdown.

## [v0.28.1] - 2026-06-08

### Fixed

- Pasting into the terminal no longer pops a "Paste" button you have to click first. Cmd+V now pastes directly through the terminal's native paste path (which also restores bracketed paste for multi-line content), and the right-click "Paste" menu reads the clipboard natively on chan-desktop instead of through the WebKit clipboard prompt.

## [v0.28.0] - 2026-06-05

Phase 19: a graph `@@mention` lens, a startup index-reconcile fix, the agent-docs reorg into a committed `.agents/` home, and a marketing story page.

### Added

- Graph `@@mention` lens. Clicking a standalone `@@handle` from the file inspector, an editor mention, or a search mention row opens a focused graph centered on the `@@{name}` node with an edge to every file that references it, each re-anchored through its parent-directory spine back to the workspace root. Mirrors the existing `#tag` lens. Search now surfaces mention rows alongside tags.
- A chan story page on the marketing site (`/story`) carrying the project motivation, an architecture diagram, and a tour of the IDE.

### Changed

- Agent and contributor docs now live in a single committed `.agents/` home (standards, roster, orchestration contracts, and skills). The near-duplicate root `CLAUDE.md` and `AGENTS.md` are removed; `README.md` and `CONTRIBUTING.md` point into `.agents/README.md`.

### Fixed

- The graph index reconciles against disk on workspace open. A markdown file added, edited, or removed while no server was watching (closed laptop, no `chan serve` running) is now picked up on the next start instead of staying invisible across restarts, so its mentions and tags get edges. Cold or empty workspaces still defer to the background full build, so open stays fast.
- Contacts (`chan.kind: contact` notes) render as contact nodes in the graph even when reached only by a link rather than an `@@mention`. They previously fell back to the generic markdown node glyph while the file browser, inspector, and `@{}` search already treated them as contacts.

## [v0.27.1] - 2026-06-05

### Fixed

- New Draft (Cmd+N) surfaces the drafts directory in the file tree.
- File browser expansion state persists across reload and tab switch.

## [v0.27.0] - 2026-06-05

### Changed

- Drafts are stored in-tree under a configurable `.Drafts/` directory and addressed as in-root workspace paths; the server surfaces the drafts directory and the web client keys draft-path logic off it.

### Fixed

- A moved or deleted draft tab now closes cleanly.

## [v0.26.2] - 2026-06-05

Phase 18 follow-up: Linux desktop (WebKitGTK) fixes found while testing the v0.26.x desktop build. macOS code paths are unchanged.

### Added

- Linux desktop File menu, built explicitly because `Menu::default` only produces a File menu on macOS: File (About, Quit), Edit, Window, no Help. "About Chan" shows the version plus a manual "Check for updates" (the only manual self-update entry point off macOS); Quit is a custom item with an `app.exit(0)` handler because muda does not implement the predefined Quit on GTK.

### Fixed

- New draft (Ctrl+N) and Show Source (Ctrl+E) now fire off macOS. The handlers were Mac-only by accident (`Mod` resolves to Ctrl on Linux/Windows, and a `!ctrlKey` guard excluded it); they now follow the per-OS chord the shortcut registry already declared.
- The Hybrid pane flip (Cmd+, / Ctrl+,) no longer sticks mirror-reversed under WebKitGTK: the rotated-away face is hidden with a state-driven visibility swap rather than relying on `backface-visibility`, which WebKitGTK ignores inside a `preserve-3d` context (Blink was already correct, so the browser build was unaffected).
- The embedded terminal stays on the DOM renderer under WebKitGTK, fixing typed and pasted input that did not paint until a later keystroke (the WebGL layer did not composite while idle). Box-drawing characters fall back to the system font's glyphs on the Linux desktop.
- Ctrl+E stays inside a focused terminal for readline (move-to-end-of-line) instead of being claimed by the Show Source toggle.

## [v0.26.1] - 2026-06-04

Phase 18 follow-up: desktop self-update and Linux AppImage fixes.

### Fixed

- Desktop self-upgrade: the updater manifest endpoint was flattened to the static `/dl/desktop/latest.json` the release generator actually publishes; the previous templated path never matched, so desktop self-update always 404'd.
- Linux AppImage: prefer the host GTK/WebKit stack so a host whose Mesa is newer than the bundle (e.g. CachyOS) no longer aborts webview creation with `EGL_BAD_PARAMETER`.
- Insp
