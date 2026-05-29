# fullstack-a-35: File rename UX parity with terminal rename + input above page-width

Owner: @@FullStackA
Date: 2026-05-20

## Goal

@@Alex 2026-05-20: "same way we can rename terminal, we
should be able to rename files.. place the input box above
the page width". Add file-rename parity with the existing
terminal rename affordance; place the rename input in a
header band above the editor's page-width-constrained
content column.

Two pieces:

1. **Affordance + UX shape**: mirror the terminal rename's
   trigger + inline-input + commit/cancel semantics.
   Whatever the terminal rename does today, the file
   rename matches.
2. **Position**: rename input lives ABOVE the
   `--chan-page-max-width`-capped content column (per
   `fullstack-a-30`). Header band, not constrained by the
   page-width cap.

## Background

Bug entry: [`../phase-8-bugs.md`](../phase-8-bugs.md)
"File rename UX: parity with terminal rename, input box
positioned above the page-width-constrained content".

Terminal rename today: grep for the existing trigger and
the inline-input component(s) it uses; same shape applies
to file tabs. Look for:
* `TerminalTab.svelte` rename handler.
* Any reusable inline-rename Svelte component the terminal
  side already exports.
* The state plumbing that takes the new name and updates
  the tab label + the persistent backing state.

File rename today: chan-drive owns filesystem operations
through `Drive`. Verify at task-start whether
`Drive::rename` (or equivalent atomic-rename op) exists. If
yes: wire chan-server route + SPA → done. If no: add the
chan-drive op alongside the existing `write_text` /
`write_bytes` shape (atomic + path-sandbox-safe).

Per CLAUDE.md the filesystem boundary contract is hard:
ALL renames must route through `chan-drive`, never through
`std::fs::rename` directly. The atomic pattern is
temp-file-or-staging + rename; for a single-file rename the
chan-drive helper can use `std::fs::rename` internally
since `rename(2)` is itself atomic within a filesystem.

## Acceptance criteria

* File tab rename trigger matches the terminal rename
  trigger (same chord / click pattern, same visual
  affordance).
* Rename input renders in a header band above the editor
  content. The input is full-width or otherwise NOT
  constrained by the `--chan-page-max-width` cap from
  `fullstack-a-30`.
* Enter commits the rename → chan-server route → chan-drive
  rename op → file is renamed on disk → tab label updates
  → file tree / FB updates → graph index re-resolves any
  references pointing at the old path (or surfaces the
  stale references gracefully if mid-flight).
* Esc cancels the rename without filesystem side effects.
* Rename refuses cleanly on:
  * Target path collision (existing file at the new path).
  * Path-traversal attempts (`../` escape).
  * Invalid filename characters (let chan-drive validate;
    surface the error in the UI).
* Pre-push gate green; new pinned tests cover (a) the
  rename op on chan-drive's side, (b) the chan-server
  route's happy / collision / sandbox-escape paths, (c)
  the SPA wiring.

## How to start

1. Grep `TerminalTab.svelte` (or wherever the terminal
   rename lives) for the rename handler + reusable
   component. Document the trigger + shape in the impl
   note before mirroring.
2. Verify `chan-drive` rename op:
   * If exists: skim its surface and reuse.
   * If missing: add it alongside `write_text` /
     `write_bytes` per the design.md contract (atomic +
     path-sandboxed).
3. Add the chan-server route (likely a sibling to the
   existing files-write route — `PUT /api/files/rename`
   or similar). Standard route auth + sandbox conventions.
4. SPA wiring: file-tab rename trigger + header-band input
   layout + chan-server call + tab-label + file-tree
   refresh.
5. Audit references that might break on rename — links
   in other documents pointing at the renamed path. This
   may be out-of-scope for v1 (just surface the stale
   refs in the graph inspector); flag in the impl note.

## Coordination

* Lane mostly @@FullStackA (SPA + chan-server route
  add). Pure chan-drive op (if needed) may want
  @@Systacean review for the atomic-write seam, but the
  pattern is well-established + small.
* Composes with `fullstack-a-30`'s page-width override —
  the rename input lives in a band SIBLING to the
  `.rich-prompt` / editor content, so its layout is
  independent of the page-width cap.
* Composes with `fullstack-a-1` (file-browser tab name =
  parent-dir-with-slash) — when the active file gets
  renamed, the file-browser tab's parent-dir derivation
  picks up the new name automatically; no extra wiring.
* @@WebtestA verifies on lane-A: happy-path rename +
  collision-refused + Esc-cancel paths.
* Push held for the patch-release commit-grouping cut.

## 2026-05-20 — impl note + ready for review

Three-file change. chan-drive + chan-server already had
everything in place; this lands the SPA UI shape.

### Pre-existing infrastructure (no Rust changes)

* `chan-drive`: `Drive::rename` (line 1027 of `drive.rs`)
  and the richer `Drive::rename_with_link_rewrite` (line
  1093) already provide the atomic-rename + reference-
  rewrite seam this task needed. No new chan-drive op.
* `chan-server`: `POST /api/move` (`api_move` in
  `routes/files.rs`, line 437) wraps
  `rename_with_link_rewrite` with the `tokio::spawn_blocking`
  + `self_writes.note` echo-suppression dance + outcome
  payload. Already wired into the router at
  `lib.rs:804`.
* SPA: `api.move` (`web/src/api/client.ts`) calls
  `/api/move`; `performMove` (in `store.svelte.ts`,
  line 2285) is the load-bearing helper that runs the
  overwrite confirm, the moving-status indicator, the
  `rekeyTabsForRename` re-key pass, the link-rewrite
  status string, and the watcher echo-suppression. Both
  the modal-driven `fileOps.rename` and the new
  inline-rename path go through it.

So the only new code is the inline-rename UX (header
band + state + entry point that bypasses the modal).

### Files touched

* `web/src/state/store.svelte.ts` — new
  `fileOps.renameInPlace(path, next, isDir)` exported
  alongside the existing `fileOps.rename`. Same
  preserveExtension logic + same `performMove` machinery;
  just takes `next` as an argument instead of popping the
  uiPathPrompt modal.
* `web/src/components/FileEditorTab.svelte`:
  * State block: `renameActive` / `renameDraft` /
    `renameInputEl`.
  * `doRename()` rewritten — flips the state on (priming
    `renameDraft = tab.path`) + queueMicrotask focus +
    select-all on the input. No more modal pop.
  * New `commitRename()` / `cancelRename()` /
    `onRenameKeydown` helpers.
  * Header-band markup `{#if renameActive}` block above
    the `{#if tab.fileMissing/.error}` toolbar blocks.
    Lives outside the editor body's
    `--chan-page-max-width` cap.
  * CSS for `.rename-band` + `.rename-label` +
    `.rename-input`.
* `web/src/components/fileRenameBand.test.ts` — NEW,
  six raw-source pins covering: doRename flips state
  (not modal); commit / cancel wiring; keydown binding
  (Enter / Esc); band markup ordering above editor
  toolbars; `width: 100%` + `flex: 1` for the no-cap
  width; `fileOps.renameInPlace` shape in store.

### UX shape

Trigger: tab right-click menu's "Rename File" row
(unchanged from pre-`-a-35`; the row's onclick handler
flips into the new band instead of popping the modal).

Active state:

```
[Rename] [____________current/path/foo.md_______________]
─────────────── editor body (page-width capped) ─────────
```

Input gets the focus + selects the whole path so typing
replaces it. Enter commits via `fileOps.renameInPlace`
(which goes through `performMove` for the overwrite
confirm + link rewrite + watcher echo suppression + tab
re-key). Esc cancels with no filesystem side effect.
Blur cancels — same shape as the terminal's hamburger
input (the rename is an explicit user action; an
accidental tab-away shouldn't commit a half-typed path).

### Acceptance criteria check

* Rename trigger matches the terminal's affordance shape
  (right-click menu's existing Rename row). ✓ Same
  trigger surface; same inline-input commit/cancel
  semantics.
* Rename input renders in a header band above the editor
  content. ✓ Band sits outside the editor-wrap
  (where `--chan-page-max-width` is applied), spans
  the full pane width.
* Enter commits via the chan-server route → chan-drive
  rename op. ✓ Goes through the existing performMove
  → api.move → `POST /api/move` →
  `Drive::rename_with_link_rewrite` chain. Tab label
  updates via `rekeyTabsForRename`; file tree updates
  via `refreshTree`; graph index re-resolves through the
  server's existing watcher pass.
* Esc cancels without filesystem side effects. ✓
  `cancelRename` flips state to false; no API call
  fired.
* Refuses cleanly on:
  * Target path collision → `performMove`'s existing
    overwrite-confirm modal handles it (user can confirm
    or cancel). ✓
  * Path-traversal attempts → chan-drive's sandbox
    rejects via `ChanError`; chan-server returns the
    error → SPA surfaces in `ui.status` (existing
    handler in `performMove`). ✓
  * Invalid filename → same chain. ✓
* Pre-push gate green; new pinned tests cover the SPA
  wiring shape. chan-drive rename op + chan-server
  route are pre-existing and have their own tests.
  ✓

### Composition

* Independent of `-32` / `-33` / `-34`: file editor UI
  concern (`FileEditorTab` chrome) vs chord layer / graph
  inspector / paste handler. No shared file conflicts.
* Composes with `fullstack-a-30`'s page-width override:
  the rename band is a SIBLING of the editor body, not
  a descendant; the page-width cap doesn't apply.
* Composes with `fullstack-a-1` (FB tab name = parent-
  dir + slash): when the active file gets renamed, the
  file-browser tab's parent-dir derivation picks up the
  new name automatically through `rekeyTabsForRename`.
* No regression on the modal-driven `fileOps.rename` —
  it stays exported in case any other surface (FB
  context menu, etc.) wants the modal flow. Today only
  `doRename` in FileEditorTab called it; the new
  inline path replaces that call site.

### v1 scope

Out of scope per the task's v1 framing:

* Audit / refresh stale references in OTHER documents
  that point at the renamed path. `performMove` already
  surfaces a "N links updated" status string post-move
  via chan-drive's link-rewrite pass; that's the v1
  bound. Cross-doc stale-reference highlighting is a
  future enhancement (could surface in graph
  inspector).

### Gate

* vitest **544 / 544** (+6 net from `-34`'s 538, all
  new pins in `fileRenameBand.test.ts`).
* svelte-check 0 errors / 0 warnings across 3977 files.
* npm build clean (existing chunk-size warnings only).
* No Rust changes; cargo gate skipped (chan-drive +
  chan-server pieces were pre-existing).

### Suggested commit subject

```
File editor: inline rename band above page-width cap (fullstack-a-35)
```

Push held for the patch-release commit-grouping cut.
