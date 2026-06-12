# task-Lead-ChanDesktop-3 — file-drop behavior: stop the webview takeover, terminal path-print

From: @@Lead. To: @@ChanDesktop. QUEUED: pick this up after your
round-1 tidy task completes — do not preempt it. This file carries
the joint spec; @@Chan's web half (task-Lead-Chan-3.md) references
it. Coordinate the mechanism with @@Chan before implementing.

## The bug (@@Alex, today, severity high)

Dragging an image from Finder onto a chan-desktop window anywhere
outside the editor makes the webview navigate into bare image view
with no way back — the SPA is gone until reload. Cause: desktop
builds the window with `.disable_drag_drop_handler()`
(desktop/src-tauri/src/serve.rs:656), so WKWebView's default
drop-navigation fires wherever the SPA doesn't intercept (editor and
file browser have DOM drop handlers; graph/search/terminal/etc.
don't).

## Required behavior (acceptance criteria)

1. Dropping any OS file anywhere on a chan window NEVER navigates
   the webview away from the SPA. Desktop and plain-browser alike.
2. Drop onto a TERMINAL pane (desktop): insert the file's absolute
   path at the cursor — macOS Terminal behavior. Multiple files:
   space-separated, shell-escaped. Implementation shape: feed the
   PTY input through the existing terminal write path.
3. Drop onto the EDITOR: keep today's embed behavior unchanged.
4. Drop onto the FILE BROWSER's intentional upload zone: keep
   today's upload behavior unchanged.
5. Drop onto anything else (graph, search, dashboard, ...): inert.
   No-op, no visual change.
6. Plain browser: same no-takeover guarantee; terminal path-print is
   desktop-only (browsers don't expose OS paths), so browser
   terminal drop is a no-op.

## Mechanism (verify empirically, then agree with @@Chan)

The DOM File API never exposes OS paths, so item 2 needs Tauri's
native drag-drop events (paths + physical drop position). The open
question: on macOS, does ENABLING the Tauri drag-drop handler
suppress the DOM drag/drop events the editor and file browser rely
on (as it does on Windows)? Test this FIRST; it decides the design:

- If DOM DnD survives alongside the native handler: enable the
  handler, keep all existing DOM zones, and add a desktop-only
  terminal drop path driven by the native event.
- If it does not: the desktop build routes ALL drops through the
  native event — SPA hit-tests the drop position to the pane under
  it and dispatches (terminal → PTY input, editor → embed via a
  path-based read/upload, file browser → upload, else no-op) — while
  the plain-browser build keeps the DOM zones. More work; only if
  forced.

Either way @@Chan lands an SPA-global drop guard (default-deny
preventDefault outside allowlisted zones) that fixes the takeover on
every surface immediately and independently of your half. Write the
agreed event contract (event name, payload: paths, physical
position, window label) into your task files before implementing.

## Verification

- You + @@Chan: browser smoke of the guard (drop on graph/search →
  nothing happens), vitest for the guard logic and the
  shell-escaping.
- The WKWebView drop arc itself is desktop-only: compile-verified +
  gate-green, then @@Alex hand-smokes (Finder → terminal, editor,
  graph) on a local build. Tell me when it's ready for his pass.

## Coordination

- Behavior change is AUTHORIZED for this bug (exception to the
  round's refactor-only rule).
- Same commit discipline as the round plan. Report completion as
  task-ChanDesktop-Lead-N.md + poke.
