# systacean-2: write-timeout investigation

Owner: @@Systacean
Cut by: @@Architect
Date: 2026-05-18

## Goal

Diagnose and fix the "failed to write after 10s" error
@@Alex hit while editing a small markdown file in the chan
editor. Small `.md` writes should never block 10s on the
write path. The 10s ceiling is masking a real contention or
deadlock somewhere downstream of the editor's save call.

## Relevant links

* [../request.md](../request.md) Bugfixes (the "failed to
  write after 10s" bullet with repro image
  `../image-1.png#w=250`).
* [../architect/journal.md](../architect/journal.md) Round 1
  bugfix checklist (B5).
* CLAUDE.md "Drive is the boundary" section — all drive
  writes go through `chan_drive::Drive::write_text` /
  `write_bytes` (atomic via tempfile + rename + fsync
  parent).
* `crates/chan-drive/` for the write path.
* `crates/chan-server/src/routes/files.rs` for the HTTP write
  handler.
* `crates/chan-server/src/indexer.rs` for the indexer
  (likely co-conspirator).

## Acceptance criteria

### Diagnosis

* Reproduce the 10s timeout reliably. @@Alex saw it while
  writing the phase-7 `request.md`; a similar drive with a
  few notes and the indexer running should repro it.
* Root cause identified, written up in an append to this
  task file with:
  * Where the 10s comes from (which timeout in which layer).
  * What's actually blocking (lock contention, fsync stall,
    indexer holding a lock, async deadlock, etc.).
  * Whether the slowdown is on the write or on some
    pre-write check (e.g., a search/index lookup that
    happens during auto-save).

### Fix

* Small-file writes (a few KB markdown) complete in low
  hundreds of milliseconds at most, even with the indexer
  running.
* No regression in indexer correctness (the file still gets
  re-indexed after the write).
* No regression in the atomic-write guarantee — chan-drive
  still does tempfile + rename + fsync parent. We're not
  relaxing durability.

### Guard

* Add a test that catches a future regression: a synthetic
  drive with the indexer enabled writing a small file
  completes under a generous bound (e.g., 1s) and never
  trips the 10s timeout.
* If the 10s is currently a hard timeout in some layer,
  consider whether it should remain or be lowered with a
  proper error path — write up the call in an append.

## Out of scope

* Indexer redesign. We're fixing a contention/deadlock, not
  re-architecting how the indexer pulls work.
* Search-result UX (that's fullstack-3's territory).

## How to start

1. Reproduce: spin up a test drive seeded with ~20 notes,
   start `chan serve`, type into a markdown file at normal
   editing pace for a couple of minutes. The timeout shows
   up during auto-save, not first save.
2. Trace: enable a debug logging build of chan-server and
   look at what the write handler is awaiting when the
   10s ceiling fires.
3. The most likely culprit categories, in order:
   a. The indexer holds an `RwLock` write guard during scan,
      and the write path is waiting on a read guard (or
      vice versa).
   b. An `inotify`-driven re-scan triggered by the previous
      write is still running and holds a per-file lock.
   c. A debounce/coalesce buffer in the auto-save path is
      issuing serialized writes when it should batch.
   d. An fsync-of-parent stall on the user's filesystem
      (less likely — this would be persistent, not
      intermittent).
4. Document findings as you go; do not jump to a fix until
   you have a clear root cause.

## Hand-off

Same shape as systacean-1. Pre-push gate must pass.

## 2026-05-18 12:49 BST - Diagnosis and fix

Diagnosis:

* The visible 10s error comes from the frontend request wrapper:
  `web/src/api/transport.ts` aborts all normal API requests after
  10,000 ms. The save path reports that as an autosave failure.
* The server write route did not have its own 10s timeout. It ran
  `Drive::write_text(_if_unchanged)` synchronously inside the async
  axum handler. The drive write still preserves tempfile + fsync +
  rename + parent-dir fsync, but doing that on a Tokio worker is the
  wrong execution class and can stall unrelated request progress when
  storage is slow.
* The drive indexer is not holding a lock that blocks normal writes.
  `Drive::index_file` serializes graph/search mutation through
  `write_serial`; `Drive::write_text` intentionally does not take
  that lock. I added a regression test that holds `write_serial` and
  asserts a small markdown write still completes quickly.
* A frontend autosave race was adjacent and likely explains the
  "small edit eventually times out / cursor jumps" cluster: while a
  save was in flight, another autosave could start, and the first
  save marked `t.saved = t.content` after `await`. If the user typed
  during the request, unsent text could be marked saved and overlapping
  PUTs could pile up behind the browser/server request path.

Fix:

* `crates/chan-server/src/routes/files.rs`
  * moved file writes and file-create writes onto
    `tokio::task::spawn_blocking`;
  * factored the CAS/write/stat logic into `write_file_sync`;
  * kept the existing conflict response semantics and atomic
    chan-drive write guarantees.
* `crates/chan-drive/src/drive.rs`
  * added `write_text_does_not_wait_for_indexer_serial_lock`.
* `web/src/state/tabs.svelte.ts`
  * serialized saves per tab;
  * snapshots `(content, savedMtime)` before awaiting the API call;
  * if an autosave fires while a save is in flight, it queues one
    follow-up save for the latest buffer instead of overlapping PUTs.
* `web/src/state/tabs.test.ts`
  * added a regression test for overlapping autosaves.

Verification:

* `cargo test -p chan-drive write_text_does_not_wait_for_indexer_serial_lock`
* `cargo test -p chan-server write_file_sync`
* `npm test -- --run src/state/tabs.test.ts`
* `cargo check -p chan-server -p chan-drive`
* `npm run check`
* `cargo clippy -p chan-server -p chan-drive --all-targets -- -D warnings`

Notes:

* I did not run a live browser typing repro because `systacean-1` is
  frozen in the same working tree pending @@Alex commit authorization.
  The fix is based on code-path diagnosis plus synthetic regression
  coverage.
* `systacean-1` files were not touched.

Specialist review requested from @@Architect.

## 2026-05-18 14:00 BST — @@Architect review: APPROVED for commit (gated on @@Alex)

Solid diagnosis — and importantly, you cleared the indexer of guilt
with `write_text_does_not_wait_for_indexer_serial_lock`. That's the
kind of negative result that should be in tests permanently; thanks
for landing it.

Root cause acceptance:

* Server-side: `Drive::write_text` was running on a Tokio worker —
  correct critique, `spawn_blocking` is the right move for fsync-
  heavy paths. The `write_file_sync` factoring keeps the CAS
  semantics intact.
* Frontend: the autosave race was adjacent (and was likely the
  perceived "cursor jumps" cluster Alex noted in `request.md`).
  Per-tab serialization + queue-the-latest-buffer is the cheap
  correct shape.
* The 10s timeout in `transport.ts` stays; we're not relaxing
  client patience, we're making the server faster.

No issues. Pre-push gate ran green per your verification chain.

### Commit clearance

**APPROVED from @@Architect's side.** Hold the commit until @@Alex
authorizes. Per project rule, only @@Alex commits.

Proposed commit message:

```text
Fix write-path stalls and autosave races

Move chan-drive writes onto spawn_blocking so fsync-heavy IO does
not stall the Tokio runtime. Serialize autosaves per editor tab
and queue a single follow-up save for the latest buffer instead
of letting overlapping PUTs pile up. Adds a regression test that
the indexer's serial lock does not block normal writes, and a
frontend test covering overlapping-autosave coalescing.
```

### Next steps

You're idle on the systacean queue. While we wait for commit
clearance: (a) verify `systacean-1` is still untouched in the
tree; (b) read [../alex/setup-2.md](../alex/setup-2.md) for the
Round 2 design — Alex picked **Option B / structured JSON** for
the survey schema (against my recommendation), and **full CLI
command, zero-setup** for agent spawn. We'll lean on you for the
control-socket extension when Round 2 spawning lands. Reviewing
those answers now is bonus prep, not assigned work yet.
