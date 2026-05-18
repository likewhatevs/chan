# architect-syseng-frontend-3: Syseng idle after settings layout wiring

Owner: @@Architect.

Status: REVIEW.

Related:

- [journal.md](./journal.md)
- [syseng-frontend-4.md](./syseng-frontend-4.md)
- [backend-3.md](./backend-3.md)
- [frontend-1.md](./frontend-1.md)

## Summary

@@Syseng completed [syseng-frontend-4.md](./syseng-frontend-4.md) and moved it
to REVIEW.

## Completed

- Settings / Layout now shows Standard and Compact, not Tight and Standard.
- Settings writes canonical `standard | compact` values.
- Frontend preference normalization maps legacy `tight` reads to `compact` and
  falls back to `standard` for unknown or missing values.
- Editor density values now match the task: Wysiwyg compact `1.65`, Source
  compact `1.55`, with standard unchanged.

## Verification

- `cd web && npm run check` — pass, 0 errors / 0 warnings.
- `cd web && npm test -- --run` — pass, 14 files / 168 tests.

## Asks back to @@Architect

- Update journal dispatch for [syseng-frontend-4.md](./syseng-frontend-4.md)
  to REVIEW.
- Route browser validation to @@Webtest / @@WebtestB using the steps recorded
  in [syseng-frontend-4.md](./syseng-frontend-4.md).

## Idle and ready

@@Syseng is idle and available for the next focused assignment.
