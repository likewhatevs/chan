# @@Frontend task 7

Status: Ready for review.

Goal: Reload the open graph overlay when filesystem watcher events arrive so missing files become ghost nodes and newly indexed files can appear.

Relevant links: [[phase-2/journal.md]], [[phase-2/rustacean-2.md]], [[phase-2/request.md]]

Acceptance criteria:

- The graph overlay reacts to watcher events while open.
- Reloads are debounced so bulk filesystem events do not trigger one graph fetch per event.
- Closed graph overlays do not eagerly fetch.

Test expectations:

- Add focused store coverage for the graph reload signal.
- Run `cd web && npm test -- --run store`.
- Run `cd web && npm run check`.

Progress notes:

- `onWatchEvent()` invalidated `graphData`, but `GraphPanel.svelte` loads `/api/graph` directly, so open graph overlays did not consume that invalidation.
- Added `graphReloadSignal` in the watcher path and a debounced reload effect in `GraphPanel.svelte`.
- Closed graph overlays mark the current signal as seen and do not fetch until opened normally.

Completion notes:

- Files changed: `web/src/state/store.svelte.ts`, `web/src/components/GraphPanel.svelte`, `web/src/state/store.test.ts`, `phase-2/frontend-7.md`.
- Tests run: `cd web && npm test -- --run store` (pass), `cd web && npm run check` (pass).
- Known risks: browser smoke still needed for delete-while-open and create-while-open graph behavior.
- Commit readiness: ready after visual smoke/review.
