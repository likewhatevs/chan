# backsystacean-2: chan-drive file classifier

Owner: @@Backsystacean
Status: REVIEW

## Goal

Implement the file classifier in chan-drive so the inspector and the
graph share a single verdict for every path: regular vs symlink vs
hardlink vs FIFO vs socket vs device, plus read-only / writable
permissions. Read-only directories become dead-ends in the graph;
symlinks pointing outside the drive render but are not traversed.

## Relevant links

* Request: [request.md](./request.md) architectural cleanups section,
  items 3 and onward.
* Design memo: [architect-2.md](./architect-2.md) (File classifier in
  chan-drive section).
* chan-drive design: [../crates/chan-drive/design.md](../crates/chan-drive/design.md)
* chan-drive boundary today: `Drive::write_text` /
  `Drive::write_bytes` already refuse special files on the write
  side; this is the read-side twin.

## Scope

* New module or struct in `crates/chan-drive/src/` exposing a
  classifier API. Suggested shape:
  ```rust
  pub struct PathClass {
      pub kind: PathKind,
      pub permission: PathPermission,
      pub link_count: u64,        // nlink for regular files; 1 elsewhere
      pub target: Option<PathBuf>, // symlink target, when kind is Symlink
      pub target_escapes_drive: bool,
  }
  pub enum PathKind {
      Directory,
      Symlink,
      RegularFile,
      Fifo,
      Socket,
      BlockDevice,
      CharDevice,
      Other,
  }
  pub enum PathPermission { ReadWrite, ReadOnly }
  ```
* Implementation uses `std::fs::symlink_metadata` to avoid following
  symlinks, then `FileType` / Unix `MetadataExt`. The classifier
  never opens files.
* Hardlink note: when `nlink > 1` on a regular file, set
  `link_count` and surface a flag callers can use; do not treat
  hardlinks as a separate `PathKind`. The graph rendering uses the
  flag to add a small badge.
* `target_escapes_drive` is true when the symlink resolves
  (lexically + canonicalize) to a path outside the drive root. Use
  the existing drive root accessor.
* Watcher classifier (phase 5 path) does not change. The new
  classifier is the read / inspector side; the watcher classifier
  stays focused on indexability.
* Exposure to chan-server: add an endpoint or extend
  `/api/files/<path>` so the inspector payload carries the
  classification. Coordinate with [backsystacean-3](./backsystacean-3.md)
  if a unified inspector route is the right shape.

## Out of scope

* Inspector aggregation across a subtree (lives in
  [backsystacean-3](./backsystacean-3.md)).
* Frontmatter kind ladder (lives in
  [backsystacean-4](./backsystacean-4.md)).
* Terminology codemod (lives in
  [backsystacean-5](./backsystacean-5.md)).
* libmagic / content sniffing.

## Acceptance criteria

* `PathClass` API in chan-drive, with unit tests covering each
  `PathKind` and the read-only / writable permission.
* Symlink classification correctly flags `target_escapes_drive`
  (test with a sibling symlink and an escaping symlink under a
  tempdir).
* Hardlink classification reports `link_count > 1` for files with
  multiple links (POSIX-only test; gated `#[cfg(unix)]`).
* chan-server route surface returns the classification in the
  inspector payload.
* No new heavy dependencies. Re-use chan-drive's existing
  `std::os::unix::fs::MetadataExt` usage.

## Tests

* `cargo test -p chan-drive` covering the new classifier path
  (Unix-gated).
* `cargo test -p chan-server` covering the new route surface.
* `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo build --no-default-features` all green.

## Review and hardening

* @@Backsystacean self-review on the symlink escape check
  (canonicalize order, broken symlink behavior, escape across drive
  rename).
* @@Architect to confirm the API surface matches
  [architect-2](./architect-2.md) before commit.

## Progress notes

* 2026-05-18: Added `PathClass`, `PathKind`, and
  `PathPermission` in `chan-drive::fs_ops`.
* 2026-05-18: Exposed `path_class` on `/api/files` JSON surfaces
  used by the file browser / inspector (`TreeEntryView`) and
  editable-file reads (`FileResponse`).
* 2026-05-18: Reused the classifier in `/api/fs-graph`; graph nodes
  now carry permission/link metadata and read-only directories render
  but do not expand.
* 2026-05-18: While running full server tests, fixed terminal PTY
  spawn to clear inherited `CHAN_MCP_*` variables before optionally
  setting chan's MCP env. This keeps `mcp_env=off` true even when
  the parent process has chan MCP variables.

## Completion notes

Files changed:

* `crates/chan-drive/src/fs_ops.rs`
* `crates/chan-drive/src/lib.rs`
* `crates/chan-server/src/routes/files.rs`
* `crates/chan-server/src/routes/fs_graph.rs`
* `crates/chan-server/src/terminal_sessions.rs`
* `web/src/api/types.ts`

Verification:

* `cargo test -p chan-drive classify_path`
* `cargo test -p chan-server path_class`
* `cargo test -p chan-server read_only_directory_is_a_dead_end`
* `cargo test -p chan-drive -- --test-threads=1`
* `cargo test -p chan-server`
* `cargo fmt --check`
* `cargo build --no-default-features`
* `cargo clippy --all-targets -- -D warnings`

Note: an initial parallel `cargo test -p chan-drive` run failed with
`Too many open files (os error 24)`. The same suite passed serially;
no test failure remained after lowering concurrency.

Ready for @@Architect contract review and @@Frontend consumption in
[frontend-4](./frontend-4.md).
