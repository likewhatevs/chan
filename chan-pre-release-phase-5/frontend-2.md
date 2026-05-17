# @@Frontend task 2: residual Agent/LLM cleanup in store + api layer

Owner: @@Frontend
Status: REVIEW
Depends on: [frontend-1](./frontend-1.md) (overlay UI removed)
Coordinates with: [backend-1](./backend-1.md) (no longer serves `/api/llm/*`
or `/api/assistant/*`), [systacean-1](./systacean-1.md) (chan-llm strip).

## Goal

Remove the still-live Agent / LLM client bindings and types from the web
package now that the overlay UI is gone and the backend no longer serves
those routes.

## State at task creation

@@Backend already removed the `/api/llm/*` client methods (`llmStatus`,
`llmCliDetection`, `llmComplete`, `llmTools`) from `web/src/api/client.ts`
as part of [backend-1](./backend-1.md). The residue listed below is what
remains after that pass.

## Acceptance criteria

* `web/src/api/client.ts` no longer exports any LLM/assistant entry
  point that has no remaining caller. Specifically check and remove
  (after grep confirms zero callers):
  * `assistantHash16` (and its internal SHA-256 helper if it has no
    other consumer)
  * `getAssistantBlob`, `putAssistantBlob`, `deleteAssistantBlob`,
    `listAssistantBlobs`, `putAssistantBlobKeepalive`
  * Any `getAnswer` / `putAnswer` style helpers that pointed at
    `/api/answers` if those still exist.
* `web/src/api/types.ts` deletes the remaining LLM/assistant types
  (verified still present): `AssistantBackendKind`, `LlmRole`,
  `LlmMessage`, `LlmImageInput`, `LlmToolSpec`, `LlmToolCall`,
  `LlmCompletionRequest`, `LlmCompletionResponse`, `LlmStopReason`,
  `LlmStatusFrame`, `LlmActivityFrame`, `LlmUserRequestFrame`.
  (`LlmStatus`, `LlmModelEntry`, and `AssistantPrefs` were already
  removed in [backend-1](./backend-1.md).)
* `web/src/state/store.svelte.ts` and any peer state module no longer
  reference `assistantStream`, `assistantOverlay`, `scopeHistoryOverlay`,
  `AssistantConversation`, `AssistantTurn`, `AssistantToolEvent`,
  `AgentActivity`, `AgentStatus`, or related `Llm*` imports. The
  conversation-state machinery is fully removed; tests that exercised
  it are removed or rewritten.
* `web/src/components/AppStatusBar.svelte`, `web/src/editor/Wysiwyg.svelte`,
  and other components flagged in the earlier audit no longer carry
  agent-banner / agent-activity references.
* Hash-state code paths for `assistant` and `scope-history` overlays
  are removed (URL no longer encodes them).

## Verification

* `npm --prefix web run check` clean.
* `npm --prefix web test -- --run` green.
* `npm --prefix web run build` clean.
* Manual: load the dev build against an existing drive, confirm the
  console is free of 404s for `/api/llm/*` or `/api/assistant/*`,
  confirm no dangling "Open Agent" menu item, and confirm the URL hash
  no longer accepts `assistant=` / `scopes=`.

## Test expectations

* Delete or rewrite any svelte/store tests that asserted on agent
  conversation state, scope history hash, or LLM activity frames.
  Coverage on the remaining surface should stay green.

## Hardening expectations

* @@Systacean (or @@Architect on a quiet day) reads the diff before
  commit to make sure nothing MCP-facing was deleted by mistake.

## Progress

* 2026-05-17 @@Backend/@@Systacean picked this up as wave-1 residue
  after checking the update files.
* Removed the remaining assistant / LLM state exports from
  `web/src/state/store.svelte.ts`, including `assistantStream`,
  `assistantOverlay`, scope history state, hash restore/persist paths,
  session sidecar persistence, and delete/rename hooks.
* Removed stale tab-state cancellation hooks for assistant streams.
* Removed assistant/scope-history store tests and rewrote
  `web/src/state/store.test.ts` around the remaining graph hash/watch
  behavior.
* Confirmed `web/src/api/client.ts` and `web/src/api/types.ts` no longer
  expose LLM/assistant client methods or types.
* Cleaned stale assistant wording from shared markdown/transport comments,
  shared overlay chrome, editor helpers, and design notes.
* Cleaned the stale favicon comment in `web/index.html` that still referenced
  the removed assistant button / `--assistant-accent` token.
* Addressed Webtest B's hash observation: `persistStateToHash()` now
  canonicalizes the URL hash to known Chan keys only, so stale
  `assistant=` / `scopes=` keys are stripped on the next normal hash write.
  Added a regression test in `web/src/state/store.test.ts`.

## Completion notes

* Verification:
  * `npm --prefix web run check`
  * `npm --prefix web test -- --run`
  * `npm --prefix web run build`
* Build completed with existing Vite chunk-size / ineffective dynamic-import
  warnings, but no errors.
* `rg` for Agent/LLM/assistant terms in `web/src` only leaves unrelated
  substrings (`userAgent`, `dragenter`, terminal `magenta`) and real browser
  `Blob` usage; `web/index.html` is also clean of the old assistant favicon
  wording. No live store/API references remain.
* Unknown legacy hash keys are stripped during persistence; the new
  `persistence strips unknown legacy hash keys` test covers
  `assistant=` / `scopes=`.
