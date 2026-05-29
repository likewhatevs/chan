@@Architect

Findings

MCP server broken for Codex v0.13.0

- Repro status: reproduced before fix.
- Evidence: a Content-Length framed initialize request to
  `target/debug/chan __mcp /tmp/chan-wave1-mcp-drive` failed with
  `serde error expected value at line 1 column 1` and `connection closed:
  initialize request`. The same JSON as a newline-delimited request
  succeeded.
- Root cause: `rmcp` 1.6 async_rw transport is newline JSON, while Codex
  stdio MCP clients send Content-Length framed JSON-RPC. `chan __mcp`
  and chan-server's MCP socket both reached `Server::serve_io`, so the
  transport mismatch was below both entry points.
- Suspected owner/module: `chan-llm` MCP transport boundary,
  `crates/chan-llm/src/mcp.rs`.
- Behavior changed: `Server::serve_stdio` and `Server::serve_io` now accept
  either Content-Length framed MCP or the existing newline JSON transport.
  Framed responses are converted back to Content-Length. Newline behavior is
  preserved.
- Files changed: `crates/chan-llm/src/mcp.rs`.
- Recommended commit boundary: MCP transport compatibility only.

Too many open files while editing

- Repro status: not reproduced as a deterministic live editor failure in
  this turn. Confirmed the local soft limit is 256 with `ulimit -n`.
  Existing Phase 7 and Phase 8 journal evidence records full test runs
  hitting macOS fd 256 during chan-drive reindex tests, with serial or
  higher-limit reruns passing.
- Evidence: a targeted serial reindex group passed:
  `cargo test -p chan-drive --lib drive::tests::reindex -- --test-threads=1`.
- Root cause candidate: aggregate fd pressure, not a single proven leak.
  Current contributors are:
  - `chan-drive` graph reader pool opens SQLite read connections per drive.
  - `chan-drive` Tantivy index opens writer and reader resources.
  - `chan-drive` `Index::build_all` runs parallel file read/chunk workers.
  - `chan-server` terminal defaults allow 32 sessions; each spawned session
    opens a PTY/child pipes and two OS threads.
- Suspected owner/module: `chan-server` terminal/session admission,
  `chan-drive` graph/index resource caps, and future multi-drive server
  resource budgeting.
- Behavior changed: new/restart terminal session creation now checks process
  fd headroom on Unix before spawning a PTY. When the process is too close to
  `RLIMIT_NOFILE`, HTTP terminal creation/restart returns 503 and terminal WS
  creation sends an error frame plus close code 1013. Existing sessions and
  attaches are unchanged. Configured session caps are unchanged.
- Files changed: `crates/chan-server/Cargo.toml`,
  `crates/chan-server/src/terminal_sessions.rs`,
  `crates/chan-server/src/routes/terminal.rs`.
- Recommended commit boundary: fd admission-control batch, separate from MCP.

Search/index changing the file being edited

- Repro status: root-caused enough to reject "search writes the active file"
  as the direct mechanism. No live UI repro was run in this lane.
- Evidence: server bus intentionally forwards watcher events to the indexer
  before self-write suppression, because in-app saves must be indexed. The
  indexer path re-reads and commits graph/search state. The normal index
  path does not write user content. The file write API still accepts
  seconds-precision `expected_mtime` over HTTP, then translates to the
  nanosecond CAS path only after that seconds check.
- Root cause candidate: either stale watcher/index/graph state is being
  reflected back into the editor by the UI, or seconds-precision write CAS
  permits sub-second overwrite confusion. The lower layer indexer itself is
  not the direct file mutator for ordinary edits.
- Suspected owner/module: `chan-server/src/routes/files.rs` wire contract for
  edit CAS, `chan-server/src/bus.rs` and `self_writes.rs` event ordering,
  plus frontend editor state application.
- Behavior changed: none yet.
- Recommended commit boundary: first add a server-side nanosecond mtime wire
  field and regression test, then coordinate any UI state fix separately.

`[[` search discrepancy

- Repro status: code-level discrepancy confirmed.
- Evidence: `/api/search/files` is a server-side substring scan over
  `drive.list_tree()`. `/api/search/content` calls `drive.search()` over the
  content index. A query that exists in note body but not in path can appear
  in content search while missing from `[[` file/typeahead search.
- Root cause: two different backends and matching semantics.
- Suspected owner/module: `chan-server/src/routes/search.rs`, with frontend
  ownership for deciding which endpoint powers `[[`.
- Behavior changed: none yet.
- Recommended commit boundary: product/UX decision first. Backend can expose
  a combined endpoint or extend file search, but the desired semantics need
  dispatch.

Synchronous/blocking calls that can block tokio runtime

- Repro status: no blocking-runtime repro captured in this turn.
- Evidence: Phase 8 says most route handlers were wrapped, but lower-layer
  startup/config/index paths still use sync fs, SQLite, Tantivy, and PTY
  operations. Current code still has sync indexing and graph work below
  server routes.
- Root cause candidate: remaining sync work is mostly in lower-layer
  libraries and lifecycle paths. It is safe if called through
  `spawn_blocking` or outside hot async request paths, unsafe if called
  directly from runtime workers during interactive editing.
- Suspected owner/module: audit boundary between `chan-server` routes/tasks
  and `chan-drive` sync APIs.
- Behavior changed: none yet.
- Recommended commit boundary: blocking audit batch after MCP and fd triage.

Proposed batches

1. MCP transport compatibility.
   - Status: implemented and verified.
   - Scope: `chan-llm/src/mcp.rs` only.

2. FD budget and admission control.
   - Status: terminal session admission control implemented and verified.
   - Added per-process fd headroom detection on Unix.
   - Reject new/restart terminal sessions when headroom is low.
   - Keep edit/write paths above background search/index work.
   - Do not change durable defaults such as terminal session cap without
     product approval.

3. Edit/index interference.
   - Status: backend CAS fix implemented and verified.
   - Added optional string `expected_mtime_ns` to `PUT /api/files/{path}`.
   - Added string `mtime_ns` and `current_mtime_ns` response fields.
   - Preserved old seconds `expected_mtime`, `mtime`, and `current_mtime`
     fields for current clients.
   - Confirm frontend does not apply index/search payloads as editor content.

4. Search semantics for `[[`.
   - Decide whether `[[` should be path-only, note-title-plus-alias, or
     content-inclusive.
   - Implement the selected backend contract in `chan-server` and pin with
     endpoint tests.

5. Metadata isolation and multi-drive server planning.
   - Status: active registry/path-key batch was completed enough to compile
     after Architect's path-key metadata decision.
   - Put per-drive metadata under `~/.chan/drives/{path-key}/`.
   - Keep the `chan` binary fixed on `~/.chan` for macOS/Linux.
   - Added/kept in-memory compatibility aliases on `KnownDrive` so older
     chan/chan-server call sites compile while the on-disk registry stops
     persisting user-facing names.
   - Multi-drive server routing and shared scheduler remain future work.

Test plan

- Done:
  - `cargo fmt --check`
  - `git diff --check`
  - `cargo test -p chan-server mcp_discovery::tests::codex_publish -- --nocapture`
  - `cargo test -p chan-drive --lib drive::tests::reindex -- --test-threads=1`
  - `cargo test -p chan-llm --features mcp initialize_roundtrips -- --nocapture`
  - `cargo test -p chan-llm --features mcp newline_initialize_still_roundtrips -- --nocapture`
  - `cargo test -p chan-llm --features mcp --lib mcp::tests -- --test-threads=1`
  - `cargo test -p chan-server terminal_sessions::tests::fd_headroom_keeps_terminal_spawns_away_from_process_limit -- --nocapture`
  - `cargo test -p chan-server terminal_sessions::tests::cap_exceeded_refuses_create -- --nocapture`
  - `cargo test -p chan-server routes::terminal::tests::api_create_terminal_spawns_command_and_returns_session -- --nocapture`
  - `cargo test -p chan-server routes::files::write_tests -- --nocapture`
  - `cargo test -p chan-server routes::files::tests::file_response_serializes_path_class_for_inspector_payload -- --nocapture`
  - `cargo check -p chan-drive`
  - `cargo test -p chan-drive registry::tests -- --nocapture`
  - `cargo test -p chan-drive paths::tests -- --nocapture`
  - `cargo test -p chan-drive library::tests::register_then_list -- --nocapture`
  - `cargo test -p chan-drive library::tests::move_drive_preserves_metadata_key_and_metadata_dirs -- --nocapture`
  - `cargo test -p chan-drive library::tests::sweep_orphans_in_reclaims_unknown_metadata_keys -- --nocapture`
  - `cargo build -p chan`
  - Raw Content-Length probe:
```bash
node -e 'const msg=JSON.stringify({jsonrpc:"2.0",id:1,method:"initialize",params:{protocolVersion:"2024-11-05",capabilities:{},clientInfo:{name:"probe",version:"0"}}}); process.stdout.write(`Content-Length: ${Buffer.byteLength(msg)}\r\n\r\n${msg}`)' | env HOME=/tmp/chan-wave1-mcp-home timeout 5 target/debug/chan __mcp /tmp/chan-wave1-mcp-drive
```
  - Raw newline probe:
```bash
printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"probe","version":"0"}}}\n' | env HOME=/tmp/chan-wave1-mcp-home timeout 5 target/debug/chan __mcp /tmp/chan-wave1-mcp-drive
```

- Still needed:
  - A live editor fd pressure repro with many terminal sessions plus active
    indexing under `ulimit -n 256`.
  - Background search/index fd admission, if Architect wants that in Wave 1.
  - Frontend adoption of `mtime_ns` as the editor CAS token.
  - A rapid-edit browser/server repro for stale index or frontend state races.
  - Endpoint tests for whichever `[[` search contract Architect selects.
  - Blocking audit for remaining direct sync calls from async server paths.

Risks / sequencing constraints

- MCP fix is safe to land alone. It adds a compatibility adapter and keeps
  existing newline clients working.
- FD admission is intentionally limited to new terminal process creation.
  Broader background indexing throttles should stay separate.
- Edit/index fixes may require frontend and backend coordination. Backend
  now exposes ns CAS, but current frontend still sends seconds unless updated.
- Metadata isolation is only partially complete. Registry/path-key and current
  compile fallout are handled; multi-drive routing and resource scheduling are
  not.

Known gaps

- Broad edit note: I touched active path-key metadata files to unblock the
  workspace after Architect's registry change left chan-drive uncompilable.
- No destructive actions taken.
- `docs/journals/phase-9/roadmap-round1.md` was read but not edited.
