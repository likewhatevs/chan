# syseng-frontend-1: Frontend lane for editor image-selection residuals

Owner: @@Syseng.

Status: REVIEW.

Related:

- [request.md](./request.md)
- [journal.md](./journal.md)
- [frontend-2.md](./frontend-2.md)
- [frontend-b-2.md](./frontend-b-2.md)
- [webtest-1.md](./webtest-1.md)
- [webtest-2.md](./webtest-2.md)

## Role change

Alex reassigned @@Syseng into a frontend implementation/support lane for the
rest of this phase.

Load the frontend/webdev skill before working. Keep the syseng hardening notes
in [syseng-1.md](./syseng-1.md) closed; this task is frontend work.

## Goal

Own the deferred editor/image-selection residuals from
[frontend-2.md](./frontend-2.md), after @@Webtest / @@WebtestB produce a
repro:

- cursor height inherited from an image on the previous line;
- image-line guide bars breaking around embedded images;
- stale blue selection rectangles around image/list blocks.

## Boundaries

- Do not edit path prompt completion files while [frontend-b-2.md](./frontend-b-2.md)
  is active.
- Do not edit graph/resource-color work from [frontend-3.md](./frontend-3.md).
- Do not restart the webtest service; coordinate through [webtest-1.md](./webtest-1.md)
  or [webtest-2.md](./webtest-2.md).
- Read current dirty work before editing and do not revert changes from other
  agents.

## Acceptance criteria

- Wait for or create a precise browser repro before changing code.
- Fix only the reproduced editor/image-selection issue(s).
- Add focused CodeMirror/editor tests where practical.
- Record files changed, tests run, residual risks, and commit readiness here.

## Test expectations

- `cd web && npm run check`.
- Focused Vitest tests for any helper/plugin behavior.
- Browser validation through @@Webtest / @@WebtestB.

## Progress notes

- 2026-05-16 @@Syseng: Started frontend support lane. Loaded webdev skill.
  Waiting on a precise browser repro from [webtest-1.md](./webtest-1.md) or
  [webtest-2.md](./webtest-2.md) before editing image/cursor/list-guide
  residuals.
- 2026-05-16 @@Syseng: Rechecked [webtest-2.md](./webtest-2.md). WebtestB could
  not reproduce the cursor-height-after-image issue and verified the image
  selected ring / selection rectangles clear on caret or selection movement.
  WebtestB did produce a precise repro for image-containing list lines: the
  line box is 159-208 px tall and `.cm-md-list-line::before` was pinned
  `top: 0; bottom: 0`, so the vertical guide rendered image-height/chunky
  during the 1.5s guide grace window.
- 2026-05-16 @@Syseng: Fixed the reproduced list-guide residual only. List line
  decorations now add `cm-md-list-line-image` only for list lines containing
  markdown image syntax, and Wysiwyg CSS caps that guide bar to a
  text-height, bottom-anchored segment. Normal list lines keep the existing
  full-line guide behavior.
- 2026-05-16 @@Syseng: Added focused helper coverage for image-bearing list-line
  classing, including ordinary link and escaped-image non-matches.

## Files changed

- `web/src/editor/decorations/blocks.ts`
- `web/src/editor/decorations/blocks.test.ts`
- `web/src/editor/Wysiwyg.svelte`

## Tests run

- `cd web && npm run check` — pass, 0 errors / 0 warnings.
- `cd web && npm test -- --run src/editor/decorations/blocks.test.ts` — pass,
  1 file / 6 tests.
- `cd web && npm test -- --run` — pass, 14 files / 168 tests.

## Residual risks

- No code change for cursor height inherited from an image on the previous
  line: WebtestB could not reproduce it in the seeded fixture; lines after
  images measured normal text height.
- No code change for stale image/list selection rectangles in this task:
  WebtestB verified image `data-selected` clears on caret movement and
  synthetic selection rectangles clear after drag-selecting across images.
- Browser validation of the guide cap should happen in the next Webtest pass
  against `projects/phase3/list-image.md`.

## Commit readiness notes

- Ready for review; do not commit without Alex approval.
