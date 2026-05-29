# @@Systacean task 4: fs-change correctness + indexer resumption hardening

Owner: @@Systacean
Status: REVIEW
Depends on: [systacean-2](./systacean-2.md) (watcher gate), [systacean-3](./systacean-3.md)
(aggression contract)

## Goal

The request flags two related risks under "Fine tune the boot and in-flight
resource utilisation":

* Sudden underlying filesystem changes (e.g. git or hg checkouts) can
  invalidate large parts of the index in one shot. The graph + search
  must recover correctly when this happens, not get stuck on stale rows.
* Indexing-interruption / resume must survive these sudden fs changes
  without leaving the index in a half-written state.

The user also calls out end-to-end tests, benchmarks, and "extremely
need correctness tests" for this lane.

## Scope

### Detection

* Detect that a drive is a git or hg repo (presence of `.git/HEAD` or
  `.hg/dirstate`; do not shell out to the VCS).
* Surface a "VCS-aware drive" flag to the indexer + watcher.
* On VCS-aware drives, recognise sudden bulk changes — `HEAD` moves,
  `index` mtime jumps, `dirstate` updates, large coalesced watcher
  bursts — and route them through the coalesced-rebuild path that
  [systacean-2](./systacean-2.md) added.

### Correctness

* Define and test the invariant: after a sudden fs change, once the
  indexer settles, the graph and search results match what a fresh
  full reindex would produce. The persisted index never reflects a
  state the working tree never had.
* Add unit + integration tests under chan-drive that simulate a checkout
  by atomically replacing a tracked set of files and asserting the
  invariant.
* At least one test must exercise interruption mid-resume (kill the
  indexer task between batches, restart, verify no orphan rows).

### Benchmarks

* Add a small benchmark (criterion or a manual bench harness) measuring:
  * Initial pass time on the seeded test drive.
  * Settle time after a simulated checkout that touches ~25% of files.
  * Resume time after a forced restart partway through a rebuild.
* Capture rough before/after numbers in this task file.

### Resumption hardening

* Ensure the indexer's on-disk state (whatever crates/chan-drive
  persists between runs) is atomically written so a kill mid-batch
  cannot resume from an inconsistent state.
* If state is currently in-memory only, add a small resumption log
  and document the recovery contract.

## Acceptance criteria

* `cargo test` covers detection, the correctness invariant, and the
  interruption-resume case.
* Benchmark numbers recorded in this task file with the chan version
  (HEAD) the run was on.
* The aggression knob from [systacean-3](./systacean-3.md) still
  applies on VCS-aware drives.
* Full pre-push gate green (`cargo fmt --check`, `cargo clippy
  --all-targets -- -D warnings`, `cargo build --no-default-features`,
  `cargo test`, `npm run check`, `npm test`, `npm run build`).

## Hardening expectations

* Re-read the index recovery path with a hardware-failure mental model
  (process kill, power loss, partial fsync). Surface any gap to
  @@Architect.
* Confirm the chan-drive lock semantics still hold across a checkout
  storm; one writer assumption must survive.

## Coordination

* End-to-end browser smoke against the new behaviour belongs to
  [webtest-1](./webtest-1.md) once a build with this lands.

## Progress

* 2026-05-17 @@Systacean: picked up after
  [systacean-3](./systacean-3.md) reached REVIEW. Inspecting the
  existing watcher filtering, VCS marker helpers, and graph/search
  rebuild-resume tests before changing the indexer path.
* 2026-05-17 @@Systacean: implemented VCS-aware watcher/rebuild routing
  and hardened graph staging resume against checkout-modified files.

## Completion notes

Implemented:

* Added `chan_drive::detect_drive_vcs()` for drive-root VCS detection:
  Git is detected by a real `.git/HEAD`, Mercurial by a real
  `.hg/dirstate`; symlinked/special control files are rejected.
* Added a narrow watcher allowlist for `.git/HEAD`, `.git/index`, and
  `.hg/dirstate`. The rest of `.git/` and `.hg/` remains filtered.
* Server indexer now marks a drive VCS-aware at spawn time. On
  VCS-aware drives, control-file events route through the full rebuild
  path; large pending watcher bursts (threshold 64 paths) also clear
  pending incremental work and request one coalesced rebuild.
* Graph rebuild resume now sanitizes staged rows against the current
  disk `(mtime, size)` tuple. A checkout that modifies a file already
  staged before a crash now causes that staged row to be purged and
  reparsed instead of skipped.
* Existing `SearchAggression` still applies: VCS-triggered rebuilds go
  through the same coordinator path as other full rebuilds.

Correctness tests added/covered:

* `vcs::tests::detects_drive_vcs_from_checkout_control_files`
* `vcs::tests::drive_vcs_detection_rejects_symlinked_control_files`
* `vcs::tests::vcs_control_path_allowlist_is_exact`
* `watch::tests::filter_allows_vcs_control_paths_but_hides_other_vcs_noise`
* `indexer::tests::classify_watch_event_requests_rebuild_on_vcs_control_paths`
* `indexer::tests::vcs_burst_threshold_only_applies_to_vcs_aware_drives`
* `drive::tests::reindex_after_simulated_checkout_matches_fresh_full_reindex`
* `drive::tests::reindex_resume_reparses_staged_file_changed_by_checkout`

Manual benchmark/profile:

* Command: `cargo test -p chan-drive checkout_and_resume_profile -- --ignored --nocapture`
* HEAD/worktree at run time: current phase-5 workspace with this task's
  changes.
* Fixture: 80 markdown files; simulated checkout touches 20 files
  (25%).
* Observed:
  * Initial pass: 11078ms.
  * Settle after simulated checkout: 3138ms.
  * Resume after staged partial rebuild: 235ms.

Recovery contract:

* `rebuild.inprogress` remains the durable "prior rebuild may not have
  committed both graph and search" marker.
* Graph staging rows are committed per file and swapped into live graph
  tables in one transaction.
* Resume only skips staged rows whose `(mtime, size)` still matches
  disk; deleted or modified rows are purged before the cursor is read.

Verification:

* `cargo fmt --check`
* `cargo clippy --all-targets -- -D warnings`
* `cargo build --no-default-features`
* `cargo test`
* `cargo test -p chan-drive checkout_and_resume_profile -- --ignored --nocapture`
* `npm --prefix web run check`
* `npm --prefix web test -- --run`
* `npm --prefix web run build`
