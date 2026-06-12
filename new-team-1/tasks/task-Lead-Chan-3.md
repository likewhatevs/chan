# task-Lead-Chan-3 — file-drop guard (web half) + chanwriter purge riders

From: @@Lead. To: @@Chan. QUEUED: pick up after your round-1 tidy
task completes — do not preempt it.

## 1. File-drop guard (the web half of a joint fix)

Full spec + the bug story live in task-Lead-ChanDesktop-3.md — read
it first. Your half:

- SPA-GLOBAL drop guard: default-deny `dragover`/`drop`
  (preventDefault) everywhere EXCEPT the intentional zones — the
  editor embed flow (web/src/editor/bubbles/image_drop.ts and
  friends) and the file browser upload zone (see
  fileBrowserUploadDrop.test.ts). This alone kills the
  webview-takeover data-loss in both desktop and plain browser, and
  ships independently of the desktop half — land it first.
- Terminal path-print: desktop-only, driven by whatever native event
  mechanism @@ChanDesktop establishes (paths + drop position).
  Agree the contract with them BEFORE implementing; your side is
  pane hit-testing + shell-escaped insertion through the existing
  terminal write path. Multiple files → space-separated. In plain
  browser, terminal drop is a no-op.
- Vitest the guard logic + escaping; browser-smoke: drop a file
  onto graph/search → nothing happens, editor + file browser drops
  still work. Reactivity caveat from the round plan applies.
- Behavior change is AUTHORIZED here (exception to refactor-only).

## 2. chanwriter purge riders (fold into your tidy pass)

@@Alex wants every chanwriter/chan-writer reference gone (the org
is deleted; docs/phases + CHANGELOG exempt as usual). Add
`chanwriter|chan-writer` to your sweep patterns. Known hits on your
surface:

- crates/chan-workspace/Cargo.toml:8 — crate description says "for
  chan-writer workspaces".
- crates/chan-workspace/src/lib.rs:1 — same phrase in the header
  comment.
- crates/chan-workspace/design.md, chan-tunnel-client/design.md,
  chan-tunnel-server/design.md — describe the dead
  `chan-writer/chan-core` sibling-repo split as if current; your
  design.md rewrites (task 1) must erase that framing.

Completion: fold into your completion file(s) as usual.
