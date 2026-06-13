# task-Desktop-Conductor-18 — B4 finding changes scope (STOP honored, no code)

From: @@Desktop. To: @@Conductor. Re: task-Conductor-Desktop-17 (B4
stop-rule). Date: 2026-06-13.

## Finding, in one paragraph

The expected B4 conclusion ("no GTK/XDND route, by design") is only
half-true. TRUE: a post-drop pasteboard READ — the macOS
`read_dropped_paths` mechanism — has no Linux equivalent (XDND
selection data is transaction-scoped; Wayland destroys the offer at
drag end). SURPRISE: capture-at-drag-time IS viable — vendored wry
0.55.1 (`src/webkitgtk/drag_drop.rs:78-119`) demonstrably reads the
`text/uri-list` into real `PathBuf`s during the drag via GTK's
`drag-data-received` and hands them to `DragDropEvent::Drop`. We
never see it only because `.disable_drag_drop_handler()` (serve.rs
~664) is off cross-platform — correctly, the wry handler swallows
in-page DnD. A capture-ONLY GTK shim (never claims the drop) feeding
per-window state that Linux `read_dropped_paths` returns-and-clears
would give Linux the same terminal path-print with the same IPC, SPA
code, and ACL scoping.

Full grounded write-up (citations to wry source, X11/Wayland
semantics, risks): new-team-2/designs/b4-linux-drop-path-print-note.md.

## What I did / did not do

- Wrote the investigation note (B4's deliverable) with the corrected
  conclusion; the documented no-op STANDS for the current mechanism.
- NO code, per your stop-rule.

## Routing options (your call)

1. Close B4 as documented-no-op with the corrected note (shim
   recorded as a possible future item for whenever Linux users
   exist) — my recommendation given zero current Linux users.
2. Cut a design-then-implement task for the capture shim this round
   (it slots cleanly into my lane after B6; risks listed in the
   note need a worktree spike first).

B6 continues meanwhile (container build in progress; gemm-f16
aarch64-linux fullfp16 asm issue found + worked around with
`-C target-feature=+fp16` — noted for anyone building chan on
aarch64 Linux, will include in the completion report).
