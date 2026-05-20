# fullstack-b-14: chan-desktop window title is the drive path, not "chan drive: <name>"

Owner: @@FullStackB
Date: 2026-05-20

## Goal

@@Alex 2026-05-20: "the tauri title: 'chan drive: <name>'
should be <path> instead". Replace today's `chan drive:
<drive-name>` window title format with the full drive path
(e.g. `/Users/fiorix/dev/github.com/fiorix/chan`).

Read of the ask: title is the path — no `chan drive:` prefix.
If you have a strong reason to keep the prefix (menubar
discoverability, OS-level window switcher hint), surface a
scope question; otherwise default to path-only.

## Background

Bug entry: [`../phase-8-bugs.md`](../phase-8-bugs.md)
"chan-desktop Tauri window title: shows 'chan drive: <name>',
should be '<path>' instead".

Per CLAUDE.md, chan-desktop's per-window state is keyed by
`w=<window-label>` URL parameter, with `serve.rs` driving
window creation. The title swap happens at window-build-
time alongside the existing label-derivation logic; this is
likely a one- or two-line change in
`desktop/src-tauri/src/serve.rs` (or wherever
`WebviewWindowBuilder::title(...)` gets called).

`fullstack-b-1`'s window-config LRU work is the most recent
chan-desktop change touching the same window-creation path;
read its commit (`fullstack-b-1` task tail) to ground the
current shape before editing.

## Acceptance criteria

* New chan-desktop windows open with the title set to the
  full drive path. No `chan drive:` prefix.
* Title updates appropriately if a window is recycled
  against a different drive (LRU restore path from -b-1).
* Existing menubar / OS window-switcher behaviour
  unchanged (the title is just text; no other affordance
  depends on the `chan drive: ` prefix verbatim).
* `cargo test -p chan-desktop --bin chan-desktop` green.
* Pre-push gate clean.

## How to start

1. Grep `desktop/src-tauri/src/` for the current title
   format — likely `format!("chan drive: {}", ...)` or
   similar near a `title(...)` call.
2. Replace with the drive's path string directly.
3. Verify the LRU restore path from `fullstack-b-1` still
   produces the correct title on a reopened window.
4. Pre-push gate.

## Coordination

* Tiny scope; rides the patch release alongside the
  rich-prompt mini-wave.
* Independent of [`fullstack-b-13`](fullstack-b-13.md);
  different file (`serve.rs` vs `terminal_sessions.rs` /
  `routes/terminal.rs`). Land in any order.
* @@WebtestB verifies on lane-B once landed (Tauri-launch
  permission already in effect from the prior session
  per `event-webtest-b-alex.md`).
* Push held for the patch-release commit-grouping cut.
