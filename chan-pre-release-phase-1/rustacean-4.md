# rustacean-4: Indexer watch-loop blocker fix

Owner: rustacean. Picked up after syseng filed
`architect-syseng-2.md` as a release blocker during the
hardening pass.

Status: REVIEW.

## Goal

Resolve the blocker: a single watcher event for a symlink, FIFO,
or vanished path was sticking `/api/index/status` on `error` until
some other path successfully indexed. The dashboard then read as
"search is broken" while the underlying drive was fine.

## Approach

Adopt syseng's recommended option (a): pre-filter at the chan-
server watch path; no chan-core change needed. The walker
`chan_drive::fs_ops::walk_drive_with` already drops symlinks /
specials on cold boot; the watch path now mirrors that.

## Files changed

- `crates/chan-server/src/indexer.rs`
  - Replaced the unconditional `index_file` / `forget_file`
    dispatch in the watcher worker with a call to a new helper
    `apply_watch_change(drive, path, deleted)`.
  - The helper resolves the drive-relative path with
    `chan_drive::fs_ops::resolve_safe`, then uses explicit
    `std::fs::symlink_metadata` so symlinks are classified before
    `Drive::index_file` can follow them. It decides between four
    outcomes:
    - `Indexed` — regular file, normal path.
    - `Forgotten` — delete event (or vanished / replaced path).
    - `SkippedSpecial` — path exists but is not indexable (symlink,
      FIFO, socket, device, dir). Stale index rows are best-effort
      cleared via `forget_file`.
    - `SkippedMissing` — path resolution or `symlink_metadata`
      returns Err. Treated as a delete race.
  - The status mapping flips `Indexed` / `Forgotten` / `Skipped*`
    all to `set_idle`. Only a genuine `Drive` error (sqlite / IO
    on the index DB) still drops to `IndexStatus::Error`.

## Tests (7 in indexer module)

- `apply_watch_change_indexes_regular_file`
- `apply_watch_change_forgets_on_delete_flag`
- `apply_watch_change_skips_missing_path`
- `apply_watch_change_skips_symlink_to_existing_target` (unix)
- `apply_watch_change_skips_broken_symlink` (unix)
- `apply_watch_change_skips_fifo` (unix, mkfifo-gated; skips if
  the binary isn't on PATH so minimal-container CI stays green)
- `apply_watch_change_special_clears_prior_index_entry` (unix,
  exercises the "regular .md was replaced by a symlink" case;
  asserts `indexed_docs` drops as the stale row is cleared)

## Live repro verification

Pre-fix repro (from `architect-syseng-2.md`):

```
mkdir /tmp/chan-blocker-repro
echo hi > /tmp/chan-blocker-repro/top.md
ln -s top.md /tmp/chan-blocker-repro/alias.md
chan serve /tmp/chan-blocker-repro --no-token --no-browser --port 8790
touch -h /tmp/chan-blocker-repro/alias.md
curl http://127.0.0.1:8790/api/index/status
```

Post-fix result:

```
{"state":"idle","indexed_docs":1,"indexed_vectors":1,
 "model":"BAAI/bge-small-en-v1.5"}
```

Same probe pre-fix returned `{"state":"error", ...}`.

## Verification gate

```
cargo fmt --all -- --check                # clean
cargo build -p chan-server                # ok
cargo clippy --all-targets -- -D warnings # clean (incl. items-after-test-module)
cargo test -p chan-server                 # 92 passed
cargo test -p chan                        # 46 passed
```

## Residual / non-blocking

- syseng noted a one-off `ENOENT` against `hardlink-to-top.md`
  during the original pass. The fix routes path-resolution or
  `symlink_metadata` races into the `SkippedMissing` branch, so the
  same race no longer sticks the indexer on `Error`. If the
  underlying cause is a stale rename-log / staging race in
  chan-drive, it stays in chan-core's lane.
- The watch loop's debounce window (1 s) means a quick rename-
  then-modify burst could still produce a single SkippedMissing
  result against a path the user will index a moment later; that's
  fine because the next event on the same path will indeed call
  `index_file`. No status flap visible to the dashboard.
