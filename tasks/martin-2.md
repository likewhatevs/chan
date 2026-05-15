# martin-2: Reducer + assistantStream state for new lifecycle frames

Owner: Martin. Depends on: martin-1. Unblocks: martin-3, martin-4.

## Goal

Teach the store reducer in `web/src/state/store.svelte.t` to consume
the three new frames (`llm.status`, `llm.activity`, `llm.user_request`)
and expand the `assistantStream` reactive state so InlineAssist has
something to render.

## Files to touch

- `web/src/state/store.svelte.ts`:
  - The frame router around line 213 (the `if (frameType === "llm.delta"
    || ...)` block).
  - The `assistantStream` `$state` block around line 1729.
  - The `beginAssistantStream` and `endAssistantStream` helpers
    (lines 1758-1785) to initialise and clear the new fields.

## Imports from martin-1

The types you need are already exported from `web/src/api/types.ts`:

- `AgentStatus` (discriminated on `kind`, with `UnknownAgentStatus`
  as the open-ended fall-through for forward-compat).
- `AgentActivity` (same shape).
- `UserRequest`, `UserQuestion`, `UserOption`.
- `LlmStatusFrame`, `LlmActivityFrame`, `LlmUserRequestFrame` for the
  three new WS frame variants.

Server side uses `chan-llm`'s serde `#[serde(tag = "kind")]` so the
JSON shape lines up with Martin's discriminated unions byte-for-byte.

## Required reducer changes

Where the current code matches on `llm.delta` / `llm.tool_call` /
`llm.tool_result` / `llm.done` / `llm.error`, extend the match to
also accept `llm.status`, `llm.activity`, `llm.user_request`. Keep
the existing session-id guard:

```ts
if (!assistantStream.sessionId || f.session_id !== assistantStream.sessionId) {
  return;
}
```

Effects per frame:

- `llm.status` with `kind === "heartbeat"`: update
  `assistantStream.lastHeartbeatAt = Date.now()` and set
  `assistantStream.status = { ... }`.
- `llm.status` with `kind === "thinking" | "ready" | "spawned" |
  "turn_stopping" | "rate_limit" | "cancelled"`: just replace
  `assistantStream.status` with the new value.
- `llm.status` with `kind === "unhealthy" | "exited"`: replace
  `assistantStream.status`. The UI will read these to switch on a
  visible "unhealthy" indicator; this task does not render it
  (that's martin-3).
- `llm.activity`: push onto `assistantStream.activity`, capped at
  the most recent 32 entries (drop the oldest with `.shift()` or a
  ring buffer — keep allocations bounded for long turns). The plan
  calls this "bounded activity history".
- `llm.user_request`: replace `assistantStream.userRequest` with the
  new value (kind === "survey" today; tolerate future kinds without
  throwing).

Future variants: an unknown `frameType` starting with `llm.` MUST
not crash the reducer. Today the catch-all already falls through to
the filesystem-watch branch, which is wrong for unknown llm.* frames.
Add an early `if (frameType?.startsWith("llm.")) return;` guard
AFTER the recognised-frame branch so unknown llm frames silently
no-op. Add a `console.debug` log if you want, but no error toast.

## Required assistantStream additions

Add four optional fields to the `$state` shape:

```ts
status: AgentStatus | null;
lastHeartbeatAt: number | null;
activity: AgentActivity[];
userRequest: UserRequest | null;
```

Update `beginAssistantStream` to reset them (`null`, `null`, `[]`,
`null`) and `endAssistantStream` to clear them the same way.

## Acceptance criteria

1. `pnpm exec svelte-check` passes.
2. `pnpm exec vitest run` passes (martin-4 will add the new tests;
   existing tests must still pass).
3. Reducer ignores frames whose `session_id` does not match the
   active stream (same rule as existing variants).
4. Unknown `llm.*` frame types do not throw and do not reach the
   filesystem-watch branch.
5. `assistantStream` exposes the four new fields with the documented
   lifecycle (initialised by `beginAssistantStream`, cleared by
   `endAssistantStream`).

## Hints

- Source-of-truth for activity-history cap: 32 entries. Cheap, fits
  in a small chip strip, plenty for a long turn's worth of tool
  starts/finishes. Don't make it configurable.
- The session_id branch around line 220 already narrows the frame
  shape via inline type assertion. Mirror that pattern for the new
  variants; do NOT introduce a new transport layer.
- Do not couple to InlineAssist in this task. The reducer is
  upstream of any UI; martin-3 will read these fields.

## Done means

Post an update to `tasks/journal.md` (status DONE for martin-2, plus
a one-line log entry) so Alice can dispatch martin-3 and martin-4 in
parallel.
