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

## 2026-05-22 — implementation + commit readiness (option (c) fold-into-`-16`)

Architect routed option (c) hybrid (chan-report bucket + graph composition via existing chan-drive `FileClass`). Per the architect's "your call on scope" framing on the graph-indexer composition: **folded into `-16`** because the composition is mechanical via the existing `/api/report/file` endpoint — chan-report's `bucket` field flows through unchanged once exposed on `FileStats`. The frontend (already shipped G6 colours via `fullstack-a-51`/`362aa96`) can read the bucket directly from existing report responses. No graph-route edits needed in this commit.

### Changes

* **`crates/chan-report/src/summary.rs`** (+37 lines): new `FileBucket` enum (`Markdown` / `SourceCode { language: String }`) with serde tag-style JSON shape. New `bucket: Option<FileBucket>` field on `FileStats`, marked `#[serde(default, skip_serializing_if = "Option::is_none")]` so pre-`-16` JSONL loads cleanly + the field stays absent in serialized output when None. SCHEMA_VERSION stays at 1 (additive change).
* **`crates/chan-report/src/count.rs`** (+22 lines): new `classify_bucket(language: LanguageType) -> FileBucket` helper. `LanguageType::Markdown` → `FileBucket::Markdown`; everything else tokei recognizes → `FileBucket::SourceCode { language: tokei.name() }`. `count_file_impl` populates the bucket alongside the existing language/stats fields.
* **`crates/chan-report/src/lib.rs`** (+1 line): re-export `FileBucket` from the crate root.
* **`crates/chan-drive/src/lib.rs`** (+1 line): re-export as `ReportFileBucket` alongside the existing `ReportFileStats`/`ReportLanguageStats` aliases.
* **`crates/chan-server/src/routes/graph.rs`** (+1 line): test-helper `report_file()` updated to add `bucket: None` (matches the new struct shape; the helper builds synthetic ReportFileStats values for graph route tests).
* **`crates/chan-report/tests/integration.rs`** (+107 lines): 4 new tests covering the bucket population + JSONL round-trip + backward-compat. Existing 24 tests still pass.

### What lives in chan-report vs chan-drive (option (c) separation)

* **chan-report `FileBucket`** = source-code-shaped axis. Two variants: `Markdown` (G6 orange) + `SourceCode { language }` (G6 royalblue, with per-language metadata for grouping/display). Populated by tokei language detection; lives on files chan-report tracks (those with a recognized language).
* **chan-drive `FileClass`** = IO-contract axis (unchanged from systacean-1+). `EditableText` / `Text` / `Image` / `Pdf` / `Other`. The graph indexer's existing `chan_drive::classify()` call site (graph.rs:591 `is_media_graph_path`, inspector.rs:147 etc.) carries the non-source classification for media + binary + other.
* **Graph indexer composes** at render time: when a node has a chan-report bucket (consultable via `/api/report/file?path=...`), it uses that for the G6 colour (markdown vs source code); when not (binary/media/non-tracked), it falls back to the existing `FileClass`-based colour mapping.

No chan-drive or graph-route code change required for this composition because the frontend already reads `/api/report/file` for inspector data; the bucket field is now automatically present in those responses.

### Schema-compat invariant

JSONL files written BEFORE `-16` don't have the `bucket` field. The new test `file_bucket_absent_in_old_jsonl_loads_as_none` synthesizes such an old row + asserts `load_jsonl` reads it cleanly with `bucket: None`. SCHEMA_VERSION stays at 1; no migration needed.

### Tests (4 new in `crates/chan-report/tests/integration.rs`)

* `file_bucket_is_markdown_for_md_files` — `notes/intro.md` → `Some(FileBucket::Markdown)`.
* `file_bucket_is_source_code_for_known_languages` — `.rs` / `.py` / `.ts` / `.toml` → `SourceCode { language: "Rust" / "Python" / "TypeScript" / "TOML" }`. Pins the language-string contract consumers depend on for grouping.
* `file_bucket_round_trips_through_jsonl` — write_jsonl → load_jsonl preserves bucket field across serialization.
* `file_bucket_absent_in_old_jsonl_loads_as_none` — backward-compat with pre-`-16` files.

### Pre-push gate

All green at HEAD (post-architect routing):

* `cargo fmt --check` — clean (after applying fmt for the new tests + the `pub use summary` re-export).
* `cargo clippy --all-targets -- -D warnings` — clean (`bucket: None` added to the chan-server graph route's test helper to satisfy the new field).
* `cargo test` (workspace) — chan-report `24 passed / 0 failed / 0 ignored`; chan-server `205 passed`; all other crates green.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features` — green.
* `cd web && npm run check` — 0 errors / 0 warnings / 3994 files.
* `cd web && npm test -- --run` — 685 / 685 passed (64 files).

### Files

| File                                         | +    | -  |
|----------------------------------------------|------|----|
| `crates/chan-report/src/summary.rs`          | +37  | 0  |
| `crates/chan-report/src/count.rs`            | +22  | -1 |
| `crates/chan-report/src/lib.rs`              | +3   | -1 |
| `crates/chan-report/tests/integration.rs`    | +107 | 0  |
| `crates/chan-drive/src/lib.rs`               | +3   | -3 |
| `crates/chan-server/src/routes/graph.rs`     | +1   | 0  |

Plus this task tail append + outbound poke. Foreign files in dirty tree stay un-staged per shared-worktree discipline.

### Suggested commit subject

```
chan-report: add FileBucket (Markdown / SourceCode { language }) on FileStats (systacean-16)
```

### Smoke plan

Atomic audit-commit pattern + push to fresh `systacean-16-smoke` branch + dispatch CI. Expected green across Ubuntu + macOS (additive change; bucket field is backward-compat).

### Sequencing after `-16` lands

`-12` (tauri-plugin-updater verify) is the only remaining queued item, parked on a fresh @@Alex runtime-permission ask. If @@Alex hasn't surfaced a new permission window, the systacean queue is empty post-`-16`.

Holding for @@Architect commit clearance + smoke-branch authorization.
