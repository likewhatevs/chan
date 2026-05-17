# rustacean-1: Rust quality review for agent CLI/backend changes

Owner: @@Rustacean.

Status: REVIEW.

Related:

- [request.md](./request.md)
- [journal.md](./journal.md)
- [backend-1.md](./backend-1.md)
- [backend-2.md](./backend-2.md)

## Goal

Review and support Rust changes needed for phase 3:

- Agent CLI resume/session correctness.
- Agent/Assistant naming cleanup in Rust-owned code.
- Agent metadata/status event response changes.
- Graph endpoint changes from [backend-2.md](./backend-2.md), if any.

## Acceptance criteria

- Identify Rust API/schema/config names that must remain `assistant` for
  compatibility versus names that should become `agent`.
- Review or implement focused fixes after [backend-1.md](./backend-1.md) freezes
  the bug shape.
- Ensure tests cover resume/session selection and any changed response shape.
- Keep dependency changes out unless clearly necessary.

## Test expectations

- Run the smallest relevant `cargo test` packages first.
- Run `cargo fmt --all -- --check`.
- Run `cargo clippy --all-targets -- -D warnings` before marking REVIEW.
- Record exact commands and results.

## Review expectations

- Coordinate with @@Backend through task notes.
- Request @@Syseng review for process/path/session persistence behavior.

## Progress notes

- 2026-05-16 @@Rustacean: started. Read [journal.md](./journal.md),
  [request.md](./request.md), [backend-1.md](./backend-1.md), and Rust
  agent/assistant references.
- Initial compatibility map:
  - Must remain `assistant` for now: LLM transcript role
    `Role::Assistant` / JSON role `"assistant"` in
    `crates/chan-llm/src/session.rs` and backend protocol parsers; Claude,
    Gemini, and Codex upstream event names; persisted assistant conversation
    blob routes `/api/assistant/*`; drive state directory names for assistant
    blobs; existing `assistant.*` config keys and `/api/config.preferences.assistant`
    response fields until a compatibility alias/migration plan is agreed.
  - Good rename candidates: CLI/help/user-facing strings (`Assistant` shortcut
    label, "loading/saving assistant config" contexts, errors mentioning the
    UI concept), Rust server preference view type names (`AssistantPrefsView`,
    `AssistantBackendKind`) if serde field names remain compatible, docs/comments
    that describe the product feature rather than LLM protocol roles.
  - Already agent-oriented and should stay: `AgentStatus`, `AgentActivity`, and
    backend labels `claude_cli` / `gemini_cli` / `codex_cli`.
- Observations for [backend-1.md](./backend-1.md):
  - `LlmConfig::active_backend()` correctly gates the selected backend on the
    matching enabled flag.
  - `/api/llm/status` reports `backend = cfg.backend.unwrap_or(ClaudeCli)` even
    when `enabled = false`; that preserves selected-provider display, but could
    mislead banner selection if the frontend treats status backend as the active
    chat backend after a stale or failed preferences load.
  - `LlmSession::backend()` returns raw `config.backend`, not
    `active_backend()`. No current Rust call sites found, but this should be
    reviewed before adding resume/session selection logic.
  - Status/activity websocket frames already carry `session_id` and backend
    inside the event body; frontend may need routing metadata rather than Rust
    schema changes.
- Changed `crates/chan-llm/src/session.rs` so `LlmSession::backend()` reports
  `active_backend()` rather than the sticky raw `backend`, plus a regression
  test for selected-but-disabled backend state. This is intended to keep any
  future session/banner caller from treating a disabled stale default as the
  active agent.
- Verification:
  - `cargo test -p chan-llm config::tests::active_backend_gates_on_enabled -- --exact`
    passed: 1 test.
  - `cargo test -p chan-llm session::tests::disabled_backend_reports_not_configured -- --exact`
    matched 0 tests; replaced with the exact test below.
  - `cargo fmt --all -- --check` passed.
  - `cargo test -p chan-llm session::tests::backend_reports_none_when_selected_backend_is_disabled -- --exact`
    passed: 1 test.
- 2026-05-16 @@Rustacean: checked [journal.md](./journal.md) and all
  phase task files for updates. No new @@Rustacean task file or backend findings
  yet. [syseng-1.md](./syseng-1.md) independently noted the `session.rs` change
  aligns with stale/wrong-backend hardening and verified
  `cargo test -p chan-llm backend_reports_none_when_selected_backend_is_disabled`
  passed.
- 2026-05-16 @@Rustacean: reviewed [backend-1.md](./backend-1.md) after it
  moved to REVIEW. No blocking Rust issues found. Recorded review notes in
  [backend-1.md](./backend-1.md).
- Backend-2 graph audit landed no Rust code changes, so no Rust implementation
  review was required for [backend-2.md](./backend-2.md).
- 2026-05-16 @@Rustacean: reviewed [backend-3.md](./backend-3.md) after it
  moved to REVIEW. No blocking Rust issues found. Recorded review notes in
  [backend-3.md](./backend-3.md).
- Final verification for Rustacean review:
  - `cargo test -p chan-llm session::tests::backend_reports_none_when_selected_backend_is_disabled -- --exact`
    passed: 1 test.
  - `cargo fmt --all -- --check` passed.
  - `cargo test -p chan config_assistant_keys_round_trip -- --exact` matched
    0 tests; reran without `--exact`.
  - `cargo test -p chan config_assistant_keys_round_trip` passed: 1 test.
  - `cargo clippy --all-targets -- -D warnings` passed.
- Backend-3 verification:
  - `cargo test -p chan-server line_spacing` passed: 4 tests.
  - `cargo test -p chan config_line_spacing` passed: 4 tests.
  - `cargo fmt --all -- --check` passed.
  - `cargo clippy --all-targets -- -D warnings` passed.
  - `cargo test -p chan-server -p chan` passed: 107 chan-server tests and 54
    chan tests.

## Commit readiness notes

- Ready for @@Architect review/commit coordination. Files changed by
  @@Rustacean:
  - `crates/chan-llm/src/session.rs`
  - [rustacean-1.md](./rustacean-1.md)
  - [backend-1.md](./backend-1.md) review notes
  - [backend-3.md](./backend-3.md) review notes

Known risks:
- The `LlmSession::backend()` behavior change is intentionally narrower than a
  schema/API rename, but any external caller expecting the sticky configured
  default from this method should instead read config directly. No Rust call
  sites currently depend on the old behavior.

Proposed commit message:

```
chan-llm: report only active session backend

Return the launchable active backend from LlmSession::backend() instead of the
sticky configured default, so selected-but-disabled providers cannot be reused
as stale session/banner signals.

Adds a regression test for the selected-but-disabled state. Keeps assistant
schema/API names intact for compatibility.
```
