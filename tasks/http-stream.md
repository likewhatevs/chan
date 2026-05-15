# ../chan HTTP Stream Follow-Up Plan

Status: planning note only. Keep this file untracked; do not include it in the
chan-core release commit or tag.

## Goal

Wire the new chan-core 0.11 CLI lifecycle stream events into ../chan so the
server and web client can show agent status, tool/activity progress, and
user-request prompts in the existing inline-assist experience.

## Prerequisite

../chan currently pins chan-core path dependencies at `=0.10.0`. After the
chan-core release, bump those to `=0.11.0` and update Cargo.lock.

Expect a migration pass, not only a version edit. The current ../chan server
code still references older llm backend/status concepts in a few places, so the
consumer branch may need to adapt those to the CLI-oriented chan-llm API before
the new stream frames compile cleanly.

## Backend

1. Update `../chan/Cargo.toml` chan-core dependencies to `=0.11.0`.
2. Extend `crates/chan-server/src/bus.rs`:
   - import `AgentStatus`, `AgentActivity`, and `UserRequest`
   - emit `llm.status` with `{ session_id, status }`
   - emit `llm.activity` with `{ session_id, activity }`
   - emit `llm.user_request` with `{ session_id, request }`
   - keep existing `llm.delta`, `llm.tool_call`, `llm.tool_result`,
     `llm.done`, and `llm.error` frames unchanged
3. Extend `CollectListener` in `crates/chan-server/src/routes/llm.rs` to
   forward the new callbacks to `LlmBroadcastListener`.
4. Keep `/api/llm/complete` response semantics unchanged for the first pass.
   The new lifecycle data should flow through websocket events, not through the
   final JSON response.
5. Add server tests around listener serialization and session filtering.

## Frontend

1. Add TypeScript types in `web/src/api/types.ts` for:
   - `AgentStatus`
   - `AgentActivity`
   - `UserRequest`
   - the new websocket frame variants
2. Extend the store reducer in `web/src/state/store.svelte.ts`:
   - handle `llm.status`
   - handle `llm.activity`
   - handle `llm.user_request`
   - keep ignoring frames for other `session_id` values
   - tolerate unknown future event variants
3. Expand assistant stream state with status/activity/user-request fields:
   - current status and health
   - last heartbeat timestamp
   - bounded activity history
   - active user request, if any
4. Update `web/src/components/InlineAssist.svelte`:
   - render heartbeat/running/unhealthy state as compact assist status
   - render tool/activity events as inline progress chips or a collapsed log
   - render `UserRequest::Survey` as an explicit in-app prompt
   - keep old delta/tool/done behavior intact

## User-Request Limitation

The chan-core 0.11 abstraction can emit `UserRequest`, but answering a prompt
mid-turn is not solved end-to-end yet. For the first ../chan pass, display the
request clearly and preserve session state. A later pass should add a
bidirectional answer path, likely either:

1. a server route such as `/api/llm/input` tied to the running session, or
2. a deeper chan-llm runner change that keeps stdin/control open for supported
   CLIs instead of relying only on print-style completion.

Do not present survey answering as complete until that control path exists.

## Tests

1. Rust tests:
   - `LlmBroadcastListener` serializes all new frame types
   - `CollectListener` forwards all new callbacks
   - existing delta/tool/done frames remain unchanged
2. Frontend tests:
   - reducer accepts status/activity/user_request frames
   - frames for other sessions are ignored
   - unknown variants do not crash the store
3. Manual smoke checks:
   - long-running stream updates heartbeat/status
   - tool calls still render
   - user-request prompt appears without corrupting assistant text
   - two sessions do not cross streams

## Suggested Order

1. Land chan-core 0.11 release.
2. Bump ../chan chan-core deps to 0.11.0 and fix compile breaks.
3. Add server websocket pass-through frames.
4. Add frontend types and reducer handling.
5. Add UI rendering for status/activity/user-request display.
6. Design and implement the bidirectional answer path as a separate follow-up.
