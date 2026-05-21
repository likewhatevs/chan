# systacean-19 — graceful BM25-only degradation when BGE model not present

Owner: @@Systacean
Cut: 2026-05-21 by @@Architect
Status: dispatched

## Goal

Make chan-drive's `write_file` indexing path **degrade
gracefully** when the BGE-small embedding model isn't
downloaded. Today's behaviour: the embed step panics, the
indexing fails completely, the file gets no index entry
(neither vector nor BM25). The user has a BROKEN install
until they run `chan index download-model`.

Wanted behaviour: when the embed step fails with
`ModelNotDownloaded`, log a one-shot warning, skip the
vector commit, but STILL commit the BM25 entry. The user
gets working BM25 search out of the box; semantic search
is the upgrade path.

After this lands, the 28 `#[ignore]` gates from `-18` +
its follow-ups (chan-drive lib 14, chan-drive integration
5, chan-server lib 9) can REVERT. Coverage restored
without per-test iteration.

## Background

Surfaced 2026-05-21 by @@Systacean as **Option C2** in
their `-18` follow-up #3 scope poke. See
[`../alex/event-systacean-architect.md`](../alex/event-systacean-architect.md)
"scope poke (-18 follow-up #3...)" §"Option C — pivot to a
different gating strategy" for the original framing.

### Why this is more than test-infra

Today, a user who installs the default chan binary (no
`--features embed-model`, no model downloaded) gets:

* `chan add <drive>` succeeds.
* `chan serve <drive>` starts.
* First file edit → `write_file` panics on the embed step.
* Indexing breaks. The file gets no index entry. No
  search recovery without manual model download.

With C2:

* First file edit → `write_file` tries embed; gets
  `ModelNotDownloaded`; logs once; skips vector commit;
  commits BM25 entry.
* User gets working BM25 keyword search immediately.
* `chan index download-model` upgrades them to hybrid
  semantic+BM25 retroactively (the existing reindex
  flow already handles that path).

This **aligns with the architectural decisions from
`systacean-6` + `-7`**: the BGE bundle is opt-in. C2
makes the opt-out runtime behavior consistent with the
opt-in build-time behaviour — both gracefully degrade
to BM25-only.

## Decision: fix shape

Single-discriminator early-return in `write_file`'s embed
step. When the embed call returns an `Err` matching the
`ModelNotDownloaded` variant (or its equivalent —
implementer audits the exact error shape):

* Log once via `tracing::warn!` with a clear "BM25-only
  fallback active; run `chan index download-model` to
  enable semantic search" message. Use the once-cell /
  `Once`-shaped one-shot logger pattern so the warning
  doesn't spam the log.
* Skip the vector commit (don't call the vector-side of
  the persist flow).
* CONTINUE to the BM25 commit (call the BM25 side of the
  persist flow).
* Return `Ok(())` (or whatever the success shape is).

For ALL OTHER error shapes from the embed step (transient
I/O, model-file corruption, etc.), keep the existing
error propagation. Only `ModelNotDownloaded` triggers the
fallback path.

## Acceptance criteria

### Graceful degradation in write_file

1. Audit `crates/chan-drive/src/drive.rs::write_file` (or
   wherever the indexing path lives — `index_file` if
   that's the canonical entry). Identify the embed step
   + its error shape.
2. Add the `ModelNotDownloaded` discriminator: on match,
   log once + skip vector commit + continue with BM25.
3. Other error shapes propagate as before.

### One-shot warning

1. `tracing::warn!` fires ONCE per process lifetime when
   the fallback kicks in. Suggested implementation:
   `std::sync::Once` or `OnceCell<()>` guard around the
   log call.
2. Warning message names the root cause + the upgrade
   path: "Embedding model `BAAI/bge-small-en-v1.5` not
   downloaded; semantic search disabled. Run `chan index
   download-model` to enable. BM25 keyword search remains
   active."

### Revert the 28 #[ignore] gates

1. Once the fallback path works locally + via smoke,
   revert the `#[ignore = "..."]` attributes on:
   * chan-drive lib (14 tests, `drive.rs` + `indexer.rs`
     per `-18`'s initial commit).
   * chan-drive integration (5 tests across
     `tests/contacts_import.rs`, `tests/file_types.rs`,
     `tests/smoke.rs`, `tests/remove_cleanup.rs` per
     follow-ups #1-#3).
   * chan-server lib (9 tests per follow-up #4 — across
     `indexer.rs`, `routes/graph.rs`, `routes/inspector.rs`,
     `routes/search.rs`).
2. With the fallback active, these tests should now run
   to completion + verify their actual contract (the BM25
   side of the indexing, plus whatever they were
   originally checking). They may need minor assertion
   adjustment if any of them was implicitly relying on
   the embed step succeeding.

### Local verification

1. `cargo test -p chan-drive` — all tests (including the
   ungated 19) green. Compare test counts vs the
   pre-`-18` baseline.
2. `cargo test -p chan-server` — all tests (including the
   ungated 9) green.
3. `cargo fmt --all`; `cargo clippy --all-targets -- -D
   warnings` — workspace-wide green.
4. Manual smoke: build chan with default features (no
   `embed-model`), run `chan add /tmp/test-bm25`, `chan
   serve /tmp/test-bm25`, edit a file in the drive, verify
   the indexing didn't panic + the file becomes searchable
   via BM25 query. Confirm the one-shot warning fired in
   the log.

### CI verification

1. Push to a `systacean-19-smoke` branch + `gh workflow
   run ci.yml`. Confirm:
   * Ubuntu cargo test fully green (no more BGE panics;
     no need for `#[ignore]` gates).
   * Windows clippy + test green.
   * macOS green.
2. Smoke joins the audit-trail-keep set; prunes with the
   `chan-v0.11.99-dryrun.{1..4}` tag cleanup beat.

## How to start

1. Read `crates/chan-drive/src/drive.rs::write_file`
   (and/or `index_file`); identify the embed call + its
   error path.
2. Read the embedding module to understand the error
   shape for `ModelNotDownloaded` — exact variant name +
   how it's currently constructed.
3. Apply the discriminator + fallback path.
4. Add the one-shot warning.
5. Revert the 28 `#[ignore]` gates from `-18` + its
   follow-ups (chan-drive lib + integration + chan-server
   lib).
6. Local pre-push gate green workspace-wide.
7. CI smoke via `gh workflow run ci.yml` on
   `systacean-19-smoke`.
8. Append "Commit readiness" + fire poke to @@Architect.

## Coordination

* @@Systacean lane (chan-drive + chan-server scope).
* SEQUENCING: pick up AFTER `-18` follow-up #4 lands
  (the chan-server gating closes the per-PR gate for
  the immediate beat; `-19` then reverts ALL the gates
  + makes them obsolete).
* No interaction with other lanes — single-author
  end-to-end task.
* @@FullStackB's `-24` work + smoke fixups are
  orthogonal (Windows dead_code, not embedding path).

### Shared-infra authorization

This task touches:

* `crates/chan-drive/src/drive.rs` (or `indexer.rs` —
  wherever the embed call lives).
* `crates/chan-drive/src/index/embeddings.rs` (potentially,
  for the error-shape audit; not for changes unless
  needed).
* `crates/chan-drive/tests/*.rs` (revert `#[ignore]` on
  5 tests).
* `crates/chan-drive/src/{drive,indexer}.rs` (revert
  `#[ignore]` on 14 tests).
* `crates/chan-server/src/{indexer.rs,routes/{graph,inspector,search}.rs}`
  (revert `#[ignore]` on 9 tests).
* `docs/journals/phase-8/systacean/systacean-19.md`
  (task tail).
* `docs/journals/phase-8/alex/event-systacean-architect.md`
  (outbound).

**Authorization: yes** for all of the above. The fix is
narrow + the revert is mechanical. chan-server edits are
test-only (no production-route changes here). @@Systacean
may proceed without further in-chat confirmation from
@@Alex.

## Numbering

Highest committed `systacean-N` is `-18` + follow-ups
(commits up to `147a06f`); next available is `-19`.
This is `-19`.

### Queue (revised 2026-05-21)

```
-18 follow-up #4 (chan-server gating + 2 fs_graph lints) — same task lineage
-19 (this task; C2 graceful degradation + revert all 28 #[ignore] gates)
-16 (chan-report file-class buckets — feature work; deferred if needed)
-12 (tauri-plugin-updater verify; still parked on permission ask)
```

`-19` is the bigger structural fix; `-16` parks behind
it as feature work.

## Out of scope

* Auto-downloading the model on first use. That's a
  separate feature (could be `systacean-N+1` in Round 3
  if @@Alex wants it). C2 alone is the discrete unit
  here: graceful degradation, not opportunistic install.
* Changing the build-default to include `embed-model`.
  Architecture is unchanged: feature is opt-in.
* Refactoring chan-drive's `write_file` beyond the
  embed-step discriminator. Stay narrow.
* Modifying chan-server's search/inspector routes
  beyond reverting the test `#[ignore]`s. The BM25
  index entry is the contract those tests verify; with
  the fallback active, the contract holds.
* Adding fallback in `chan index reindex` flows. The
  `write_file` path is the user-facing one; reindex is
  the recovery flow + already needs the model (it's an
  explicit re-embed). Document the asymmetry in the
  task tail.

## What this task is NOT

* A rewrite of chan-drive's embedding architecture.
* A test-infra change (C1 was the alternative; we're
  going with C2 instead because it fixes user experience).
* A pre-fetch model in CI shape (option (c) from @@CI's
  original ci-12 analysis — declined long ago).
