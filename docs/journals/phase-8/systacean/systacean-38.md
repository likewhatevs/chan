# systacean-38 — Drafts BM25 STILL not searchable (5th-degree slice e); audit ACTUAL write path empirically

Owner: @@Systacean
Cut: 2026-05-22 by @@Architect
Status: dispatched
Priority: HIGH (5th-round on Drafts saga; v0.12.0 blocker)

## Goal

Identify why BM25 search returns 0 hits for content
inside Drafts files, EVEN AFTER `-37`'s
unconditional boot walk in `Indexer::spawn` and
`-36`'s apply_watch_change Drafts/ branch.

@@WebtestA's empirical (`5223a42`): 5th-round
PARTIAL — the boot walk runs (per code review) but
the BM25 layer still doesn't have the content.

## Reference

@@WebtestA's 5th walk (`5223a42`):

* Created `Drafts/untitled/draft.md` with marker
  "UNIQUEMARKER37BM25CLOSURE".
* Restarted chan serve.
* Waited 16+ seconds.
* `/api/search/content?q=UNIQUEMARKER37BM25CLOSURE`
  returns `{ready: true, mode: bm25, hits: []}`.

Not a timing issue. The write either doesn't fire
OR fires but doesn't persist OR persists but search
doesn't find it.

## Audit directive: EMPIRICAL not just code-review

The prior 6 rounds (`-25`/`-26`/`-29`/`-32`/`-34`/
`-36`/`-37`) each shipped fixes that LOOKED right
in code but missed the empirical surface. Don't
trust code-review alone this round.

### Mandatory diagnostic steps

1. **Add temporary `tracing::info` logs** at each
   step in `Drive::index_draft_file`'s BM25 write
   path. Confirm each log fires for a `Drafts/...`
   path on a real boot.
2. **Capture the BM25 write parameters**: doc-id,
   path, content excerpt. Verify they're the
   unified `Drafts/<name>/<file>` form.
3. **Inspect the BM25 store directly** post-boot:
   does the index actually contain the Drafts
   doc? (e.g. via the same low-level inspection
   `-32`'s tests used).
4. **Trace the search path** for `Drafts/...`-
   prefixed keys: does the search layer filter
   them out? Is there a path-classification step
   that excludes `Drafts/`?
5. **Compare** to a working drive-root content
   write path step-by-step.

### Likely causes (rank-ordered)

* **(A)** `Drive::index_draft_file` calls graph
  write but NOT BM25 write (graph + BM25 are
  separate code paths; `-25` may have only wired
  one). HIGH likelihood.
* **(B)** BM25 write fires but uses drive-root
  capfs handle for content read → fails silently
  for Drafts paths → indexes empty doc.
* **(C)** BM25 write persists correctly but
  search-side filters out non-drive-root keys.
* **(D)** Boot walk doesn't fire on the test
  drive's restart trajectory (gating condition).

## Fix shape (TBD per audit)

Likely (A): extend `index_draft_file` to call the
same BM25 indexer surface that `index_file` uses,
with the unified `Drafts/<...>` key.

OR (C): if search filters, make filter prefix-aware.

## Acceptance

1. **BM25 search returns hits** for content in
   draft files. Verify with @@WebtestA's repro
   shape: create file with unique marker → restart
   → search → hit returned.
2. **Tracing logs / diagnostic comments** removed
   or labeled before commit (or leave as `tracing::debug`
   for future audits).
3. **Drafts saga COMPLETELY closed** after this.

### Tests

* End-to-end empirical test on a fixture drive +
  the actual chan-server `Indexer::spawn` path.
* The TEST MUST FAIL if the BM25 write path
  silently drops Drafts (i.e. assert content
  search returns a hit, not just that the function
  was called).

### Gate

`cargo fmt / clippy / test`; smoke green.

## Coordination

* @@Systacean lane.
* 5th-round on the Drafts saga — please go DEEP on
  the empirical audit before assuming code-review
  is enough.

## Authorization

Yes for `crates/chan-drive/src/*.rs` + chan-server
indexer / search routes if cross-layer gap + tests
+ task tail + outbound.

## Numbering

This is `-38`.

## Out of scope

* SPA-side search rendering.
* Graph drafts root + edge (HOLD via `-36`).
