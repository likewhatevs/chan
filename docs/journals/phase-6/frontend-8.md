# frontend-8: file-browser dismiss on open, focus loading tab

Owner: @@Frontend
Status: REVIEW

## Goal

Make opening a file from the file browser feel responsive: dismiss
the overlay immediately on click and focus the destination tab,
even if the tab is still loading. The tab can render a LOADING
state in place of the editor body while the file fetch completes.

## Symptom today

* Click a file in the file browser.
* The tab opens behind the overlay.
* If the file fetch is slow, the overlay stays in front longer
  than necessary.
* The user is stuck waiting on the overlay rather than seeing the
  loading tab.

## Behavior change

1. On file click in the file browser, dismiss the overlay
   synchronously (no await on the file fetch).
2. Switch focus to the destination tab in the same tick.
3. Tab body renders a LOADING placeholder until the file fetch
   resolves; on success, the editor body replaces the placeholder;
   on failure, the tab surfaces the error in place.
4. Same behavior for graph "Open in this pane" / "Open" actions
   that today wait on the file fetch before dismissing the graph
   overlay.

## Relevant links

* File browser: `web/src/components/FileBrowserOverlay.svelte`,
  `web/src/components/FileTree.svelte`.
* Tab state + loading flags:
  `web/src/state/tabs.svelte.ts`,
  `web/src/state/store.svelte.ts`.
* File editor tab body: `web/src/components/FileEditorTab.svelte`.

## Out of scope

* New global loading indicator. The LOADING state is per-tab.
* Pre-fetching adjacent files. Stays a click-driven fetch.

## Acceptance criteria

* Click-to-open from the file browser dismisses the overlay in
  the same tick.
* Destination tab is focused and shows a clear LOADING state
  during the fetch.
* Graph "Open in this pane" follows the same pattern.
* Error path renders the error in the tab body, not as an
  overlay-blocking toast.

## Tests

* Vitest covering: dismiss-on-click ordering (overlay closed
  before fetch resolves), tab body switches LOADING -> Editor /
  Error based on the fetch outcome.
* `npm --prefix web run check`, `npm --prefix web test -- --run`,
  `npm --prefix web run build` green.

## Review and hardening

* @@Frontend self-review for race conditions if the user clicks
  multiple files in quick succession before the first fetch
  resolves.

## Progress notes

* FileTree double-click open no longer awaits the file fetch before
  dismissing the file-browser overlay.
* The tab state path already created and focused a loading tab
  synchronously before awaiting `api.read`; added regression coverage
  for that ordering.
* FileEditorTab now renders load failures as the tab body instead of
  falling through to an empty editor surface.
* Graph "Open in this pane" actions already dispatch `openInActivePane`
  without awaiting and close the graph overlay synchronously.

## Completion notes

Verification:
* `npm --prefix web run check` passed.
* `npm --prefix web test -- --run` passed: 19 files, 185 tests.
* `npm --prefix web run build` passed with existing Vite warnings.
* Webtest round 4 passed with a delayed `/api/files/<path>` read:
  overlay dismissed immediately, tab focused, loading placeholder
  rendered, then content replaced it.
