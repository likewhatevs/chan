# v0.65.0

v0.65.0 makes the command launcher configurable and makes Settings the one place
for interactive configuration. It adds per-OS shortcut assignment, restores a
small set of default chords, moves workspace-specific controls into a conditional
"This workspace" Settings tab, removes duplicated back-of-pane config controls,
and finishes the late workspace polish around A/B pane sides, graph links, and
close shortcuts. Feature range `d53467cd..85830b76` on `devserver-access`,
folded from the rc-pinned `config-cleanup-v0650` stream. GA cut target:
2026-07-06.

## The round

The main delivery was a shared-worktree, multi-lane round coordinated through
`dev/v0.65.0/`:

- Lead: App.svelte integration, release and gate coordination, shortcut dispatch
  seams, the Settings mount, the CI/release workflow parallelization, and the
  changelog/report closeout.
- Config-Web: the launcher-reachable Settings surface over the split config
  store, live save-as-you-go PATCH calls, and the Keyboard Shortcuts assignment
  UI.
- Keymap: the per-OS shortcut override model, conflict detection, writable
  `chordFor`, the generated shortcut tables, and native key bridge updates.
- Server: the config wire for shortcut overrides, the notify watcher for
  external config edits, and the config-shape confirmation.
- Shell: the launcher redesign, Apps/Tabs catalog split, empty-pane surface,
  pane hamburger, dashboard jump commands, CWD-copy fix, and editor bug fixes.

The rc cycle reached `0.65.0-rc4`. After rc4 smoke accepted the Settings
workspace tab and native menu realignment, the closeout removed duplicated config
controls from pane backs. Later host follow-ups restored the Settings/Search
default shortcuts, landed A/B pane sides with tab movement, improved the flip
animation, fixed graph expansion/spine regressions, let `cs open` consume
`chan://graph?...` links, and added the amber side-button flash for blocked close
shortcuts.

## What shipped

- **Settings as the configuration surface.** The launcher opens a Settings form
  over the per-library config with Appearance, Editor, Terminal, Files & search,
  and Keyboard Shortcuts sections. Edits persist as single-field changes and
  refresh live windows. Workspace windows also get a conditional "This workspace"
  tab for index rebuild, semantic search, embedding model, excluded directories,
  reports, metadata archive, and screen lock.
- **Per-OS shortcut assignment.** Every launcher command is rebindable from
  Settings, stored per platform, resolved by the running client, and shown in the
  launcher and shortcut tables. A chan-desktop override applies locally and to
  devservers it opens; browser clients use the web set.
- **A trimmed but useful default keymap.** Opinionated spawn and pane chords were
  dropped because they are now assignable. Defaults stay for Settings
  (Cmd/Ctrl+,), Search (Cmd+Shift+S on macOS, Ctrl+Alt+S elsewhere), launcher
  (Cmd+K on macOS, Ctrl+Alt+K elsewhere), close tab (Cmd+W on macOS and Ctrl+D
  everywhere), plus universal editor/browser conventions. Close window, new
  terminal, reopen tab, and Rich Prompt were realigned across the SPA, desktop
  key bridge, and native menus.
- **A spotlight launcher and cleaner catalog.** The launcher opens as a centered
  command palette over a plain dark scrim. It keeps the full catalog browsable,
  pins the active surface first, splits spawn commands into Apps and tab commands
  into Tabs, and adds Reload, Open Inspector, and dashboard jump commands.
- **Back-of-pane config duplicates removed.** Editor, Terminal, and File Browser
  backs are shell-only flip backs with OK. Graph keeps only its read-only node
  colour legend. Dashboard keeps the slot navigator and Workspace recents. All
  interactive config lives in Settings.
- **A/B pane sides.** Each pane has side A and side B tab sets. Tabs can move
  between sides, the side glyph flips A/B, Hybrid Nav and tab commands respect
  the visible side, and keep-alive behavior remains scoped to visible active
  tabs.
- **Better pane flipping.** The side flip uses a real 3D card effect, chooses the
  flip axis from the pane shape, and keeps close-pane and close-tab chords
  non-editable but functional.
- **Close shortcut feedback for hidden-side tabs.** Pressing Ctrl+D, Cmd+W, or
  Cmd+Shift+W on an empty visible side now keeps the pane/window open when the
  other side still has tabs, and flashes the A/B button amber to explain why.
- **Graph and shell link closeout.** Graph expansion keeps selected nodes in
  view, directory-scoped graph spines keep their ancestor edges, and `cs open`
  now accepts the same `chan://graph?...` links that the editor already opens,
  creating a new graph tab through the existing parser.
- **Workspace and editor fixes.** Empty single-pane workspaces show the absolute
  path without action buttons. The pane menu says "Hybrid Nav" under Commands.
  Tab labels fade only when they overflow. The CWD-unavailable and copy-failed
  status pills are dismissable; Copy path to `$CWD` writes the absolute working
  directory; image paste keeps list continuation; editor image copy copies
  markdown; reopening a deleted draft opens a fresh draft; the stale selection
  band clears.
- **Security and release infrastructure.** Devserver tunnel writes gained
  double-submit CSRF protection, tighter origin/session checks, 0600 IPC sockets,
  and gateway identity assertion pinning. CI and release workflow macOS validation
  now fan out in parallel with Linux and Windows.

## Validation

The rc4 full `make pre-push` gate was green at `29f61272`, covering Rust fmt,
clippy with `-D warnings`, Rust tests, no-default-features build, the separate
gateway workspace, web checks, SPA production builds, and marketing checks.
Earlier rc gates were green at rc1, rc2, and rc3. The post-rc4 closeout was
covered with focused Rust and workspace-app checks for Settings/Search defaults,
A/B side behavior, pane close chords, graph expansion/spines, and `cs open`
graph links, followed by workspace-app `svelte-check`.

The final GA procedure remains the release-skill sequence: push the accepted rc
state to `origin/main`, strip every `0.65.0-rc4` pin to `0.65.0` in one GA
version commit with the dated changelog section, run the full local gate,
dispatch `release.yml` with `publish=false`, then tag `v0.65.0` on the GA commit
and push that tag.

## Carryover

- The desktop reconnect fix stays deferred. rc3/rc4 smoke accepted the current
  reconnect behavior, and the suspected gateway/WKWebView path needs live desktop
  confirmation before changing it.
- No rc tag was pushed. rc states were pins only, so no prerelease could update
  live `latest.json`.
