# systacean-16 — chan-report file-classification buckets (markdown / source / binary / media)

Owner: @@Systacean
Cut: 2026-05-21 by @@Architect
Status: dispatched

## Goal

Add explicit file-classification buckets to
`chan-report`: markdown, source code, binary, media.
These buckets feed the graph overhaul's new colour
scheme (G6) and language-node features (G7 + G8).

## Background

Locked design context:
[`../architect/graph-overhaul-plan.md`](../architect/graph-overhaul-plan.md)
§"Cross-cutting prereqs" — "chan-report
file-classification buckets". Today's chan-report
does language detection via tokei but doesn't expose
the markdown / source / binary / media split as an
explicit dimension. The graph overhaul needs that
split:

* **markdown** → orange node colour (G6).
* **source code** → royalblue node colour (G6).
* **binary** → grey node colour (G6).
* **media** → purple node colour (G6, unchanged).

Plus G7/G8's language-dir relationships:

* Language nodes (e.g. "Rust", "TypeScript", "Python")
  connect to directories containing source files of
  that language.
* "Graph from language" plots first-depth dirs
  containing files of that language.

Both need a confident "this file is source code in
language X" classification, not just "file has
extension .rs". The classification should also catch
edge cases (vendored binaries with source-like
extensions, generated files, etc.).

## Acceptance criteria

* chan-report classifies every file into one of:
  `Markdown`, `SourceCode { language }`, `Binary`,
  `Media`, `Other`. The bucket is exposed alongside
  the existing per-file language detection.
* Classification heuristic uses:
  * File extension (primary signal).
  * MIME-type detection (tokei or file's
    `infer` crate; implementer picks).
  * Content sniffing (for ambiguous cases like
    plain-text vs binary).
* The bucket is included in `.chan/report.jsonl`
  entries.
* Public API / chan-server endpoint surfaces the
  bucket alongside the existing per-file stats.
* Tests cover: classification correctness for a
  fixture tree with mixed types (markdown, Rust,
  TypeScript, Python, JPG, PNG, MP4, binary .so,
  vendored .gen.rs).
* Pre-push gate green.

## How to start

1. Read `crates/chan-report/README.md` + `src/lib.rs`
   for the current language-detection path.
2. Design the bucket enum + classification function.
3. Decide where the classification happens (during
   file ingest, or at query time). Recommend ingest-
   time so the bucket lives in `.chan/report.jsonl`
   for free.
4. Audit existing chan-report code for any vestigial
   bucket-like classification (file-class detection
   may already exist partially).
5. Wire tests against a fixture tree.

## Coordination

* @@Systacean lane.
* Parallel to `systacean-15` (cross-dir aggregation);
  both extend chan-report's surface but the
  aggregation work doesn't depend on the buckets.
  Sequence the commits in either order.
* @@FullStackA will consume the bucket in the graph
  overhaul wave (G6, G7, G8); API contract is theirs
  to validate at integration time.
* Pre-push gate green before commit clearance.
* Append "Commit readiness" + poke @@Architect when
  ready.

## Numbering

This is `-16`. See `-15` for the broader wave
numbering note.
