# backsystacean-5: terminology codemod (crates side)

Owner: @@Backsystacean
Status: REVIEW

## Goal

Replace "folder" with "directory" (or "dir" as the short form)
across the Rust crates. Coordinated with
[frontend-5](./frontend-5.md) on the web side.

## Relevant links

* Request: [request.md](./request.md) architectural cleanups section,
  item 3.1.
* Design memo: [architect-2.md](./architect-2.md) (Terminology
  section).

## Scope

Files known to carry the old vocabulary (from `grep -r folder`
across crates):

* `crates/chan/src/main.rs`
* `crates/chan-server/src/static_assets.rs`
* `crates/chan-server/src/routes/fs_graph.rs`
* `crates/chan-server/src/routes/contacts.rs`
* `crates/chan-server/src/routes/report.rs`
* `crates/chan-server/src/routes/graph.rs`
* `crates/chan-server/build.rs`
* `crates/chan-drive/src/paths.rs`
* `crates/chan-drive/tests/contacts_import.rs`
* `crates/chan-drive/src/index/vectors.rs`

Replacement rules:

* User-visible strings (CLI help, log lines, error messages):
  "folder" -> "directory", "Folder" -> "Directory".
* Doc comments and module headers: same replacement.
* Identifiers: rename `folder` / `Folder` to `directory` /
  `Directory` (or `dir` / `Dir` if the surrounding code already
  uses the short form).
* Persisted state keys: leave alone this phase (follow-up).
* External-facing API identifiers (anything used by the desktop or
  iOS shells): only rename if the rename is contained; flag in the
  task notes if it crosses the boundary.

## Out of scope

* Web-side codemod (in [frontend-5](./frontend-5.md)).
* Persisted state migration.

## Acceptance criteria

* `rg -n '[Ff]older' crates/` returns only comments referencing
  third-party libraries, if any. Record any exceptions.
* `cargo build --workspace`, `cargo test --workspace`,
  `cargo clippy --all-targets -- -D warnings`,
  `cargo build --no-default-features` all green.

## Tests

* Existing tests must continue to pass (text matches may need
  updates).

## Review and hardening

* @@Backsystacean self-review for missed identifiers in test
  fixtures.

## Progress notes

* Renamed crates-side "folder" vocabulary to "directory" / "dir"
  across CLI help, comments, tests, fs-graph scope handling, report
  route docs, language graph node naming, and drive docs.
* Renamed owned Rust identifiers such as `GraphScope::Folder`,
  `FsGraphScope::Folder`, `walk_folder`, `folder_depth_in_scope`,
  and language-graph `Folder` nodes to their `Directory` forms.
* The crates-side API vocabulary now emits / accepts
  `scope=directory`, fs-graph node kind `"directory"`, and language
  graph node IDs prefixed with `directory:`. The frontend side still
  needs the paired [frontend-5](./frontend-5.md) / graph update before
  live UI traffic should rely on this wire shape.
* `rg -n '[Ff]older' crates/` now returns only
  `crates/chan-server/src/static_assets.rs:29`, the `rust-embed`
  `#[folder = "../../web/dist/"]` macro attribute. That is a
  third-party API name and cannot be renamed.

## Completion notes

Ready for review.

Verified:

* `cargo test -p chan-server fs_graph`
* `cargo test -p chan graph_scope`
* `cargo test -p chan-server language_graph`
* `cargo build --workspace`
* `cargo test --workspace`
* `cargo clippy --all-targets -- -D warnings`
* `cargo build --no-default-features`
* `cargo fmt --check`
