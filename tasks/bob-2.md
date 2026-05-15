# bob-2: Wire AgentStatus/AgentActivity/UserRequest into the WS bus

Owner: Bob. Depends on: bob-1. Unblocks: bob-3.

## Goal

Extend the websocket frame surface so `/ws` subscribers receive the
three new chan-llm 0.11 lifecycle frames, plumbed through
`LlmBroadcastListener` and forwarded by `CollectListener` so the
synchronous `/api/llm/complete` path still gets the full sidechannel
fan-out.

## Files to touch

- `crates/chan-server/src/bus.rs` (LlmBroadcastListener impl).
- `crates/chan-server/src/routes/llm.rs` (CollectListener impl, near
  line 358 after bob-1's migration shrank the file by ~390 lines).

## Required changes

### bus.rs

In the `use chan_llm::{...}` line near the top, add `AgentStatus`,
`AgentActivity`, `UserRequest` next to the existing imports.

Implement three new `SessionListener` methods on
`LlmBroadcastListener`:

```rust
fn on_status(&self, status: AgentStatus) {
    self.send("llm.status", serde_json::json!({"status": status}));
}
fn on_activity(&self, activity: AgentActivity) {
    self.send("llm.activity", serde_json::json!({"activity": activity}));
}
fn on_user_request(&self, request: UserRequest) {
    self.send("llm.user_request", serde_json::json!({"request": request}));
}
```

Frame envelope is the same as existing variants: `type`,
`session_id`, plus the payload field. The `send` helper already
merges the inner object, and `AgentStatus` / `AgentActivity` /
`UserRequest` derive `Serialize` upstream so `serde_json::json!`
serialises them directly using their `#[serde(tag = "kind")]`
discriminator.

Do NOT change existing `on_delta`, `on_tool_call`, `on_tool_result`,
`on_done`, `on_error` bodies. Plan says they stay byte-identical.

### routes/llm.rs CollectListener

Add three forwarding methods that mirror the existing
`on_tool_result` pattern (no in-memory collection, just pass through
to `self.forward`):

```rust
fn on_status(&self, status: chan_llm::AgentStatus) {
    self.forward.on_status(status);
}
fn on_activity(&self, activity: chan_llm::AgentActivity) {
    self.forward.on_activity(activity);
}
fn on_user_request(&self, request: chan_llm::UserRequest) {
    self.forward.on_user_request(request);
}
```

Do NOT add fields to `CollectState`. Plan section "Backend (4)"
keeps the `/api/llm/complete` JSON response unchanged for this pass;
lifecycle data flows only over the websocket.

## Acceptance criteria

1. `cargo build` passes.
2. `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`
   pass.
3. Existing tests still pass (no new tests in this task; bob-3 owns
   tests).
4. The three new methods exist on both `LlmBroadcastListener` and
   `CollectListener` with the signatures above.
5. No changes to `/api/llm/complete` response JSON, no changes to
   existing frame types.

## Hints

- `AgentStatus` / `AgentActivity` / `UserRequest` are re-exported
  from `chan_llm` (see chan-core
  `crates/chan-llm/src/lib.rs` lines 69-71). You can `use chan_llm::
  {AgentActivity, AgentStatus, UserRequest, ...}` next to the
  existing imports.
- The CLAUDE.md "Comments" rule is in force: explain WHY, not WHAT.
  These methods are mechanical pass-throughs; do not narrate them.
  One short comment near the new bus.rs block noting that these
  frames are observational and never affect the JSON response is
  enough.

## Done means

Post an update to `tasks/journal.md` (status DONE for bob-2, plus a
one-line log entry) so Alice can dispatch bob-3.
