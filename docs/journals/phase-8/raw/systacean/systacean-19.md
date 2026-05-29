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

## 2026-05-21 — implementation + commit readiness

C2 graceful degradation landed at the per-file embed chokepoint + the bulk-reindex batch path. All 28 BGE `#[ignore]` gates reverted. Workspace tests green; unit coverage added for the new fallback discriminator.

### Changes

* **`crates/chan-drive/src/index/facade.rs`** (+108 / -6):
  * New free function `warn_bm25_only_once()` — `std::sync::Once`-guarded `tracing::warn!`; fires once per process lifetime with a clear "BGE not downloaded; falling back to BM25-only; run `chan index download-model` to enable semantic search" message. Avoids log spam on bulk reindex where every file would otherwise trip the warning.
  * New associated function `Index::handle_embed_load_error(e) -> Result<(), IndexError>` — single discriminator. `Err(IndexError::Embed(EmbedError::ModelNotDownloaded { .. }))` → log once + return Ok (caller skips vector commit, falls through to BM25). Any other error variant propagates as `Err(e)`.
  * `write_file` (per-file path used by `index_one` watcher + bulk reindex): match-discriminator on `self.embedder()`. ModelNotDownloaded routes through `handle_embed_load_error`; other errors propagate via the `?`-shape that was there before.
  * `flush_embed_batch` (bulk reindex's vector-batch optimization): same discriminator. ModelNotDownloaded → `warn_bm25_only_once()` + `pending.drain(..)` + `Ok(empty errors)`. The BM25 indexing in `build_all` happens BEFORE `flush_embed_batch` in the loop (line ~468 `self.bm25.index_chunks`), so dropping the vector batch leaves BM25 correct + `summary.errors` stays clean.
  * 2 new unit tests in `mod tests` directly exercising `handle_embed_load_error` with synthetic `ModelNotDownloaded` + `Candle("synthetic")` errors. Workstation has the BGE model cached so the end-to-end fallback path can never naturally trip in `cargo test`; the unit tests give the discriminator direct coverage regardless of model presence.

* **All 28 `#[ignore]` gates reverted**:
  * `crates/chan-drive/src/drive.rs` (-12 lines): `reindex_consumes_pending_rename_log_after_reopen`, `pending_writes_journal_is_empty_on_a_clean_path`, `pending_writes_journal_replay_converges_after_simulated_crash`, `pending_writes_replay_degrades_index_op_to_forget_when_file_is_gone`, `pending_writes_journal_handles_forget_op`, `reconcile_picks_up_files_added_offline`, `reconcile_picks_up_modified_files`, `reconcile_catches_same_mtime_different_size_rewrite`, `index_file_stamps_pre_read_stat_so_concurrent_writes_stay_visible`, `reconcile_on_empty_graph_indexes_everything_like_a_fresh_reindex`, `link_targets_finds_file_after_index`, `resolve_link_returns_contact_kind_for_contact_node`, `resolve_link_returns_file_kind_for_plain_note`.
  * `crates/chan-drive/src/indexer.rs` (-2 lines): `writes_to_disk_get_indexed_after_debounce`, `debounce_coalesces_rapid_writes_into_one_index`.
  * `crates/chan-drive/tests/contacts_import.rs` (-1): `removing_contact_frontmatter_demotes_node_back_to_file`.
  * `crates/chan-drive/tests/file_types.rs` (-1): `file_type_policy_end_to_end`.
  * `crates/chan-drive/tests/smoke.rs` (-1): `end_to_end_register_open_write_index_search_graph`.
  * `crates/chan-drive/tests/remove_cleanup.rs` (-2): `remove_single_file_drops_graph_and_index`, `remove_directory_cascades_through_graph_and_index`.
  * `crates/chan-server/src/indexer.rs` (-3): `apply_watch_change_indexes_regular_file`, `create_event_admits_new_indexable_file_into_bm25`, `apply_watch_change_special_clears_prior_index_entry`.
  * `crates/chan-server/src/routes/graph.rs` (-3): `link_to_non_markdown_disk_file_resolves_to_real_file`, `link_to_directory_does_not_synthesize_ghost_file_node`, `merged_graph_layers_emit_filesystem_media_and_language_nodes`.
  * `crates/chan-server/src/routes/inspector.rs` (-1): `inspector_payload_covers_drive_directory_text_and_binary`.
  * `crates/chan-server/src/routes/search.rs` (-2): `indexing_state_endpoint_requires_auth`, `indexing_state_endpoint_returns_dir_nodes`.

  Total revert: 28 lines (14 chan-drive lib + 5 chan-drive integration + 9 chan-server lib).

### Local verification

* `cargo test -p chan-drive`: **425 passed; 0 failed; 2 ignored** (was 411 passed / 16 ignored pre-revert; 14 newly-ungated tests now run + pass with the model loaded on workstation).
* `cargo test -p chan-server`: **205 passed; 0 failed; 0 ignored** (was 196 / 9 pre-revert; 9 newly-ungated tests now run + pass).
* `cargo test` (workspace): all green, no FAILED anywhere.
* `cargo fmt --check`: clean.
* `cargo clippy --all-targets -- -D warnings`: clean.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`: green.
* `cd web && npm run check`: 0 errors / 0 warnings / 3991 files.
* `cd web && npm test -- --run`: 658 / 658 passed.

### Verification of the fallback path

Workstation has BGE cached at `~/.cache/chan/models/...` so the unit tests with synthetic errors are the load-bearing fallback-path coverage on this host. End-to-end fallback validation will happen on the CI smoke (Ubuntu runner has no model cached; if all previously-gated tests pass, the fallback path is exercised end-to-end with no panics).

### Architecture aligns with systacean-6 / -7

* `systacean-6`: BGE bundle opt-in via `embed-model` cargo feature; default builds don't ship the bytes.
* `systacean-7`: `chan index download-model` CLI + `/api/index/semantic/*` endpoints; user-driven model download.
* **`systacean-19` (this task)**: graceful runtime fallback when the model isn't on disk. The default-build install path (no `embed-model` + no prior download) now works: BM25 keyword search out of the box; `chan index download-model` upgrades to hybrid retroactively.

The 3 pieces together = consistent opt-in semantic-search architecture across build-time, runtime CLI, and runtime fallback.

### Suggested commit subject

```
chan-drive: graceful BM25-only fallback when BGE model not present + revert 28 #[ignore] gates (systacean-19)
```

### Files for commit

```
crates/chan-drive/src/index/facade.rs            +108 / -6   (C2 impl + 2 unit tests)
crates/chan-drive/src/drive.rs                   0    / -12  (revert lib gates)
crates/chan-drive/src/indexer.rs                 0    / -2   (revert lib gates)
crates/chan-drive/tests/contacts_import.rs       0    / -1   (revert integration gate)
crates/chan-drive/tests/file_types.rs            0    / -1   (revert integration gate)
crates/chan-drive/tests/smoke.rs                 0    / -1   (revert integration gate)
crates/chan-drive/tests/remove_cleanup.rs        0    / -2   (revert integration gates)
crates/chan-server/src/indexer.rs                0    / -3   (revert chan-server gates)
crates/chan-server/src/routes/graph.rs           0    / -3   (revert chan-server gates)
crates/chan-server/src/routes/inspector.rs       0    / -1   (revert chan-server gate)
crates/chan-server/src/routes/search.rs          0    / -2   (revert chan-server gates)
docs/journals/phase-8/systacean/systacean-19.md  (this append)
docs/journals/phase-8/alex/event-systacean-architect.md  (outbound poke)
```

13 paths total. Foreign files in dirty tree stay un-staged per shared-worktree discipline.

### Smoke verification ask

Push to `systacean-19-smoke` branch (new branch; tracks `-19` as its own gate-unblocker confirmation) + `gh workflow run ci.yml --ref systacean-19-smoke`. Expected:

* **Ubuntu cargo test fully green** without the 28 `#[ignore]` gates: the fallback path lets all previously-gated tests run + pass on the model-less CI runner.
* **macOS cargo test fully green**: workstation has model cached; no change in behavior (the fallback never triggers).
* **Windows**: not in the matrix anymore per @@Alex's CI decision (`ci-13`).
* **web + rustfmt + build-no-default-features**: green.

If the smoke surfaces ANY failure other than known fullstack-b-24 / fullstack-a-N follow-ups already routed, escalate via scope poke per the prior discipline.

### Round-3 follow-up flags (out of scope here)

* The remaining 3 `#[cfg(unix)]` gates from `-20` (lock contract) + 1 from smoke #2 fixup (`watcher_keeps_report_current` + its helpers) STAY. They document Windows gaps independent of BGE; Round-3 polish for the real cross-platform fixes when Windows becomes a real-user surface again.

Holding for @@Architect commit clearance + smoke-branch authorization. Same shape as the prior cleared smokes.

## 2026-05-21 — committed inside 5685be4 (cross-agent commit-hygiene incident)

The C2 fallback implementation + all 28 `#[ignore]` reverts described above ARE in HEAD, but landed inside commit `5685be4` whose subject is misattributed to `fullstack-a-49`. Audit anchors here so future readers walk the task file rather than relying on `git log`.

### What happened

A `git add` race during the cleared commit attempt: my 13 paths landed in an index already pre-staged with 5 FullStackA files (`event-fullstack-a-architect.md`, `fullstack-a-49.md`, `fullstack-a/journal.md`, `web/src/components/GraphCanvas.svelte`, `web/src/components/GraphCanvas.test.ts`). @@FullStackA's `git commit -m "Graph layout: filesystem-hierarchy as backbone (fullstack-a-49)"` fired BEFORE my `git restore --staged` could partition the stowaways, sweeping all 18 files into one commit under the `-a-49` subject.

### Audit trail

* **`5685be4`** — the load-bearing commit. `git show 5685be4 --stat` lists 18 files including all 13 `-19` paths (the C2 fix in `facade.rs` + the 14 chan-drive lib reverts + 5 integration reverts + 9 chan-server lib reverts) + the 5 expected `-a-49` paths.
* **`cc3a888`** — @@FullStackA's incident flag (event-fullstack-a-architect + fullstack-a-49.md tail).
* **`88a084c`** — my incident flag (event-systacean-architect.md tail with options A/B).
* **`75b0953`** — @@Architect's routing: option (a) accepted (audit-trail correction; soft-reset declined because the incident-flag commits already reference `5685be4`). Memory entry `feedback-atomic-audit-commit` saved.

### Audit anchor for `-19`

The canonical implementation summary for `systacean-19` is THIS TASK FILE — specifically the "## 2026-05-21 — implementation + commit readiness" section above. Not the commit subject of `5685be4` (which reads `-a-49`).

For git-blame on `crates/chan-drive/src/index/facade.rs` C2 fallback (the `warn_bm25_only_once` + `handle_embed_load_error` + the discriminator matches at `write_file` + `flush_embed_batch`): the blame attribution will read `5685be4 fullstack-a-49` but the actual scope is `-19`'s.

### Process-lesson logged

The atomic-audit-commit pattern (collapse `git add` + audit + `git commit` into a single chained `&&` invocation, OR use `git commit --only <paths>`) closes the inter-command race window that produced this incident. The pattern is saved to memory as `feedback-atomic-audit-commit`. Going forward all my shared-worktree commits use this discipline.

### Smoke verification

Per the architect's routing: `-19`'s smoke verification proceeds unaffected by the attribution incident. The C2 fallback path is the load-bearing thing under test; it's in `5685be4`'s file contents regardless of the misattribution. Smoke validates the fix end-to-end on the model-less Ubuntu runner.

Next beat: push fastforward to `systacean-19-smoke` (the work is already in `5685be4` on main; the smoke branch just needs HEAD) + dispatch CI.
