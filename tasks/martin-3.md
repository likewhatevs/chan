# martin-3: InlineAssist rendering for status / activity / user-request

Owner: Martin. Depends on: martin-2.

## Goal

Surface the new lifecycle data in `InlineAssist.svelte` so the user
sees agent status, tool/activity progress, and any `UserRequest`
prompt without disturbing the existing delta/tool/done UX. First
pass is display-only: a `UserRequest::Survey` is rendered as a
clear in-app prompt but answering it is NOT in scope for this task
(see plan section "User-Request Limitation").

## Files to touch

- `web/src/components/InlineAssist.svelte` only.

## What to render

Read `assistantStream.status`, `.lastHeartbeatAt`, `.activity`,
`.userRequest` (these were added in martin-2). All four are
`$state`-tracked, so reactive blocks just work.

### 1. Status indicator

A compact pill or text node, near where the current "thinking…"
placeholder shows, that summarises the most recent `AgentStatus`:

- `thinking` (or no status yet, mid-turn): show "thinking…" same
  as today.
- `heartbeat`: "thinking… (idle Xs)" where X is `idle_ms / 1000`
  rounded. Use `lastHeartbeatAt` to also detect stalls: if more
  than 15s has passed since the last heartbeat AND no terminal
  status has landed, show "thinking… (slow)".
- `ready`: nothing visible (this is the normal flow state).
- `turn_stopping`: "wrapping up…".
- `rate_limit`: "rate limited (resets at <resets_at>)".
- `unhealthy`: red/warning badge with the `reason` field as label.
- `exited` with `success === false`: red "agent exited" with code
  if present.
- `cancelled`: "cancelled".

Use the project's existing pill styling (look at
`web/src/components/BottomPill.svelte` and
`web/src/components/AccessoryPill.svelte` for examples). Do NOT
introduce a new design system.

### 2. Activity log / progress chips

Render `assistantStream.activity` as a thin strip of chips inline
between the user message and the assistant bubble. Each chip shows:

- `tool_started`: "running <name>"
- `tool_finished`: "<name> done" (or "<name> failed" if
  `is_error === true`)
- `tool_denied`: "<name> denied"
- `agent_note`: render as a muted-color note line, not a chip
- everything else: hidden by default

Optional: a collapse toggle that shows the full activity stream in
a scrollable list. Keep it minimal; this is a single-user local
app, not a production observability UI.

Important: do NOT duplicate the existing `llm.tool_call` /
`llm.tool_result` chips. The reducer already builds richer "tool"
turns in `currentAssistantConversation()` (see
`web/src/state/store.svelte.ts` around line 244). The
`assistantStream.activity` chips are a parallel, lower-fidelity
view sourced from the CLI lifecycle stream; render them only when
they add information (e.g. `thinking_delta` text, `tool_args_delta`
partial JSON typing-out, or activity from a backend that does NOT
emit the structured tool_call frame).

When in doubt, prefer "show less". The plan section "Frontend (4)"
explicitly calls for either "inline progress chips OR a collapsed
log"; pick one.

### 3. UserRequest::Survey display

When `assistantStream.userRequest` is non-null and `kind ===
"survey"`, render the questions clearly:

- Each `UserQuestion`: bold question text, optional `header` as a
  pill, then the options as plain bullet points (label + optional
  description).
- If `multi_select` is true, show checkbox-style bullets; else
  radio-style. These are read-only — no `onClick` handlers, no
  submit button.
- Add a small italic note: "answering surveys mid-turn is not yet
  supported; the agent is waiting for input it cannot receive
  through this UI yet." This wording matches the plan's
  User-Request Limitation section.
- Future variants (`kind` not `survey`): show a generic "agent
  requested input" placeholder and the raw `kind` so we have a
  trace.

### 4. Behaviour invariants

- Existing `llm.delta` / tool chips / `llm.done` UX MUST stay
  intact. Test the golden path: a turn that streams deltas, calls a
  tool, returns a tool_result, and completes should look the same
  as before martin-2/martin-3 except for the new status pill.
- Two concurrent sessions (same window, different files) must NOT
  cross streams. This is already enforced by the reducer's
  session-id guard; verify by inspection that no rendering path
  reads from a global outside `assistantStream`.

## Acceptance criteria

1. `pnpm exec svelte-check` passes.
2. `pnpm exec vitest run` passes.
3. Manual smoke (open the dev server, fire a request against a
   chan-llm CLI backend if you have one configured; otherwise
   forge frames by injecting into the WS handler):
   - long-running stream updates heartbeat/status indicator
   - tool calls still render the same conversation chips
   - a forged `user_request` survey appears and does not corrupt
     the assistant bubble text
   - cancelling clears all four new fields (handled by
     `endAssistantStream`)
4. No new dependencies. No new design tokens. Use existing
   variables from `web/src/design.md` and existing pill components.

## Hints

- InlineAssist is huge (`InlineAssist.svelte` is ~140KB). Treat it
  as several rendering blocks glued together; add yours adjacent to
  whichever block currently renders the streaming assistant bubble.
- Heartbeat staleness check needs a reactive tick. The cheapest
  approach is `setInterval(() => stale = Date.now() - last > 15000,
  1000)` inside the component's `$effect`, cleared on destroy.
  Don't add a global clock.
- Do NOT call any new server endpoint. This is a display task.

## Done means

Post an update to `tasks/journal.md` (status DONE for martin-3, plus
a one-line log entry).
