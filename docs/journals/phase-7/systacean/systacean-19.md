# systacean-19: watcher directory must stay under the drive root

Owner: @@Systacean
Cut by: @@Architect
Date: 2026-05-19

## Goal

Refuse to attach a terminal-tab watcher to any directory
outside the active drive's root. Relative paths are
already sandboxed via `chan_drive::fs_ops::resolve_safe_strict`;
the absolute-path branch is not, so a client can hand the
server an arbitrary disk path (e.g. `/etc`, `~/Downloads`)
and the watcher will accept it.

This restores the chan-drive boundary invariant for the
watcher seam: the watcher is part of user content
machinery and must respect the same drive sandbox as
every other filesystem operation.

## Relevant code

* `crates/chan-server/src/routes/terminal.rs:721` —
  `resolve_watcher_dir(drive_root, raw)`. The
  `path.is_absolute()` branch (line 727-728) takes the
  caller's absolute path as-is and never checks it
  against `drive_root`.
* `chan_drive::fs_ops::resolve_safe_strict` — the
  existing sandbox helper used for the relative-path
  branch. Read its contract before deciding whether to
  reuse it or wrap it.

## Acceptance criteria

### Behavior

* Absolute path that resolves to **inside the drive
  root** (after symlink canonicalization) → accepted,
  same as today.
* Absolute path that resolves **outside the drive root**
  → rejected with the same `invalid watcher path: ...`
  error shape the relative branch already produces.
  Wording can be reused; the failure mode the client
  sees stays consistent.
* Relative path behavior unchanged (still routed
  through `resolve_safe_strict`).
* Symlink-escape (a path *inside* the drive root that
  resolves *outside* via symlink) must also be
  rejected. Canonicalize both sides before comparing.

### Tests

* Extend `resolve_watcher_dir_rejects_empty_escape_and_files`
  (line 849) or add a sibling test that asserts an
  absolute path outside `tmp.path()` (e.g. a sibling
  tempdir, or `std::env::temp_dir()` parent) is
  rejected.
* Add a symlink-escape regression: create a symlink
  inside the drive that points outside; assert the
  absolute path to the symlink target (or to the
  symlink itself) is rejected.
* Keep
  `resolve_watcher_dir_allows_absolute_and_drive_relative_directories`
  passing — in-drive absolute paths must still work.

### Gate

* `cargo test -p chan-server --no-default-features`
* `cargo clippy -p chan-server --all-targets
  --no-default-features -- -D warnings`
* `cargo fmt --check`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* Single-user, single-machine threat model — this is
  about invariant hygiene + defense-in-depth, not a
  remote-exploit fix. The SPA is the only legitimate
  client today and only sends drive-relative paths, so
  no on-disk fallout is expected. But the boundary is
  load-bearing: anything we hand to `notify` should be
  drive-contained for the same reason every other
  filesystem path in this codebase is.
* If `resolve_safe_strict` already handles absolute
  paths cleanly (canonicalize → containment check),
  prefer routing the absolute branch through it rather
  than open-coding a second check. Check its contract
  first; pick whichever lands cleaner.

Standing topic-level commit clearance.

## 2026-05-19 14:36 BST - ready to land

Implemented the watcher containment fix in
`crates/chan-server/src/routes/terminal.rs`.

`resolve_watcher_dir` now canonicalizes the drive root and absolute
watcher path before accepting an absolute request. Paths that
canonicalize outside the drive root, including in-drive symlink escapes,
return the existing `invalid watcher path: ...` error shape. Relative
paths still use `chan_drive::fs_ops::resolve_safe_strict`.

Verification:

* `cargo test -p chan-server resolve_watcher_dir --no-default-features`
* `cargo test -p chan-server --no-default-features`
* `cargo clippy -p chan-server --all-targets --no-default-features -- -D warnings`
* `cargo fmt --check`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`
