# systacean-36 — path_classification unified-path for Drafts (closes -a-66 slice e STILL PARTIAL 3rd round)

Owner: @@Systacean
Cut: 2026-05-22 by @@Architect
Status: dispatched
Priority: HIGH (closes 3rd-round PARTIAL on -a-66 slice e)

## Goal

Audit + fix the `path_classification` code path that
silently fails for `Drafts/`-prefixed files, even
after `-34`'s boot walker calls `index_draft_file`
for each Drafts file.

## Reference

@@WebtestA's 3rd-round walk (`121e109`):

* `-34`'s walker present + invoked per file.
* `-32`'s unified `Drive::stat` present.
* BUT graph payload STILL empty under `Drafts/`.
* BM25 search returns 0 hits for draft content.

Hypothesis: a `path_classification` step downstream
of `index_draft_file` doesn't use the unified
`Drive::stat`/`exists` paths, so it fails for
`Drafts/<path>` even when the walker hands it the
file.

## Audit path

1. Trace `index_draft_file` call chain in chan-drive.
2. Identify path-classification / file-validation
   helpers it consults.
3. For each: check if it uses unified-path API
   (`Drive::stat` post-`-32`) or routes through a
   non-unified surface (raw `self.dir.*`).
4. If a helper takes a path string + does any kind
   of disk-stat lookup that's drive-root-only,
   that's the gap.

Likely candidates (audit will narrow):
* `path_class_for_wire` or similar formatting helpers.
* `is_indexable_text` / mime-type or extension checks
  that may consult disk metadata.
* Watcher / indexer's classification step before
  emit to graph / BM25.

## Fix shape

Apply unified-path routing to the failing helper
(same `resolve_io` pattern as `-26`/`-29`/`-32`).
OR refactor `index_draft_file` to pre-classify
the file before calling downstream helpers that
expect drive-root paths.

Implementer's call after audit.

## Acceptance

1. **Restart chan serve with Drafts content present →
   graph payload includes Drafts root + drafts_link
   edges + draft file nodes**.
2. **BM25 search returns hits for draft content**.
3. **No regression on drive-root indexing**.

### Tests

* End-to-end: write Drafts file via std::fs (bypass
  watcher) → restart drive → reindex → graph + BM25
  populated.
* Existing chan-drive tests stay green.

### Gate

`cargo fmt / clippy / test`; smoke green.

## Coordination

* @@Systacean lane.
* HIGH priority — closes 3rd-round PARTIAL on the
  `-a-66` umbrella; v0.12.0 needs Drafts visible
  in graph + searchable.

## Authorization

Yes for `crates/chan-drive/src/*.rs` + tests +
task tail + outbound.

## Numbering

This is `-36`.

## Out of scope

* Watcher behavior (`-25` covers).
* SPA-side rendering (`-a-66 slice e` shipped).
* Drive::list / stat / read unification (`-29`
  / `-32`).
