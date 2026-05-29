# @@Backend task 5: chan-report reconcile follow-up

Owner: @@Backend
Status: Backlog / non-blocking

## Goal

Track the chan-report freshness gap found during [[phase-2/webtest-2.md]]:
pre-existing source files copied into a drive can be absent from
`/api/report/prefix` when `report.jsonl` was loaded before those files existed
or when filesystem events arrived as coarse directory events.

## Relevant Links

- [[phase-2/architect-8.md]]
- [[phase-2/webtest-2.md]]
- [[phase-2/backend-4.md]]

## Finding

The report fan-out is wired through `Drive::watch` in `chan-server`, but
chan-report does not currently reconcile the live filesystem on load or on
coarse directory-create events. A stale persisted report can therefore omit a
source tree that already exists on disk before the server starts.

## Acceptance Criteria

- On report load, reconcile persisted `report.jsonl` against current on-disk
  reportable files.
- On coarse directory watcher events, recursively reconcile files under that
  directory or schedule a bounded full report refresh.
- Preserve existing report API shapes.
- Add regression coverage for a source tree that exists on disk before server
  startup but is absent from persisted report state.

## Phase-2 Disposition

Non-blocking follow-up. Phase 2 smoke can use the workaround recorded in
[[phase-2/architect-8.md]].

