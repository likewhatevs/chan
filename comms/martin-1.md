# martin-1: Claude CLI interactive lifecycle

From: Alice
Owner: Martin
Status: first-pass done; waiting on Alice's common API

## Goal

Analyze and implement the Claude-specific side of the interactive abstraction.

Focus on what the `claude --print --output-format stream-json --include-partial-messages --verbose` stream can expose beyond today's `on_delta`, `on_tool_call`, and `on_tool_result`.

## Current code

Primary file:

- `crates/chan-llm/src/backends/claude_cli.rs`

Relevant existing behavior:

- launches `claude` as a subprocess
- reads NDJSON stdout with an inactivity timeout
- drains stderr
- emits text deltas from SDK partial events
- emits tool calls from final assistant `tool_use` blocks
- emits tool results from user `tool_result` blocks
- kills child on cancel/timeout/error

## Your first pass

Update this file with:

- exact Claude stream event types/subtypes that can represent:
  - child/session started
  - thinking/progress/heartbeat
  - tool start
  - tool output/result
  - background task start/finish, if Claude exposes it
  - user prompt/choice request, if Claude exposes it
- gaps where Claude exposes only text and chan-llm must infer or pass through
- proposed mapping into Alice's common event families in `comms/journal.md`

## Implementation scope

After writing the analysis, implement only Claude-specific parsing/emission once Alice's common API lands.

Write scope:

- `crates/chan-llm/src/backends/claude_cli.rs`
- Claude-specific tests in that module
- Claude-specific docs in `crates/chan-llm/design.md` only if needed

Avoid:

- editing Codex backend files
- changing common public API before Alice posts the final shape
- changing MCP tool semantics

## Test requirements

Add e2e-style fake-CLI tests that cover:

- long-running but alive child emits heartbeat/status before final terminal event
- timeout/unhealthy path when stdout goes silent
- tool start and result activity frames
- text-only `1. 2. 3.` prompts remain deltas when no typed event exists
- subprocess crash produces typed unhealthy/error status and terminates cleanly

## Martin first pass: Claude stream capabilities

Local CLI inspected:

- `claude --version`: `2.1.142 (Claude Code)`.
- `claude --help` confirms the streaming flags already used today
  (`--print`, `--output-format stream-json`, `--input-format text`,
  `--include-partial-messages`, `--verbose`, `--append-system-prompt`,
  `--mcp-config`, `--allowedTools`, `--disallowedTools`,
  `--permission-mode`) plus three flags worth highlighting for this
  work:
  - `--include-hook-events`: emits PreToolUse/PostToolUse/SessionStart/
    etc. hook frames when hooks are configured. Off by default; we
    should keep it off for chan so the user's local hooks don't leak
    into chan-server events.
  - `--input-format stream-json`: realtime user messages on stdin
    (needed for mid-turn UserRequest answer injection in a v2 round).
  - `--brief`: enables the `SendUserMessage` tool (agent-to-user
    push messages). Off by default; useful for "still working"
    notes.
- Built-in tool catalog reported in `system/init` includes the typed
  user-prompt mechanism we want: `AskUserQuestion` (multi-question,
  multi-select capable).

### Observed live `claude --print --output-format stream-json
--input-format text --include-partial-messages --verbose` events

Frame envelope `{"type": ...}` on stdout:

- `system` (subtypes seen: `init`, `status`)
  - `init` carries `session_id`, `cwd`, `model`, `tools[]`, `mcp_servers[]`,
    `permissionMode`, `claude_code_version`, `agents[]`, `skills[]`,
    `plugins[]`, `memory_paths`, `fast_mode_state`, `apiKeySource`,
    `output_style`, `slash_commands[]`, `uuid`.
  - `status` carries `status` (observed value: `"requesting"`),
    `uuid`, `session_id`. Fires once per upstream API request, so
    once per turn-half (one before tool_use, one before final
    answer, in a 2-turn run).
- `stream_event` (Anthropic SDK partial envelope, only with
  `--include-partial-messages`):
  - `event.type = message_start`: full per-message metadata
    (model, message id, initial usage).
  - `event.type = content_block_start`: index + content_block of
    type `text`, `tool_use` (with id, name, partial input, caller),
    or `thinking`.
  - `event.type = content_block_delta`: deltas of type
    `text_delta` (assistant prose), `input_json_delta`
    (`partial_json` for in-flight tool args), `thinking_delta`
    (extended-thinking text), `signature_delta` (cryptographic
    signature for thinking blocks; opaque payload).
  - `event.type = content_block_stop`: index.
  - `event.type = message_delta`: per-message `stop_reason`
    (`end_turn`, `tool_use`, `max_tokens`, `stop_sequence`) and
    final per-message usage. Earlier than the terminal `result`.
  - `event.type = message_stop`.
- `assistant`: final assistant message envelope with `content[]`
  blocks (text, tool_use, thinking) and consolidated usage.
  Today's parser reconciles this against streamed partials.
- `user`: tool_result blocks plus a side-channel `tool_use_result`
  carrying `stdout`, `stderr`, `interrupted`, `isImage`,
  `noOutputExpected` for Bash-shaped tools.
- `rate_limit_event`: `rate_limit_info` with `status`, `resetsAt`,
  `rateLimitType`, `overageStatus`, `overageResetsAt`,
  `isUsingOverage`.
- `result` (terminal): `subtype` (`success` / error variants),
  `is_error`, `duration_ms`, `duration_api_ms`, `ttft_ms`,
  `num_turns`, `result` (final text), `stop_reason`,
  `terminal_reason` (`completed` and friends), `total_cost_usd`,
  `usage`, `modelUsage`, `permission_denials[]`,
  `fast_mode_state`.

Confirmed shapes when interactive tools fire:

- `AskUserQuestion` arrives as `assistant.message.content[*].tool_use`
  with the schema:
  ```json
  {
    "id": "toolu_...",
    "name": "AskUserQuestion",
    "input": {
      "questions": [{
        "question": "Which color do you prefer?",
        "header": "Color",
        "multiSelect": false,
        "options": [
          { "label": "Red",   "description": "..." },
          { "label": "Green", "description": "..." },
          { "label": "Blue",  "description": "..." }
        ]
      }]
    }
  }
  ```
  In `--print` mode there is no host to answer, so claude emits a
  synthetic `tool_result` with `is_error:true` and content
  `"Answer questions?"`, and the terminal `result.permission_denials`
  array records the cancelled invocation with the full tool_input
  echoed back.
- `parent_tool_use_id` on `assistant` / `user` / `stream_event`
  frames is non-null when the event came from a sub-agent spawned
  by the `Task` tool. Useful for nesting sub-agent activity under
  its parent in the UI.

### Current code-supported event families

`crates/chan-llm/src/backends/claude_cli.rs` parses (as
`StreamEvent`):

- `assistant.message.content[*]`:
  - `Text` -> `on_delta` (with the per-block partial-tracker
    dedupe against streamed text_delta).
  - `ToolUse` -> `on_tool_call`.
  - `ToolResult` and `Other` -> dropped (ToolResult shouldn't
    appear here in claude's protocol; it's a user-frame block).
- `user.message.content[*].tool_result` -> `on_tool_result`.
- `stream_event.event`:
  - `message_start` -> push a fresh partial-tracker for the
    upcoming assistant message.
  - `content_block_delta.text_delta` -> `on_delta` + tracker
    update.
  - everything else (signature_delta, input_json_delta,
    thinking_delta, content_block_start, content_block_stop,
    message_delta, message_stop) -> dropped.
- `result` -> sets terminal `stop_reason` (defaults `EndOfTurn`;
  `Error` when `is_error`), falls back to `result.result` text
  when no incremental deltas streamed.
- `system`, `rate_limit_event`, future types -> dropped via the
  `#[serde(other)] Other` catch-all.

### Gaps relative to the requested UX

- `system/init` is dropped today. It carries the entire spawn-time
  context (session_id, model, permissionMode, tools, mcp_servers,
  plugins, claude_code_version). The web layer has to guess at any
  of this. Map to `AgentStatus::Spawned { session_id, model,
  version, tools, mcp_servers, permission_mode, ... }`.
- `system/status/requesting` is dropped today. Strongest typed
  signal that claude is currently waiting on the model API. Map to
  `AgentStatus::Thinking` (or `Heartbeat` if Alice prefers a single
  liveness variant). One frame per API request is the natural
  cadence, so a separate fixed-interval ticker is still needed for
  gap coverage while a tool runs locally.
- No heartbeat today between events. We have an inactivity timeout
  (default 300s) that fires only after silence; nothing tells the
  upper layer "child still alive" before that. A 5s tokio ticker
  driven by `last_line_at` is the cheapest fix and matches Bob's
  Codex recommendation.
- `content_block_start` for tool_use is dropped today. The earliest
  "claude is about to call tool X" signal arrives here, with the
  tool name and id, BEFORE the final assistant envelope. Today the
  UI only learns about a tool call when the assistant envelope
  closes the whole message, which can be hundreds of tokens later.
  Map to `AgentActivity::ToolStarted { id, name }` and let the
  existing `on_tool_call` (from the assistant envelope) keep
  firing for compat.
- `content_block_delta.input_json_delta` is dropped. It carries the
  tool args being typed token by token. Optional for v1 but cheap
  to surface as `AgentActivity::ToolArgsDelta { id, partial_json }`
  for a typewriter view (matches how the TUI shows
  `Bash(command: ls -la /tmp/...)` while it streams in).
- `content_block_start/delta` for `thinking` is dropped. Extended
  thinking text + the cryptographic signature pass through
  invisibly. Frontend currently can't show a "thinking" pane. Map
  to `AgentActivity::ThinkingStarted` /
  `AgentActivity::ThinkingDelta { text }`. `signature_delta` should
  stay internal (model-provenance only, not user-facing).
- `message_delta.stop_reason` is dropped. The per-message
  stop_reason is more granular than the terminal `result`'s and
  arrives earlier; useful for "the turn is finishing because of
  tool_use, expect a tool result loop" UX. Map to
  `AgentStatus::TurnStopping { reason }` or equivalent.
- `rate_limit_event` is dropped. Map to either a typed
  `AgentStatus::RateLimited { resets_at, kind, in_overage }` or
  fold into `on_error_kind(LlmEventError::RateLimited {...})` when
  `status != "allowed"`; for the `allowed` case it is informational
  and could feed the same status family.
- `result.permission_denials` is dropped. Map to
  `AgentActivity::ToolDenied { name, reason, tool_use_id,
  tool_input }`. Distinguishes a cancelled `AskUserQuestion` from
  a refused `Write` for the frontend.
- `result.total_cost_usd`, `result.modelUsage`, `result.usage`,
  `result.duration_ms`, `result.ttft_ms` are dropped. None are
  load-bearing for lifecycle UX, but the cost / token numbers
  would feed the activity-pane footer cheaply via a typed
  `AgentActivity::TurnUsage { ... }` if Alice wants it; otherwise
  leave for a later round.
- `AskUserQuestion` tool_use is forwarded only as a generic
  `on_tool_call` today. The frontend has no machine-readable hook
  to render a 1/2/3 picker. Detecting the tool name and forwarding
  the typed `UserRequest::Survey { id, questions }` is the highest-
  value addition for the requested UX. Continue to fire
  `on_tool_call` for backward compatibility.
- `SendUserMessage` (only with `--brief`) is invisible today.
  Map to `AgentActivity::AgentNote { text }` so the frontend can
  show "still working, here's an interim update" notes distinct
  from final assistant text.
- `parent_tool_use_id` is dropped. Sub-agent (Task tool) output
  cannot be visually nested under its parent. Map by carrying
  `parent_tool_use_id` on every activity/status frame; the
  frontend can group by id.

### Where Claude exposes only text (chan-llm must pass through)

- Numbered-choice prompts not authored through `AskUserQuestion`
  ("Pick one: 1. apples / 2. oranges / 3. pears" written in
  assistant text). These stay as ordinary `on_delta` text. The
  frontend can opt into a lightweight "reply with 1/2/3"
  affordance based on heuristics, but chan-llm should not try to
  parse them.
- Background-task lifecycle outside of `Task` sub-agent spawns:
  Claude does not expose long-running background jobs as separate
  typed events in `--print` mode. The Task tool covers the only
  observable nested-agent case today via `parent_tool_use_id`.

### Proposed Claude mapping into Alice's common families

Assumes Alice's families finalize close to the journal sketch
(`AgentStatus`, `AgentActivity`, `UserRequest`). Concrete mapping:

- `command.spawn()` ok -> `AgentStatus::Spawned { backend:
  "claude_cli", pid, ... }`. Enrich from `system/init` once the
  first frame arrives.
- `system/init` -> `AgentStatus::Ready { session_id, model,
  version, permission_mode, tools[], mcp_servers[], plugins[] }`
  (or fold into Spawned with optional fields).
- `system/status/requesting` -> `AgentStatus::Thinking { since:
  monotonic_ts }`. Cleared on first delta/tool event.
- Fixed-interval tokio ticker while child alive ->
  `AgentStatus::Heartbeat { idle_for_ms, last_stdout_at }`. 5s
  cadence; aligns with Bob's Codex side.
- `stream_event.message_start` -> `AgentActivity::MessageStarted
  { message_id, parent_tool_use_id }`. Lightweight, helps the
  frontend group blocks.
- `stream_event.content_block_start { tool_use }` ->
  `AgentActivity::ToolStarted { id, name, parent_tool_use_id }`.
- `stream_event.content_block_delta { input_json_delta }` ->
  `AgentActivity::ToolArgsDelta { id, partial_json }` (optional;
  v1 may skip).
- `stream_event.content_block_start { thinking }` ->
  `AgentActivity::ThinkingStarted { parent_tool_use_id }`.
- `stream_event.content_block_delta { thinking_delta }` ->
  `AgentActivity::ThinkingDelta { text }`.
- `stream_event.content_block_delta { text_delta }` -> existing
  `on_delta` (text). No change.
- `stream_event.message_delta { stop_reason }` ->
  `AgentStatus::TurnStopping { reason }`. Frontend can flip the
  "thinking" state off as soon as this arrives.
- `assistant.message.content[*].tool_use { AskUserQuestion }` ->
  also emit `on_user_request(UserRequest::Survey { id,
  questions: [Choice { question, header, multi_select, options }
  ...] })`. Keep firing `on_tool_call` for compat.
- `assistant.message.content[*].tool_use { SendUserMessage }` ->
  also emit `AgentActivity::AgentNote { text }` (the tool's input
  carries a `message` field).
- `assistant.message.content[*].tool_use { * }` ->
  `AgentActivity::ToolFinalized { id, name, args,
  parent_tool_use_id }` plus existing `on_tool_call`. The
  finalize frame closes any ToolStarted that's still open with
  the canonical args.
- `user.message.content[*].tool_result` -> existing
  `on_tool_result` PLUS `AgentActivity::ToolFinished { id, output,
  is_error, stdout, stderr, interrupted, no_output_expected }`.
  Pulls the side-channel `tool_use_result` fields when present.
- `rate_limit_event` -> `AgentStatus::RateLimit { status,
  resets_at_unix, kind, in_overage }`. Frontend can pre-warn the
  user before the next API request would fail.
- `result` (terminal):
  - `is_error: true` -> `on_error_kind` typed appropriately,
    `AgentStatus::Unhealthy { reason: subtype }`, terminal Error
    stop.
  - `is_error: false` -> `AgentStatus::Exited { code: 0,
    duration_ms, ttft_ms }` after `child.wait()`. Existing
    `stop_reason` derivation stays.
  - `permission_denials[*]` -> one
    `AgentActivity::ToolDenied { tool_use_id, name, reason,
    tool_input }` each.
- inactivity timeout -> `AgentStatus::Unhealthy { reason:
  "no_output_for_Ns" }`, then existing error path.
- non-zero `child.wait()` -> `AgentStatus::Unhealthy { reason:
  "exit:{status}", stderr_tail }`, then existing error path.
- cancel flag flipped -> `AgentStatus::Cancelled { backend:
  "claude_cli" }`, then existing cancelled outcome.

### Implementation plan once Alice's common API lands

1. Extend `StreamEvent` and `PartialEvent` parsers to surface
   `system { subtype, ... }`, `rate_limit_event`,
   `content_block_start`, `content_block_delta { input_json_delta
   | thinking_delta | signature_delta }`, `message_delta`,
   `message_stop`. Catch-all `Other` stays for forward-compat.
2. Add per-message tracking that keys on the Anthropic message id
   from `message_start` instead of (or alongside) the FIFO of
   trackers. Today's FIFO is correct but the message id makes
   ToolStarted/ToolFinalized pairing trivially correct even when
   messages overlap or claude reorders frames.
3. Emit `AgentStatus::Spawned` immediately after `Command::spawn`
   succeeds, before reading any stdout, so the host sees a frame
   even if claude dies before printing init.
4. Drive a heartbeat ticker via `tokio::select!` alongside the
   stdout read loop; tick interval read from `LlmConfig` (default
   5s, capped at half of `stream_inactivity_timeout_secs`).
   `last_line_at` is updated on every successful line read.
5. Detect `AskUserQuestion` tool_use in the final assistant
   envelope and forward it as `on_user_request(UserRequest::
   Survey { ... })` in addition to `on_tool_call`. Helper to
   translate `input.questions[]` into the common
   `UserRequest::Choice` / `Survey` shape lives in `claude_cli.rs`
   so the common API stays backend-agnostic.
6. Carry `parent_tool_use_id` on every activity frame so the
   frontend can nest sub-agent (Task) output.
7. Add typed Unhealthy/Cancelled/Exited status frames at each
   existing fatal/terminal return site. The current `on_error`
   strings stay for back-compat; `on_error_kind` already exists
   and is what gets enriched.
8. Keep the existing tool-call dedupe (against streamed partials)
   and the per-turn assistant text cap; no behavioural change
   there. ToolStarted is additive; ToolFinalized fires once per
   tool_use block in the canonical message and the listener can
   coalesce by id.

### Test requirements coverage

Fake-CLI tests (mirror Bob's Codex coverage for parity):

- long-running alive run: emit `system/status/requesting` plus
  several stream_event partials over wall-clock seconds, assert
  one or more `AgentStatus::Heartbeat` frames arrive before the
  terminal `result`.
- silent timeout: child writes one line, then sleeps past the
  configured inactivity timeout; assert
  `AgentStatus::Unhealthy { reason: "no_output_for_Ns" }` then
  `on_done(Error)`.
- tool start/result activity: assert ToolStarted fires from
  `content_block_start { tool_use }` BEFORE the final assistant
  envelope's ToolFinalized.
- AskUserQuestion: assert one `on_user_request(Survey {...})`
  with the multi-question / multiSelect shape preserved, AND the
  existing `on_tool_call`.
- text-only numbered choice: prompt produces a plain `text_delta`
  with "1. apples / 2. oranges / 3. pears"; assert deltas pass
  through unchanged and no `on_user_request` fires.
- subprocess crash: fake child exits with non-zero code after one
  line; assert `AgentStatus::Unhealthy { reason: "exit:..." }`
  with stderr tail, then `on_done(Error)`.
- clean EOF without `result` frame: assert existing
  "stream ended without a result event" error path stays and
  emits `AgentStatus::Unhealthy { reason: "no_terminal" }` first.

### Open questions for Alice

- Should `UserRequest` accommodate multi-question surveys natively
  (`UserRequest::Survey { questions: Vec<Choice> }`) or should
  Claude's multi-question `AskUserQuestion` be split into
  multiple sequential `UserRequest::Choice` events? Claude's
  schema treats them as one atomic survey; splitting loses the
  "answer all in one screen" UX intent.
- Should `AgentStatus::Thinking` and `AgentStatus::Heartbeat` be
  separate variants or one with a `kind` field? `Thinking` is a
  CLI-typed signal (request in flight); `Heartbeat` is our
  liveness ticker. Separate variants make the frontend's
  state-machine clearer.
- Should `AgentActivity::ToolStarted` carry the partial
  `partial_json` args (final shape unknown yet) or wait until
  ToolFinalized? Bob's Codex side uses the final command string
  for `command_execution`. Recommend: ToolStarted carries only
  `{ id, name }`; ToolArgsDelta is optional and Claude-only;
  ToolFinalized always carries the canonical args.
- Rate-limit handling: typed `AgentStatus::RateLimit` versus
  reusing `on_error_kind(LlmEventError::RateLimited)`. The
  `allowed`/informational case doesn't fit `LlmError`, so a
  separate status variant feels cleaner. Either is fine; pick
  one and document.
