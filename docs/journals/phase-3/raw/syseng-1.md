# syseng-1: Hardening pass for session, status, graph, and filesystem behavior

Owner: @@Syseng.

Status: REVIEW.

Related:

- [request.md](./request.md)
- [journal.md](./journal.md)
- [backend-1.md](./backend-1.md)
- [backend-2.md](./backend-2.md)
- [rustacean-1.md](./rustacean-1.md)

## Goal

Provide hardening review for phase 3 behavior that depends on processes,
persisted state, filesystem semantics, or operational event routing.

## Acceptance criteria

- Review agent CLI resume/session behavior for stale state, wrong backend
  selection, process command confusion, and persistence edge cases.
- Review status event routing for stale events and wrong overlay targets.
- Review graph changes involving folders, symlinks, hardlinks, broken links,
  common ancestors, and path-to-root behavior.
- Record concrete residual risks and recommended fixes.

## Test expectations

- Add targeted repro scripts or command sequences where useful.
- Run relevant Rust/backend tests after fixes are available.
- Record exact commands and results.

## Progress notes

- 2026-05-16 @@Syseng: Started baseline hardening prep. Upstream
  implementation/review notes from [backend-1.md](./backend-1.md),
  [backend-2.md](./backend-2.md), and [rustacean-1.md](./rustacean-1.md) are
  not available yet, so initial work is limited to mapping relevant code paths
  and identifying edge cases to apply once fixes land.
- 2026-05-16 @@Syseng: Baseline code map:
  - Agent CLI process discovery/execution: `crates/chan-llm/src/cli.rs`,
    `crates/chan-llm/src/session.rs`, and
    `crates/chan-llm/src/backends/*_cli.rs`.
  - Agent stream/status routing:
    `crates/chan-server/src/bus.rs`,
    `crates/chan-server/src/routes/llm.rs`, and
    `web/src/state/store.svelte.ts`.
  - Graph/path semantics:
    `crates/chan-server/src/routes/graph.rs`,
    `crates/chan-server/src/routes/fs_graph.rs`, and
    `crates/chan-drive/src/fs_ops.rs`.
- 2026-05-16 @@Syseng: Baseline hardening observations before upstream changes:
  - CLI discovery already canonicalizes the executable, rejects non-regular or
    non-executable candidates, rejects group/other-writable binaries, rejects
    world-writable containing dirs, preserves wrapper args, and passes a bounded
    child `PATH`.
  - LLM WS frames carry `session_id` and frontend routing drops stale frames
    when they do not match `assistantStream.sessionId`.
  - Filesystem graph uses lstat semantics, does not traverse symlinks, caps
    depth/node count, emits hardlink/symlink/ghost edges, and explicitly rejects
    mid-path symlink escapes while allowing symlink leaves to be represented.
  - Current status-bar transient messages are plain strings; routeable status
    click behavior will need a typed target/source field or equivalent frontend
    routing state to avoid stale/wrong-overlay clicks.
- 2026-05-16 @@Syseng: Baseline verification:
  - `cargo test -p chan-server fs_graph` passed: 18 passed.
  - `cargo test -p chan-server bus::tests` passed: 4 passed.
  - `cargo test -p chan-server forwards_status_activity_and_user_request`
    passed: 1 passed.
  - `cargo test -p chan-server cli_detection_public_tunnel_shape_has_three_backends`
    passed: 1 passed.
  - `cargo test -p chan-llm host_path_wins_before_conventional_dirs` passed:
    1 passed.
  - `cargo test -p chan-llm configured_wrapper_keeps_args_and_resolves_first_argv`
    passed: 1 passed.
  - `cargo test -p chan-llm heartbeat_fires_while_child_is_alive_but_quiet`
    passed when isolated: 2 passed.
  - `cargo test -p chan-llm cli::tests` was too broad because the substring
    also matched backend `*_cli::tests`; it failed two heartbeat tests under
    concurrent load, and both passed in the isolated rerun above.
- 2026-05-16 @@Syseng: Waiting for implementation/review notes from
  [backend-1.md](./backend-1.md), [backend-2.md](./backend-2.md), and
  [rustacean-1.md](./rustacean-1.md) before final hardening review or fixes.
- 2026-05-16 @@Syseng: Noted an existing, unowned workspace edit in
  `crates/chan-llm/src/session.rs`: `LlmSession::backend()` now reports
  `active_backend()` and adds
  `backend_reports_none_when_selected_backend_is_disabled`. This aligns with
  the stale/wrong-backend hardening concern; I did not modify the file.
  Verification: `cargo test -p chan-llm backend_reports_none_when_selected_backend_is_disabled`
  passed: 1 passed.
- 2026-05-16 @@Syseng: Reviewed [backend-1.md](./backend-1.md) and
  [backend-2.md](./backend-2.md) after they moved to REVIEW.
  - Session/process selection: no backend-side resume process is persisted; each
    `/api/llm/complete` builds a fresh `LlmSession`, and `send()` gates process
    spawn through `active_backend()`. Rustacean's `LlmSession::backend()` change
    removes a stale selected-but-disabled backend footgun for future callers.
  - Command dispatch: CLI discovery remains bounded and canonicalized before
    spawn; wrapper args are preserved and child `PATH` is constructed from the
    same bounded search set.
  - Status routing: current WS frames carry `session_id`; stale-frame rejection
    is already frontend-side. No backend schema change is required for routing
    agent events, but the frontend click target must be typed state, not just a
    transient status string.
  - Graph/filesystem semantics: `/api/graph` and `/api/fs-graph` share
    drive-relative POSIX path ids for in-drive resources; fs graph uses lstat
    semantics, depth/node caps, non-traversed symlink classification, hardlink
    edges, and mid-path symlink escape rejection.
  - Documentation risk: [backend-2.md](./backend-2.md) describes fs graph node
    kinds as `dir`, `ghost-outside`, and `ghost-broken`, but the implementation
    emits `kind: "folder"` or `kind: "ghost"` with `outside` / `broken` flags.
    Frontend should follow the code/API types, not that prose.
- 2026-05-16 @@Syseng: Additional targeted verification:
  - `cargo test -p chan-llm session::tests::backend_reports_none_when_selected_backend_is_disabled -- --exact`
    passed: 1 passed.
  - `cargo test -p chan-server routes::fs_graph::tests::build_fs_graph_rejects_mid_path_symlink_escape -- --exact`
    passed: 1 passed.

## Completion notes

- Hardening review for current backend/rustacean graph, process, session, and
  status-routing scope is complete.
- No code changes from @@Syseng.
- No blocking findings.
- Residual follow-ups:
  - Frontend status-bar click routing should keep a typed target/source object
    with a freshness check; do not route solely from the latest status string.
  - Frontend graph work should use actual fs-graph node kinds
    (`folder`, `file`, `symlink`, `ghost` + flags), not the stale wording in
    [backend-2.md](./backend-2.md).
  - If a future implementation adds persisted CLI session ids, backend labels,
    or resumable subprocess state, request another @@Syseng review before
    commit.

## Commit readiness notes

- Ready for @@Architect review. No @@Syseng code changes to commit.
