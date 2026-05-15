# bob-3: Rust tests for new listener frames

Owner: Bob. Depends on: bob-2.

## Goal

Lock in the wire format of the three new lifecycle frames and the
session-id filtering contract with focused unit tests. Cover both
the broadcast bridge (`LlmBroadcastListener`) and the collect path
(`CollectListener`), and assert that existing frames remain
unchanged in shape.

## Where to put the tests

- `crates/chan-server/src/bus.rs`: add a `#[cfg(test)] mod tests`
  block at the bottom covering `LlmBroadcastListener`. Use a
  `tokio::sync::broadcast::channel` with a small capacity and
  collect emitted strings into JSON `Value`s.
- `crates/chan-server/src/routes/llm.rs`: add a `#[cfg(test)] mod
  collect_listener_tests` block testing forwarding behaviour by
  driving a `CollectListener` directly. Use the same broadcast
  channel pattern. Keep these self-contained; do not spin up an
  axum router.

Both files currently have no test module, so you are starting from
scratch in each. Match the project test style used in
`crates/chan-server/src/preferences.rs` (snapshot tests, no helper
crates).

## What to cover

### bus.rs

1. `on_status` emits a JSON frame with:
   - `type == "llm.status"`
   - `session_id == <expected id>`
   - `status` matches the input `AgentStatus` round-tripped through
     serde (tag/kind/snake_case).
2. Same for `on_activity` → `type == "llm.activity"`, payload key
   `activity`.
3. Same for `on_user_request` → `type == "llm.user_request"`,
   payload key `request`.
4. Existing frames sanity check (one assertion each is enough):
   `on_delta` still emits `type == "llm.delta"` with `text` field;
   `on_tool_call` still emits `type == "llm.tool_call"` with `call`;
   `on_tool_result` still emits `type == "llm.tool_result"` with
   `result`; `on_done` emits `type == "llm.done"` with `reason`;
   `on_error` emits `type == "llm.error"` with `error`.
5. Session filtering: a listener built with `session_id == "A"`
   stamps `session_id: "A"` on every frame. (No need to test
   filtering on the consumer side; that lives in the frontend
   reducer.)

### routes/llm.rs

1. `CollectListener::on_status` forwards to the inner
   `LlmBroadcastListener` (assert a frame lands on the broadcast
   receiver).
2. Same for `on_activity`, `on_user_request`.
3. Existing forwarding tests: `on_delta` still updates the in-memory
   transcript text AND forwards; `on_tool_call` pushes into
   `tool_calls` AND forwards; `on_done` flips `finished = true`,
   stamps `stop_reason`, and notifies. These tests are new but
   guard against accidental regressions in this task's edits.

## Acceptance criteria

1. New tests pass: `cargo test -p chan-server` is green.
2. `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`
   pass.
3. Tests assert serialised JSON shape, not just "didn't panic". Use
   `serde_json::from_str::<serde_json::Value>` plus field
   assertions; avoid string-equality on whole frames since field
   order is not guaranteed.

## Hints

- Construct example payloads with literal `AgentStatus::Heartbeat {
  backend: "claude_cli".into(), idle_ms: 1500 }`. Source of truth
  for variant shapes is
  `~/dev/github.com/chan-writer/chan-core/crates/chan-llm/src/session.rs`
  lines 155-275.
- `tokio::sync::broadcast::Receiver::try_recv()` is enough for these
  tests; you do not need an async runtime if you avoid `.await`.
  Where you do need async (CollectListener uses `tokio::sync::Notify`),
  wrap the test in `#[tokio::test(flavor = "current_thread")]`.

## Done means

Post an update to `tasks/journal.md` (status DONE for bob-3, plus
a one-line log entry).
