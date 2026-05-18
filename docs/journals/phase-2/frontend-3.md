# @@Frontend task 3

Status: Ready for review.

Goal: Collapse content search results so multiple matching headings from the same file render as one file result.

Relevant links: [[phase-2/request.md]]

Acceptance criteria:

- Content search rows show at most one section hit per file.
- When a file has multiple section hits, the displayed row uses the strongest hit for that file.
- Search inspector selection remains file-based, not heading-based.

Test expectations:

- Add focused Vitest coverage for the content-hit reducer.
- Run `cd web && npm test -- --run search`.
- Run `cd web && npm run check`.

Progress notes:

- Search content rows are built in `web/src/components/SearchPanel.svelte` from `ContentHit[]`.
- Added `collapseContentHitsByFile()` and wired SearchPanel to collapse `/api/search/content` hits before rendering.
- The reducer keeps the highest-score section per file, using earlier line number as a tie-break.

Completion notes:

- Files changed: `web/src/search/results.ts`, `web/src/search/results.test.ts`, `web/src/components/SearchPanel.svelte`, `phase-2/frontend-3.md`.
- Tests run: `cd web && npm test -- --run search` (pass), `cd web && npm run check` (pass).
- Known risks: backend still returns section-level hits; the frontend now collapses the currently fetched result window.
- Commit readiness: ready after review.
