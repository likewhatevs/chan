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
