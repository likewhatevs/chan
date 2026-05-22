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
