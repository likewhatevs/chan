# Phase 20 - chan-desktop refinements + standalone terminal windows

Status: closed. Merged to main and released in v0.29.0, together with
        phase-21, which builds on and completes this work.
Span: 2026-06-09 to 2026-06-10.
Versions: v0.29.0 (Part A + Part B cut together with phase-21; no separate
          point release).
Tags: #desktop #terminal #about #updater #orchestration #docs

## Roadmap (the asks)

A single brainstorm note (`dev/phase20/brainstorm/desktop.md`) defined three
chan-desktop asks - two small refinements and one substantial feature - none of
which were to change existing chan / chan-desktop behavior:

**Update notification.** The in-app "Chan Desktop update" dialog concatenated
the GitHub release body, which a native dialog renders as literal markdown
(`**Full Changelog**: https://...` shown raw, link not clickable). Replace it
with plain text plus a single changelog link.

**Unified About.** The About window differed by platform - Linux showed a
custom dialog with "Check for updates" + "OK" buttons; macOS used the system
About panel. Unify both on one About that shows the same information as the
in-app Dashboard / About page.

**Standalone terminals.** Add File ▸ New Terminal (Cmd+T) opening a new window
that holds only a terminal, with no workspace. These windows can split panes
and use Hybrid Nav (minus the o/p/g/n/f staging), configure the terminal via
the Cmd+, tab-flip, and keep broadcast + shortcuts - but no rich prompt and no
team work. Cmd+T from such a window adds a tab; Cmd+Shift+N opens another
terminal window.

## Rounds and waves

The round opened with an architect planning pass: explore the desktop window /
menu / shortcut system, the update + About surfaces, and the terminal/server
coupling, then choose an orchestration model (see Team and coordination) and a
sequencing - refinements first as a quick point release, then the feature.

### Part A - refinements (solo, branch `phase-20-refinements`)

- **Update dialog** (`desktop/src/main.js`): dropped the raw `update.body`;
  the prompt now shows plain text plus a `compare/<prev>...<new>` changelog URL
  (falling back to the release-tag page when the installed version is unknown).
- **Unified About**: deleted the Linux-only "Check for updates"/"OK" dialog and
  its now-dead manual-update Rust path; added a bundled About webview
  (`desktop/src/about.{html,css,js}` + the donation QR + an `about.json`
  capability) opened by a `chan-about` menu item on both platforms. On macOS
  the system About item is stripped and redirected to this window. Content
  mirrors the Dashboard About slide; the version is passed as a `?v=` query
  param so the page needs no `app`-plugin capability, and external links route
  through the opener plugin.
- A later request dropped the third-party font + screensaver attributions from
  both About surfaces (the bundled window and the Dashboard slide), which
  cascaded into the carousel source-pin tests.

### Part B - standalone terminal windows (worktree off origin/main)

Built in an isolated worktree (`worktree-phase-20-terminals`) so it could land
while Part A's release was in flight. The orchestrator fixed the cross-surface
wire contract first, then fanned out one subagent per surface against it.

- **Lane S - chan-server.** A workspace-less "terminal tenant" the embedded
  multi-tenant `WorkspaceHost` mounts under its own `/terminal-<seq>` prefix.
  `open_terminal_session` mirrors `open_workspace` but builds a
  `workspace_cell: None` `AppState` (no watcher / indexer / MCP bridge / control
  socket; PTY cwd = `$HOME`) and a SLIM router: terminal (ws + CRUD + restart),
  per-window session, build-info, health, config, and `/ws`, plus the SPA
  shell. Workspace-content routes are absent, so a stray request 404s instead
  of panicking on the `None` cell. The existing `test_support::make_test_state`
  (which already builds a `None`-cell `AppState`) was the template.
- **Lane R - desktop.** `EmbeddedServer::open_terminal` mounts the tenant and
  the window loads the SPA with `&kind=terminal`. An always-enabled File ▸ New
  Terminal (Cmd+T) routes by focused-window kind: launcher → new terminal
  window; any embedded SPA window → dispatch `app.terminal.toggle` (so a
  workspace window keeps toggling a pane and a terminal window adds a tab).
  Cmd+Shift+N from a terminal window opens another; closing one tears down its
  tenant prefix. `terminal-*` joins the workspace capability.
- **Lane W - web.** A terminal-only surface on `?kind=terminal`: bootstrap
  skips `/api/workspace` and the pollers behind a `ui.terminalOnly` flag,
  restoring layout from `/api/session`. The existing pane/tab/Hybrid-Nav
  components are reused with the o/p/g/n/f staging, rich prompt, team work, and
  the terminal New File / Browser / Graph / MCP-env actions gated off.

The orchestrator then closed the loop: a per-window `Library`-leak fix, mounting
`/api/config` on the slim router and repointing the Cmd+, terminal-config form
to read/write it (so terminal config persists with no `workspace.info`), then a
whole-workspace gate.

## Team and coordination

The round's defining decision was the orchestration model. Prior phases ran N
dedicated lane agents plus an architect coordinating via task files, pokes,
gates, and surveys - the right shape for broad, independent bug-sweeps. Phase-20
was the opposite shape: one tightly-coupled feature (terminal windows spanning
chan-server + desktop + web on a shared wire contract) plus two small
independent refinements. So it ran as a single orchestrator spawning subagents,
not N standing lanes:

- The orchestrator did Part A solo and owned the Part B wire contract
  (`dev/phase20/contract.md`: the `/terminal-<seq>` window kind, the
  workspace-less serve mode, the `?kind=terminal` URL signal, one-owner-per-
  crate) and all the integration.
- Three subagents implemented the disjoint surfaces against the contract -
  Lane S and Lane W in parallel (Rust vs TS, no shared compile), Lane R after
  Lane S's API existed. Each gated its own crate (`cargo`/`make web-check`)
  before reporting.
- The orchestrator ran the whole-workspace integration gate (`fmt`, `clippy
  -D warnings`, `test --all-targets`, `web-check`).

Part A landed as three commits on `phase-20-refinements`; Part B as three
surface-focused commits (server → web → desktop, for bisectability) on the
worktree branch.

## What shipped, tried, and undone

**Shipped (gated, not yet released).** Part A: the plain-text update prompt and
the unified About window (both platforms), plus the attribution removal from
both About surfaces. Part B: standalone terminal windows end to end - the
workspace-less server tenant, the desktop window/menu/teardown wiring, and the
terminal-only SPA surface, with Cmd+, terminal config persisting via
`/api/config`.

**Tried then corrected.** The first exploration claimed the terminal coupled to
`/api/workspace/terminal/<id>/ws`; verifying against source showed the routes
are session-id keyed (`/api/terminal/ws` + `POST /api/terminals`), which is what
made the workspace-less tenant tractable. The Cmd+T menu conflict (it already
toggles a pane in workspace windows) was first scoped as a dynamic
enable/disable on focus, then replaced with a single always-enabled handler
that routes by focused-window kind - more robust, and it preserves workspace
Cmd+T exactly. Lane S's first cut leaked a throwaway `Library` per window and
left the Cmd+, config form inert (no `workspace.info`); both were closed in
integration.

**Resolved in phase-21.** The desktop GUI smoke (Tauri can't be driven headless
in-session) cleared on the human's pass against the rebuilt signed `Chan.app`,
and the release cut happens there as v0.29.0. The full pre-push's
`gateway-build` + `web-marketing-check` (unaffected by this work) run at push
time.

## Retrospective

**Highlights.**
- Contract-first parallelism paid off: fixing the wire contract before fan-out
  let Lane S (Rust) and Lane W (TS) run concurrently with no integration drift,
  and the whole workspace compiled + gated green on first assembly.
- Reuse over new code: the workspace-less tenant rode the existing multi-tenant
  `WorkspaceHost` + per-tenant router rather than a new server; the
  `None`-cell `make_test_state` was the AppState template; the terminal routes
  were already session-id keyed.
- The focused-window-kind menu routing dissolved the Cmd+T conflict without the
  fragile "does a disabled menu item pass its accelerator through" assumption.
- Grounding descriptions in source caught the terminal-route error before it
  shaped the design.

**Lowlights / contention.**
- No GUI smoke possible in-session: the riskiest parts (macOS menu insert,
  Cmd+T pre-emption, tenant teardown) are correct-by-construction but
  empirically unverified - flagged for a human pass.
- The About attribution removal sprawled into multiple source-pin tests
  (`dashboardTabAndCarousel`, `sourceCodeProTogglePlacement`); the first grep
  missed `sourceCodeProTogglePlacement` because it didn't grep every literal
  form ("Source Code Pro Regular" vs "SIL OFL") - the grep-all-literals lesson,
  again.
- Part A and Part B both edited `dashboardTabAndCarousel.test.ts` on separate
  branches; their merge to main will conflict there (small, known).

**Lessons worth carrying forward.**
- Match the orchestration shape to the work: one coupled feature wants an
  orchestrator + subagents on a fixed contract, not N standing lanes.
- Fix the cross-surface wire contract before fan-out; gate per-crate in the
  lanes, then once whole-workspace at integration.
- When removing source-pinned UI text, grep every literal form and re-run
  `web-check` before assuming the source-pins are covered.

## Notes

Both branches merged to main: `phase-20-refinements` (Part A + the About
removal) and `worktree-phase-20-terminals` (Part B, off origin/main). The Part B
wire contract lived in `dev/phase20/contract.md` (gitignored scratch). Closed
and released as v0.29.0 alongside phase-21, which extends Part B's shared
terminal tenant with cross-window awareness.
