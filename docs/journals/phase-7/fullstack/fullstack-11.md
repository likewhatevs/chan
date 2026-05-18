# fullstack-11: file-moved-while-open UX wedge

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-18

## Goal

Soften the experience when an open file is moved or
deleted on disk by something outside chan. Today the tab
surfaces a raw "i/o error file not found" string —
technically correct, terrible UX. We need a clear UI state
that lets the user decide what to do, plus an
inode-following auto-recovery for the unambiguous case.

## Relevant links

* [../request.md](../request.md) Bugfixes — fs-move bullet.
* @@WebtestB final sweep ([webtest-b-1.md](../webtest-b/webtest-b-1.md))
  contains a clean repro of the i/o error state.

## Acceptance criteria

### UI state (wave-1 from the request)

* When chan detects the open file has been moved or
  deleted, the tab shows a clear "this file was moved or
  deleted" state instead of the raw i/o error.
* The state offers affordances: **Re-open** (file dialog
  to point at the new location), **Find** (Cmd+F-style
  search if we know a fragment of the content), **Close**
  (drop the tab cleanly without saving).
* Detection uses `notify`-style filesystem watching on the
  open path (already plumbed in chan-drive for the indexer
  and watch features; reuse the watcher).

### Auto-follow (wave-2 stretch, can defer)

* If the moved file's inode is unchanged (same device, same
  inode, same content hash) and we can resolve a single
  unambiguous new path, the tab silently re-binds to the
  new path. No user action required.
* When the new path is ambiguous (multiple matches) or the
  file's truly gone (inode reused / deleted), fall back to
  the UI state above.

## Out of scope

* External-agent MCP-routed moves (deferred to the
  auto-discovery enhancement; not blocked on this).
* Rename indicators in the file tree (separate UX work).

## How to start

1. Find where the editor reports the i/o error. Likely in
   `web/src/components/FileEditorTab.svelte` or the load
   path it calls.
2. Reach into `chan_drive`'s watch facility: the indexer
   already subscribes to drive events. Add a per-tab
   watcher (or piggy-back on the indexer event stream) for
   the open path.
3. On move/delete event: dispatch into the tab's state
   machine; render the new state component.
4. If you do the auto-follow stretch, the inode + content-
   hash comparison needs a small chan-drive helper. Talk
   to @@Systacean if you want to add it on the drive side.

## Hand-off

Implemented in `web/src/state/tabs.svelte.ts`,
`web/src/state/store.svelte.ts`, and
`web/src/components/FileEditorTab.svelte`.

* Added a transient `fileMissing` state on file tabs. Missing-path
  read failures (`404`, `ENOENT`, `os error 2`, "not found") clear
  the raw error and keep the last buffer available for recovery.
* Piggy-backed the existing watch event path: external `Removed` or
  old-side `Renamed` events mark matching open tabs as moved/deleted,
  including dirty tabs that should not be refreshed from disk.
* Rendered the moved/deleted state with **Re-open**, **Find**, and
  **Close** actions. Re-open opens Files at the parent and rebinds the
  missing tab to the next selected editable file; Find seeds the global
  search panel with a saved content fragment when available.
* Auto-follow is deferred; no inode/hash helper exists on the web-side
  API yet.

Gate:

* `npm run test -- tabs`
* `npm run check`
* `npm run build`
* `scripts/pre-push`
