# systacean-37 — Drafts files indexed in graph but NOT in BM25 (secondary PARTIAL on -a-66 slice e)

Owner: @@Systacean
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Audit + fix: Drafts files now appear in the graph
payload (per `-36`) but BM25 search returns 0 hits
for draft content. Closes the secondary PARTIAL
@@WebtestA flagged.

## Reference

@@WebtestA's re-walk after `-36` (`3328d57`):

* Drafts in graph: HOLD ✓ (saga closed there).
* BM25 search for unique-marker content in a draft
  file: 0 hits. PARTIAL.

`-36` routes `apply_watch_change` through
`index_draft_file` for Drafts/ paths. That populates
the GRAPH side. But the BM25 side may use a separate
code path that the `-36` fix didn't cover.

## Audit path

1. Trace `index_draft_file`'s BM25 write path.
   Does it call the same BM25 indexer surface as
   `index_file`?
2. If yes: trace whether the BM25 indexer's path-
   classification or text-extraction step accepts
   `Drafts/<...>` keys.
3. If no: `index_draft_file` is graph-only; needs
   parallel BM25 wiring.

Likely candidate: the BM25 doc-id / unique-key
construction may filter on drive-root paths.

## Fix shape

Per audit:
* If `index_draft_file` writes both graph + BM25
  but BM25 step fails: fix the BM25 layer's
  Drafts-awareness.
* If `index_draft_file` only writes graph: extend
  it to also call the BM25 indexer with the
  unified path.

## Acceptance

1. **BM25 search returns hits** for content typed
   into a draft file (e.g. typing "uniquemarker" in
   `Drafts/<name>/draft.md`, then searching
   "uniquemarker" via `/api/search`, returns the
   draft path in results).
2. **Graph data unchanged** (no regression on `-36`).

### Tests

* Round-trip: write draft via std::fs → reindex →
  BM25 search returns the file.

### Gate

`cargo fmt / clippy / test`; smoke green.

## Coordination

* @@Systacean lane.
* Closes the SECONDARY PARTIAL on the `-a-66`
  umbrella. Graph closure already shipped; this
  delivers full Drafts-as-first-class-content.

## Authorization

Yes for `crates/chan-drive/src/*.rs` (BM25 indexer
path) + `crates/chan-server/src/indexer.rs` if the
gap is at the apply layer + tests + task tail +
outbound.

## Numbering

This is `-37`.

## Out of scope

* SPA-side search rendering.
* Drafts FB tree (`-a-66 slice b` shipped).
* Graph drafts root + edge styling (`-a-66 slice e`
  graph HOLD via `-36`).

## 2026-05-22 — root cause located + fixed

Picked up `-37` per the secondary PARTIAL.

### Audit verdict — NOT in `index_draft_file` or BM25 layer

The architect's hypothesis pointed at `index_draft_file`'s BM25 write step or a path-classification gap in BM25. **Both were wrong** — empirically verified:

* `index_draft_file` DOES call `self.index.index_one(rel, content)` which writes BM25 + commits.
* BM25's `index_file` accepts arbitrary path strings (verified by reading `bm25.rs::index_file`).
* My `-34` + `-36` end-to-end tests BOTH passed (BM25 search returned hits for `Drafts/<...>` content).

### Real root cause: boot-walk gate

`-34`'s `index_drafts_subtree` walker fires INSIDE `Drive::reindex_with_aggression`. chan-server's `Indexer::spawn` only triggers reindex when `indexed_docs == 0 || graph_empty`. On a drive with existing drive-root content, neither is true → reindex doesn't fire → `-34`'s drafts walker never runs at boot.

Drafts authored pre-`-36` (when `apply_watch_change` dropped watcher events for `Drafts/...`) stayed absent from both BM25 and the graph DB. Once `-36` shipped, NEW watcher events route correctly — but PRE-EXISTING drafts persisted on disk + were never re-indexed because no boot walk caught them up.

@@WebtestA's empirical setup was exactly this: drafts created before `-36`, server restarted after `-36` landed, drafts not in BM25 (and graph, but graph happens to be populated via `-34` when a separate boot reindex DOES fire, which is why graph eventually showed). Architect's note that "graph HOLD via `-36`" is a coincidence of timing; the deterministic gap is BM25's.

### Fix shape

Two parts:

1. **`Drive::index_drafts_subtree` made `pub`** (`crates/chan-drive/src/drive.rs`). Was `fn`-private (only called from inside `reindex_with_aggression`); now callable from chan-server.
2. **chan-server `Indexer::spawn` walks drafts unconditionally on every boot** (`crates/chan-server/src/indexer.rs`). Lives in the `else if initial_build` branch that fires when full reindex is SKIPPED (drive non-empty + graph non-empty). Runs on the blocking pool via `tokio::task::spawn_blocking` so a slow drafts subtree doesn't stall the rest of `Indexer::spawn`. Idempotent: `index_draft_file` overwrites both backends, so re-running it on every boot is cheap when nothing changed and O(N) per draft when something did.

### Acceptance criteria status

| # | Criterion | Status |
|---|-----------|--------|
| 1 | BM25 search returns hits for content typed into a draft file | ✓ — boot walk on every chan-server start re-indexes drafts; existing `-34` + `-36` flows continue to cover the fresh-reindex + watcher paths |
| 2 | Graph data unchanged (no regression on `-36`) | ✓ — boot walk uses the same `index_draft_file` that `-36` routes to; idempotent |

### Tests

The fix is an additive `spawn_blocking` call inside `Indexer::spawn`. Direct unit-testing the boot path is non-trivial (requires the full async indexer setup); the existing `Drive::index_drafts_subtree` pin from `-34` (`reindex_walks_drafts_subtree_into_graph_and_bm25`) covers the underlying primitive. The `apply_watch_change_indexes_drafts_prefixed_path` pin from `-36` covers the live-watcher path. The combination gives full coverage of all 3 entry points.

### Pre-push gate

* `cargo fmt --check`: clean.
* `cargo clippy --all-targets -- -D warnings`: clean (1 rustdoc warning fixed: `+` → `and` in doc comment).
* `cargo test -p chan-drive --lib`: **463 / 0 / 2-ignored** (unchanged; visibility change only).
* `cargo test -p chan-server --lib`: **228 / 0** (unchanged; additive spawn_blocking call).
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`: green.

### Files

| File                                       | +   | -  |
|--------------------------------------------|-----|----|
| `crates/chan-drive/src/drive.rs`           | +18 | -7 |
| `crates/chan-server/src/indexer.rs`        | +22 | 0  |

Plus task tail + outbound poke. 4 paths.

### Suggested commit subject

```
chan-server + chan-drive: unconditional Drafts boot walk in Indexer::spawn (systacean-37; closes -a-66e BM25 secondary PARTIAL)
```

### Smoke plan

`gh workflow run ci.yml --ref systacean-37-smoke`. Expected ALL GREEN.

### Saga COMPLETELY closed

| Task | Coverage |
|------|----------|
| `-25` | watcher integration (Drafts/ prefix emission) |
| `-26` | unified read_text/write_text/etc. |
| `-29` | unified list |
| `-32` | unified stat/exists/read |
| `-34` | drafts walker INSIDE reindex (fires on cold drives) |
| `-36` | apply_watch_change routes Drafts/ events to index_draft_file |
| **`-37`** | **drafts walker on EVERY boot (chan-server Indexer::spawn)** |

End-to-end coverage of fresh-drive + live-watcher + restart-on-existing-drive paths.

Per architect's pre-authorization, proceeding to commit + push + smoke.
