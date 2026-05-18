# syseng-frontend-2: Agent quote insertion caret placement

Owner: @@Syseng.

Status: REVIEW.

Related:

- [request.md](./request.md)
- [journal.md](./journal.md)
- [frontend-1.md](./frontend-1.md)
- [syseng-frontend-1.md](./syseng-frontend-1.md)
- [webtest-1.md](./webtest-1.md)
- [webtest-2.md](./webtest-2.md)

## Role

Frontend implementation/support lane. Load the frontend/webdev skill before
editing.

## Goal

Fix caret placement when opening the Agent from selected editor text with
Cmd+I.

## Bug

When the user selects text from a file and presses Cmd+I to open the Agent, the
selected text is inserted as a quote correctly, but the Agent prompt caret lands
at the beginning of the quote. It should land on the first editable line after
the quote, so the user can immediately type the prompt.

## Acceptance criteria

- Cmd+I with selected editor text opens the Agent with the quote preserved.
- The prompt caret is placed after the quote, on the first line where the user
  should type.
- The fix works for single-line and multi-line selections.
- Existing Agent prompt behavior without a selection is unchanged.
- Add a focused test if the relevant prompt/quote helper is testable.

## Boundaries

- Keep this scoped to Agent quote insertion / caret placement.
- Do not edit path prompt completion while [frontend-b-2.md](./frontend-b-2.md)
  is active.
- Do not edit webtest files except to record validation notes if assigned.

## Test expectations

- `cd web && npm run check`.
- Focused Vitest test if practical.
- Browser validation through @@Webtest or @@WebtestB.

## Progress notes

- 2026-05-16 @@Syseng: Started. Loaded webdev skill. Investigating Agent quote
  insertion path in `web/src/components/InlineAssist.svelte` and editor
  command dispatch.
- 2026-05-16 @@Syseng: Fixed. Root cause: `openAssistant()` seeded
  `assistantOverlay.prompt` with a blockquote but the prompt CodeMirror editors
  mounted/focused at document start. Added a non-persisted one-shot
  `assistantOverlay.promptCaretTarget` when quote prefill is created, consumed
  by `InlineAssist.svelte` after the active prompt editor mounts. Added
  `focusAt(pos)` to both prompt editor components so Wysiwyg and source mode
  can place the caret at the target.
- 2026-05-16 @@Syseng: Added focused store coverage for quote formatting and
  caret-target seeding in `web/src/state/store.test.ts`.

## Completion notes

- Files changed:
  - `web/src/state/store.svelte.ts`
  - `web/src/components/InlineAssist.svelte`
  - `web/src/editor/Wysiwyg.svelte`
  - `web/src/editor/Source.svelte`
  - `web/src/state/store.test.ts`
- Behavior:
  - Cmd+I/open Agent with selected editor text still preserves the blockquote.
  - The prompt caret is moved to the end of the seeded quote block, on the
    blank line where the user should type.
  - No-selection opens do not set a caret target.
- Tests:
  - `cd web && npm run check` passed: 0 errors, 0 warnings.
  - `cd web && npm test -- --run src/state/store.test.ts` passed: 21 tests.
  - `cd web && npm test -- --run` passed: 14 files, 166 tests.
- Browser validation still belongs to [webtest-1.md](./webtest-1.md) or
  [webtest-2.md](./webtest-2.md).

## Commit readiness notes

- Ready for @@Architect / @@Webtest review.
