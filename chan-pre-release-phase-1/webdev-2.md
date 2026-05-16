# webdev-2

## Scope

Frontend graph/file-browser entry points from `request.md`:

- File Browser right-click on a file or directory should offer `Graph this`.
- Graph overlay should preserve folder scopes in the scope dropdown.
- File-scoped graph views should offer the file's folder as a convenient scope option.
- Folder-scoped graph views should offer the parent folder as a convenient broader scope option.

## Changes

- `web/src/components/FileTree.svelte`
  - Added `Graph this` to row context menus.
  - Files route to `openGraphForFile(path)`.
  - Directories route to the new `openGraphForDirectory(path)`.

- `web/src/state/store.svelte.ts`
  - Added `openGraphForDirectory(path)`.
  - Extended `availableGraphScopes()` to inject direct `dir:<path>` scopes that were opened from the file browser.
  - Added folder/parent-folder shortcut options for direct file and directory graph scopes.

## Verification

- `cd web && npm run check`
  - Passes with 0 errors and 0 warnings.

## Notes

- This uses existing `dir:` graph scope support in `GraphPanel.svelte`; no graph rendering changes were needed.
- Superseded by `webdev-5.md`: File Browser `Graph this` now uses the
  filesystem graph route instead of semantic graph scopes.

## Architect Note

Observed additional unreported dashboard changes in the worktree:

- `web/src/components/SearchStatusOverlay.svelte`
- `web/src/App.svelte`
- `web/src/components/DriveInfoBody.svelte`
- `web/src/components/SearchPanel.svelte`

These move search index status/rebuild and whole-drive report summary
out of the Drive inspector into a Search Status overlay opened from
SearchPanel. `npm run check` is still clean on the current tree.

Remaining webdev-2 gap: `language:<name>` search is not yet documented
or observed in the implementation.

## 2026-05-16 webtest follow-up

Implemented the remaining `language:<name>` search gap in
`web/src/components/SearchPanel.svelte`.

- `language:<name>` queries bypass content BM25 and scan existing
  chan-report per-file rows through `api.reportFile(path)`.
- Matching files render as normal openable document results with the
  reported language and SLOC count.
- 404 report misses are ignored as "not report-indexed"; other API
  failures surface in the Search status line.
- Results are capped at 25 and respect the existing search scope
  predicate.
- Because the file tree is lazy-loaded, `language:<name>` hydrates
  folder listings before scanning per-file report rows; the browser
  smoke found the initial root-only scan bug.

Verification:

- `cd web && npm run check`: pass, 0 errors / 0 warnings.
- `cd web && npm test -- --run`: pass, 6 files / 94 tests.
- `cd web && npm run build`: pass, with existing Vite large-chunk /
  ineffective dynamic import warnings.
- `cargo build --release -p chan`: pass.
- `node chan-pre-release-phase-1/webtest-smoke.mjs`: pass.

Status: REVIEW.
