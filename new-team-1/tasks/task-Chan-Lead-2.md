# task-Chan-Lead-2 — COMPLETE: file-drop guard (web half) + terminal path-print

From: @@Chan. To: @@Lead. Re: task-Lead-Chan-3 part 1 + task-Lead-
Chan-4 amendments + the frozen contract (task-Chan-ChanDesktop-1).

## What landed

Commit a19d7d40 (8 files): web/src/state/fileDropGuard.ts (+13-test
vitest file), guard install in App.svelte, data-file-drop-zone
markers (Wysiwyg, Source, RichPrompt hosts, terminal panes),
readDroppedPaths() in api/desktop.ts, terminal drop handler in
TerminalTab.svelte.

Guard mechanics: acts ONLY on Files-type drags (requirement 1 —
vitest-pinned both ways: text/plain dragover NOT prevented, Files
outside a zone prevented). dragover always cancelled (an uncancelled
dragover navigates on release) with dropEffect=none outside zones;
drop cancelled outside zones at capture; a bubble-phase NET cancels
anything zone handlers leave unhandled (covers read-only CodeMirror
instances — a hole in the literal outside-zones-only spec wording).
Zones: data-file-drop-zone opt-in + .cm-editor (editable CM6 already
owns its file drops; image embed + native text-file insert verified
unchanged).

Terminal path-print: drop on a terminal → preventDefault →
readDroppedPaths() (the desktop IPC, landed their side as 79de0e95;
contract verified name-for-name against their main.rs/serve.rs pins)
→ POSIX single-quote escape, space-separated, single trailing space
→ the normal user-typed input path (sendUserInput, broadcast
semantics like typing). Requirement 2: ACL-refused invoke or plain
browser resolves [] → silent no-op; the guard alone still kills the
takeover.

## Verification

- vitest: 174 files / 1719 tests green (13 new). svelte-check 0
  errors / 0 warnings. Build green (make web-check exit 0).
- Chrome smoke on a throwaway --standalone workspace (torn down,
  server pkill scoped to its path, workspace unregistered):
  - Files drop on body/non-zone surface: prevented at guard capture,
    SPA alive, URL unchanged — no takeover.
  - text/plain dragover: untouched (in-page DnD).
  - Editor zone: guard capture does NOT prevent (probed at
    document-capture between guard and CM6); the editor's own
    handlers take the drop, net cancels after. Today's editor
    behavior intact.
  - Synthetic-event caveat: Chrome coerces dropEffect on constructed
    DataTransfers, so the not-allowed-cursor claim rests on the
    vitest assertions, not the browser probe (the probe's 'none' on
    the untouched text/plain control exposed the coercion).
- NOT verifiable by me: the real WKWebView Finder-drop arc + the
  desktop path-print end-to-end — ready for @@Alex's hand-smoke
  whenever @@ChanDesktop calls it; my half is on main.

## Judgment notes

- File Browser needs NO zone exemption: external OS drops are
  Upload-button-only by design (fileBrowserUploadDrop.test.ts pins
  the tree must not handle Files) — the task-3 wording assumed an FB
  upload drop zone that no longer exists. Drops on the tree are now
  guard-inert, which IS today's behavior preserved.
- Path-print rides sendUserInput (the typed-input path incl. group
  broadcast), matching what typing/pasting the same path would do.

Next: task-Lead-Chan-5 stragglers (picking up now).
