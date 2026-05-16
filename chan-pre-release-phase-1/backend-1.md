# backend-1: CLI graph/status parity

Owner: backend

## Scope

- Add backend CLI coverage for release-roadmap command-line parity.
- First slice: `chan graph`, `chan status`, and `chan config`.
- `chan config` now covers editor prefs and server config. Assistant config remains a follow-up because it is owned by chan-llm and has backend-specific validation.

## Completed

- Added `chan status [PATH] [--json]`.
  - Reports drive root/name, BM25/vector index stats, graph file/edge/tag counts, and chan-report SLOC/language/COCOMO summary.
- Added `chan graph <PATH>`.
  - Supports `--scope all|file|folder`.
  - Supports folder `--target`, `--depth`, text output limit, and `--json`.
  - Folder scope starts at depth 1, matching the roadmap request.
- Added unit coverage for folder depth scope selection.
- Added / reviewed `chan config get|set`.
  - Supports `editor.*` keys for theme, editor theme, line spacing, date format, and pane widths.
  - Supports `server.attachments_dir` and `server.answers_dir`.
  - Whole-config output now includes both `[editor]` and `[server]` namespaces, with `--json` for tooling.
- Reviewed `/api/fs-graph` route wiring and tests.
  - Route covers filesystem graph nodes/edges for folders, files, symlinks, hardlinks, and ghosts.
  - Existing focused fs-graph tests pass.

## Verification

- `cargo fmt`
- `cargo test -p chan graph_scope_depth`
- `cargo test -p chan`
- `cargo build -p chan`
- `target/debug/chan graph --help`
- `target/debug/chan status --help`
- `cargo test -p chan config_`
- `cargo test -p chan-server fs_graph`
- `cargo test -p chan-server`

## Follow-ups

- Extend `chan config get|set` to assistant settings after deciding the CLI shape for chan-llm backend/model/cmd overrides.
- Decide whether `chan graph --scope folder` should include outgoing edges to files outside the folder. Current implementation keeps scoped folder output internal to the selected node set.
- Decide whether `chan graph` should query `/api/fs-graph`-equivalent filesystem structure or stay focused on the semantic link graph. Current CLI graph is over the existing content/link graph index.
