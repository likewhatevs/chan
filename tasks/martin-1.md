# martin-1: TypeScript types for AgentStatus/AgentActivity/UserRequest

Owner: Martin. Depends on: bob-1. Unblocks: martin-2.

## Goal

Mirror the chan-llm 0.11 lifecycle types into `web/src/api/types.ts`
so the rest of the frontend (reducer, store state, InlineAssist) has
a typed surface to work against. Also define the three new websocket
frame variants.

## Source of truth

The Rust types live in
`~/dev/github.com/chan-writer/chan-core/crates/chan-llm/src/session.rs`.
Use them verbatim. Relevant pieces:

- `AgentStatus` enum (lines 152-198): tag is `"kind"`, snake_case.
  Variants: `spawned`, `ready`, `thinking`, `heartbeat`,
  `turn_stopping`, `rate_limit`, `exited`, `unhealthy`, `cancelled`.
- `AgentActivity` enum (lines 202-261): tag is `"kind"`, snake_case.
  Variants: `session_started`, `message_started`, `thinking_started`,
  `thinking_delta`, `tool_started`, `tool_args_delta`,
  `tool_finished`, `tool_denied`, `agent_note`, `turn_usage`.
- `UserRequest` enum (lines 263-275): tag is `"kind"`, snake_case.
  Currently one variant: `survey`. `UserQuestion` (lines 277-286)
  and `UserOption` (lines 288-293) are supporting structs.

All Rust enums are `#[non_exhaustive]`. Mirror that in TypeScript by
keeping the discriminated-union open: prefer
`type AgentStatus = SpawnedStatus | ReadyStatus | ... | { kind: string }`
or a comment that unknown `kind` values must round-trip without
errors. Plan section "Frontend (2)" requires tolerating unknown
future variants.

## Files to touch

- `web/src/api/types.ts` only. Do NOT modify the reducer or store
  in this task; martin-2 owns that.

## Required types

1. `AgentStatus`: discriminated union on `kind`. Field names match
   Rust serde-snake_case. `pid`, `session_id`, `model`, `version`,
   `idle_ms`, `reason`, `detail`, `resets_at`, `rate_limit_type`,
   `in_overage`, `code`, `success`, `status` are the field names
   you'll need — copy from the Rust definitions.
2. `AgentActivity`: discriminated union on `kind`. Includes
   `parent_id?: string | null` on most variants; `partial_json`,
   `text`, `output` (as `unknown` since it's `Json`), `is_error`,
   `usage` (also `unknown`).
3. `UserRequest`: discriminated union on `kind` with `survey`
   variant. Include `UserQuestion` and `UserOption` interfaces.
4. Three new frame variants on whatever discriminated union
   represents websocket frames (look for the existing
   `llm.delta` / `llm.tool_call` types; mirror their shape):
   - `{ type: "llm.status"; session_id: string; status: AgentStatus }`
   - `{ type: "llm.activity"; session_id: string; activity: AgentActivity }`
   - `{ type: "llm.user_request"; session_id: string; request: UserRequest }`

If the existing file does not have a typed WS frame union (today
the reducer narrows via the `frameType === "..."` string), add the
new frame types as exported interfaces and leave wider unification
to martin-2.

## Acceptance criteria

1. `pnpm exec svelte-check` (or whatever the repo runs — check
   `web/package.json` scripts) passes with zero errors.
2. `pnpm exec vitest run` still passes.
3. New types are exported and ready to be imported by the reducer
   and components.
4. No runtime code touched outside `types.ts`.

## Hints

- The existing pattern for new types lives elsewhere in
  `web/src/api/types.ts`; this file is large but skim for the
  closest analogue (e.g. how the `Edit`/tool_call shapes are typed)
  and follow the same style.
- TypeScript discriminated unions on snake_case `kind`s are routine:
  `type AgentStatus = | { kind: "spawned"; backend: string; pid?: number | null } | ...`
- Keep `unknown` for the `Json` fields (`output`, `usage`). The
  reducer can narrow on the way out.

## Done means

Post an update to `tasks/journal.md` (status DONE for martin-1, plus
a one-line log entry) so Alice can clear martin-2.
