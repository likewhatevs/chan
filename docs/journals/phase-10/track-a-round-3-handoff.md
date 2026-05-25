# Phase 10 Track A Round 3 Handoff

Date: 2026-05-25.

Snapshot code baseline: `454bfa2` (`fix(desktop): install web deps before
build`).

This handoff lists Track A work still pending at the snapshot code baseline
above.
Track C is complete. Track B has a separate Round 3 handoff.

Track A owns backend, desktop shell, CLI handoff, MCP, API, release, and
server-side Rich Prompt follow-ups.

Do not touch unrelated dirty files. This docs pass does not edit desktop
implementation files; the desktop build fix at `454bfa2` is recorded only as
already landed.

## Current State

- Track A immediate sequence items 1 through 20 are recorded complete in
  `roadmap-track-a.md`.
- The older Track A round-2 handoff still says macOS native drag-out manual
  smoke is ready to run, but `roadmap-track-a.md` later records that macOS
  Finder drag-out smoke passed on 2026-05-25.
- Linux desktop launch and Linux drag-out remain unvalidated.
- CLI-to-desktop handoff is still a design checkpoint, not implementation.
- Release validation is deferred until explicitly selected.
- Track B cut two desktop-facing follow-ups to Track A:
  app icon regeneration and desktop docs/config audit.
- The Rich Prompt watcher audit added server-side follow-up candidates.

## Ad-Hoc Track A Work Landed Before Round 3

These fixes are in the snapshot code baseline and are not pending Track A
implementation work unless a regression is found.

- Terminal xterm.js/session routing: `3ce1db0` restored xterm.js after the
  ghostty-web experiment and isolated PTY session routing across split panes,
  tab moves, reconnects, broadcast input, and Rich Prompt target injection. It
  also kept the trailing resize fit patch and Cmd+. shortcut handling fix.
- Draft management: `05c5cee` made draft preflight/list handling skip team
  workspace metadata under `Drafts/` instead of treating those directories as
  broken drafts missing `draft.md`.
- Desktop new-clone build: `454bfa2` made the desktop Makefile install
  `web` dependencies before building the embedded web bundle and bundled
  helper binary. This is documented here only as landed context.

## Pending Queue

### 1. CLI-To-Desktop Handoff Design

Status: pending design checkpoint.

Do not implement first. Produce a short design note with options,
trade-offs, and one recommendation.

Questions to settle:

- Should `chan serve <path>` attach to an existing desktop-owned server, ask
  desktop to open the drive, or keep owning a separate server process?
- How should same-user desktop discovery work?
- How are ownership, bearer token discovery, lifecycle, version, and
  capability mismatch represented?
- What is the no-desktop-running behavior?
- Which flags force standalone server behavior?

Read first:

- `crates/chan/src/main.rs`
- `crates/chan-server/src/lib.rs`
- `desktop/src-tauri/src/serve.rs`
- `desktop/src-tauri/src/main.rs`
- `desktop/design.md`
- `design.md`

Expected output:

- A new focused design note under `docs/journals/phase-10/`, or an append-only
  Track A journal update.
- No compatibility or migration path unless explicitly requested.

### 2. Linux Desktop Launch

Status: pending validation and possible fixes.

Known prior blocker:

- Linux desktop opened white windows for at least one manual tester.
- Expected drive/onboarding surface did not render.
- Menus were incomplete.

Validate:

- Desktop shell launches on Linux.
- Embedded server starts.
- First-window routing works.
- Drive list or first-run flow renders.
- Menus expose expected platform actions.

Read first:

- `desktop/README.md`
- `desktop/design.md`
- `desktop/src-tauri/src/main.rs`
- `desktop/src-tauri/src/serve.rs`
- `desktop/src-tauri/tauri.conf.json`

Keep browser/editor regressions with Track C only if new Track C work is
explicitly reopened.

### 3. Linux Native Drag-Out

Status: pending, blocked on Linux desktop launch viability.

macOS native Finder drag-out passed on 2026-05-25. Linux remains unsmoked.

Validate after Linux desktop launches:

- File Browser file drag-out to the Linux file manager.
- Export filename matches selected file basename.
- Export bytes match `/api/files/<path>?download=1`.
- Directory drag-out exports the expected `.tar`.
- Archive bytes and entries match the server download route.
- Cancelled or rejected drags do not leave visible temp files.
- Staging cleanup stays inside the private drag-out staging directory.

Do not read drive content directly from desktop code. Compare against the
authenticated server download response.

### 4. Release Validation

Status: deferred until release validation is selected.

Expected scope:

- Run the agreed release gates for touched crates and desktop packaging.
- Verify standalone `chan` artifact launch.
- Verify desktop artifact launch.
- Verify web bundle embedding behavior where relevant.
- Verify GitHub Release assets include standalone binaries, `VERSION`, and
  `SHA256SUMS`.
- Keep standalone binary release workflow separate from desktop packaging.
- Record exact commands and results in Track A docs.

Read first:

- `AGENTS.md`
- `.github/workflows/`
- `rust-toolchain.toml`
- `Makefile`
- `desktop/README.md`
- `docs/journals/phase-10/roadmap-track-a.md`

### 5. Tauri App Icon Regeneration

Status: pending Track B to Track A handoff.

Do not start as part of this docs pass.

Task:

- Regenerate Tauri app icons so Cmd+Tab and Dock show a dark background
  `#101112` with the orange enso `#ef8f58`.
- Base colors on current dark site tokens.
- Update Tauri icon assets where applicable.
- Verify the generated macOS app icon in Cmd+Tab and the Dock, not only the
  source PNGs.

Source handoff:

- `docs/journals/phase-10/track-a-handoff-from-track-b-logo-and-docs.md`

### 6. Desktop Docs And Config Audit

Status: pending Track B to Track A handoff.

Do not start as part of this docs pass.

Task:

- Audit stale desktop docs, config, and comments for old release/install
  contracts.
- Include Linux and Windows wording in `desktop/README.md`.
- Include old `/dl/latest`, MSI, and updater language in `desktop/design.md`.
- Include updater URL shape in `desktop/src-tauri/tauri.conf.json`.
- Keep corrected wording fresh-state only. Do not add migration or old-contract
  framing unless requested.

Source handoff:

- `docs/journals/phase-10/track-a-handoff-from-track-b-logo-and-docs.md`

### 7. Rich Prompt Watcher Reattach

Status: pending decision.

Finding:

- Restored Rich Prompt workspaces rehydrate `workspaceName` and watcher UI
  state, but detached server watchers are not automatically reattached to the
  existing workspace's `spool/events`.

Decision needed:

- Reattach watcher on Rich Prompt status refresh when the workspace is valid
  and the terminal session exists.
- Or keep the current behavior where restored watcher state clears on first
  not-attached poll.

Source audit:

- `docs/journals/phase-10/rich-prompt-watcher-audit.md`

### 8. Rich Prompt Pre-Flight Dispatch

Status: pending decision.

Finding:

- UI polling accepts `pre-flight` events.
- Backend `AgentEventType` does not include `pre-flight`, so pre-flight files
  are visible to BubbleOverlay but are not injected into a PTY by backend
  dispatch.

Decision needed:

- Add an explicit backend pre-flight event variant if PTY injection is desired.
- Or document that pre-flight is UI-polled only.

Source audit:

- `docs/journals/phase-10/rich-prompt-watcher-audit.md`

### 9. AgentEventEcho Replay Or Loss Contract

Status: pending decision.

Finding:

- `AgentEventEcho` rides the terminal broadcast channel and terminal
  WebSocket. If no receiver is attached during the event, the echo can be
  lost.

Decision needed:

- Add replay or a recent-event poll path.
- Or explicitly accept lost delivery during detached WebSocket windows.

Source audit:

- `docs/journals/phase-10/rich-prompt-watcher-audit.md`

## Recommended Round 3 Order

1. Pick one explicit Track A item.
2. If no release cut is planned, start with CLI-to-desktop handoff design.
3. If Linux availability matters first, start with Linux desktop launch and
   defer Linux drag-out until launch is viable.
4. Keep release validation separate from feature/design work.
5. Treat Rich Prompt watcher follow-ups as backend/API work only after Alex
   selects them.
