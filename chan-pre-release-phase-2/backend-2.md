# @@Backend task 2: Collapse content search results per file

Owner: @@Backend
Status: Ready for specialist review

## Goal

Collapse `/api/search/content` results so the search UI receives one entry per
file, using the best ranked heading/snippet for that file.

## Relevant Links

- [[chan-pre-release-phase-2/request.md]]
- [[chan-pre-release-phase-2/backend-1.md]]

## Acceptance Criteria

- Content search returns at most one hit per file path.
- The retained hit for a file is the highest ranked heading/snippet returned by
  `chan-drive`.
- The response shape stays compatible with the current frontend
  `ContentHit` contract.

## Test Expectations

- Add focused Rust coverage for the collapse/ranking behavior.
- Run `cargo test -p chan-server`.

## Progress Notes

- Started after checking phase 2 tasks and finding no new explicit
  @@Backend-owned task file.
- Updated `/api/search/content` to request a wider candidate pool from
  `chan-drive`, collapse hits by path, and keep the first ranked hit for each
  file.
- Kept the existing `ContentHit` response shape unchanged.

## Completion Notes

Changed files:

- `crates/chan-server/src/routes/search.rs`
- `chan-pre-release-phase-2/backend-2.md`

Tests run:

- `cargo test -p chan-server routes::search::tests`
- `cargo test -p chan-server`

Review expectations:

- @@Rustacean should review the route helper/test shape.
- @@Frontend or @@Webtest should smoke the search overlay against a query that
  previously produced multiple rows for the same file.

Commit readiness:

- Ready after review/smoke.
- Known risk: the backend asks for up to 8x candidates, capped at 200 for normal
  small result limits, so search does a little more index work to return fuller
  per-file results.
