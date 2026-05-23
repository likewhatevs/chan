# systacean-45 — chan-server async blocking audit fixes before v0.13.0

## Context

@@Alex asked whether chan-server still has synchronous work that can block Axum/Tokio runtime workers. The answer is yes: most heavy drive operations are already behind `tokio::task::spawn_blocking`, but several async handlers still call synchronous filesystem or synchronous chan-drive/graph/report work directly.

This is a **pre-v0.13.0 release blocker task**: remove or explicitly justify synchronous blocking work on request paths before the release cut.

## Scope

Audit and fix chan-server request paths that can block Tokio worker threads. Prefer moving bounded-but-synchronous filesystem, SQLite/graph/report, or CPU-heavy work behind `tokio::task::spawn_blocking`. Use `tokio::fs` only for simple async filesystem operations where it keeps the code clearer; for recursive walks or chan-drive calls, prefer one blocking closure per request so ownership and error mapping stay straightforward.

### Known Findings

1. `crates/chan-server/src/routes/fs_graph.rs`
   * `api_fs_graph` calls `build_fs_graph` directly.
   * `build_fs_graph` performs `canonicalize`, `symlink_metadata`, recursive `read_dir`, `read_link`, sorting, and graph assembly.
   * This is the highest-priority fix. It is bounded by depth/node caps, but still synchronous route work.

2. `crates/chan-server/src/routes/terminal.rs`
   * `api_terminal_watcher_events` calls `list_watcher_events` directly.
   * That helper uses `std::fs::read_dir`, `metadata`, and `read_to_string` on an arbitrary attached watcher directory.
   * `systacean-44` capped individual event file reads at 1 MiB, but the route is still synchronous.
   * `resolve_terminal_cwd` and `resolve_watcher_dir` also perform sync `metadata` / `create_dir_all` in route setup paths; evaluate whether these should move too.

3. `crates/chan-server/src/routes/fonts.rs`
   * `api_fonts_source_code_pro_download` uses async `reqwest`, but `download_font_files` performs `std::fs::create_dir_all`, `metadata`, `write`, and `rename` inside the async function.
   * The comment currently says heavy work runs on a Tokio blocking thread, but the implementation does not. Fix the implementation or the comment; preferred fix is to make the local filesystem work non-blocking from the runtime perspective.

4. `crates/chan-server/src/routes/index.rs`
   * `api_semantic_state` calls `build_state`, which computes model cache size via recursive synchronous `read_dir` / `metadata`.
   * `api_semantic_download` already wraps the heavy model open/download in `spawn_blocking`; keep that pattern.

5. `crates/chan-server/src/static_assets.rs`
   * `serve_font` fallback reads user-config font bytes synchronously.
   * Lower risk because files are expected to be small, but still on a request path. Either convert or document as intentionally tiny and bounded.

6. Graph/report endpoints
   * `crates/chan-server/src/routes/graph.rs`: `api_link_targets`, `api_resolve_link`, `api_headings`, `api_language_graph`, `api_graph`, `api_backlinks` call synchronous chan-drive graph/report operations directly.
   * `crates/chan-server/src/routes/report.rs`: report endpoints call synchronous report APIs directly.
   * If these operations touch SQLite or can fan out over graph/report rows, move them behind `spawn_blocking`. If any are known memory-only and cheap, document the rationale.

### Already Covered / Lower Priority

These are examples of the desired pattern and should not be churned unless the audit finds a bug:

* `routes/files.rs`: list/read/write/move are mostly behind `spawn_blocking`.
* `routes/drafts.rs`: create/promote work is behind `spawn_blocking`.
* `routes/teams.rs`: create/duplicate/load are behind `spawn_blocking`.
* `routes/storage.rs`: reset work is behind `spawn_blocking`.
* `routes/mentions.rs`: mention query is behind `spawn_blocking`.
* `routes/reports_toggle.rs`: state mutation is behind `spawn_blocking`.
* `routes/index.rs`: semantic download model open/download is behind `spawn_blocking`.

## Acceptance Criteria

1. Every chan-server async route with potentially blocking synchronous work is either:
   * moved behind `tokio::task::spawn_blocking`, or
   * explicitly documented as bounded/tiny and safe to keep on the runtime.
2. The known findings above are all addressed.
3. No lock guard is held across `.await`.
4. Error mapping remains equivalent at the HTTP boundary.
5. Add focused regression tests where route behavior or error mapping changes.
6. Verification:
   * `cargo fmt`
   * `cargo clippy --all-targets -- -D warnings`
   * `cargo test -p chan-server --lib`
   * focused tests for each touched route module
   * `cargo build --no-default-features`

## Out of Scope

* Broad chan-drive API redesign.
* SPA changes unless a route contract must change; avoid route contract changes if possible.
* Desktop/Tauri changes.
* Release workflow / CI changes.

## Coordination

This touches chan-server route handlers, so coordinate with @@FullStackA if they still have active route work. Keep the patch small and mechanical: one task, no unrelated route refactors.
