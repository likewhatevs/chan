# architect-syseng-2: Indexer trips on symlinks/specials -> /api/index/status stuck on `error`

Status: REVIEW, resolved in this repo.

From: syseng. To: architect. Severity: release blocker.

Surfaced by the syseng-1 live hardening pass against
`/tmp/chan-syseng-fixture`. Detail and reproduction steps live in
`syseng-1.md` under "## 2026-05-16 Live hardening pass" → "Blocker".

## TL;DR

Any drive that contains a symlink, hardlink, or FIFO will flip
`/api/index/status` to `state: "error"` as soon as a watcher event
fires on that path, and the status will not recover until a
successful `index_file` lands for some other path. First-time users
hitting the search dashboard would see "indexer broken" while the
underlying drive is fine.

## Repro

1. `mkdir /tmp/repro && echo hi > /tmp/repro/top.md && ln -s top.md /tmp/repro/alias.md`
2. `chan serve /tmp/repro --no-token --no-browser --port 8789`
3. `touch -h /tmp/repro/alias.md`
4. `curl http://127.0.0.1:8789/api/index/status` -> `state: error`

## Root cause (two layers)

1. `crates/chan-server/src/indexer.rs:280-291` (this repo). The
   watcher apply loop:

   ```
   let result = if change.deleted {
       tokio::task::spawn_blocking(move || drive2.forget_file(&p)).await
   } else {
       tokio::task::spawn_blocking(move || drive2.index_file(&p)).await
   };
   match result {
       Ok(Ok(())) => set_idle(&drive_w, &status_w),
       Ok(Err(e)) => {
           *status_w.lock().unwrap() = IndexStatus::Error {
               message: format!("{}: {e}", change.path),
           };
       }
       ...
   }
   ```

   Any error -> flips global `IndexStatus::Error`. No filter for
   special files; no recovery short of a successful event on a
   different path.

2. `chan_drive::Drive::index_file` -> `read_text` ->
   `ensure_regular_file_in` rejects symlinks/FIFOs/sockets/devices
   with `ChanError::SpecialFile`. The full-drive walker
   `fs_ops::walk_drive_with` silently drops these, but the per-file
   watch path has no equivalent pre-filter.

   Additionally, a `hardlink-to-top.md: io error: No such file or
   directory (os error 2)` was observed once. Hardlinks are regular
   files; the ENOENT may be a stale rename-log/staging race. Not
   fully traced; symptom reproduced twice. Worth a separate look.

## Suggested fix shape

Pick one of (a) or (b), or both:

(a) In `chan-server/src/indexer.rs` watch loop, pre-filter:
   `if !fs_ops::is_indexable_text(&change.path) { skip }` plus an
   lstat gate that drops symlinks/specials before calling
   `drive.index_file`. Treat `SpecialFile` and "file is gone"
   errors as `set_idle`, not `Error`.

(b) In `chan_drive::Drive::index_file`, return `Ok(())` for paths
   where `ensure_regular_file_in` reports `SpecialFile` (silent
   ignore, matching the walker's behavior). The caller would still
   need to handle the ENOENT case.

Either way, the watcher path's "any error -> permanent global
error state" needs softening: a single bad-path event should not
stick the whole index status to `error` until something else
indexes successfully.

## Resolution

Implemented the chan-server side fix in
`crates/chan-server/src/indexer.rs`:

- watcher apply now classifies paths with explicit
  `std::fs::symlink_metadata` after resolving the drive-relative path
  under the drive root
- regular files still call `Drive::index_file`
- symlinks, FIFOs, sockets, devices, and directories are skipped and
  any stale index row is best-effort forgotten
- missing paths are treated like delete races and best-effort forgotten
- these cases return to idle instead of sticking `/api/index/status` in
  `error`

Regression coverage added in the indexer tests:

- regular file indexes
- delete event forgets
- missing path skips
- symlink to existing target skips
- broken symlink skips
- FIFO skips
- replacing an indexed markdown file with a symlink clears the stale
  search row

Live repro after the fix:

```
target/debug/chan serve /tmp/chan-syseng-fixture-codex --here --no-token --no-browser --port 18789
touch -h /tmp/chan-syseng-fixture-codex/notes/alias-to-top.md
curl -sS 'http://127.0.0.1:18789/api/index/status'
```

Result:

```
{"state":"idle","indexed_docs":4,"indexed_vectors":4,"model":"BAAI/bge-small-en-v1.5"}
```

Verification:

```
cargo test -p chan -p chan-server   # 39 + 85 passed
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Original suggested dispatch

- This repo: small chan-server PR (probably
  `rustacean-4.md`) for the watch-loop filter + error-class
  softening. Self-contained; no chan-core changes required if (a)
  is chosen.
- chan-core: open question whether `Drive::index_file` should
  silently ignore special-file targets. If yes, `chan-core-purge-2.md`
  (or similar) wires that in. If we pick (a) only, no chan-core
  change.

## syseng status

`syseng-1` is in REVIEW and this blocker is resolved. No chan-core
change is needed for this blocker.
