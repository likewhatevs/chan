# backsystacean-3: inspector payload and chan-report aggregation

Owner: @@Backsystacean
Status: REVIEW

## Goal

Land the inspector payload shape from [architect-2](./architect-2.md):
drive and directory inspectors gain language breakdown + file kind
counts; text-file inspector carries chan-report data; markdown
inspector keeps its current content plus the frontmatter kind badge;
binary inspector stays minimal.

## Relevant links

* Request: [request.md](./request.md) architectural cleanups section.
* Design memo: [architect-2.md](./architect-2.md) (Markdown vs text
  vs binary, Language binding sections).
* chan-report: `crates/chan-report/`
* chan-drive integration with chan-report:
  `crates/chan-drive/src/` (search for `report`).
* Existing inspector routes in chan-server:
  `crates/chan-server/src/routes/{files, drive}.rs`.

## Scope

### Aggregation contract

* chan-report aggregates per-file report data into per-directory
  rollups. Use byte count for stable ordering.
* Drive rollup: full breakdown of languages, file kinds, and totals
  (bytes, file count, directory count).
* Directory rollup: same shape, scoped to the subtree.
* File payload: language chip + report data for text files; nothing
  beyond size / kind / mtime for binary.

### Route surface

* Extend `/api/files/<path>` (or add a sibling endpoint) so the
  inspector can fetch the payload in one round-trip per inspected
  entity.
* Use the classifier output from
  [backsystacean-2](./backsystacean-2.md) as the base shape; this
  task layers report data on top.
* Cache invalidation rides chan-drive's existing watcher path; no
  new event channel.

### Aggregation implementation notes

* For first pass, aggregate on demand from the indexer state rather
  than persisting roll-ups. Measure the cost on the test service
  (`/private/tmp/chan-test-phase6`) and decide whether a cache is
  needed before commit.
* If aggregation is expensive at drive scope, add a cached roll-up
  that invalidates on watcher events. Flag the decision in the task
  progress notes.

## Out of scope

* File classifier itself (in
  [backsystacean-2](./backsystacean-2.md)).
* Frontmatter kind ladder (in
  [backsystacean-4](./backsystacean-4.md)).
* Terminology codemod (in [backsystacean-5](./backsystacean-5.md)).

## Acceptance criteria

* Inspector payload returns the new shape for drive, directory,
  markdown, text, and binary entries.
* Aggregation correctness verified on a small fixture drive.
* No regressions in existing inspector consumers (file browser,
  search) until @@Frontend updates them.

## Tests

* `cargo test -p chan-drive` for the aggregation path.
* `cargo test -p chan-server` for the route surface (a focused test
  drive with mixed languages + a binary).
* Pre-push gate (fmt + clippy + no-default-features + workspace
  test) green.

## Review and hardening

* @@Backsystacean self-review for the aggregation cost on a larger
  drive (use the phase-5 80-file VCS fixture as a smoke benchmark).
* @@Architect to verify the payload shape matches
  [architect-2](./architect-2.md) before commit.

## Dependencies

Coordinates with [backsystacean-2](./backsystacean-2.md) on the
inspector payload shape; both crates land together.

## Progress notes

* 2026-05-18: Added byte counts to `chan-report` language and total
  rollups; language summaries now sort by byte count, then file
  count, then name.
* 2026-05-18: Added `GET /api/inspector?path=<rel>` in
  `chan-server`. Empty / missing `path` returns the drive-root
  payload. The payload combines `PathClass`, report file stats,
  report rollups, subtree byte/file/dir totals, and file-kind
  counts.
* 2026-05-18: Added web API types for `InspectorPayload` and
  extended report rollup types with optional `bytes` for forward /
  backward compatibility.
* 2026-05-18: Aggregation is on-demand. Small fixture tests complete
  quickly; no cache added in this pass. If Webtest sees drive-scope
  latency on the larger fixture, cache invalidation can ride the
  existing watcher/report refresh path.

## Completion notes

Files changed:

* `crates/chan-report/src/lib.rs`
* `crates/chan-report/src/summary.rs`
* `crates/chan-report/tests/integration.rs`
* `crates/chan-server/src/lib.rs`
* `crates/chan-server/src/routes/inspector.rs`
* `crates/chan-server/src/routes/mod.rs`
* `web/src/api/types.ts`

Payload shape:

* `kind`: `drive | directory | markdown | text | media | binary | special`
* `path_class`: classifier from [backsystacean-2](./backsystacean-2.md)
* `report_file`: per-file chan-report row for markdown/text files
* `report_summary`: rollup for drive/directory scopes
* `subtree`: file count, directory count, bytes, and file-kind counts

Verification:

* `cargo test -p chan-report`
* `cargo test -p chan-server`
* `cargo test -p chan-drive report`
* `npm run check`
* `cargo fmt --check`
* `cargo build --no-default-features`
* `cargo clippy --all-targets -- -D warnings`

Ready for @@Architect payload review and @@Frontend consumption in
[frontend-4](./frontend-4.md).
