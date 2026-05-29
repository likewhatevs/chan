# @@Frontend task 5

Status: Ready for review.

Goal: Add vertical visual guidance for markdown list indentation while editing.

Relevant links: [[phase-2/request.md]]

Acceptance criteria:

- Markdown list lines show a subtle vertical guide at the list marker column.
- Nested list lines show additional guides for each indent level.
- Existing list editing behavior is unchanged.

Test expectations:

- Add focused unit coverage for list-depth classing.
- Run `cd web && npm test -- --run blocks`.
- Run `cd web && npm run check`.

Progress notes:

- `web/src/editor/decorations/blocks.ts` already emits `cm-md-list-line` line decorations for bullet, ordered, and task list lines.
- Added depth-specific list line classes from leading indentation.
- Added subtle vertical list guides in `Wysiwyg.svelte` for top-level and nested list lines.

Completion notes:

- Files changed: `web/src/editor/decorations/blocks.ts`, `web/src/editor/decorations/blocks.test.ts`, `web/src/editor/Wysiwyg.svelte`, `phase-2/frontend-5.md`.
- Tests run: `cd web && npm test -- --run blocks` (pass), `cd web && npm run check` (pass).
- Known risks: visual smoke still needed to confirm guide contrast across themes.
- Commit readiness: ready after review.
