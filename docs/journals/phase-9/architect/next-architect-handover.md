# Next Architect Handover

Date: 2026-05-24
Owner: @@Architect
Status: copy-paste startup prompt for the next Phase 9 architect session

## Startup

You are @@Architect in a new Phase 9 session for `chan`.

Read first:

- `~/.ai/profile.md`
- `AGENTS.md`
- `docs/journals/phase-9/request.md`
- `docs/journals/phase-9/roadmap-round1.md`
- `docs/journals/phase-9/rich-prompt-revamp.md`
- `docs/journals/phase-9/architect/draft-workspaces.md`

Do not use `./docs/agents/bootstrap.md` unless @@Alex explicitly asks for
that process.

Baseline before this handover document: local `main` matched `origin/main` at
`d2841c0 Archive Phase 9 workflow journals`.

## Current State

Recent important commits:

- `d2841c0` archived Phase 9 journals, Rich Prompt design, screenshots, and
  agent reports.
- `5fbb6c0` hardened server state/file routes and recorded Codex MCP probe
  evidence.
- `a210034` kept bare markdown `---` visible as source text instead of hidden
  horizontal-rule rendering.
- Drafts lifecycle work landed: Drafts are hidden from File Browser, exposed
  through MCP/editor/terminal, promoted with no-clobber semantics, discarded
  into metadata trash, and checked at boot for broken workspaces.
- Metadata import/export, semantic model picker, Matrix lock fixes, screen-lock
  auto-lock, and server sync-route hardening have landed and passed live checks.

Incoming page-break work:

- Treat page-break as incoming until its owning agent lands the doc/commit.
- At this handover, page-break work exists locally but is not committed:
  `web/src/editor/commands/page_break.ts` is untracked, and there are unstaged
  modifications in `web/src/editor/Wysiwyg.svelte` and
  `web/src/editor/bubbles/triggers.ts`. Do not overwrite them. Confirm
  ownership before staging or modifying them.
- The file currently defines `PAGE_BREAK_MARKER` as
  `<hr class="chan-page-break">`, expands `@pagebreak` / `@break`, and renders a
  CodeMirror widget labelled `Page break`.
- The WYSIWYG integration currently imports the page-break extension, wires it
  into editor decorations and Space/Enter macro expansion, and reserves
  `@pagebreak` / `@break` from the contact bubble trigger.
- Required evidence after page-break integration: editor unit tests plus live
  Browser/iab verification for source mode, WYSIWYG mode, save/reopen, reload,
  and interaction with the existing bare `---` source-visibility fix.

## Two Architect Experiment

@@Alex wants the next session run as a two-Architect experiment. Use a
Core/Web split.

Core Architect owns:

- chan-drive, chan-server, MCP, terminal/session lifecycle, event watcher,
  Drafts metadata safety, filesystem failure modes, API boundaries, and tests.
- The policy for missing, chmod-broken, unsafe, or deleted draft/spool/event
  directories while Rich Prompt is active.
- The rule that hazardous watcher/draft states never fail silently.

Web Architect owns:

- Svelte UI, Rich Prompt UX, Team Work / Spawn Agents workflows, desktop-facing
  behavior, Browser/iab verification dispatch, accessibility, and reload/window
  lifecycle.
- The Rich Prompt state machine and visible user feedback.
- Page-break UI integration after the page-break owner lands the source.

Each Architect may spawn subagents as needed. Keep ownership clean. Do not let
both Architects assign edits to the same files without explicit coordination.

End the session with a survey:

1. What did each lead own?
2. Which decisions were blocked or duplicated?
3. Which interfaces between lanes were clear?
4. Which handoffs caused delay?
5. Which tests gave real confidence?
6. What should change before running three leads?
7. Should the next split be Backend/Core, Web/Product, Desktop/Platform, or a
   different charter set?

## Rich Prompt Lane

Plan this as a real workflow lane, not release polish.

Target behavior from @@Alex:

- Cmd+P / Cmd+. P always creates a new Terminal with Rich Prompt wired in.
- Rich Prompt is draft-backed and terminal-owned.
- A Rich Prompt draft directory contains `draft.md` plus `spool/`.
- `spool/` contains `process.md`, `events/`, `journals/`, and `tasks/`.
- The terminal watcher attaches to `spool/events/`.
- Closing the terminal tears down the Rich Prompt: notify user, close shell,
  stop watcher, move the whole draft workspace to metadata trash, then clean up
  UI.
- Terminal rename/restart must not break watcher identity.
- Focus starts in the editor, not the terminal.
- Submit sends the full editor buffer to the terminal, adds a final newline if
  needed, and uses agent submit behavior when appropriate.
- After submit, move the old prompt into `prompt-N.md` and present a fresh
  blank `draft.md`.
- "New Team" becomes "Spawn agents", min 1 max 9, with copy/paste config,
  preflight, survey confirmation, then broadcast identity/process prompt.

Default failure policy unless @@Alex changes it:

- Stop the watcher when its directory becomes unreadable, missing, or unsafe.
- Surface a persistent status or warning tied to the affected Rich Prompt
  session.
- Keep the terminal open when possible if the user can still recover manually.
- On terminal-owned Rich Prompt close, best-effort cleanup is acceptable only
  when failures are reported and boot preflight will warn again.

Failure modes that need explicit design and tests:

- Page reload while Rich Prompt terminal and watcher are active.
- Closing and reopening the window.
- Terminal session missing after restore.
- Watcher attached but draft, `spool/`, or `spool/events/` has been deleted.
- `rm -rf` of the draft workspace while active.
- `chmod` removes read/write/search permissions from draft, spool, events, or
  trash.
- Draft promotion/discard fails halfway.
- Agent writes unsafe files, symlinks, FIFOs, sockets, or huge event files into
  spool.
- Watcher backend drops events or reports provider errors.
- MCP resolves `Drafts/...` while the backing metadata path has disappeared.
- Terminal cwd points at a draft metadata directory that later disappears.
- User closes terminal while spawn/preflight is in progress.

## First Dispatches

Core Architect dispatches a rust/sys lane:

- Audit current terminal watcher lifecycle and Drafts metadata failure behavior.
- Produce current behavior vs desired behavior for missing, chmod-broken,
  deleted, stale-session, reload, and unsafe-entry cases.
- Identify which fixes belong in chan-drive, chan-server, terminal_sessions, MCP
  descriptions, and web state.

Web Architect dispatches a web lane:

- Audit `TerminalRichPrompt`, `tabs.svelte`, SpawnDialog/TeamDialog, and the
  team orchestrator against the Rich Prompt revamp doc.
- Produce the UI state machine for create, active, submitted, preflight, broken,
  closing, and discarded states.
- Integrate page-break only after the page-break owner confirms the source is
  ready.

Webtest/iab lane:

- Prepare live checks for current behavior before fixes: Cmd+P spawn, reload
  restore, terminal close, watcher attach/detach, draft delete/chmod during
  active prompt, terminal from draft cwd, and MCP list/read/resolve for
  `Drafts/...`.

## Three Lead Follow-Up

After the two-Architect survey, @@Alex wants to scale toward a team of three
or more leads, each owning a domain and spawning their own subagents.

Candidate charters:

- Backend/Core: chan-drive, chan-server, MCP, terminal process model, storage,
  API compatibility.
- Web/Product: editor, Rich Prompt, Team Work, File Browser, Graph,
  Infographics, frontend tests, Browser/iab verification.
- Desktop/Platform: Tauri shell, macOS/Windows/Linux packaging, process
  launch, update flow, app lifecycle, native integration.

Future verification experiment:

- Use Chan's own Rich Prompt survey/coordination system to ask a three-agent
  team to implement the Johnny Castaway screensaver.
- Treat that as a process test, not just a feature test: verify coordination,
  event watcher behavior, survey replies, role boundaries, and end-to-end
  teardown.

## Commit Discipline

Commit and push often. Commit messages must have:

- A short title.
- A wrapped body near 80 columns.
- A description of why the change exists.
- Verification bullets.

Audit before closing:

```bash
git log --format=%B -n 12 | awk 'length($0)>82{print NR ":" length($0) ":" $0}'
```
