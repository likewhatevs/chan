# webdev-3

## Scope

Frontend search-status dashboard work from `request.md`:

- Add a new overlay for search index status.
- Add a button next to the Search overlay's scope picker to open it.
- Move search-index status/rebuild out of the File Browser drive inspector.
- Include chan-report progress/data, at least SLOC by language.

## Changes

- `web/src/components/SearchStatusOverlay.svelte`
  - New window-level overlay.
  - Shows live search index state from `indexStatus`.
  - Polls index status while open so rebuild/delete/recreate progress is visible without waiting for the global slow poller.
  - Provides `Rebuild index`.
  - Loads whole-drive `api.reportPrefix("")` and shows totals plus SLOC by language.

- `web/src/components/SearchPanel.svelte`
  - Added search-status button beside the scope selector.

- `web/src/state/store.svelte.ts`
  - Added `searchStatusOverlay` state.
  - Added `search-status` to overlay stack and Escape close handling.

- `web/src/App.svelte`
  - Mounted the new overlay and included it in overlay stack synchronization.

- `web/src/components/DriveInfoBody.svelte`
  - Removed the search-index section so the drive inspector no longer duplicates status/rebuild controls.

## Verification

- `cd web && npm run check`
  - Passes with 0 errors and 0 warnings.

## Notes

- Uses existing backend API; no server changes.
- Report loading currently shows the latest completed `chan-report` summary returned by `/api/report/prefix`.
