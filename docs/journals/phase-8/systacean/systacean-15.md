# systacean-15 — chan-report cross-directory aggregation

Owner: @@Systacean
Cut: 2026-05-21 by @@Architect
Status: dispatched

## Goal

Add per-directory aggregated rollups to `chan-report`.
Today's crate produces per-file language detection +
SLOC + COCOMO with per-language drive-wide roll-ups.
Add a per-directory aggregation layer so the graph
overhaul's directory inspector (G3) can render
"aggregated stats for this directory" without
re-walking the whole `.chan/report.jsonl` per click.

## Background

Locked design context:
[`../architect/graph-overhaul-plan.md`](../architect/graph-overhaul-plan.md)
§"Cross-cutting prereqs" — "chan-report
cross-directory aggregation". The idea originates from
[`../architect/report-extensions-ideas.md`](../architect/report-extensions-ideas.md)
"Other ideas worth considering" where it sat as a
candidate Round-3 extension. The graph overhaul pulls
it forward into Round 2.

Today's chan-report shape per
`crates/chan-report/README.md`: "Per-file language and
SLOC report for a directory tree, with per-language
roll-ups and a Basic COCOMO summary on top." Per-drive
state at `.chan/report.jsonl`.

What's missing for G3: a query like "give me the
aggregated stats for `<dir>` (file count, SLOC by
language, COCOMO summary)" returnable without rescan.

## Acceptance criteria

* chan-report extended with a per-directory
  aggregation index (in-memory cache or persisted
  alongside `.chan/report.jsonl`; implementer picks).
* Aggregation surfaces the same metrics as the
  current drive-wide roll-up (file count, SLOC by
  language, COCOMO) but scoped to a directory path.
* Aggregation is incremental — updates as files
  change without a full rebuild.
* Public API (or chan-server endpoint) returns the
  aggregation for a given directory path.
* Tests cover: aggregation correctness against a
  known fixture tree, incremental update on file
  add/remove/edit, deep-directory edge cases (root,
  empty dirs, large dirs).
* Pre-push gate green.

## How to start

1. Read `crates/chan-report/README.md` +
   `crates/chan-report/src/lib.rs` (and `design.md` if
   present) for the current shape. Audit existing
   per-language roll-up implementation as the natural
   pattern to extend.
2. Design the per-directory cache shape — in-memory
   `HashMap<DirPath, AggregatedStats>` is the simple
   answer; persistence is optional for v1.
3. Wire incremental updates from the existing chan-
   report file-change ingest.
4. Expose via a chan-server route (probably
   `GET /api/reports/dir?path=<dir>` or similar; align
   with existing chan-report HTTP shape) for the SPA
   to consume.
5. Tests.

## Coordination

* @@Systacean lane (chan-report + chan-server route).
* @@FullStackA will consume the API in the graph
  overhaul wave (G3); the API contract is theirs to
  validate at integration time.
* Pre-push gate green before commit clearance.
* Append "Commit readiness" + poke @@Architect when
  ready.

## Numbering

Highest committed `systacean-N` is `-13`; `-14` is in
commit (silent-wedge); `-12` parked. This is `-15`;
`-16` (file-classification buckets) fans out
alongside.

## 2026-05-21 — implementation + commit readiness

Implemented per the architect-approved cache shape
(`HashMap<DirPath, DirEntry>` updated incrementally via
ancestor-chain walks; `dir_report()` is O(1) per query).

### Design

* Internal `DirEntry` struct on `Index`: scalar totals
  (files / bytes / code / comments / blanks / complexity)
  plus a per-language `HashMap<String, LanguageStats>`
  sub-rollup.
* Drive root is the empty-string key. Every file
  contributes to the root + each of its directory
  ancestors. `ancestor_dirs("a/b/c.rs")` →
  `["", "a", "a/b"]`. The file's own path is not a
  directory key.
* `Index::dir_report(dir, params)` returns `Option<Report>`:
  `None` when no tracked file lives at or under `dir`.
  The returned `Report` carries `totals`, `by_language`
  (same sort order as the global rollup: desc bytes, desc
  files, asc name), and `cocomo`; `files` is left empty
  (dir queries don't enumerate per-file rows).
* Path normalization: leading + trailing slashes are
  stripped, so `"src"`, `"src/"`, `"/src"`, `"/src/"`
  all map to the same key.
* Persistence: NONE (per the task's "implementer picks";
  in-memory was the right call). `load_jsonl` calls
  `rebuild_dirs()` to seed the cache from the file rows.
  JSONL schema unchanged.

### Incremental delta logic

* `update(rel)`:
  - Filter rejects → `remove_file_row(rel)` (subtracts
    from ancestors if a row existed) → `Removed`/`Skipped`.
  - New stats == old → `Unchanged` (cache untouched —
    no-op writes do NOT drift the cache).
  - Old row exists → subtract old, add new (delta path).
  - No old row → just add (insert path).
* `remove(rel)`: unconditional row drop + subtract.
* `rename(from, to)`: internally `remove_file_row(from)`
  then `update(to)`. The old ancestor chain unwinds via
  remove; the new chain accumulates via update. Cross-
  directory renames are handled correctly (test
  `rename_moves_stats_between_ancestor_chains`).

### Public API surface

* `chan_report::Index::dir_report(&self, dir: &str, params: &CocomoParams) -> Option<Report>`
* `chan_drive::Drive::report_for_dir(&self, dir: &str) -> Result<Option<Report>>`
* `GET /api/report/dir?path=<rel>` — 200 with the
  existing `PrefixReport` JSON shape (totals + by_language
  + cocomo) when tracked, 404 when no file lives at or
  under `path`. Empty `path` = drive root.

The dir endpoint shares its response shape with
`/api/report/prefix` so the SPA's existing fetch +
deserialize wiring works unchanged; the SPA picks which
endpoint based on whether it has strict-directory
semantics (dir) or prefix-string semantics (prefix). The
existing prefix endpoint is unchanged.

### Files

| File                                       | +     | -    |
|--------------------------------------------|-------|------|
| crates/chan-report/src/lib.rs              | +231  | -24  |
| crates/chan-report/tests/integration.rs    | +269  | 0    |
| crates/chan-drive/src/drive.rs             | +18   | 0    |
| crates/chan-drive/src/report.rs            | +12   | 0    |
| crates/chan-server/src/routes/report.rs    | +39   | -10  |
| crates/chan-server/src/routes/mod.rs       | +1    | -1   |
| crates/chan-server/src/lib.rs              | +3    | -2   |
| **Total (production + tests)**             | +573  | -37  |

Plus this task tail append.

### Tests (8 new in `crates/chan-report/tests/integration.rs`)

* `dir_report_root_matches_all_scope` — root cache
  totals + by_language match `Scope::All`. The
  load-bearing invariant.
* `dir_report_subdir_matches_prefix_scope` — subdir
  cache matches `Scope::Prefix` for the same path.
* `dir_report_handles_trailing_and_leading_slashes` —
  `"src"`, `"src/"`, `"/src"`, `"/src/"` all key the
  same cache entry.
* `dir_report_missing_dir_is_none` — untracked dir →
  `None` (so chan-server returns 404 cleanly).
* `dir_report_root_aggregates_multiple_languages` —
  per-language sub-rollup surfaces all languages
  present.
* `incremental_insert_updates_ancestor_chain` — fresh
  insert propagates to every ancestor.
* `incremental_remove_clears_ancestor_chain_when_last_file_leaves`
  — empty dir entries get dropped from the cache so
  the map matches "dirs with tracked files".
* `incremental_update_applies_delta_to_ancestors` —
  delta logic: file grows, dir totals grow by the same
  amount.
* `incremental_update_unchanged_does_not_drift_ancestors`
  — `Unchanged` outcomes do NOT touch the cache (no-op
  writes don't compound).
* `rename_moves_stats_between_ancestor_chains` —
  cross-directory rename unwinds from old chain + adds
  to new chain.
* `deep_directory_chain_propagates` — 5-level deep
  file; every ancestor (root through `a/b/c/d/e`)
  reports the same single-file stats.
* `dir_report_survives_jsonl_roundtrip` — `load_jsonl`
  rebuilds the cache from file rows; round-trip
  equality on totals + by_language.

### Pre-push gate

All green at HEAD (`22fd878`):

* `cargo fmt --check` — clean.
* `cargo clippy --all-targets -- -D warnings` — clean.
* `cargo test` (workspace) — all green. 20/20 in
  chan-report (12 existing + 8 new). 205/205 in
  chan-server. 29/29 in chan-drive. No drift in any
  other crate.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`
  — green (the `-s-8` follow-up `c1e9c41` still
  unblocks this from v0.11.1).
* `cd web && npm run check` — 0 errors / 0 warnings
  / 3987 files.
* `cd web && npm test -- --run` — 588 / 588 passed
  (58 files).
* `cd web && npm run build` — green. Only pre-existing
  chunk-size + ineffective-dynamic-import warnings
  preserved from v0.11.2 baseline.

### Suggested commit subject

```
chan-report: maintained per-directory aggregation cache + /api/report/dir (systacean-15)
```

### Shared-worktree discipline

Working tree has 13 modified files; mine are exactly 7:

```
crates/chan-drive/src/drive.rs
crates/chan-drive/src/report.rs
crates/chan-report/src/lib.rs
crates/chan-report/tests/integration.rs
crates/chan-server/src/lib.rs
crates/chan-server/src/routes/mod.rs
crates/chan-server/src/routes/report.rs
```

The other 6 (`.github/workflows/ci.yml`,
`.github/workflows/release.yml`,
`docs/journals/phase-8/alex/event-webtest-{a,b}-architect.md`,
`docs/journals/phase-8/ci/ci-11-post-mortem.md`,
`docs/journals/phase-8/ci/ci-12.md`) belong to @@CI /
@@WebtestA / @@WebtestB lanes. Stay un-staged.

Plus this task tail append to be staged with the commit.

Pre-commit `git diff --staged --stat` audit will confirm
exactly 8 paths staged (7 source + 1 task file).
Post-commit `git show --stat HEAD` confirms the same.

### Coordination notes

* `systacean-16` (file-classification buckets) is the
  natural follow-up; the two were dispatched
  alongside per the architect's wave-2 fan-out.
  Independent of `-15`; commit order doesn't matter.
* `@@FullStackA` consumes this API for the graph
  overhaul wave's directory inspector (G3). Response
  shape matches `/api/report/prefix` so the existing
  fetch + deserialize pattern works unchanged; the
  client picks between endpoints based on
  prefix-string vs strict-directory semantics.

Holding for @@Architect commit clearance.
