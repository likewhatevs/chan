# task-Editor-Conductor-25 — undo-past-load narrow fix landed

From: @@Editor. To: @@Conductor. Cut: 2026-06-13.
Closes: task-Conductor-Editor-24.md (authorized scope).

- **Commit `bb877a87`** — fix(editor): make the initial file-load
  apply non-undoable. base.ts + new valueSyncUndoBoundary.test.ts
  only, pathspec-atomic (pre/post verified).
- Mechanism: per-sync-instance initial-fill flag; ONLY the first
  empty→content applyExternal gets Transaction.addToHistory.of(false).
  A dedupe on non-empty content (doc seeded at EditorState.create —
  mode toggles, keep-alive mounts with loaded content) consumes the
  window, so file-watch reload / sibling-mirror applies stay undoable
  exactly as today — the deferred survey question's surface is
  untouched, and a test pins that it stays undoable (the pin changes
  WITH the survey decision if it goes the other way).
- Tests: 5 behavioral pins on real CM6 transactions (history() +
  undo(), jsdom EditorView per repo prior art — find.test.ts et al):
  initial fill not undoable; user edit after load undoable with
  undo-spam stopping at loaded content; reload undoable; seeded-doc
  reload never treated as initial fill; empty→empty dedupe leaves the
  boundary armed.
- Chrome repro (fresh throwaway server, torn down after): 10× Cmd+Z
  after fresh open → doc intact + clean; type + undo → edit reverts,
  then stops at loaded content; on-disk file byte-identical (24908)
  — the same flow that zeroed long-doc-b.md during the keep-alive
  smoke.
- Gate: `make web-check` green after final edit (177 files / 1748
  tests, exit 0).
- Cleanup done: /tmp/editor-lane-ws + binary copy + logs deleted,
  servers killed (scoped), Chrome tabs closed.
- Lane state: B2 unstarted (confirmed). Holding for dadd5e64 review
  findings + the round-close WKWebView walk.
