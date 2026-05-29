# @@Frontend task 4

Status: Ready for review.

Goal: Add a `Graph this` action to the Search Status code report.

Relevant links: [[phase-2/request.md]]

Acceptance criteria:

- Search Status Code Report exposes a `Graph this` action.
- The action opens the graph overlay at whole-drive scope.
- The Search Status overlay closes after launching the graph.

Test expectations:

- Run `cd web && npm run check`.

Progress notes:

- Search Status currently loads the whole-drive report via `api.reportPrefix("")`.
- Added a `Graph this` action to the Code Report header.
- Added `openGraphForDrive()` so the action explicitly opens the whole-drive semantic graph instead of inheriting the active editor scope.

Completion notes:

- Files changed: `web/src/components/SearchStatusOverlay.svelte`, `web/src/state/store.svelte.ts`, `phase-2/frontend-4.md`.
- Tests run: `cd web && npm run check` (pass).
- Known risks: visual smoke still needed to confirm placement and launch behavior in browser.
- Commit readiness: ready after review.
