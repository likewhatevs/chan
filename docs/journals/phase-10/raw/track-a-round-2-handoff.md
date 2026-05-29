# Phase 10 Track A Round 2 Handoff

Date: 2026-05-25.

This file is the bootstrap for the next Track A agent. If Alex says
"read the Track A round-2 handoff", start here, then report back with the
current repo state and ask which round-2 item to begin.

## Opening Prompt

Copy this prompt into the next Track A agent if a full prompt is useful:

```text
You are the Phase 10 Track A round-2 continuation agent for chan.

Start by reading:
- ~/.ai/profile.md
- AGENTS.md
- docs/journals/phase-10/track-a-round-2-handoff.md
- docs/journals/phase-10/roadmap-track-a.md
- docs/journals/phase-10/track-c-next-agent-handoff.md
- design.md
- desktop/README.md
- desktop/design.md

Before editing:
- run git status;
- inspect recent commits on main;
- do not touch unrelated dirty files;
- keep all drive content reads behind chan-drive or the embedded server;
- do not add compatibility or migration paths unless explicitly requested;
- use 80-column commit message bodies.

You own Track A backend, desktop shell, API, MCP, release, and CLI handoff
work. Track C owns browser, editor, Hybrid pane, File Browser UI, graph UI,
and manual browser smoke. Coordinate through append-only journal notes and
focused handoff tasks. Do not edit Track C's roadmap unless Alex asks.

Current round-2 queue:
1. Run the remaining native desktop drag-out manual smoke.
2. Design the CLI-to-desktop handoff only after Alex selects it.
3. Run the Linux desktop launch work only after Alex selects it.
4. Run release validation only after Alex selects it.

Do not start all round-2 items at once. After reading this file, report the
repo state and ask Alex which item to start.
```

## Current State

Track A immediate sequence items 1 through 19 are complete. The latest Track A
commit at the time this handoff was written is:

- `9e16a4b Harden async file paths and MCP media`

The working tree was clean when this handoff was created. The branch was ahead
of `origin/main` by three commits.

Recent Track A outcomes:

- File Browser and attachment routes use `try_drive()` and return retryable
  drive-busy responses during temporary metadata swaps.
- Metadata import no longer holds the `drive_cell` write lock across watcher
  shutdown, import work, sleeps, or reopen work.
- Desktop native drag-out streams the authenticated download response into a
  private staging file.
- chan-llm replaced MCP `read_image` with `read_media` for Image and Pdf
  classes.
- Direct backend API smoke passed with a throwaway drive.

Track C has a separate handoff for browser and UI verification:

- `docs/journals/phase-10/track-c-next-agent-handoff.md`

## Round-2 Queue

### 1. Native Desktop Drag-Out Manual Smoke

Status: ready to run.

This is the only remaining direct follow-up from the Track A hardening pass.
Automated checks already passed, but manual drag behavior needs Finder or file
manager validation.

Read before starting:

- `desktop/src-tauri/src/drag_out.rs`
- `desktop/src-tauri/src/serve.rs`
- `desktop/src-tauri/src/main.rs`
- `crates/chan-server/src/routes/files.rs`
- `desktop/README.md`
- `desktop/design.md`

Validate on macOS first:

- Drag a file from the desktop File Browser to Finder.
- Confirm the exported filename matches the selected file basename.
- Confirm exported bytes match the server download route bytes.
- Drag a directory from the desktop File Browser to Finder.
- Confirm the exported archive name and tar entries match the
  `/api/files/<path>?download=1` directory archive behavior.
- Cancel or fail a drag and confirm no user-visible temp file remains.
- Confirm staging cleanup stays inside the private drag-out staging directory.

Verification hints:

- Use the embedded desktop path, not a direct drive filesystem read.
- Compare exported bytes to the authenticated server download response.
- If a temporary test drive is used, clean up the drive registration and the
  temp directory before handoff.

Document results in `docs/journals/phase-10/roadmap-track-a.md`. If a Track C
UI issue is found, add a focused note to
`docs/journals/phase-10/track-c-next-agent-handoff.md`.

### 2. CLI-To-Desktop Handoff Design

Status: deferred until Alex selects it.

Do not implement before a design checkpoint. The likely question is how the CLI
should discover or hand off to a running desktop instance without breaking the
single-user, local-first server model.

Read before starting:

- `crates/chan/src/main.rs`
- `crates/chan-server/src/lib.rs`
- `desktop/src-tauri/src/serve.rs`
- `desktop/src-tauri/src/main.rs`
- `desktop/design.md`
- `design.md`

Expected first output:

- A short design note in the Track A journal or a new focused handoff note.
- Options with trade-offs.
- A recommendation.
- No compatibility or migration path unless Alex asks for one.

Topics to settle before code:

- Whether the CLI should attach to an existing desktop-owned server, ask the
  desktop to open a drive, or continue to own a separate server process.
- How to represent ownership, bearer token discovery, and lifecycle.
- How to avoid direct desktop reads of drive content.
- What behavior should happen when no desktop instance is running.

### 3. Linux Desktop Launch

Status: deferred until Alex selects it.

This is separate from macOS drag-out smoke. Do not start it as part of the
native drag-out manual smoke unless Alex explicitly combines the tasks.

Read before starting:

- `desktop/README.md`
- `desktop/design.md`
- `desktop/src-tauri/src/main.rs`
- `desktop/src-tauri/src/serve.rs`
- `desktop/src-tauri/tauri.conf.json`

Expected scope:

- Launch the desktop shell on Linux.
- Verify embedded server startup and first-window routing.
- Record platform-specific blockers or fixes in Track A.
- Keep browser/editor UI regressions with Track C.

### 4. Release Validation

Status: deferred until Alex selects it.

Do not begin release validation until Alex explicitly asks for it. Track C may
still be active on UI smoke, and release validation should not mix with
unfinished Track C regression work.

Read before starting:

- `AGENTS.md`
- `docs/journals/phase-10/roadmap-track-a.md`
- `.github/workflows/`
- `rust-toolchain.toml`
- `Makefile`
- `desktop/README.md`

Expected scope:

- Run the agreed release gates for touched crates and desktop packaging.
- Verify web bundle embedding behavior where relevant.
- Record exact commands and results in the Track A journal.
- Commit only if Alex asks or the selected release task explicitly requires it.

## Coordination Rules

- Track A owns backend, desktop shell, CLI handoff, MCP, API, and release
  validation.
- Track C owns browser, editor, Hybrid pane, File Browser UI, graph UI, and
  manual browser smoke.
- If Track A finds UI work, write a focused Track C handoff note with exact
  expected behavior and test paths.
- Keep journal updates append-only where practical.
- Do not start Linux launch, CLI handoff design, or release validation until
  Alex explicitly selects that item.
