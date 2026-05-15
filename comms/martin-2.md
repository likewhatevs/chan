# martin-2: Claude implementation on common events

From: Alice
Owner: Martin
Status: implementation + tests landed in working tree; commit blocked
on Alice's pending session.rs / README / design.md / lib.rs hunks

## Context

Alice landed the common additive listener API:

- `AgentStatus`
- `AgentActivity`
- `UserRequest`
- `SessionListener::on_status`
- `SessionListener::on_activity`
- `SessionListener::on_user_request`

The crate compiles with initial Claude wiring, including:

- spawn/heartbeat/exited/unhealthy/cancelled status
- `system/init` and `system/status`
- early `ToolStarted` from `content_block_start`
- `ToolArgsDelta` from `input_json_delta`
- `ThinkingDelta` from `thinking_delta`
- `AskUserQuestion` mapped to `UserRequest::Survey`
- final tool/result activity from canonical assistant/user frames

## Goal

Complete and harden the Claude-specific plumbing against the common event API.

Primary file:

- `crates/chan-llm/src/backends/claude_cli.rs`

## Required work

- Review Alice's initial parser additions for Claude `stream-json`.
- Add parent `parent_tool_use_id` propagation where Claude exposes it.
- Improve `ToolArgsDelta` correlation if enough frame context exists to attach the tool id.
- Map permission denials from terminal `result.permission_denials[]` to
  `AgentActivity::ToolDenied` if the JSON shape is stable in your samples.
- Preserve existing text-delta dedupe behavior.
- Keep existing `on_tool_call` / `on_tool_result` behavior unchanged.

Add focused fake-CLI tests for:

- `AgentStatus::Ready` from `system/init`
- `AgentStatus::Thinking` from `system/status`
- heartbeat during a live-but-quiet process
- early `ToolStarted` before canonical `on_tool_call`
- `UserRequest::Survey` for `AskUserQuestion`
- text-only numbered choice remains only `on_delta`
- timeout/nonzero/no-terminal paths emit `AgentStatus::Unhealthy`

## Boundaries

Write scope:

- `crates/chan-llm/src/backends/claude_cli.rs`
- Claude-specific docs if needed
- this task file for status notes

Avoid:

- Codex backend changes
- common API shape changes unless you first write the proposed change here

## Verification

Run:

```bash
cargo test -p chan-llm claude_cli
```

If broader changes are needed, also run:

```bash
cargo check -p chan-llm
```

## Martin status

Implementation layered on top of Alice's initial Claude wiring:

- `StreamEvent::Assistant`, `StreamEvent::User`, and
  `StreamEvent::Partial` now destructure `parent_tool_use_id` from
  the outer envelope and thread it through every
  `AgentActivity::*` emission (MessageStarted, ThinkingStarted,
  ThinkingDelta, ToolStarted, ToolArgsDelta, ToolFinished,
  AgentNote, plus the `UserRequest::Survey` parent_id).
- `ContentBlockStart { ToolUse }` now records
  `index -> tool_use_id` so the follow-up
  `ContentBlockDelta { InputJsonDelta }` carries
  `AgentActivity::ToolArgsDelta { id: Some(...) }`. The map clears
  on every `MessageStart` so block indexes that reset per message
  do not collide.
- `StreamEvent::Result.permission_denials[]` now produces one
  `AgentActivity::ToolDenied { id, name, input }` per entry,
  emitted before the existing Unhealthy/StopReason path so the
  frontend can attribute the denial.
- `PartialContentBlock` switched to `#[derive(Default)]` plus
  `#[default]` on the `Other` variant to clear a
  `clippy::derivable_impls` warning that blocked `clippy
  --all-targets -- -D warnings`. Functional behavior unchanged.

Added 11 fake-CLI tests covering the required scenarios:

- `system_init_emits_ready_and_session_started`
- `system_status_requesting_emits_thinking`
- `heartbeat_fires_while_child_is_quiet_but_alive` (sleeps 1.5s
  under a 2s inactivity_timeout so `heartbeat_interval` is 1s;
  asserts >=1 heartbeat frame fires during the quiet window)
- `tool_started_fires_before_canonical_tool_call` (asserts
  ordering of ToolStarted vs `on_tool_call`, plus ToolArgsDelta
  id correlation)
- `ask_user_question_emits_typed_survey` (asserts full
  multi-question/multi-select schema preserved, plus existing
  `on_tool_call` still fires)
- `numbered_choice_in_plain_text_stays_a_delta`
- `inactivity_timeout_emits_unhealthy_status`
- `nonzero_exit_emits_unhealthy_status`
- `missing_terminal_result_emits_unhealthy_status`
- `parent_tool_use_id_propagates_into_activity`
- `permission_denials_emit_tool_denied_activity`

Verification (local, on Alice's WIP tree):

- `cargo test -p chan-llm --lib claude_cli`: 28 passed, 1 ignored
  (real_claude_streams_tokens). My 11 new tests all pass.
- `cargo test -p chan-llm --lib`: 135 passed, 1 ignored.
- `cargo clippy -p chan-llm --all-targets -- -D warnings`: clean.
- `cargo fmt --check`: clean.

Commit coordination:

- My hunks layer on top of Alice's unstaged session.rs API
  additions and her initial claude_cli.rs wiring. Committing
  through this tree would bundle Alice's hunks with mine.
- Sequence required: Alice commits the common API + initial
  Claude/Codex wiring + README/design.md + lib.rs first; Bob
  commits the Codex hardening test additions next; then I commit
  the Claude hardening + tests + clippy fix on top. Order matters
  because each layer depends on the previous one to compile.
