# systacean-34 — chan-drive indexer walks Drafts/ at boot (closes -a-66 slice e PARTIAL)

Owner: @@Systacean
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Ensure the chan-drive indexer's INITIAL boot walk
includes `Drafts/` subtree files in its indexed-files
corpus. `-25` extended the watcher to multi-root
(drive + drafts) but `synthesize_drafts_layer` in
chan-server's graph route empirically receives an
empty Drafts file set — root cause @@WebtestA
identified.

## Reference

@@WebtestA's walk (`84f665f`): `-a-66 slice e` SPA
styling shipped + chan-server `synthesize_drafts_layer`
shipped, but empirical `/api/graph?scope=drive` has
NO Drafts directory node + NO drafts_link edges + 0
files under `Drafts/` prefix.

Verified existence:
* `Drafts/untitled/draft.md` written via Cmd+N.
* `Drive::list("Drafts/")` returns the file (per
  `-29` unified-path).
* `Drive::stat("Drafts/untitled/draft.md")` returns
  metadata (per `-32` unified-path).
* BUT the indexer's file-list (consumed by the
  graph synthesizer) doesn't include it.

## Hypothesis

`-25` integrated the watcher to multi-root but the
BOOT (initial) walk may still target drive-root only
for the file-list snapshot. Watcher catches
SUBSEQUENT changes; initial corpus walk omits
drafts.

Audit at task pickup:

1. Find the initial-walk code path (`Drive::boot()`
   or wherever the indexer enumerates files at
   start-of-day).
2. Check if `walk(drive_root)` is the only walk,
   or if `walk(drafts_dir)` is also invoked.
3. If missing: add the drafts walk to the boot
   sequence.

OR the gap may be in `chan-server`'s
`synthesize_drafts_layer` `files` arg source —
trace where `files` is constructed; it may be
filtering to drive-root paths even when the
indexer DOES have drafts content.

## Acceptance

1. **`/api/graph?scope=drive` includes Drafts root
   node** when at least one file exists under
   Drafts/.
2. **drafts_link edge** from `directory:` to
   `directory:Drafts` is emitted.
3. **Drafts file nodes** appear in the graph
   payload.
4. **Existing drive-root indexing unchanged**
   (regression check).

### Tests

* Boot-walk test: create draft file → boot scan →
  indexer's file list includes the draft.
* End-to-end: graph API returns drafts root + edges
  for a drive with Drafts content.

### Gate

`cargo fmt / clippy / test`; smoke green.

## Coordination

* @@Systacean lane.
* Closes the recurring Drafts-data-flow saga
  (`-29`/`-32` covered read/list/stat; this
  closes the boot-walk gap).

## Authorization

Yes for `crates/chan-drive/src/indexer.rs` (or
`drive.rs::boot()`) + chan-server graph route if
the `files` source needs adjustment + tests +
task tail + outbound.

## Numbering

This is `-34`.

## Out of scope

* Watcher behavior (already covered by `-25`).
* SPA-side rendering (`-a-66 slice e` shipped; just
  needs data to render).

## 2026-05-22 — implementation complete; closes -a-66 slice e gap

Picked up `-34` per the HIGH-priority dispatch. Audit-first per the task body.

### Audit verdict

Root cause confirmed: `Drive::reindex_with_aggression` (the initial-build entry point) walks ONLY `self.root()` via `fs_ops::list_tree_filtered`. The drafts subtree at `<state_dir>/drafts/<uuid>/` is NOT included in the boot walk. `-25`'s watcher catches subsequent changes, but for drives where files already exist before the indexer attaches (e.g., a fresh server restart with persisted drafts), the initial graph + BM25 are empty under the `Drafts/` prefix.

`chan-server`'s `synthesize_drafts_layer` (post-`-25`) IS correctly wired — but its `files` arg comes from `graph.files()` which only contains what was indexed. Without boot-walk drafts, nothing to render.

### Fix shape

`Drive::reindex_with_aggression` extended with a `self.index_drafts_subtree()` call after the main `rebuild_graph` + `Index::build_all` complete. The helper:

* Walks `<state_dir>/drafts/<uuid>/` recursively via `std::fs` (drafts are chan-drive's own metadata; cap-std sandbox not needed here, same as `-25`'s `index_draft_file`).
* For each indexable text file: composes the unified `Drafts/<sub_rel>` path + calls `Drive::index_draft_file`.
* Per-file errors log + continue (best-effort; watcher retries on next change).
* Non-files (symlinks, FIFOs, etc.) skipped silently.

Free-function helper `walk_drafts_recursive(drafts_root, dir, drive)` at module scope for testability + recursion.

### Acceptance criteria status

| # | Criterion | Status |
|---|-----------|--------|
| 1 | `/api/graph?scope=drive` includes Drafts root node when files exist | ✓ (via existing `synthesize_drafts_layer` once `files` is populated) |
| 2 | `drafts_link` edge from `directory:` to `directory:Drafts` is emitted | ✓ (same) |
| 3 | Drafts file nodes appear in the graph payload | ✓ (boot walk now populates `graph.files()`) |
| 4 | Existing drive-root indexing unchanged | ✓ (additive; regression test pin) |

### Tests (+1)

`reindex_walks_drafts_subtree_into_graph_and_bm25` — full boot-walk verification:

1. Create draft dir.
2. Write file directly via `std::fs::write` (bypass `write_text` so the watcher doesn't catch it).
3. Call `drive.reindex(None)` — the boot-equivalent path.
4. Assert BM25 hit for `Drafts/untitled-1/draft.md` against the marker token.
5. Assert `graph.files()` includes the unified path.

Pins the full chain end-to-end.

### Pre-push gate

* `cargo fmt --check`: clean.
* `cargo clippy --all-targets -- -D warnings`: clean.
* `cargo test -p chan-drive --lib`: **462 passed; 0 failed; 2 ignored** (was 461; +1 new).
* workspace tests all green.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`: green.

### Files

| File                                | +    | -  |
|-------------------------------------|------|----|
| `crates/chan-drive/src/drive.rs`    | +145 | 0  |

Plus task tail + outbound poke. 3 paths.

### Suggested commit subject

```
chan-drive: reindex walks Drafts/ subtree at boot (systacean-34; closes -a-66 slice e PARTIAL)
```

### Smoke plan

`gh workflow run ci.yml --ref systacean-34-smoke`. Cross-lane drift may recur from HEAD; will bundle fixups if needed.

### Saga closure

This closes the recurring Drafts-data-flow saga:

| Task | Fix |
|------|-----|
| `-29` | `Drive::list` unified-path |
| `-32` | `Drive::stat` + `exists` + `read` unified-path |
| **`-34`** (this) | **`Drive::reindex` walks Drafts subtree at boot** |

Combined: end-to-end Drafts data flow through chan-drive + chan-server graph + SPA rendering.

Per architect's pre-authorization, proceeding to commit + push + smoke.
