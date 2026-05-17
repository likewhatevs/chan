# architect-syseng-frontend-1: Syseng frontend lane idle after image-guide fix

Owner: @@Architect.

Status: REVIEW.

Related:

- [journal.md](./journal.md)
- [syseng-frontend-1.md](./syseng-frontend-1.md)
- [syseng-frontend-2.md](./syseng-frontend-2.md)
- [webtest-2.md](./webtest-2.md)

## Summary

@@Syseng's active frontend lane tasks are now in REVIEW.

## Per-task status

- **[syseng-frontend-2.md](./syseng-frontend-2.md)** — REVIEW.
  Agent Cmd+I selected text now opens Agent quote insertion with the caret
  placed after the inserted quote.
- **[syseng-frontend-1.md](./syseng-frontend-1.md)** — REVIEW.
  WebtestB did not reproduce the cursor-height or stale selection residuals,
  but did provide a precise repro for image-height list guide bars on list
  lines containing markdown images. Fixed that reproduced issue only by
  marking image-bearing list lines and capping their guide bar to a text-height
  segment.

## Verification

- `cd web && npm run check` — pass, 0 errors / 0 warnings.
- `cd web && npm test -- --run src/editor/decorations/blocks.test.ts` — pass,
  1 file / 6 tests.
- `cd web && npm test -- --run` — pass, 14 files / 168 tests.

## Asks back to @@Architect

- Update the journal dispatch for [syseng-frontend-1.md](./syseng-frontend-1.md)
  to REVIEW.
- Route browser validation of the image-guide cap to @@Webtest / @@WebtestB
  against `projects/phase3/list-image.md`.
- Decide ownership for [BUG-FE2-A](./webtest-2.md#bug-fe2-a-confirmed-in-second-look-smoke)
  and the Esc-in-find-bar overlay-close issue from [webtest-2.md](./webtest-2.md);
  @@Syseng has not changed those because they are outside
  [syseng-frontend-1.md](./syseng-frontend-1.md)'s image-residual scope.

## Idle and ready

@@Syseng is idle and available for the next focused assignment.
