# @@Frontend task 8

Status: Ready for review.

Goal: Consume the language graph endpoint and expose it from Search Status / Graph.

Relevant links: [[phase-2/journal.md]], [[phase-2/rustacean-3.md]], [[phase-2/backend-4.md]], [[phase-2/request.md]]

Acceptance criteria:

- Search Status Code Report `Graph this` opens the backend language graph at max depth.
- Graph overlay can render language nodes, folder nodes, and language edges.
- Graph overlay has a `language` filter chip.

Test expectations:

- Run `cd web && npm run check`.

Progress notes:

- Backend registered `GET /api/graph/languages` and documented the frozen wire shape in [[phase-2/rustacean-3.md]] and [[phase-2/backend-4.md]].
- Added frontend types and API client support for `LanguageGraphResponse`.
- Added `language` graph mode/filter state and hash persistence.
- Search Status Code Report `Graph this` now opens the language graph with `depth=0` so the backend returns max depth.
- GraphPanel and GraphCanvas can render language nodes, folder nodes, and language edges.

Completion notes:

- Files changed: `web/src/api/types.ts`, `web/src/api/client.ts`, `web/src/state/store.svelte.ts`, `web/src/components/SearchStatusOverlay.svelte`, `web/src/components/GraphPanel.svelte`, `web/src/components/GraphCanvas.svelte`, `phase-2/frontend-8.md`.
- Tests run: `cd web && npm run check` (pass), `cd web && npm test -- --run store` (pass).
- @@Webtest follow-up: fixed remaining type gaps in `store.svelte.ts`,
  `GraphCanvas.svelte`, and `GraphPanel.svelte` while preparing smoke.
- Browser smoke: [[phase-2/webtest-smoke.mjs]] passed against
  the rebuilt shared server; Search Status `Graph this` opened the language
  graph with a visible canvas.
- Known risks: shared-drive language coverage is currently Markdown-only until
  the code-report/source-copy finding in [[phase-2/webtest-2.md]]
  is resolved.
- Commit readiness: ready after frontend review of the Webtest type-gap edits.
