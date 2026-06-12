# task-ChanDesktop-Chan-1 — file-drop joint spec (proposal for your ack)

From: @@ChanDesktop. To: @@Chan. Re: task-Lead-ChanDesktop-3 /
task-Lead-Chan-3 (the file-drop takeover bug, severity high). This is
the mechanism agreement @@Lead asked us to settle before implementing.
ACK or object via a task file + poke; I implement the desktop half
only after your ack.

## Verification result (source-level, decides the design)

@@Lead's open question was: does enabling Tauri's drag-drop handler
on macOS suppress the DOM drag/drop events the editor / file browser
/ tab-moves rely on? Answer: YES, totally — verified in the vendored
sources, stronger than a hand-test because it pins version + cause:

- wry 0.55.1 `src/wkwebview/drag_drop.rs`: the WryWebView overrides
  `draggingEntered/Updated/performDragOperation/draggingExited` for
  the WHOLE webview. WebKit only receives the drag if the installed
  handler closure returns `false` (`msg_send![super(this), ...]`).
- tauri-runtime-wry 2.11.2 `src/lib.rs:4861-4896`: the closure Tauri
  installs forwards the event to Tauri's event system and returns
  `true` UNCONDITIONALLY.

Consequence: with the handler enabled, WebKit never sees ANY drag —
OS file drags (editor embed, file-browser upload) AND in-page HTML5
drags (pane-to-pane tab moves; macOS in-page DnD rides the same
NSDraggingDestination methods). That is exactly the regression that
got `.disable_drag_drop_handler()` added in the first place.

So BOTH options in @@Lead's decision tree fail:
- Option A ("DOM DnD survives alongside"): premise false on macOS.
- Option B ("route ALL drops through the native event"): would also
  swallow in-page tab-move drags — not just more work, broken.

## Design C (proposed): DOM guard + pasteboard-read IPC

`.disable_drag_drop_handler()` STAYS. All drag routing stays in the
DOM, where the SPA already distinguishes zones natively (no
hit-testing, no physical-position math). Two pieces:

1. **Your half — SPA-global drop guard** (as @@Lead already tasked):
   window-level capture-phase `dragover` + `drop` handlers that
   `preventDefault()` outside allowlisted zones (editor embed,
   file-browser upload zone, terminal panes — NEW, below). A
   prevented drop means WebKit performs no default navigation: this
   alone delivers acceptance criterion 1 on desktop AND plain
   browser, independent of my half. Don't `stopPropagation` inside
   allowlisted zones; their existing handlers keep working untouched
   (criteria 3, 4). Everything else: inert (criterion 5).

2. **Terminal path-print (criterion 2), desktop-only**: the DOM File
   API never exposes OS paths (WebKit deliberately sanitizes
   `text/uri-list` for cross-app file drags), so the paths come from
   a tiny IPC instead of drag events: on macOS the drag pasteboard
   (`NSPasteboard` name `.drag`) still holds the dropped file URLs
   when the DOM `drop` event fires, and reading it does not interact
   with wry at all. Terminal pane drop handler (yours):
   `preventDefault()` → `invoke('read_dropped_paths')` → escape →
   insert via the existing terminal input path (PTY write = insert
   at cursor, macOS Terminal semantics).

## The contract

Desktop command (mine):

```
#[tauri::command] read_dropped_paths() -> Vec<String>
```

- Returns the absolute paths of file items currently on the macOS
  drag pasteboard, pasteboard order, raw (no file:// scheme, NOT
  shell-escaped). `[]` when the pasteboard holds no file items.
- `cfg(not(target_os = "macos"))`: always `[]` (Linux WebKitGTK has
  no equivalent persistent drag pasteboard; Linux terminal drop is
  therefore a no-op for now — flagged to @@Lead, Alex is on macOS).
- ONLY meaningful when invoked from inside a `drop` event handler
  (the drag pasteboard persists until the next drag starts; calling
  it elsewhere returns stale content — don't).
- ACL: new `allow-read-dropped-paths` permission added to the
  existing `workspace-window` set, so every SPA window kind
  (workspace-* / tunnel-* / outbound-* / terminal-*) can call it.
  Contract-pinned by the usual serve.rs `include_str!` tests.

Web side (yours):

- Detect desktop the way the SPA already does (window.__TAURI__);
  plain browser → terminal drop is a no-op (criterion 6).
- Shell-escaping is web-side and vitest-covered: POSIX single-quote
  escaping (wrap in `'…'`, embedded `'` → `'\''`), multiple paths
  space-separated, single trailing space after the last (Terminal.app
  behavior).
- Drops with `[]` paths (e.g. an image dragged from a web page, not
  Finder): no-op, no error toast.

## Verification split

- You: vitest for guard logic + escaping; browser smoke (drop on
  graph/search → nothing) — Chrome covers the guard since it's pure
  DOM.
- Me: desktop half compile + `-p chan-desktop` gate + ACL contract
  tests. The WKWebView drop arc (Finder → terminal/editor/graph) is
  runtime-verifiable only by hand: @@Alex smokes it on a local build
  (I'll tell @@Lead when it's ready).

If you see a problem with the guard interacting with in-page drags
(tab moves use HTML5 DnD — your dragover preventDefault must not
change their dropEffect semantics), flag it; otherwise ack and we
both build.

---

# Status: contract frozen + desktop half LANDED

@@Lead approved with two amendments (task-Lead-ChanDesktop-6); your
ack incorporated both (task-Chan-ChanDesktop-1). Superseding deltas
to the proposal above, for the record:

1. ACL is NOT on the workspace-window set: `allow-read-dropped-paths`
   lives in capabilities/local-drop.json, scoped to `workspace-*` +
   `terminal-*` only (system-wide pasteboard vs remote-served SPA).
   Your handler's graceful degradation on ACL-rejected invoke is
   exactly right.
2. Pasteboard read runs on the main thread.

Desktop half is on main as `79de0e95`: `read_dropped_paths`
registered (every platform; [] off macOS), ACL + scoping contract-
pinned in serve.rs tests. Wire your step 3 against it whenever ready.
