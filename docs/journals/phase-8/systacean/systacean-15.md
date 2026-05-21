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
