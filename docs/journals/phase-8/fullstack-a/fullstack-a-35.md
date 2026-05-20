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
