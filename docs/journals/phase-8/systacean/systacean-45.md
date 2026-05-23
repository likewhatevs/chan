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

## 2026-05-23 16:38 BST — implementation report

Pre-v0.13 async-blocking cleanup implemented.

### Fixed

Moved potentially blocking request-path work off Tokio runtime workers:

* `routes/fs_graph.rs`: `api_fs_graph` now runs `build_fs_graph` in `spawn_blocking`.
* `routes/terminal.rs`: terminal CWD resolution, watcher-dir creation/metadata, and watcher event listing now run in `spawn_blocking`; event-reply writes were already async `tokio::fs`.
* `routes/fonts.rs`: Source Code Pro download now uses `tokio::fs` for local create/stat/write/rename; fixed the stale comment that claimed blocking-thread behavior.
* `static_assets.rs`: downloaded-font fallback now uses `tokio::fs::read`.
* `routes/index.rs`: semantic state, enable, disable, and download state-refresh work now run inside blocking tasks; recursive model-cache sizing stays off runtime workers.
* `routes/graph.rs`: link-targets, resolve-link, headings, links, language graph, full graph, and backlinks now run synchronous chan-drive graph/report/disk walks in blocking tasks.
* `routes/report.rs`: file/prefix/dir report endpoints now run report lookups/aggregation in blocking tasks.
* `routes/search.rs`: filename search, content search, and indexing-state filesystem/index snapshots now run in blocking tasks.
* `routes/inspector.rs`: inspector payload construction now runs in a blocking task.
* `routes/attachments.rs`: collision probing and attachment write now run in a blocking task.
* `routes/contacts.rs`: contact list query and CSV parse/import now run in blocking tasks; post-import indexing was already blocking.
* `routes/drive.rs`: drive rename metadata refresh and cloud-drive discovery now run in blocking tasks.

### Intentionally left

* `routes/files.rs`: left untouched per @@Architect coordination note; @@FullStackA already owns the `-96 sub-4` spawn-blocking fix there.
* Small lock-only/config reads remain on runtime workers where they do not perform filesystem, SQLite, network, recursive walk, or CPU-heavy work. No lock guard is held across `.await` in this patch.

### Verification

* `cargo fmt`
* `cargo check -p chan-server`
* Focused route tests:
  * `cargo test -p chan-server routes::terminal::tests`
  * `cargo test -p chan-server routes::graph::tests`
  * `cargo test -p chan-server routes::search::tests`
  * `cargo test -p chan-server routes::inspector::tests`
  * `cargo test -p chan-server routes::fs_graph::tests`
  * `cargo test -p chan-server static_assets::tests`
* `cargo test -p chan-server --lib`: 255/0
* `cargo clippy --all-targets -- -D warnings`
* `cargo build --no-default-features`

### Notes

No route contract changes intended; error bodies keep the same path for ordinary handler failures, with new 500s only for impossible `spawn_blocking` task panics.

## 2026-05-23 — @@Architect: approved + commit clearance

Excellent comprehensive audit + cleanup. Twelve route handlers + static_assets moved cleanly to `spawn_blocking` / `tokio::fs`, matching the `-96 sub-4` shape on the read-side. The "no lock guard held across `.await`" discipline is exactly what we want; this kind of audit prevents the worker-starvation bugs @@Alex was hitting with v0.12.0.

Verified: 255/0 cargo test -p chan-server --lib; clippy + check + no-default-features build all clean. Solid.

### Notes on the cleanup pattern

The new 500s on impossible-`spawn_blocking`-task-panics are acceptable — they only fire if the worker pool poisons, which is "the whole runtime is dead" territory anyway.

The conservative choice to leave small lock-only/config reads on runtime workers is correct — micro-tasks don't benefit from `spawn_blocking` overhead, and the audit you did to classify them is the right durable artifact.

Worktree-coordination discipline preserved: @@WebtestA's journal files + @@FullStackA's `-102` SPA files left untouched by your add set. Clean.

### Suggested commit subject

```
crates: chan-server async-blocking cleanup — 12 route handlers + static_assets to spawn_blocking / tokio::fs (systacean-45; pre-v0.13.0 release blocker)
```

### Commit instructions

Per the standing pre-authorization for your lane:

* Strict per-path `git add` only. Stage:
  * The 11 modified `crates/chan-server/src/routes/*.rs` files (attachments / contacts / drive / fonts / fs_graph / graph / index / inspector / report / search / terminal).
  * `crates/chan-server/src/static_assets.rs`.
  * `docs/journals/phase-8/systacean/systacean-45.md` (task + this clearance).
  * `docs/journals/phase-8/alex/event-systacean-architect.md` (your inbound poke about -45 completion).
* Pre-commit `git diff --staged --stat` + post-commit `git show --stat HEAD`.
* @@FullStackA has `-102` SPA work in WT; @@WebtestA has journal-only changes. DO NOT add their files.

### Lane state post-`-45`

Queue-empty. Thank you for the careful pass.

## 2026-05-23 — teardown-complete

No `chan serve`, `cargo build`, or `cargo test` processes left running from this lane. No throwaway drives or Chrome MCP tabs to clean up. Standing down final for phase 8.
