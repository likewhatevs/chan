# @@Systacean task 6: BUG-WT5-A — incremental indexer misses newly-created files

Owner: @@Systacean
Status: REVIEW
Source: [webtest-1](./webtest-1.md) BUG-WT5-A (round-3 smoke
against PID 48037, **pre-systacean-3 + pre-systacean-4**, on
`/private/tmp/chan-test-phase5`).
Coordinates with: [systacean-2](./systacean-2.md) (acceptance
criterion partially regressed) and the chan-drive `Drive::watch`
classifier.

## Status update — 2026-05-17, @@Webtest A round-4 re-smoke

The round-4 binary (built AFTER [systacean-3](./systacean-3.md) +
[systacean-4](./systacean-4.md) reached REVIEW; chan-server PID
59434 on `/private/tmp/chan-test-phase5`) does **not** reproduce
BUG-WT5-A. New `.md` and `.txt` files created while the server is
running now reach the content index within the debounce window.

Three independent confidence repros (round-4 binary, default flags):

```
TOKEN=... ; BASE=http://127.0.0.1:8787 ; H="Authorization: Bearer $TOKEN"
# baseline: 96 indexed_docs after a forced rebuild
echo "# c1\nunique-keyword: confcheckone"        > /tmp/chan-test-phase5/confcheck-1.md
echo "second probe\nunique-keyword: confcheckone-two" > /tmp/chan-test-phase5/confcheck-2.txt
mkdir -p /tmp/chan-test-phase5/deep/nested
echo "third probe\nunique-keyword: confcheckone-three" \
                                                > /tmp/chan-test-phase5/deep/nested/confcheck-3.md
sleep 10
curl -s -H "$H" "$BASE/api/index/status"
# -> {"state":"idle","indexed_docs":99,...}   # was 96, +3 -> matches the 3 new files
curl -s -H "$H" "$BASE/api/search/content?q=confcheckone-two"
# -> hits: [{path:"confcheck-2.txt", chunk_id:"whole", ...}]
curl -s -H "$H" "$BASE/api/search/content?q=confcheckone-three"
# -> hits: [{path:"deep/nested/confcheck-3.md", chunk_id:"h-0", ...}]
```

Plus the earlier round-4 single-file probes (`r4-create.md`,
`r4-create.txt`) and a git-checkout test (see
[systacean-4 acceptance in webtest-1.md](./webtest-1.md) for the
`branch-only.md` traversal).

Most likely cause of the original regression: the
pre-systacean-4 watcher path was admitting non-indexable `.git/**`
churn that drowned the debounce, OR the systacean-4 patch's
classifier rewrite (drive-root VCS detection plus the per-event
extension gate) incidentally fixed the create admission too. Static
read of [chan-server/src/indexer.rs](../crates/chan-server/src/indexer.rs)
on the round-4 source shows the per-file apply does run for
`WatchKind::Created` (the same code path as Modified), so the
collapse-into-Modified hypothesis (a) appears wrong — it was a
plain create-event admission that the new build accepts.

Suggested action:

* @@Systacean to confirm by code-read that the systacean-4 patch
  actually closed this on purpose, write a targeted regression
  test (`create_event_admits_new_indexable_file_into_bm25`), and
  flip this task to REVIEW / DONE.
* No new fix work appears necessary, but leaving the task open
  until @@Systacean signs off on the test addition.

## Goal

Fix the incremental indexer so newly-created files (.md and .txt)
reach the BM25 / vector content index without a forced rebuild.

## Symptom

* `Drive::watch` Modified events still cause the per-file apply
  path to update BM25 within ~5 s (verified live on
  `welcome.md`). The watcher / classifier wiring works on the
  modify path.
* Create events for the same extensions do not reach the content
  index: `indexed_docs` stays put and a unique-keyword content
  query returns `hits: []`.
* `/api/search/files` (filesystem walk) does enumerate the new
  files, so chan-drive's read path is fine — the regression is
  specific to the indexer's create-event handling.

Repro (from `webtest-1.md`):

```
echo 'new doc with keyword brandnewprobe' \
  > /tmp/chan-test-phase5/brand.md
sleep 10
curl -s -H "$AUTH" "$BASE/api/index/status"
# -> {"state":"idle","indexed_docs":6,...}
curl -s -H "$AUTH" "$BASE/api/search/content?q=brandnewprobe"
# -> {"ready":true,"mode":"bm25","hits":[]}
curl -s -H "$AUTH" "$BASE/api/search/files?q=brand"
# -> [{"path":"brand.md",...}]   # filesystem walk sees it
```

Modify path works:

```
echo 'extra-line modifiedprobe' >> /tmp/chan-test-phase5/welcome.md
sleep 5
curl -s -H "$AUTH" "$BASE/api/search/content?q=modifiedprobe"
# -> hits: [{path:"welcome.md", ...}]
```

## Hypotheses (from @@Webtest A)

a. chan-drive's watcher (`Drive::watch`) collapses Created into a
   Modified-only stream on macOS FSEvents; chan-server's classifier
   sees a Modified for a path the BM25 store has no row for yet, and
   the per-file apply's new-doc admission (N+1 chunk vs replace-in-
   place) drops it.
b. The classifier correctly emits a `Changes` action for the new
   path, the per-file apply succeeds, but `indexed_docs` is sourced
   from a cached count that only increments on rebuild.

Either is consistent with the observation. Capture the actual cause
in this task file before writing the fix.

## Acceptance criteria

* Repro lines above produce `hits: [...]` and updated `indexed_docs`
  within the configured debounce window.
* Same behaviour for `.txt` files.
* New regression test in chan-drive or chan-server that asserts the
  create path admits a new file into the BM25 + (when applicable)
  embedding store, using the test harness already in place for the
  watcher.
* Full pre-push gate green: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`,
  `cargo build --no-default-features`, `cargo test`,
  `npm --prefix web run check`, `npm --prefix web test -- --run`,
  `npm --prefix web run build`.
* @@Webtest A re-runs the BUG-WT5-A repro on the fixed binary and
  flips that line in [webtest-1.md](./webtest-1.md) from
  REGRESSION to PASS.

## Hardening expectations

* Decide whether `indexed_docs` should be sourced from the live
  store or remain a rebuild-cached counter; if the latter, document
  it on `/api/index/status` and stop the "indexed_docs as live
  proof" reasoning in tests.
* Confirm the watcher debounce after this fix does not regress the
  modify path (the previous fix in [systacean-2](./systacean-2.md))
  by re-running both repros.

## Debug hooks

Webtest A offered to relaunch with
`RUST_LOG=chan_server::indexer=debug,chan_drive::watch=debug` and
capture watcher events if needed. Take them up on it if the static
read does not pin the cause.

## Progress

* 2026-05-17 @@Systacean: confirmed by code-read that
  [systacean-4](./systacean-4.md) intentionally closed the create
  admission gap: `classify_watch_event` now sends both
  `WatchKind::Created` and `WatchKind::Modified` for
  `chan_drive::fs_ops::is_indexable_text` paths through the same
  `WatchAction::Changes` path, and `apply_watch_change` calls
  `Drive::index_file` for regular files.

## Completion notes

Actual cause is the pre-[systacean-4](./systacean-4.md) watcher
classification/filtering path, not a BM25 cached-count issue. The
post-systacean-4 code admits created `.md` / `.txt` paths through the
same per-file indexing path as modified files.

Added regression test
`indexer::tests::create_event_admits_new_indexable_file_into_bm25`.
It classifies `WatchKind::Created` for new `.md` and `.txt` files,
applies the watcher changes, asserts `indexed_docs == 2`, and checks
BM25 hits for both unique terms.

@@Webtest A already re-ran the live BUG-WT5-A repro on the round-4
binary after [systacean-3](./systacean-3.md) +
[systacean-4](./systacean-4.md); it did not reproduce, and three
new files reached the content index within the debounce window.

Full gate green after adding the regression: `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`,
`cargo build --no-default-features`, `cargo test`,
`npm --prefix web run check`, `npm --prefix web test -- --run`,
`npm --prefix web run build`.
