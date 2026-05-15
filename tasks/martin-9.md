# martin-9: Persistent activity/tool log in chat

Owner: Martin. Depends on: martin-2 (assistantStream activity state),
martin-3 (existing transient activity rendering).

## Why

User feedback after manual testing of martin-3:

- Tool/activity events appear one after another, replacing each
  other instead of stacking into a visible log.
- Detail is too thin: a tool chip says "running grep" but doesn't
  show the args, output, or status transition.
- Events disappear over time (transient ring buffer); user wants
  them to remain in the chat as a permanent log of what the
  assistant did.

Today there are two parallel sources of tool information:

1. `llm.tool_call` / `llm.tool_result` frames already build
   persistent `tool` turns in the conversation log (see
   `web/src/state/store.svelte.ts` around line 244). These show
   as chips like "reading docs/foo.md" with an ok/error status.
2. `llm.activity` frames feed
   `assistantStream.activity[]`, a bounded ring buffer that
   martin-3 renders as a transient strip near the live bubble.

The user wants (1) to be richer (args + output detail visible)
and wants (2) to either fold into (1) where they overlap or to
become a parallel persistent log. The architecturally clean call
is to merge: the tool_call / tool_result chip turns already exist
and persist; expand them to surface args + output rather than
introducing a second turn type that mostly mirrors them.

## Files to touch

- `web/src/state/store.svelte.ts`: extend the existing `tool` turn
  shape and the reducer code that builds it.
- `web/src/components/InlineAssist.svelte`: the renderer for tool
  turns. The chip renderer is roughly where you already render
  the conversation scrollback; locate the `kind === "tool"`
  branch.
- `web/src/components/AssistantInspectorBody.svelte` if tool
  details also surface there.

## Required changes

### 1. Tool turn data model

The current `tool` turn carries `tool_call_id`, `name`,
`label` (preformatted), `status` ("running" | "ok" | "error"),
and `result_summary`. Extend it to carry:
- `args`: the original tool_call arguments as `unknown` (already
  on the frame; keep raw JSON).
- `partial_args`: progressively-built JSON string accumulated from
  `llm.activity` `tool_args_delta` frames before the full
  `tool_call` lands. Optional. Useful for the visible "typing
  out the call" effect.
- `output`: full tool_result output (not just a summary). Render
  preview in the chip, expand on click. Truncate long outputs at
  some sensible cap (~16 KiB serialised) with "expand" affordance.
- `is_error`: explicit bool from `tool_finished` activity frames
  (today inferred from `isErrorOutput`); keep both paths working
  but prefer explicit when available.
- `started_at`, `finished_at`: timestamps for diagnostic context.
  These don't need a server source; capture at frame-arrival
  time.

### 2. Reducer wiring

- On `llm.activity` `tool_args_delta`: find the matching tool
  turn (by `id`) and append to `partial_args`. If the turn doesn't
  exist yet (the delta arrived before `tool_call`), create a
  pending turn with placeholder name "(starting...)" and fill it
  in when the proper `tool_call` lands.
- On `llm.activity` `tool_started`: ensure a turn exists; idempotent
  with `tool_call` since both fire for the same tool.
- On `llm.activity` `tool_finished`: stamp `finished_at`,
  `is_error`, and fold `output` into the matching turn. Mirrors
  `tool_result` handling; the existing path stays.

`assistantStream.activity[]` should be reduced to non-tool events
only: `thinking_started`, `agent_note`, `turn_usage`,
`message_started`. These remain transient breadcrumbs (the strip
martin-3 added) since they're high-frequency and low-signal for a
permanent log. Don't try to persist them.

### 3. Renderer

Tool turn becomes a compact stacked card:

```
[icon] grep                                                running
       pattern: "Cargo\\.toml"
       …streaming args…
```

After completion:

```
[icon] grep                                                    ok
       pattern: "Cargo\\.toml" path: "."
       ▶ output (3 matches)
       Cargo.toml:1:[workspace]
       Cargo.toml:7:members = [
       Cargo.toml:42:[dependencies]
       [expand]
```

`is_error: true` flips to a red icon and an "error" badge; output
is shown with monospace formatting.

Stacking: all tool turns in a conversation appear in order in the
chat scrollback, each as its own card. They persist for the life
of the conversation. Switching conversations or reloading the page
loses them (we don't have server-side conversation persistence
today; that's a separate roadmap item).

### 4. Don't break existing behaviour

- `write_file` tool calls still bypass the chip in favour of the
  richer edit-card flow handled elsewhere; keep that bypass.
- Session-id filtering on every reducer branch stays.
- The transient activity strip continues to render non-tool
  events.

## Acceptance criteria

1. `npm run check` and `npm test -- --run` are green.
2. Manual: trigger a turn with multiple tool calls; each appears
   as a persistent card with name + args + status + output.
3. Long-running streams show args accumulating in real time when
   the backend emits `tool_args_delta`.
4. Tool errors render distinctly from successes.
5. Existing reducer tests (martin-4) still pass; add new tests
   covering the args/output rendering path.

## Out of scope

- Server-side persistence of conversations. The chat is in-memory
  today; not Martin's problem to fix here.
- Replacing the inspector entirely. martin-8's model picker still
  lands in the inspector; don't conflict with that work.

## Done means

Post an update to `tasks/journal.md` (status DONE for martin-9, plus
a one-line log entry).
