# martin-4: Frontend vitest tests for the reducer changes

Owner: Martin. Depends on: martin-2 (can run in parallel with martin-3).

## Goal

Cover the new reducer branches with vitest. Plan section "Tests (2)"
calls for three guarantees, all of which must be asserted here:

1. Reducer accepts `llm.status` / `llm.activity` / `llm.user_request`
   frames and mutates `assistantStream` as documented in martin-2.
2. Frames for other sessions are ignored.
3. Unknown `llm.*` frame variants do not crash the store.

## Where to put the tests

Add `web/src/state/store.test.ts` (a new file — there isn't one
today). Follow the style of
`web/src/state/scope_history.test.ts`: plain vitest, no Svelte
component mounting, no DOM.

The reducer hook lives in `store.svelte.ts` as a function (the
`onWatchEvent` / WS frame handler — locate it by searching for
`frameType === "llm.delta"`). Export the relevant entry point from
the store module if it is not already exported; if exporting the
entire frame handler is too invasive, export a small `applyWsFrame`
helper that wraps the same switch and call that from the test. Do
not refactor unrelated branches — only carve out what the test
needs.

## Required cases

1. **status updates state**: call `beginAssistantStream("s1", "drive")`,
   inject `{type: "llm.status", session_id: "s1", status: {kind:
   "heartbeat", backend: "claude_cli", idle_ms: 2000}}`, assert
   `assistantStream.status?.kind === "heartbeat"` and
   `assistantStream.lastHeartbeatAt` is a number near `Date.now()`.
2. **activity buffers**: inject a `tool_started` activity followed
   by a `tool_finished`; assert `assistantStream.activity` has both
   in order.
3. **activity cap**: inject 40 activities; assert `activity.length
   === 32` and the most recent entries are retained.
4. **user_request stored**: inject a `survey` user_request; assert
   `assistantStream.userRequest?.kind === "survey"` and
   `userRequest.questions` round-trips intact.
5. **session filtering**: with active stream `s1`, inject frames
   carrying `session_id: "s2"`; assert nothing in `assistantStream`
   changed.
6. **unknown variant**: inject `{type: "llm.future_thing",
   session_id: "s1", payload: {whatever: true}}`; assert no throw
   and no state mutation.
7. **end clears**: call `endAssistantStream("s1")`; assert
   `status`, `lastHeartbeatAt`, `activity`, `userRequest` are reset
   to null/null/[]/null.

Existing frame types (delta, tool_call, tool_result, done, error)
need ONE smoke test confirming nothing regressed — pick whichever
case is easiest to write.

## Acceptance criteria

1. `pnpm exec vitest run` is green.
2. `pnpm exec svelte-check` is green.
3. Tests do not require a running chan-server, do not hit network,
   do not mount components.
4. Tests use the same patterns as `scope_history.test.ts` (plain
   imports from the store, plain `expect(...)` assertions).

## Hints

- If the store module reads from globals (window, localStorage,
  etc.) on import, you may need a `vi.stubGlobal` setup at the top
  of the test file. Look at `scope_history.test.ts` for prior art.
- Mocking `Date.now()` is unnecessary; `expect(ts).toBeGreaterThan(
  Date.now() - 100)` is good enough for the heartbeat case.

## Done means

Post an update to `tasks/journal.md` (status DONE for martin-4, plus
a one-line log entry).
