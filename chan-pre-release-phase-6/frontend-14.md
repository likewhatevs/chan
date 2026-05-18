# frontend-14: rich-prompt overlay for the terminal

Owner: @@Frontend
Status: PARTIAL

## Goal

A markdown-rich prompt overlay that sits on top of a single
terminal pane. The user composes a prompt with the same editor
surface as the main markdown editor (toolbar + source / render
toggle only), then `Cmd+Enter` sends the raw markdown source to
that terminal's PTY. Designed for driving claude / codex / other
CLIs that accept markdown + image attachments.

## Triggers

* Right-click inside the terminal → "Rich prompt" menu item.
* Keyboard shortcut **Alt+Space** (register in
  `web/src/state/shortcuts.ts`; do not collide with existing
  bindings).

The overlay attaches to **one specific terminal** (the one the
trigger fires from). Each terminal keeps its own overlay state.

## Shape

* Reuses the existing overlay-shell visuals (same chrome family
  as file browser / search / graph) but positioned **inside the
  terminal pane**, not on the app root.
* Width: full pane width.
* Height: starts at half the terminal pane (top edge at the
  vertical midpoint, bottom edge near the bottom of the
  terminal). Resizable **in height only** via a top-edge drag
  handle. No width resize.
* The drag bound: top edge can move up (overlay grows) to a
  small minimum gap from the top of the terminal, or down
  (overlay shrinks) until it hits a sensible minimum height
  (say two editor lines + toolbar).

## Content

* Markdown editor surface, **trimmed feature set**:
  * Toolbar (bold / italic / code / link / image / heading,
    matching the main editor's toolbar where it makes sense).
  * Source ↔ Render toggle.
  * No file-tree / no inspector / no graph integration in
    the overlay; it is a pure composer.
* Reuse the main editor's CodeMirror 6 base
  (`web/src/editor/`) — same WYSIWYG / source split as the
  file-tab editor. Frontend-7's `{#key tab.id}` lifecycle
  pattern applies per-terminal.

## Image paste and attachments

* Pasting an image (or other binary attachment) into the
  editor:
  1. Upload via existing `/api/attachments` route (phase 5).
  2. Insert markdown image syntax referencing the returned
     path: `![alt](attachments/<hash>.png)`.
* The submit payload is the **raw markdown source** including
  the image reference; the terminal-side CLI fetches /
  interprets the attachment via the drive path.
* No data-URL embedding for images: the attachment route exists
  for this exact purpose and keeps the markdown text compact.

## Submit: Cmd+Enter

* `Cmd+Enter` (macOS) / `Ctrl+Enter` (other) — uses the same
  modifier mapping as [frontend-13](./frontend-13.md).
* Behavior: take the **raw markdown source** of the buffer
  (not the rendered HTML), write it byte-by-byte into the
  terminal's PTY input via the existing WebSocket input path.
  No extra encoding; the terminal sees it as if the user typed
  the markdown.
* After submit: the buffer **stays** so the user can edit and
  resend. The overlay does not auto-hide on submit (Esc
  dismisses).

## Dismiss: Esc

* `Esc` hides the overlay but **keeps the buffer**.
* Reopening the overlay (right-click "Rich prompt" or
  Alt+Space) restores the buffer and the last height.

## Persistence

* Per-terminal buffer + overlay height stored in the same
  per-window session blob used by tabs / terminal sessions.
  Key suggestion: include the terminal session id so reload
  preserves the draft alongside the PTY.

## Right-click menu inside the overlay

* The overlay's right-click context menu gains **"New File
  from here"**: opens the new-file dialog seeded with the
  current markdown source as the initial content. User picks
  a path + name; existing chan-drive write path saves it.
* Other context-menu entries inherit from the markdown
  editor's existing right-click surface (Copy / Paste /
  toolbar shortcuts).

## State model

```ts
// per-TerminalTab, alongside existing fields
richPrompt: {
  buffer: string;     // markdown source
  heightPx: number;   // user-set; default mid-pane
  open: boolean;      // visible vs hidden (Esc toggles)
}
```

Closing the terminal tab discards the overlay state with the
tab.

## Out of scope

* Multi-terminal broadcast of the prompt (overlay is per-tab;
  user toggles broadcast in the bar afterward if they want).
* Snippets / templates / saved-prompt library (file-via-
  "New File from here" is the manual save path).
* Streaming preview of the terminal's response inside the
  overlay (the response shows in the terminal as normal).

## Relevant links

* Request follow-up: Alex's pre-close addition 2026-05-18.
* Modifier-Enter chord plumbing: [frontend-13](./frontend-13.md).
* Existing overlays: `web/src/components/{FileBrowserOverlay,
  SearchPanel, GraphPanel}.svelte`.
* Editor surface: `web/src/editor/`,
  `web/src/components/FileEditorTab.svelte`.
* Attachments: `crates/chan-server/src/routes/attachments.rs`.
* New-file dialog: `web/src/components/PathPromptModal.svelte`.
* Shortcut registry: `web/src/state/shortcuts.ts`.

## Acceptance criteria

* Right-click "Rich prompt" + Alt+Space both open the overlay
  scoped to the terminal under the cursor / active terminal.
* Overlay starts at mid-pane height, resizes by dragging the
  top edge, never resizes width.
* Markdown editor surface with toolbar + source/render toggle;
  no other editor features.
* Cmd+Enter (macOS) / Ctrl+Enter (others) writes the raw
  markdown to the PTY; buffer remains; overlay stays open.
* Esc hides the overlay; buffer persists; reopen restores the
  buffer and height.
* Image paste uploads via `/api/attachments` and inserts the
  markdown reference inline.
* Overlay right-click → "New File from here" opens the
  new-file dialog seeded with the buffer.
* Per-terminal state survives reload (rides the per-window
  session blob).

## Tests

* Vitest:
  * Open / close state machine (Esc hides, buffer persists,
    reopen restores).
  * Cmd+Enter submit ordering (raw source extracted; PTY input
    write called with the right bytes).
  * Height resize bounds (min / max within pane).
  * Per-terminal isolation (overlay state on tab A doesn't
    bleed to tab B).
  * New-file-from-here seeding.
* `npm --prefix web run check`, `npm --prefix web test -- --run`,
  `npm --prefix web run build` green.

## Review and hardening

* @@Frontend self-review for editor lifecycle (apply
  frontend-7's `{#key terminalId}` pattern to avoid stale
  CodeMirror state across terminal tabs).
* @@WebtestA live: open overlay on a terminal, paste an
  image, Cmd+Enter to claude / codex, confirm the CLI
  receives the markdown + image reference.

## Progress notes

* 2026-05-18: Added `TerminalRichPrompt.svelte`, mounted inside a
  single terminal tab above the xterm surface. It opens from terminal
  right-click menu "Rich prompt" and from Alt+Space, starts at roughly
  half terminal height, and resizes by dragging the top handle.
* 2026-05-18: Composer reuses the existing markdown editor surfaces:
  `Wysiwyg` for rendered mode, `Source` for source mode, and
  `StyleToolbar` for formatting + source/render toggle.
* 2026-05-18: Cmd+Enter/Ctrl+Enter sends the raw markdown buffer to
  the PTY input path without adding extra bytes; Esc hides the overlay
  and keeps the buffer.
* 2026-05-18: Added "New File from here" in the overlay chrome and
  right-click chrome menu. It writes the current buffer through the
  existing file create API and opens the new file.
* 2026-05-18: Rich-prompt buffer, height, open state, and mode persist
  in the per-window terminal session layout, not in shareable URL hash.
* 2026-05-18: Added component coverage for Esc hide preserving the
  buffer, Cmd/Ctrl+Enter raw markdown submit, send-button parity,
  resize min/top-gap clamps, source/render mode persistence,
  per-terminal isolation, and New File from here seeding/writes.

## Completion notes

Partial implementation is green, but live/editor hardening remains:

* Need @@Webtest live pass for paste/upload image behavior inside the
  embedded composer and for real CLI receive semantics.
* Component-level tests for open/close, submit ordering, resize bounds,
  send-button parity, mode persistence, per-terminal isolation, and
  New File from here seeding are now covered in
  `web/src/components/TerminalRichPrompt.test.ts`; session
  serialization remains covered in `web/src/state/tabs.test.ts`.

Validation:

* `npm --prefix web run check`
* `npm --prefix web test -- --run src/components/TerminalRichPrompt.test.ts`
  (7 tests)
* `npm --prefix web test -- --run` (20 files, 192 tests)
* `npm --prefix web run build` (passes with existing Vite chunk-size,
  ineffective dynamic import, and plugin timing warnings)
