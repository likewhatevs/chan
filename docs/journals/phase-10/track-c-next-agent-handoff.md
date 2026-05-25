# Phase 10 Track C Next Agent Handoff

Date: 2026-05-25.

This handoff is for the next Track C agent continuing the browser and Hybrid
polish lane after commit `f5696db Tighten docked file browser actions`, plus
Track A's streaming relationship API handoff from commit
`36af846 Stream relationship APIs`.

## Opening Prompt

Copy this prompt into the next Track C agent:

```text
You are the Phase 10 Track C continuation agent for chan.

Start by reading:
- ~/.ai/profile.md
- AGENTS.md
- docs/journals/phase-10/roadmap-track-c.md
- docs/journals/phase-10/track-c-next-agent-handoff.md
- docs/journals/phase-10/track-c-handoff-from-track-a.md
- docs/journals/phase-10/track-c-handoff-streaming-ui-iab.md

Do not use docs/agents/bootstrap.md unless @@Alex explicitly asks.

You are not alone in the codebase. Other agents may be active on Track A,
Track B, desktop, server, or docs. Before editing, check git status and scope
your changes to Track C. Do not revert or stage work you did not make.

Follow the process used here:
- Read the Track C journal and any handoff tasks other agents cut to Track C.
- Make small scoped changes only after confirming the relevant code path.
- Verify with the smallest useful tests first, then broader checks when risk
  warrants it.
- Document progress in the Track C journal before handoff or commit.
- Commit only when @@Alex asks or when the current request explicitly includes
  a commit.
- If @@Alex asks you to create work for another track, write a focused handoff
  task for that track or update the requested track journal, and keep ownership
  boundaries clear.

Current goal: run the remaining Track C live regression pass, then cut focused
fixes only for regressions found in that pass.
```

## Current State

Recently completed:

- Terminal pane-switch font and glyph corruption was fixed and manually
  confirmed with split panes, terminal output, ANSI output, and focus changes.
- Plain `Cmd+L` was returned to the browser. Screen lock remains `Cmd+. L`.
- Matrix lock was ported closer to `dcragusa/MatrixScreensaver`, with visible
  credit and bundled MIT notice.
- Shared file and directory inspectors expose Upload and Download.
- Draft editor tabs expose Save-to-drive instead of a path Name row.
- Docked File Browser context menus now have docked-only Upload and Download,
  plus Open in File Browser for selected rows and no-selection drive Details.

Latest relevant commit:

- `f5696db Tighten docked file browser actions`
- `36af846 Stream relationship APIs`

## Next Tasks

### 1. Shared Inspector Transfer Regression

Live-regress Upload and Download across every shared inspector surface:

- File Browser tab inspector, file selected.
- File Browser tab inspector, directory selected.
- Graph node inspector, file selected.
- Graph node inspector, directory selected.
- Editor Show Details inspector for the open markdown file.

Expected:

- File Upload replaces the selected file through the chan-drive-backed route.
- Directory Upload adds the uploaded file inside that directory.
- File Download retrieves the selected file bytes.
- Directory Download retrieves the existing directory archive flow.
- Uploading binary bytes to an editable text path is rejected or shown as
  non-renderable, not rendered as markdown.
- Status bar transfer state is clear, cancel works for active uploads, and
  File Browser refresh keeps expansion state.

### 2. Draft Explicit Save Regression

Live-regress Draft Save-to-drive:

- Single-file Draft opens as `Drafts/untitled/draft.md`.
- Draft tab back or right-click settings shows Save instead of Name.
- Save opens the same promotion workflow as close-tab Save.
- After Save, the tab continues on the promoted drive path.
- Saved file appears in docked File Browser without reload.
- Saved file appears in File Browser tab without losing expansion state.
- Repeat with a Draft workspace that has attachments if available.

### 3. Completed Chrome Live Regression

Run a broader live visual pass over completed Track C chrome:

- Terminal scroll-heavy and ANSI output pane switching.
- Graph filesystem spine from drive root, scoped file root, and scoped
  directory root.
- File Browser expansion restore after reload for docked and tab variants.
- Matrix lock against the upstream reference, with attention to overlay leaks.
- Actionable drive-warning dialog for broken Draft metadata.
- Docked File Browser context actions:
  - row selected: Open in File Browser opens a tab with Details on that row;
  - no row selected: Open in File Browser opens a tab with Details on drive;
  - Upload and Download are docked-only row menu actions;
  - tab and overlay File Browser row menus omit Upload and Download.

### 4. Track A Handoff Intake

Read `docs/journals/phase-10/track-c-handoff-from-track-a.md` before closing
Track C:

- Rich Prompt browser validation remains a Track C browser/editor task.
- Rapid-edit stale editor/index validation remains a Track C browser/editor
  task.
- If validation shows server queue churn instead of editor state drift, cut a
  minimal repro back to Track A.

### 5. Streaming Inspector And Graph Intake

Read `docs/journals/phase-10/track-c-handoff-streaming-ui-iab.md` before the
live browser pass.

Track A added API streams:

- `GET /api/report/file?path=<rel>&stream=1`
- `GET /api/backlinks/<rel>?stream=1`
- `GET /api/graph?scope=drive|directory|file&path=<rel>&depth=<n>&stream=1`

Track C owns the browser consumption:

- typed NDJSON readers in `web/src/api/client.ts`;
- inspector report, references, and backlinks partial loading in
  `FileInfoBody.svelte`;
- graph node upserts and edge batch appends in `graphData.svelte.ts` and
  `GraphPanel.svelte`;
- reload/cancel behavior for in-flight relationship streams.

Browser/IAB smoke to include in the live pass:

- Build and serve the current repo as a drive with `--no-token --no-browser`.
- Open `CHANGELOG.md` in the editor.
- Confirm editor content appears before the full file stream completes and
  editing is disabled until full load.
- Open the inspector for `CHANGELOG.md`.
- Confirm report, references, and backlinks show loading or partial state
  without a 10 second timeout.
- Open Graph from the same file.
- Confirm nodes and edges appear before the graph stream reaches `done`.
- Trigger Reload in the inspector and graph UI, then confirm a fresh stream
  starts and the partial state resets cleanly.

## Suggested Verification Setup

Use a throwaway HOME and drive:

```bash
npm run build
cargo build -p chan
mkdir -p /tmp/chan-track-c-home /tmp/chan-track-c-drive
HOME=/tmp/chan-track-c-home ./target/debug/chan serve --no-browser /tmp/chan-track-c-drive
```

Use the printed bearer URL in Browser/iab.

Seed enough content for the pass:

- a markdown file with links, tags, and headings;
- a root markdown file large enough to exercise streaming open and inspector
  relationships, for example repo `CHANGELOG.md`;
- a nested directory with at least one markdown file and one binary file;
- a Draft with non-whitespace content;
- an optional Draft workspace with an attachment;
- a broken Draft metadata directory only if testing the warning dialog.

## Reporting Template

Use this structure when reporting to @@Alex or @@Architect:

```text
Track C live regression report

Commit:
URL:
Browser:
Viewport:
Build:
Console:

PASS/FAIL:
- Shared inspector Upload/Download:
- Draft explicit Save:
- Terminal ANSI pane switching:
- Graph filesystem spine:
- File Browser expansion restore:
- Matrix lock:
- Drive warnings:
- Docked File Browser context actions:
- Track A handoff items:
- Streaming inspector/graph UI:

Screenshots:
- Only failures or suspicious visuals.

Known gaps:
- ...

Follow-up tasks cut:
- ...
```

## Ownership Notes

- Track C owns web and embedded-app behavior.
- Track A owns native desktop shell, Linux desktop launch failures, and native
  drag-out/download bridges.
- Track B owns its current journal tasks. Do not edit Track B unless the user
  asks for a handoff or coordination note.
- Keep handoffs factual and scoped. Include exact repro steps, expected
  behavior, observed behavior, and owner rationale.

## Track A Backend Acceptance Notes

Added on 2026-05-25 for Track C transfer regression context:

- File Browser and inspector upload/download still use the same
  chan-drive-backed `/api/files` routes and the same
  `/api/files/<path>?download=1` download contract.
- File downloads preserve basename and bytes. Directory downloads preserve the
  existing `.tar` archive contract.
- During metadata import, file and attachment routes may now return a
  retryable drive-busy response while the drive cell is temporarily absent.
  Treat this as transient if it appears during live UI smoke.
- Native desktop drag-out still uses the same download URL. Track A changed
  only the desktop staging implementation so the HTTP body streams into the
  temp export file instead of buffering before staging.
- No Track C UI change is required for MCP `read_media`; it is external-agent
  surface only.
