# B4 investigation note — Linux drop path-print

Author: @@Desktop. Authorized: task-Conductor-Desktop-17 (note only,
no code). Carryover origin: phase-23 ("Linux terminal path-print
no-op — no drag-pasteboard equivalent").

## Question

Can any GTK/XDND route recover dropped-file paths on Linux the way
`read_dropped_paths` reads the macOS drag pasteboard at DOM drop
time? Expected answer was "no, by design". The grounded answer is
split — and half of it is a surprise.

## Half 1 — the pasteboard-READ approach is indeed impossible (as documented)

`read_dropped_paths` (dropped_paths.rs) works on macOS because
`NSPasteboard(name: .drag)` is a system-wide pasteboard that
persists after the drop completes; an IPC arriving milliseconds
later (from the DOM `drop` handler) can still read it. X11 XDND has
no such persistence: file URIs travel as `XdndSelection` selection
data inside the drag transaction, and after `drop_finish` the
source relinquishes the selection — there is nothing to poll
afterwards. Wayland is stricter still: `wl_data_offer` objects are
destroyed when the drag ends. WebKitGTK's DOM `DataTransfer.files`
exposes sandboxed File objects without OS paths (same as every
WebKit). So a post-drop *read*, mirroring the macOS mechanism, does
not exist on Linux. The phase-23 statement is correct as written.

## Half 2 — SURPRISE: a capture-at-drag-time route exists

wry's GTK backend proves the paths are obtainable — during the drag
rather than after it. Vendored wry 0.55.1
(`src/webkitgtk/drag_drop.rs`):

- `connect_drag_data_received` (lines 78-89): when the
  `text/uri-list` target arrives, `data.uris()` (gtk
  `SelectionData::uris`) yields the real `file://` URIs, converted
  to `PathBuf`s and STORED on a controller.
- `connect_drag_drop` (lines 106-119): at drop time the stored paths
  are taken and delivered as `DragDropEvent::Drop { paths, .. }`.

This is exactly the data the terminal path-print needs, on both X11
and Wayland (it rides GTK signals, not raw XDND). chan-desktop never
sees it because `.disable_drag_drop_handler()` (serve.rs ~664) turns
wry's handler off on ALL platforms — correctly, since
tauri-runtime-wry's handler returns `true` unconditionally and
swallows in-page DnD (the phase-23 macOS takeover bug; the disable
comment documents this).

A Linux implementation would NOT re-enable wry's handler. Instead:
a worktree-tested shim could connect a *capture-only*
`drag-data-received` handler on the WebKitGTK widget (obtainable via
tauri's `with_webview` platform accessor), store the uri-list in
per-window state at drag time, never claim the drop (WebKit's own
DnD continues untouched), and have `read_dropped_paths` on Linux
return-and-clear that captured state. Same IPC, same SPA code, same
ACL scoping; macOS keeps the pasteboard read. Risks to evaluate
before anyone writes it: double-registration of the uri-list target
on a widget WebKit already manages, GTK3-vs-GTK4 signal differences
if wry migrates, and whether capture-only handlers perturb WebKit's
in-page DnD on Wayland.

## Conclusion

- Documented no-op stands for the *current mechanism* (post-drop
  pasteboard read): nothing to wire, nothing to fix.
- The blanket "impossible on Linux by design" is WITHDRAWN: a
  capture-at-drag-time shim is a viable, scoped design. Whether it
  is worth building for a platform with zero current users is a
  product call, not a technical blocker.
- Per the task's stop-rule, no code was written;
  task-Desktop-Conductor-18 surfaces the finding for routing.
