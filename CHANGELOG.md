# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [v0.33.0] - 2026-06-13

### Added

- The Rich Prompt keeps a submitted message visible until the agent
  actually consumes it: the text stays in the prompt (read-only) with
  a "queued" indicator, and the terminal tab shows a queue-depth badge
  counting pending messages (including teammate pokes). Mirrors the
  Claude/Codex desktop behavior.
- The graph right-click menu has a Reload item again, between Depth and
  Copy link to graph, for refetching the graph on demand.
- The survey overlay can be dismissed from the keyboard with X (in
  addition to Escape and the Dismiss button).
- The desktop launcher's Open button is always enabled: opening a
  stopped workspace turns it on automatically, and a turn-on failure
  (for example, the workspace is already open in another process) now
  shows a dialog explaining why instead of silently flipping the
  toggle back.

### Fixed

- Switching away from and back to an editor tab no longer shows raw
  un-decorated markdown until you click, and no longer resets the
  scroll position. Editor tabs are kept alive across switches, so
  scroll, caret, undo history, and find state are all preserved.
- Switching to a graph tab no longer reloads and re-lays-out the
  graph. Graph tabs are kept alive across switches; pan, zoom, and
  selection survive, and large workspaces no longer pay a reload on
  every tab focus. On-disk changes still refresh the visible graph,
  and the new Reload item forces a manual refetch.
- Clicking a terminal tab now lands keyboard focus in the terminal so
  you can type immediately, matching the keyboard pane-switch shortcut.
- Undo can no longer walk back past a file's initial load to an empty
  document (which autosave would then have written to disk).

### Changed

- New teams start with broadcast off; enable it per tab when you want a
  lead terminal to fan keystrokes to the others.
- Buried desktop windows (closed but kept warm in memory) no longer
  count against the per-workspace window cap, and the Window menu's
  "Hidden Windows" header shows how many are kept warm.

## [v0.32.0] - 2026-06-12

### Added

- Dropping files from Finder onto a terminal pane types their
  shell-escaped absolute paths at the cursor, like macOS Terminal
  (multiple files space-separated). macOS desktop only; remote
  (tunnel/outbound) windows deliberately excluded.

### Fixed

- Dropping a file anywhere outside the editor on a desktop window no
  longer navigates the webview into a bare image view with no way
  back. Drops are now inert on every non-editor, non-terminal
  surface, in the desktop app and the browser alike; editor image
  embeds and in-page tab drags are unaffected.
- SVG images embedded in documents render again: the file API served
  SVG (valid UTF-8 text) as an editor JSON envelope instead of image
  bytes, so the image widget showed "image not found". Image- and
  PDF-class reads now return raw bytes with the correct content type.

### Changed

- The macOS bundle identifier is now `app.chan.desktop` (was
  `com.chanwriter.desktop`). After upgrading, expect a one-time
  keychain "Always Allow" prompt and a launcher theme reset;
  workspaces, configuration, and self-update continuity are
  unaffected.
- Documentation overhaul: README content that duplicated the manual
  is now pointed into it (serve flags, tunnel walkthrough), every
  design document was rewritten against current source, and the
  config reference was trued up field-by-field. Code comments and
  help text no longer narrate project history; several stale claims
  (a help text inverting the reports default, docs citing removed
  commands and wrong env vars) were corrected.
- Internal hygiene: compiler and frontend warnings are at zero
  across every workspace; several many-parameter functions gained
  config structs; the last ad-hoc keyboard shortcuts moved into the
  chord registry (fixing a Linux menu label that displayed a chord
  the handler ignores).

## [v0.31.1] - 2026-06-12

### Added

- Linux and Windows gained File > Close Window on Ctrl+Shift+W (plain Ctrl+W
  remains a terminal readline chord): it closes the active tab in a workspace
  window, cancels a connecting window, and closes other windows natively —
  the same routing macOS has on Cmd+W.

### Changed

- The About window no longer shows the application menubar on Linux and
  Windows; the fixed-size dialog is just the About content.

### Fixed

- Quitting (Cmd/Ctrl+Q or the Quit menu) now actually asks for confirmation
  while windows are open or hidden. The v0.31.0 dialog never appeared on
  macOS: the system's predefined Quit item exits through a flow the
  confirmation hook cannot stop, so Quit is now Chan's own menu item that
  asks before any exit begins.
- Outbound connecting/retry windows are closable again: the close button
  closes them for real instead of hiding an invisible retry loop, and
  Cmd+W (macOS), Ctrl+Shift+W (Linux/Windows), and Ctrl+D all cancel the
  connection attempt from the keyboard.
- Discarding Hybrid Nav staging (Esc) now kills the shell a staged terminal
  spawned; previously a staged-then-cancelled split left its shell running
  invisibly until the idle pruner collected it.

## [v0.31.0] - 2026-06-12

### Added

- Closing a desktop window with the OS close button now hides ("buries") it
  instead of destroying it: terminals keep running, the layout stays warm, and
  an informational dialog explains the behaviour. Buried windows are listed in
  a "Hidden Windows" section of the Window menu for reopening; a standalone
  terminal window with no shells left still closes for real.
- Cmd/Ctrl+Shift+N now reopens the most recently hidden window of the focused
  window's family before opening a new one, and "New Window" follows the
  focused connection everywhere: another window of the same local workspace,
  the same outbound or tunneled remote, or another standalone terminal window.
- Remote windows are reopenable ad hoc: chan-server gained `GET /api/windows`
  (saved per-window layouts joined with live socket presence), and chan-desktop
  polls outbound/tunnel connections to offer their reopenable windows in a
  "Remote Windows" menu section.
- `cs window list` (or `cs w l`) shows every window the server knows about —
  open (a live event socket is connected) and/or saved (a persisted layout
  exists). Works in workspaces and standalone terminals.
- Standalone terminal windows now expose the chan control socket: `cs terminal
  list/write/restart/scrollback`, `cs pane`, `cs terminal survey`, and `cs
  window list` work inside them, while workspace-only commands (open, graph,
  dashboard, search, team) refuse with a clear "this is a standalone terminal
  session" message.
- Quitting Chan Desktop (Cmd+Q or the Quit menu) now asks for confirmation
  while any window is open or hidden, since quitting stops their terminals and
  local workspaces. A bare launcher still quits silently.
- A window now reloads itself when the server process behind it restarts
  (e.g. an outbound `chan serve` was ^C'd and re-run): previously the window
  sat on a stale view with stuck terminals until a manual reload.

### Changed

- The workspace launcher is a singleton titled "Chan Desktop" (no more
  "Window N" suffix), and Cmd/Ctrl+Shift+N on it opens a standalone terminal
  window instead of another launcher.
- The mislabeled "Settings… Cmd+," Window-menu item is gone; Cmd+, (the
  Hybrid pane flip) is handled by the app itself and keeps working.
- In standalone terminal windows, the Hybrid Nav cheatsheet now shows only
  terminal-relevant commands; the workspace-only rows (File Browser, Graph,
  New Draft, Search, docks) no longer render as dead controls.
- `make clean` now also scrubs the gateway workspace (its own cargo target,
  npm trees, and SPA dist), the desktop extras, and the web build stamp.
- Tab titles get a little fade headroom so short names ("Terminal-1") keep
  their trailing character legible instead of fading out.
- CI macOS desktop builds select the newest Xcode on the runner so the shipped
  app gets the modern window chrome (the look follows the SDK the binary was
  linked against; older CI Xcode produced the legacy opaque title bar).

### Fixed

- Splitting a pane no longer leaves the original terminal showing only its
  last line until a window reload. Root cause: a remounted terminal kept a
  replay cursor and skipped the server's scrollback replay; the cursor was
  removed and every remount (split, swap, drag, move, reload) now replays the
  full ring.
- Opening a standalone terminal window no longer logs a spurious
  "503 Service Unavailable" error in the desktop console: `/api/health` now
  answers on workspace-less tenants (the indexer block is simply null there).
- The dead "p Stage Team Work Terminal" row was removed from the Hybrid Nav
  cheatsheet; Team Work spawning lives in the lead-only Cmd+P dialog.

## [v0.30.1] - 2026-06-10

### Changed

- The "Set MCP env vars" control moved from the terminal right-click menu into
  Terminal Settings, where it is a single global toggle (off by default) that
  applies to newly opened workspace terminals.
- Desktop windows are now numbered in the Window menu — "<workspace> Window 1",
  "Terminal Window 1", "Chan Desktop Window 1", and so on — with a number
  reused when a window closes, so duplicate windows are no longer
  indistinguishable.
- The broadcast-input Select All / Deselect All shortcut now works on Linux and
  Windows as Ctrl+Shift+I (Cmd+Shift+I on macOS); it previously had no binding
  outside macOS.
- The install script now also symlinks `cs` to `chan` in the install directory.

### Fixed

- Enabling MCP env vars now actually sets CHAN_MCP_* in newly opened workspace
  terminals; the toggle had no effect after MCP was made off-by-default.
  Standalone terminal windows have no workspace and still do not expose MCP.
- Dragging a terminal tab into another window no longer pulls the Chan Desktop
  launcher to the front when the source window closes — focus stays on the
  window you dropped into.

## [v0.30.0] - 2026-06-10

### Changed

- The Dashboard carousel now opens on Workspace first, then Search, then About
  (previously About led).
- The per-workspace config — your default workspace directory and the recent
  workspaces list — moved off the Workspace dashboard slide and onto that
  slot's settings. Flip the slide with Cmd+, to reach it, below chan-reports
  and the metadata archive.
- The workspace inspector's "Notes directories" section is now titled
  "Workspaces".

### Fixed

- The chan-desktop menu bar no longer shows two "File" menus on macOS.
- Cmd+W works again on the chan-desktop launcher (Workspaces) window, where it
  closes the window; workspace and terminal windows still close the active tab.
- New terminals reuse the lowest free number: open Terminal-1 and Terminal-2,
  close Terminal-2, and the next terminal is Terminal-2 again instead of
  Terminal-3.
- Dragging a terminal to another window keeps its name when nothing clashes,
  instead of always appending a "-N" suffix. A suffix is added only on a real
  name conflict, and then the terminal shows the "$CHAN_TAB_NAME stays until
  restart" notice so you can resync the env.

## [v0.29.0] - 2026-06-10

### Added

- Standalone terminal windows on chan-desktop: File > New Terminal (Cmd+T)
  opens a window that holds only a terminal, with no workspace. These windows
  split panes, use Hybrid Nav, keep broadcast + shortcuts, and configure the
  terminal via the Cmd+, tab flip; Cmd+T adds a tab and Cmd+Shift+N opens
  another terminal window.
- Broadcast input now spans terminal windows. A terminal's broadcast menu
  lists same-group terminals in other windows, Select All / Deselect All
  (Cmd+Shift+I on macOS) applies to the whole group across every window, and
  every participating terminal shows the broadcast sign in its own window.

### Changed

- Terminal-N numbering is consistent across every window of a tenant: all
  standalone terminal windows share one sequence, and all windows of a
  workspace share that workspace's sequence, instead of restarting at 1 in each
  new window.
- The desktop About window is unified across macOS and Linux and shows the same
  information as the in-app Dashboard.

### Fixed

- Cross-window broadcast respects group boundaries: a terminal with broadcast
  turned off no longer receives input broadcast from another window.
- Terminal names are unique across all windows, not just within one window, so
  renaming or regrouping a terminal can no longer collide with a terminal in
  another window.
- The desktop update notification shows plain text plus a changelog link
  instead of rendering the release notes as raw markdown.

## [v0.28.1] - 2026-06-08

### Fixed

- Pasting into the terminal no longer pops a "Paste" button you have to
  click first. Cmd+V now pastes directly through the terminal's native
  paste path (which also restores bracketed paste for multi-line content),
  and the right-click "Paste" menu reads the clipboard natively on
  chan-desktop instead of through the WebKit clipboard prompt.

## [v0.28.0] - 2026-06-05

Phase 19: a graph @@mention lens, a startup index-reconcile fix, the
agent-docs reorg into a committed `.agents/` home, and a marketing
story page.

### Added

- Graph `@@mention` lens. Clicking a standalone `@@handle` from the file
  inspector, an editor mention, or a search mention row opens a focused
  graph centered on the `@@Name` node with an edge to every file that
  references it, each re-anchored through its parent-directory spine back
  to the workspace root. Mirrors the existing `#tag` lens. Search now
  surfaces mention rows alongside tags.
- A chan story page on the marketing site (`/story`) carrying the project
  motivation, an architecture diagram, and a tour of the IDE.

### Changed

- Agent and contributor docs now live in a single committed `.agents/`
  home (standards, roster, orchestration contracts, and skills). The
  near-duplicate root `CLAUDE.md` and `AGENTS.md` are removed; `README.md`
  and `CONTRIBUTING.md` point into `.agents/README.md`.

### Fixed

- The graph index reconciles against disk on workspace open. A markdown
  file added, edited, or removed while no server was watching (closed
  laptop, no `chan serve` running) is now picked up on the next start
  instead of staying invisible across restarts, so its mentions and tags
  get edges. Cold or empty workspaces still defer to the background full
  build, so open stays fast.
- Contacts (`chan.kind: contact` notes) render as contact nodes in the
  graph even when reached only by a link rather than an `@@mention`.
  They previously fell back to the generic markdown node glyph while the
  file browser, inspector, and `@{}` search already treated them as
  contacts.

## [v0.27.1] - 2026-06-05

### Fixed

- New Draft (Cmd+N) surfaces the drafts directory in the file tree.
- File browser expansion state persists across reload and tab switch.

## [v0.27.0] - 2026-06-05

### Changed

- Drafts are stored in-tree under a configurable `.Drafts/` directory and
  addressed as in-root workspace paths; the server surfaces the drafts
  directory and the web client keys draft-path logic off it.

### Fixed

- A moved or deleted draft tab now closes cleanly.

## [v0.26.2] - 2026-06-05

Phase 18 follow-up: Linux desktop (WebKitGTK) fixes found while testing
the v0.26.x desktop build. macOS code paths are unchanged.

### Added

- Linux desktop File menu, built explicitly because `Menu::default` only
  produces a File menu on macOS: File (About, Quit), Edit, Window, no
  Help. "About Chan" shows the version plus a manual "Check for updates"
  (the only manual self-update entry point off macOS); Quit is a custom
  item with an `app.exit(0)` handler because muda does not implement the
  predefined Quit on GTK.

### Fixed

- New draft (Ctrl+N) and Show Source (Ctrl+E) now fire off macOS. The
  handlers were Mac-only by accident (`Mod` resolves to Ctrl on
  Linux/Windows, and a `!ctrlKey` guard excluded it); they now follow the
  per-OS chord the shortcut registry already declared.
- The Hybrid pane flip (Cmd+, / Ctrl+,) no longer sticks mirror-reversed
  under WebKitGTK: the rotated-away face is hidden with a state-driven
  visibility swap rather than relying on `backface-visibility`, which
  WebKitGTK ignores inside a `preserve-3d` context (Blink was already
  correct, so the browser build was unaffected).
- The embedded terminal stays on the DOM renderer under WebKitGTK, fixing
  typed and pasted input that did not paint until a later keystroke (the
  WebGL layer did not composite while idle). Box-drawing characters fall
  back to the system font's glyphs on the Linux desktop.
- Ctrl+E stays inside a focused terminal for readline (move-to-end-of-
  line) instead of being claimed by the Show Source toggle.

## [v0.26.1] - 2026-06-04

Phase 18 follow-up: desktop self-update and Linux AppImage fixes.

### Fixed

- Desktop self-upgrade: the updater manifest endpoint was flattened to the
  static `/dl/desktop/latest.json` the release generator actually
  publishes; the previous templated path never matched, so desktop
  self-update always 404'd.
- Linux AppImage: prefer the host GTK/WebKit stack so a host whose Mesa is
  newer than the bundle (e.g. CachyOS) no longer aborts webview creation
  with `EGL_BAD_PARAMETER`.
- Inspector: the workspace-root split action button.

## [v0.26.0] - 2026-06-04

Phase 18: a hybrid-surface bug sweep, the inspector pill redesign, and a
repo/docs consolidation, cut as v0.26.0.

### Added

- Inspector: each item category (File Browser Directory / File / Media /
  Binary, and the editor "Show Details") now shows a single pill for the
  main action plus a dropdown for the secondary actions, replacing the
  flat button stack. "New terminal here" seeds the terminal with the
  relative path after the cursor.
- Editor: `[[` completion now offers local workspace paths, not only
  filename and heading targets.
- File Browser: the tab right-click menu adds "New file or Directory",
  "New Terminal", and "New Graph" (all from the workspace root) below
  "Expand all directories", and shows keyboard-shortcut hints in the
  selection context menu.
- Graph: a "Copy link to graph" right-click action that serializes the
  tab to a `chan://graph?...` link (scope, depth, mode, filters, selected
  node) which can be pasted into a markdown file and clicked to reopen.
- Terminal: context-menu copy/paste chords (Cmd+C / Cmd+V on macOS,
  Ctrl+Shift+C / Ctrl+Shift+V elsewhere so bare Ctrl stays SIGINT).

### Changed

- Editor: bullet and hyphen lists now behave like ordered lists for
  cursor, indent, and clicks; hyphen lists render distinctly again
  (phase-17's glyph change was meant for bullet lists only). Bullet
  markers are now real glyph-character widgets, so CodeMirror handles
  cursor, click, and arrow positioning natively.
- Consolidated `docs/journals` into per-phase `docs/phases/phase-N.md`
  documents and distilled `docs/agents` into a minimal set plus a
  lessons-learned playbook; removed the raw journals, `docs/archive`, and
  related scaffolding.

### Fixed

- Editor: trackpad free-scroll no longer hangs or jumps in the opposite
  direction when the caret is far from the scroll target (removed
  `scroll-behavior: smooth` from the CodeMirror scroller).
- Graph: "Graph from here" now selects the originating node on the
  redrawn graph and persists the selection across a window reload; no
  directory node is plotted without a visible edge back to the workspace
  root; binary files and symlinks no longer render as contact nodes; and
  the graph no longer reloads on every out-of-scope workspace file edit.
- File Browser: directory expand no longer hangs at "Loading" until a
  window reload (a `history.replaceState` SecurityError); hash writes are
  now debounced.
- Terminal: UTF-8 multibyte text renders correctly in `less` and `vim`
  (PTY now spawns with `LANG=C.UTF-8` when the inherited env selects no
  UTF-8 codeset); hiding the rich prompt returns focus to the terminal.
- Inspector: the Drafts graph node and draft files (which live outside
  the workspace tree) now populate the inspector with a single
  Terminal-from-here action.
- Desktop: turning a workspace OFF then quickly ON no longer strands the
  row "ON but no Open"; the toggle is disabled across the start/stop
  transition and `open_workspace` retries on a still-releasing flock.

### Removed

- chan-desktop: the old local-disk New-workspace pre-flight dialog (and
  its now-dead Rust backend); pre-flight moved to the SPA boot menu in
  phase 17.
- File Browser and graph context menus: the "Reload" entry.

## [v0.25.0] - 2026-06-03

Phase 17: a host bug sweep, survey system v2, and a desktop connecting
screen, cut as v0.25.0. The release also carries phase 16's closing-round
desktop launcher redesign, which landed after the v0.24.0 cut.

### Added

- Survey system v2: surveys now reach team-dialog-created terminals, and
  every survey offers options plus an F follow-up plus a Dismiss (with a
  distinct "dismissed" reply so the asking agent can tell). Surveys are
  per-terminal rather than window-wide.
- Desktop: an outbound remote-workspace window that cannot reach its URL
  now shows a connecting screen immediately (spinner, URL, live elapsed
  timer, one timestamped row per retry) instead of a blank white webview;
  the retry loop is page-driven over a Rust `probe_url` IPC. The window
  title shows the workspace kind (home / computer / outbound / inbound)
  plus the locator.
- Desktop launcher redesign (phase 16 closing round): the separate [Open
  workspace] and [Attach] header buttons merged into one [New] modal with
  three choices (Local directory / Remote outbound / Remote inbound);
  remote rows show a connection dot.
- Path autocomplete in the lazy file tree, search, and the image-draft
  save dialog; team-load path autocomplete; a Spawn-agents auto-assign
  button; `cs pane split RIGHT|BOTTOM`.
- About page: open-source attributions (trimmed to a one-line
  free-and-open-source tagline).
- README and home page: a `curl | bash` install plus `chan serve ./repo`
  usage example, plus chan-desktop and `gateway/` self-hosted manuals.
- Editor: files are now editable by content sniff (a `.zshrc` or
  `*.service` opens as text); a serve-progress heads-up so a large
  workspace shows progress before the URL prints.

### Changed

- The rich prompt (Cmd+Shift+P) now acts only on the focused terminal in
  the focused pane rather than toggling on every terminal; survey bubbles
  stay on top.
- MCP env is now off by default per terminal, with a team-config opt-in
  toggle; chan never writes the user's config files for MCP.
- Editor: unordered-list bullet glyphs use the Google-Docs depth-cycle
  look.
- Submit chord derivation refactored to `SubmitAgent::derive(command,
  CHAN_AGENT)` (dropping the stored per-member agent field); submit chords
  are now runtime-overridable.
- Pre-flight bubble: the per-row OFF/ON label-plus-button became a single
  checkmark toggle.
- Release: the CI macOS DMG now uses a Finder-less dmgbuild layout so it
  matches the local layout deterministically.

### Fixed

- `cs terminal write --submit codex` now submits correctly (the write is
  wrapped in bracketed paste, since codex coalesced text and CR into a
  paste burst that ate a bare CR).
- Graph: a fresh Cmd+Shift+M window can now expand directories without a
  "graph from here" first, and keeps its depth slider and non-directory
  layers; a file's language edge refreshes on a bare FSEvents rename.
- Editor: pasting a link into a list no longer indents the list (turndown
  was emitting a stray list marker); Shift-Tab outdents at top level.
- One-shot `cs` commands no longer enter hybrid-nav transaction mode or
  steal focus from the sending terminal.
- `cs` window commands now error cleanly when no window is connected;
  global shortcuts are blocked behind the disconnect overlay; the
  rendered mermaid diagram's right margin is aligned.
- Release: the new dmgbuild DMG is codesigned before notarization (the
  dry-run caught it unsigned before the tag).

## [v0.24.0] - 2026-06-02

Phase 16: lead-orchestration CLI tooling and a long host feature stream
converging on a single per-session input queue, cut as v0.24.0.

### Added

- CLI: `cs terminal scrollback` (read a tab's scrollback by name) and
  `cs pane` (query windows/panes/layout/selected pane; set focus; split
  left/bottom; close tab/all-tabs/pane, with `--force`), over a new
  bidirectional control-socket channel. SPA-visible CLI team spawn.
- A per-session FIFO write queue with idle-drain that serializes all
  terminal and agent input (control-socket writes, Rich Prompt, Team
  Work).
- Mermaid: cursor-based render (no flip button), horizontal flip, up/down
  step-in, reverse-flip symmetry, visible selection inside code blocks,
  and error line/column locatability.
- Image viewer prev/next navigation; a live source-row indicator for the
  image drag-to-move.
- Per-workspace directory blocklist (global baseline plus per-workspace
  additions) with a File Browser settings UI.
- Pre-flight: a non-blocking check that the `cs` symlink exists in `$PATH`
  (offers to create it, continues if it cannot); a first-load onboarding
  card; Reports on by default.
- Dashboard: a carousel navigator, a real-engine screensaver preview
  (shown inside the Screen-lock box only when locked).
- Graph: lens plots (language, hashtags, mentions) now draw the directory
  spine back to the workspace root, leaving no edgeless file node.
- Editor: external-link "open" affordance and internal markdown-link
  previews; body context menus (Cut/Copy/Paste/Find/split).
- Docs: a gateway self-host guide and a Terminal manual page (the `cs`
  family, pokes, survey, MCP).

### Changed

- Tunnel/gateway messaging reframed: the tunnel is a core chan
  capability, and the `gateway/` online service is experimental, off by
  default, and meant as a self-hosted offering.
- `cs terminal team load` now resolves paths cwd-relative and actually
  spawns the team instead of only summarizing it.
- Terminal context menus made contextual on right-click; agent terminals
  now carry `CHAN_WINDOW_ID` so `cs pane` / `cs open` / `cs survey` can
  target a window from an agent context.
- Rich Prompt returned as a floating Cmd+Shift+P bubble, then
  re-architected to be Drafts-backed with editor-style image paste (paste
  writes real files any agent can read via MCP).
- CI: bumped the Node-20 GitHub Actions to Node-24 majors ahead of the
  2026-06-16 deprecation.

### Removed

- The in-terminal Team Work bubble (the lead is now a normal terminal;
  identity flows through the queue).

### Fixed

- Cross-window editor-tab drag-drop no longer loses the tab on drop.
- Terminal names are enforced unique (auto `-N`) on create and rename;
  Alt+Shift+[/] reaches tab navigation instead of the PTY.

## [v0.23.0] - 2026-06-01

Phase 15 round 4: desktop and release engineering, native macOS
Export-to-PDF, and semantic-search gating, cut as v0.23.0.

### Added

- Native macOS Export-to-PDF via the print pipeline (paginates and honors
  `@pagebreak`); the button is hidden on Linux.
- Linux chan-desktop builds for ubuntu, fedora, and arch (AppImage / .deb
  / .rpm on amd64 and arm64) plus the gateway .deb packages, all built
  from a macOS host via sdme/lima; a static-musl standalone Linux `chan`
  CLI; a multi-arch desktop CI matrix.
- `cs terminal team new|load --script` as the CLI equivalent of the Cmd+P
  team dialog, with server-side lead-first spawn.

### Changed

- Semantic (hybrid) search is now requested only when `semantic_enabled`
  is on and the model is present, instead of building vectors on every
  reindex but never querying them.
- The indexing spine pulses orange during the background embed sweep.
- Unified the favicon to the orange transparent enso across all chan
  sites.
- Only Markdown counts as a graph document; `.txt` stays searchable text
  but is no longer a graph node.

### Fixed

- Desktop no longer crashes when closing a window whose navigation
  failed.
- Silenced tokei "Unknown extension" log spam.

## [v0.22.0] - 2026-06-01

Phase 15 round 3: Team Work moved into the workspace, the survey rebuilt,
the `chan-shell` crate, relative-markdown links, and a BM25 improvement,
cut as v0.22.0.

### Added

- `[[` completion writes relative markdown links on disk (not wiki links),
  with heading `#` and block `^` anchors and click-to-place-caret;
  relative-link pills are openable.
- A `chan-shell` crate so `chan` and `chan-desktop` share the `cs` client,
  plus a per-agent submit-encoding map; `cs terminal survey` exposes its
  wire JSON in `--help`.
- The Team Work survey rebuilt for real (overlay, reply round-trip, `[F]`
  follow-up file) with a per-member agent field; desktop `cs`.

### Changed

- Team Work config moved from a `/tmp` path into the workspace under a
  user-chosen `{team-name}/` directory.
- BM25 now matches @@mentions, paths, and filenames via a subtoken split.
- Halved the embed batch size to shorten the in-flush chip freeze.

### Fixed

- Pre-flight no longer re-locks the boot overlay on an incremental
  reindex (the session-crashing RELOAD-HANG); only a cold cold-build
  locks.
- The background-embed chip survives a watcher reindex.
- Graph: dropped ghost nodes for unresolved link targets; the anchor
  joins the edges primary key so multi-anchor links survive.

### Removed

- The in-terminal `chan open` command (superseded by desktop `cs` and the
  OS file association); the dead Team Work bubble stub; the embeddings row
  from the Dashboard About card.

## [v0.21.0] - 2026-05-31

Phase 15 round 2: the dropped round-1 Dashboard items, terminal UX,
the `cs` rename, Team Work self-start, and the indexing rework, cut as
v0.21.0.

### Added

- Dashboard: the Search-slot directory inspector actions (Show Directory /
  Graph from here / New Terminal), per-tab carousel autoRotate, and the
  remaining part-1 items (license placement, screensaver preview).
- Terminal: clickable URLs; a Shift+Enter LF newline fallback while an
  agent is running.
- `cs` renamed from `cs term` to `cs terminal`, with subcommand
  prefix-matching, `cs terminal restart`, `cs search`, and `cs dashboard
  --carousel-off`; team terminals join the team tab group;
  `cs terminal write --submit` appends the agent submit chord.
- `chan open` as the OS file-association entry (desktop).

### Changed

- Indexing: pre-flight now unblocks on BM25-ready and embeds in the
  background instead of a synchronous embed pass that wedged boot;
  workspaces over the file cap skip embeddings; the background-embed chip
  advances per file.
- Editor: Cmd+R remapped to Ctrl+Shift+R off macOS so bash reverse-search
  survives.

### Fixed

- True two-face CSS card flip for Cmd+, (the old keyframe raced focus and
  only fired once focus left the pane).
- Conceal marks re-decorate on a tab-switch remount.
- Editor focus follows the active pane; the indexing graph survives a
  pane flip.
- The team lead launches its agent via the worker spawn path
  (TEAM-SELFSTART).
- `cs search` renders snippet highlights as markdown bold.

## [v0.20.0] - 2026-05-31

Phase 15 round 1: the Dashboard carousel redesign, the Search cleanup,
and the `cs` shell surface, cut as v0.20.0.

### Added

- Dashboard: a controlled carousel with per-slot front and back surfaces
  (About / Workspace / Search), a relabeled Search slot with a
  conditional legend, and a shared matrix-rain screensaver preview.
- A `chan shell` subcommand with `argv[0]=="cs"` dispatch so a `cs`
  symlink works directly (open / graph / term / term-write / dashboard).
- Terminal tab groups (`$CHAN_TAB_GROUP`) so Cmd+Shift+I broadcast is
  group-scoped.

### Changed

- Search is now workspace-wide.

### Removed

- The Search SCOPE selector, the SEARCH STATUS button, and the search
  status overlay (and the dead scope/overlay code they orphaned).

## [v0.19.1] - 2026-05-30

Phase 14 patch: Cmd+, pane-flip guarding and editor focus.

### Fixed

- Cmd+, pane flip is now guarded behind every over-pane modal (it no
  longer fires while an overlay or modal owns the keyboard).
- Editor focus follows the active pane, and the indexing graph survives a
  flip.

## [v0.19.0] - 2026-05-30

Phase 14: the gateway monorepo migration with the drive-to-workspace
rename, a frontend pristine cleanup for the first public release, paced
graph delivery, and the new-workspace pre-flight, cut as v0.19.0.

### Added

- Cursor-paged `/api/fs-graph` delivery: opt-in via `limit`, resumed via
  an opaque `cursor`, bounded DFS batches (at most 256 nodes / 64 KiB),
  scope-bound rejection. The frontend consumes it incrementally, yielding
  a frame between batches. The whole-scope path stays byte-identical.
- New-workspace pre-flight: a `GET /api/preflight` poll plus
  `POST /api/preflight/decision`, derived from live indexer state, shown
  on a locked overlay (no close button, ESC ignored) until completion, so
  local and remote workspaces share one flow.
- A depth-slider "next degree, not a re-walk" primitive.
- The chan.app gateway (account, sign-in, reverse-proxy) brought into the
  repo as a nested Cargo workspace (NOT a member of the root workspace, so
  the core build stays Postgres-free), with its own CI gate and four .deb
  packages wired into the release flow.

### Changed

- The tunnel domain is now `workspace.chan.app` (previously
  `drive.chan.app`); tunnel mode dials `workspace.chan.app/v1/tunnel` and
  publishes at `{user}.workspace.chan.app/{workspace}/`. Applied across
  the chan client default, the chan-tunnel-* crates, chan-server, the
  desktop shell, the manual, and the marketing copy.
- The `drive` to `workspace` rename applied across the gateway suite:
  `drive-proxy` is now `workspace-proxy`; the `workspace_gate` cookie, the
  `workspace.chan.app` host, the `/api/workspaces/*` routes, the
  `WORKSPACE_*` env vars, and the `workspaces` / `workspace_grants`
  tables. Single-source domain config derives every host from
  `CHAN_DOMAIN` plus `PUBLIC_SCHEME`.
- Graph directory nodes expand/collapse in place on double-click, with the
  expanded set persisting across a window reload; the old "graph from
  here" double-click rescope was dropped (rescope stays in the inspector).
- Reviewed the frontend comments, documentation, and user-facing copy so
  they read as a present-state snapshot rather than a development history:
  the editor design note now describes the current CodeMirror 6 editor,
  stale `chan-core` references were corrected, and user-facing strings
  were normalized to ASCII typography.

### Fixed

- False "unsaved changes" banner: a per-page-load `SESSION_ID` plus an
  mtime-stale guard so own-session edits never raise the banner while a
  genuine crashed session still recovers.
- The `/dl` circular 404: the listing regenerates from the latest GitHub
  Release instead of self-fetching the live site.
- The gateway `configure.sh` `install /dev/stdin` write over an existing
  file.
- De-flaked three indexer/PTY tests with capability-gated skips rather
  than bigger timeouts.

### Removed

- The vestigial `team-work-N` draft convention.

## [v0.18.0] - 2026-05-29

Phase 13 round 2. Builds directly on the v0.17.0 cleanup below.

### Changed

- Renamed the "Rich Prompt" feature to "Team Work" across the UI and the
  code: the chord id (`app.terminal.richPrompt` -> `app.terminal.teamWork`),
  the component, CSS, the tab field + its session serialization, and the
  backend draft convention all moved to team-work. Cmd+P now instantiates
  a Team Work lead terminal with an embedded editor first, then the
  Spawn-agents dialog over it (Cancel deletes the lead tab; Bootstrap
  runs the lead-first bootstrap).
- Editor list markers render in a new style: en-dash for `-`, a filled
  circle for top-level `*` and a hollow circle when nested; ordered lists
  keep the source numbers. Source bytes are unchanged.
- Dashboard moved off Cmd+I so the editor can use Cmd+I for italic;
  Dashboard stays on Hybrid Nav (`Cmd+. i`) and the hamburger.
- Hamburger split-right / split-bottom rows show the direct `Cmd+/` and
  `Cmd+?` chords instead of the Pane-Mode prefix.
- Cmd+, pane flip is strictly per-pane: only panes with at least one tab
  can flip, focus changes never flip other panes, and the flip persists
  across window reloads.

### Added

- Editor Bold (Cmd+B) and Italic (Cmd+I) chords.
- Desktop: Cmd+Shift+N opens a new window of the currently focused
  workspace (previously the workspace picker).

### Removed

- The filesystem-watcher agent-event coordination backend (the event
  watcher, the event-reply / submit-mode endpoints, the Rich Prompt
  workspace archival + spool, and the orphaned team name-registry API)
  and the Spawn-agent(s) dialog/process. The notification bubble overlay
  is reduced to a frontend-only static stub; equivalent functionality is
  planned to return in a later phase.

## [v0.17.0] - 2026-05-28

Phase 13 round 1: a broad cleanup-and-polish pass across the graph,
dashboard, editor, inspector, and pane chrome. The round-2 Team Work and
editor work builds directly on this foundation.

### Added

- Graph KIND lenses: clickable path / tag / contact / language chips in
  the inspector open a focused subgraph; the tag and contact lenses walk
  a bidirectional BFS so backlinks are included.
- Dashboard (renamed from Infographics): About, Workspace-info, and a
  read-only indexing-graph widget, plus a per-surface settings flip-back.
- Editor `@`-completion surfaces the `@@mention` corpus; a language-bubble
  inspector body.

### Changed

- Renamed Infographics -> Dashboard across labels, menus, and aria text.
- Cmd+, now flips the focused Hybrid surface's config view; the global
  SettingsPanel overlay was retired.
- Inspector: the workspace root reads like a directory; absolute path +
  a COPY button.

### Fixed

- Editor: new-document cursor focus; the fresh-draft "Unsaved changes"
  prompt no longer fires on a pristine draft; list markers preserve the
  authored character (a hyphen stays a hyphen, `*` stays `*`, ordered
  numbers stay numbers); terminal Shift+Enter inserts a newline.
- Pane: focus-ring thickness parity + an outer-halo focus wobble.
- Indexing graph: fit-on-resize, a working depth slider, double-click
  "graph from here", and clearer embedding-phase progress.

### Removed

- The empty-pane right-click context menu; its spawn entries (now
  including Search + Dashboard) live on the single pane hamburger.

## [v0.16.0] - 2026-05-27

Phase 12 release. The headline is a breaking terminology rename from
"drive" to "workspace", plus graph and File Browser carryover,
cross-platform keyboard shortcuts, terminal robustness fixes, and editor
changes. Supersedes the 0.15.x line: `chan upgrade` only offers 0.16.0+.

### Changed

- BREAKING: renamed the "drive" concept to "workspace" across the crate
  (`chan-drive` -> `chan-workspace`), the on-disk registry, the HTTP
  routes, the CLI subcommands, config, and error text. Clean break with
  no migration: existing registries and bookmarks stop resolving. Delete
  the prior state directory and re-register your workspaces. The
  `drive.chan.app` tunnel domain is the one preserved "drive" string.
- Editor: stopped auto-reloading a file while you are typing; an external
  change now shows a "changed on disk" banner instead of replacing the
  buffer.
- Moved "Export to PDF" from the editor menu into the Inspector.

### Added

- Graph: workspace root pinned at the bottom with the spine growing
  upward (GI-10); an in-flight-index loading state that pulls back
  dead-ends while the index builds; right-click opens the tab menu
  anywhere on the canvas.
- File Browser: per-instance tree expansion state.
- Cross-platform keyboard policy across web, Linux desktop, and macOS
  native, plus Cmd+Shift+I to toggle broadcast to all terminals on macOS.
- Terminal: pulse the unseen-output dot while output arrives.
- Editor: drag an image embedded in a row to move the whole row.

### Fixed

- Terminal: recover the renderer after macOS sleep/wake; harden blur
  repaint for the WKWebView pane focus-switch.
- Editor: flush the caret to the URL hash on reload so Cmd+R restores the
  cursor position.
- Server: close the self-write suppression race that surfaced phantom
  external edits.
- Release/CI: emit the Linux .rpm into the workspace target dir so the
  release workflow stages it, gate vitest in the CI build, and fix a
  flaky unhandled rejection from the debounced workspace-info refresh.

## Pre-v0.16.0 (prototyping)

Versions before v0.16.0 were pre-release prototyping. chan has not made
an official public release yet; the early development logs, files, and
tags were cleaned up, so those versions (roughly v0.6.x through v0.15.x)
carry no tags in this repository and are not detailed here. Their history
lives in the per-phase reports under `docs/phases/`.

[Unreleased]: https://github.com/fiorix/chan/compare/v0.33.0...HEAD
[v0.33.0]: https://github.com/fiorix/chan/compare/v0.32.0...v0.33.0
[v0.28.1]: https://github.com/fiorix/chan/compare/v0.28.0...v0.28.1
[v0.28.0]: https://github.com/fiorix/chan/compare/v0.27.1...v0.28.0
[v0.27.1]: https://github.com/fiorix/chan/compare/v0.27.0...v0.27.1
[v0.27.0]: https://github.com/fiorix/chan/compare/v0.26.2...v0.27.0
[v0.26.2]: https://github.com/fiorix/chan/compare/v0.26.1...v0.26.2
[v0.26.1]: https://github.com/fiorix/chan/compare/v0.26.0...v0.26.1
[v0.26.0]: https://github.com/fiorix/chan/compare/v0.25.0...v0.26.0
[v0.25.0]: https://github.com/fiorix/chan/compare/v0.24.0...v0.25.0
[v0.24.0]: https://github.com/fiorix/chan/compare/v0.23.0...v0.24.0
[v0.23.0]: https://github.com/fiorix/chan/compare/v0.22.0...v0.23.0
[v0.22.0]: https://github.com/fiorix/chan/compare/v0.21.0...v0.22.0
[v0.21.0]: https://github.com/fiorix/chan/compare/v0.20.0...v0.21.0
[v0.20.0]: https://github.com/fiorix/chan/compare/v0.19.1...v0.20.0
[v0.19.1]: https://github.com/fiorix/chan/compare/v0.19.0...v0.19.1
[v0.19.0]: https://github.com/fiorix/chan/compare/v0.18.0...v0.19.0
[v0.18.0]: https://github.com/fiorix/chan/compare/v0.17.0...v0.18.0
[v0.17.0]: https://github.com/fiorix/chan/compare/v0.16.0...v0.17.0
[v0.16.0]: https://github.com/fiorix/chan/releases/tag/v0.16.0
