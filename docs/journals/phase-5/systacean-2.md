# @@Systacean task 2: indexer scheduling priority and watcher loss gate

Owner: @@Systacean
Status: REVIEW
Depends on: [systacean-1](./systacean-1.md)
Coordinates with: [architect-1](./architect-1.md), [webtest-1](./webtest-1.md)

## Goal

Start the wave-2 indexer lane by tightening the server-side watcher
gate and documenting the priority boundary:

* chan-report stays warmed by `Drive::watch` and sees events before
  chan-server forwards them to clients / the background search indexer.
* Full rebuilds stay graph-first, search-second inside
  `Drive::reindex_with`.
* Watcher events that imply event loss (provider error or path-less
  scope) must request a full rebuild instead of leaving search/graph
  stale until a manual reindex.
* Incremental watch scheduling must match chan-drive's
  `is_indexable_text` gate, not a hard-coded `.md` suffix.

## Acceptance criteria

* `crates/chan-server/src/indexer.rs` maps provider-error and
  path-less watcher events to a coalesced rebuild request.
* Incremental server indexing accepts every chan-drive indexable text
  extension (`.md` and `.txt` today).
* Deletion changes are applied before upserts when a due batch contains
  both, so stale graph/search rows disappear before new rows are added.
* Focused unit coverage for the watcher-event scheduler.
* Verification:
  * `cargo fmt --check`
  * `cargo test -p chan-server indexer`
  * broader gate if the patch touches shared chan-drive behavior.

## Progress

* 2026-05-17 @@Systacean: created after the update check found wave-1
  cleanup in REVIEW and Architect's wave-2 Systacean lane ready to
  dispatch.
* 2026-05-17 @@Systacean: patched `crates/chan-server/src/indexer.rs`
  so provider-error/path-less events clear pending incremental work and
  request a coalesced rebuild; incremental events now use
  `chan_drive::fs_ops::is_indexable_text` instead of a hard-coded
  `.md` suffix.
* 2026-05-17 @@Systacean: due batches now apply deletions before
  upserts. Added unit coverage for `.txt` scheduling, ignored
  non-indexable source files, provider/path-less rebuild requests,
  rename fan-out, and delete-before-upsert ordering.

## Completion notes

Verification passed:

* `cargo fmt --check`
* `cargo test -p chan-server indexer`
* `cargo clippy -p chan-server --all-targets -- -D warnings`
