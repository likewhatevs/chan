# @@Frontend task 3: close confirmation, per-window state, editor scroll

Owner: @@Frontend
Status: REVIEW
Depends on: [frontend-1](./frontend-1.md), [frontend-2](./frontend-2.md)
Coordinates with: [webtest-2](./webtest-2.md) for reload and shortcut smoke.

## Goal

Land the wave-2 frontend bug fixes from the journal:

* Closing a tab with unsaved file edits or a live terminal session prompts
  for confirmation. Browser/window reload remains uninterrupted.
* chan-desktop windows keep distinct pane/tab state across reloads.
* Editor scroll behavior does not jump when the cursor is near the top of a
  screen-sized page.

## Acceptance criteria

* Tab-close paths prompt before closing dirty file tabs.
* Tab-close paths prompt before closing live terminal tabs.
* Reload/pagehide/session flushing does not show a blocking confirmation.
* Session persistence uses the chan-desktop per-window identifier instead of
  sharing one `default` session across all windows. Initial plumbing is in
  [backend-2](./backend-2.md); @@Frontend should review it with the rest of
  this task's state-management changes.
* Editor cursor visibility logic only scrolls when the caret is outside the
  useful viewport margin; cursor movement near the top of a screen-sized page
  must not force the page to jump.

## Verification

* `npm --prefix web run check`
* `npm --prefix web test -- --run`
* `npm --prefix web run build`

## Progress

* 2026-05-17 @@Frontend created this missing task file from the journal
  dispatch and started implementation.
* 2026-05-17 @@Backend landed the desktop/web session-key plumbing in
  [backend-2](./backend-2.md): desktop drive windows append `w=<label>`,
  web session API calls use that key, pagehide keepalive uses the same path,
  and browser mode still falls back to `default`.
* Added shared in-app tab-close confirmation for dirty file tabs and live
  terminal tabs, with a force-close option for internal flows that already
  performed their own confirmation.
* Split confirm modal state into `web/src/state/confirm.svelte.ts` so shared
  state code can use the existing modal path without `window.confirm` or a
  store/tabs import cycle.
* Reviewed per-window session persistence: `sessionWindowId()` /
  `sessionPath()` are wired through the web API and covered by
  `web/src/api/client.test.ts`.
* Changed WYSIWYG/source saved-caret restore from center scrolling to nearest
  scrolling so restore does not jump when the caret is already visible near
  the top of a screen-sized page.
* Added `web/src/state/tabs.test.ts` coverage for dirty-file and live-terminal
  close confirmation through the modal state.

## Completion notes

* Verification:
  * `npm --prefix web run check`
  * `npm --prefix web test -- --run`
  * `npm --prefix web run build`
* Build completed with existing Vite chunk-size / ineffective dynamic-import
  warnings, but no errors.
