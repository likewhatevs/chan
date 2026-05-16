# @@Backend task 1: Markdown-only tag extraction

Owner: @@Backend
Status: Ready for specialist review

## Goal

Only index `#tag` graph edges from Markdown files. Do not let non-Markdown
files such as `.txt` or source-like text produce tag nodes.

## Relevant Links

- [[chan-pre-release-phase-2/request.md]]
- [[chan-pre-release-phase-1/summary.md]]

## Acceptance Criteria

- `#tag` tokens in `.md` files still create graph tag edges.
- `#tag` tokens in non-Markdown indexed text do not create graph tag edges.
- Existing search/index behavior for indexed text files is otherwise unchanged.

## Test Expectations

- Add or update focused Rust regression coverage in `chan-drive`.
- Run the smallest useful Rust tests for the changed behavior.

## Progress Notes

- Started by reviewing [[chan-pre-release-phase-2/request.md]] and
  [[chan-pre-release-phase-1/summary.md]].
- Found token extraction happens in sibling `chan-core` via
  `Drive::parse_for_graph`, which is consumed by this repo through the local
  `chan-drive` path dependency.
- Changed `chan-drive` so graph tag extraction runs only when the indexed
  source path is a Markdown file.
- Left `.txt` indexing/search/headings/link extraction behavior unchanged.

## Completion Notes

Changed files:

- `../chan-core/crates/chan-drive/src/fs_ops.rs`
- `../chan-core/crates/chan-drive/src/drive.rs`
- `../chan-core/crates/chan-drive/tests/file_types.rs`
- `chan-pre-release-phase-2/backend-1.md`

Tests run:

- `cargo test -p chan-drive file_type_policy_end_to_end`
- `cargo test -p chan-drive fs_ops::tests::is_markdown_file_excludes_plain_text`
- `cargo test -p chan-server api_search_content` (compiled, filter matched 0 tests)
- `cargo test -p chan-server`

Review expectations:

- @@Rustacean should review the `chan-drive` API/helper addition and test scope.

Commit readiness:

- Ready after @@Rustacean review.
- Known risk: this intentionally stops `.txt` files from creating tag edges;
  `.txt` remains searchable and linked as before.
