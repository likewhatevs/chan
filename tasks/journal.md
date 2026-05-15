# HTTP Stream Follow-Up: Coordination Journal

Owner: Alice (architect). Team: Bob (backend, rustacean), Martin
(frontend, webdev).

Source plan: `tasks/http-stream.md`.

## Plan summary

Wire chan-core 0.11 lifecycle stream events (`AgentStatus`,
`AgentActivity`, `UserRequest`) into ../chan so the websocket fan-out,
store reducer, and InlineAssist surface agent status, tool/activity
progress, and user-request prompts. First pass is display-only; the
bidirectional answer path is out of scope.

## Dispatch

| Task     | Owner  | Status      | Depends on   |
|----------|--------|-------------|--------------|
| bob-1    | Bob    | DONE        | -            |
| bob-2    | Bob    | DONE        | bob-1 (DONE) |
| bob-3    | Bob    | DONE        | bob-2 (DONE) |
| martin-1 | Martin | DONE        | bob-1 (DONE)    |
| martin-2 | Martin | DONE        | martin-1 (DONE) |
| martin-3 | Martin | DONE        | martin-2 (DONE) |
| martin-4 | Martin | DONE        | martin-2 (DONE) |
| martin-5 | Martin | DONE        | martin-3 (DONE) |
| martin-6  | Martin | DONE    | martin-5 (DONE)            |
| bob-4     | Bob    | DONE    | martin-6 (DONE)            |
| alice-1   | Alice  | DONE    | martin-5 (DONE) + feedback |
| bob-5     | Bob    | DONE        | -                      |
| martin-7  | Martin | DONE    | bob-5, martin-6            |
| martin-8  | Martin | DONE    | martin-7                   |
| martin-9  | Martin | DONE    | martin-2 (DONE)            |
| martin-10 | Martin | DONE    | -                          |
| martin-11 | Martin | DONE    | -                          |

Statuses: TODO, IN_PROGRESS, BLOCKED, REVIEW, DONE.

## Critical path

```
bob-1 ──┬─> bob-2 ──> bob-3
        │
        └─> martin-1 ──> martin-2 ──┬─> martin-3
                                    └─> martin-4
```

bob-1 unblocks both tracks because the chan-core 0.11 bump may cause
compile churn that Martin's TypeScript types should mirror. Once bob-1
lands, the backend and frontend tracks run in parallel.

## Notes & decisions

- First pass is display-only. `UserRequest::Survey` is rendered as a
  read-only prompt; answering is deferred to a follow-up that needs a
  new bidirectional control path (see plan section "User-Request
  Limitation").
- `/api/llm/complete` JSON response shape stays unchanged. All new
  lifecycle data flows over the existing `/ws` channel.
- Existing `llm.delta`, `llm.tool_call`, `llm.tool_result`, `llm.done`,
  `llm.error` frames must remain byte-identical.
- Per-session filtering on `session_id` is mandatory on every new
  frame, matching the existing pattern in
  `web/src/state/store.svelte.ts` around line 220.
- Manual testing should use the Vite dev frontend at
  `http://127.0.0.1:5173/`. The backend's embedded SPA at `:8787`
  serves whatever bundle was baked at build time and may be stale
  until `npm run build` is rerun. When a feedback item is "I don't
  see X in the UI", first confirm the user is hitting `:5173`.
- alice-1 follow-up split (post manual-testing feedback): bob-5 is
  the new CLI detection + cmd-override surface; martin-7 is the
  Settings layout redesign that consumes it; martin-8 moves model
  picking into the assistant inspector; martin-9 turns the
  transient activity strip into a persistent stacked tool log;
  martin-10 fixes the backend banner + preserves the session on
  switch; martin-11 is a targeted fix for codex_cli missing from
  the inspector picker and `displayAgentName`.

## Log

- 2026-05-15 Alice: read plan, surveyed code, split into 7 tasks
  (3 backend, 4 frontend), wrote journal + per-task briefs.
- 2026-05-15 Bob: completed bob-1; bumped chan-core deps to 0.11.0,
  migrated server LLM config/status compatibility to CLI-only
  chan-llm, regenerated Cargo.lock, and verified build/test/fmt/clippy.
- 2026-05-15 Alice: noted that `Cargo.toml` was bumped to `=0.11.0`
  out of band before bob-1 picked up the task. bob-1 still owns
  `Cargo.lock` regen and the migration compile pass; brief updated.
- 2026-05-15 Alice: dispatch ready. Bob starts on bob-1; Martin
  blocks on bob-1 completion before starting martin-1.
- 2026-05-15 Martin: started martin-1 after bob-1 completion; adding
  TypeScript lifecycle stream types only.
- 2026-05-15 Martin: completed martin-1; added AgentStatus,
  AgentActivity, UserRequest, and new llm lifecycle frame types in
  `web/src/api/types.ts`; verified `npm run check` and
  `npm test -- --run`.
- 2026-05-15 Bob: started bob-2 lifecycle websocket frame wiring.
- 2026-05-15 Bob: completed bob-2; wired AgentStatus,
  AgentActivity, and UserRequest through LlmBroadcastListener and
  CollectListener, then verified build/test/fmt/clippy.
- 2026-05-15 Alice: verified bob-1 independently. `cargo build`,
  `cargo test` (25 + 56 pass), and `cargo clippy --all-targets --
  -D warnings` all green on the dispatched tree. Bob's migration
  shrank `routes/llm.rs` by ~390 lines; `CollectListener` (now line
  358) and `LlmBroadcastListener` (imported line 23) are still in
  place, so bob-2's edit points are intact. Updated `bob-2.md` with
  the new line number so Bob doesn't go hunting.
- 2026-05-15 Bob: started bob-3 listener serialization and forwarding tests.
- 2026-05-15 Bob: completed bob-3; added JSON-shape tests for
  lifecycle websocket frames plus CollectListener forwarding/state
  coverage; verified `cargo test -p chan-server`, fmt, and clippy.
- 2026-05-15 Alice: verified bob-2 and martin-1 independently.
  Backend: `cargo build` + `cargo clippy --all-targets -- -D warnings`
  green; three new methods (`on_status`, `on_activity`,
  `on_user_request`) present in `bus.rs` (lines 147/150/153) and
  forwarded by `CollectListener` (lines 400/403/406). Frontend:
  `npm run check` clean (0 errors / 0 warnings across 3905 files),
  `npm test -- --run` green (71 tests / 4 files). Types include
  `Unknown{AgentStatus,AgentActivity,UserRequest}` open variants so
  the reducer in martin-2 can tolerate future kinds without code
  edits. Updated `martin-2.md` with the exact exported type names.
- 2026-05-15 Alice: dispatched next wave. Bob already on bob-3;
  Martin clear to start martin-2.
- 2026-05-15 Martin: completed martin-2; reducer now handles
  llm.status/activity/user_request with session filtering, bounded
  activity history, unknown llm.* no-op guard, and assistantStream
  lifecycle fields; verified `npm run check` and `npm test -- --run`.
- 2026-05-15 Alice: verified bob-3 + martin-2 independently. Backend:
  `cargo test -p chan-server` went 56 to 64 passing (8 new tests,
  4 in `bus.rs` covering each frame type's JSON shape, 3 in
  `routes/llm.rs` covering CollectListener forwarding/state, plus
  one more); `cargo clippy --all-targets -- -D warnings` and
  `cargo fmt --all -- --check` clean. Frontend: `npm run check`
  clean (3905 files, 0 errors), `npm test -- --run` green (71/71);
  reducer matches lines 224-260 in `store.svelte.ts`, activity ring
  capped at 32 via `.shift()` (line 256), heartbeat stamp only on
  `kind === "heartbeat"` (line 249), unknown `llm.*` swallowed by
  `frameType?.startsWith("llm.")` guard at line 329. Backend track
  fully landed; martin-3 + martin-4 dispatched in parallel.
- 2026-05-15 Martin: completed martin-4; added store reducer tests
  for lifecycle frames, session filtering, activity cap, unknown
  llm.* no-op, end clearing, and delta regression; verified
  `npm run check` and `npm test -- --run`.
- 2026-05-15 Martin: completed martin-3; InlineAssist now renders
  lifecycle status, selected activity chips/notes, and display-only
  survey prompts; verified `npm run check` and `npm test -- --run`.
- 2026-05-15 Martin: started martin-5; replacing dead HTTP/keychain
  assistant settings with CLI backend toggles/default/model rows.
- 2026-05-15 Martin: completed martin-5; Settings assistant section
  now shows Claude/Gemini/Codex CLI rows with enable/default/model
  controls and active-backend readiness, with legacy keychain UI
  removed; verified `npm run check` and `npm test -- --run`.
- 2026-05-15 Alice: scoped martin-5 after user reported the
  SettingsPanel still surfaces anthropic/gemini API-key inputs and
  no CLI backend selector after bob-1's chan-core 0.11 migration.
  Cause: server's preferences view exposes claude_cli/gemini_cli/
  codex_cli config + CLI detection but `SettingsPanel.svelte` only
  renders the legacy keychain flow that maps to dead
  `AssistantBackendKind::{Claude, Ollama, Gemini}` variants.
- 2026-05-15 Alice: cut bob-4 to delete the dead surface after
  martin-5 lands. Scope: drop `AssistantBackendKind::{Claude,
  Ollama, Gemini}` enum variants, `ProviderPrefsView` /
  `OllamaPrefsView` structs, the `/api/llm/keys*` routes
  (anthropic + gemini POST/DELETE/GET), the
  `api_llm_{anthropic,gemini,ollama}_models` handlers, and
  `LlmKeyView.keychain_available`. Brief includes a pre-flight
  grep gate so Bob does not delete a route the SPA still calls.
  Sequenced after martin-5 to avoid breaking the SPA mid-flight.
- 2026-05-15 Alice: verified martin-5 independently. `npm run check`
  clean (3906 files), `npm test -- --run` green (79 / 5 files, +8
  from martin-4's vitest additions). SettingsPanel CLI rows live at
  lines 343-347 with claude_cli/gemini_cli/codex_cli kinds + hints.
  No KeychainProvider references remain in the panel itself.
- 2026-05-15 Alice: bob-4 pre-flight gate FAILED. `grep -rn
  "/api/llm/keys" web/src/` matched 5 client methods in
  `web/src/api/client.ts` (`llmKeysStatus`, `setAnthropicKey`,
  `clearAnthropicKey`, `setGeminiKey`, `clearGeminiKey`) plus a
  `LlmKeysStatus` type in `web/src/api/types.ts`. SettingsPanel
  also retains dead switch arms for the legacy `claude` / `gemini`
  enum kinds at lines 119-122 / 141-145. These were out of scope
  for martin-5 (which I only scoped to the panel's UI). Cut
  martin-6 to clean them up, re-pointed bob-4's dependency to
  martin-6, marked bob-4 BLOCKED until martin-6 lands.
- 2026-05-15 Martin: recorded new user feedback in `tasks/alice-1.md`
  for Alice to split and coordinate: full per-CLI readiness/error
  reporting, redesigned assistant settings layout with binary/PATH
  override validation, model persistence in assistant inspector,
  persistent detailed tool/activity log, and correct assistant banner
  without clearing the current session.
- 2026-05-15 Alice: completed alice-1 triage. Surveyed chan-llm
  config (per-CLI `models` field already exists; no Bob work
  needed for martin-8 persistence), chan-drive path code
  (`resolve_safe_strict` is drive-scoped, wrong tool for binary
  validation — bob-5 will use chan-llm's CLI detection instead),
  and agentBanner.ts (hard-coded Claude branding + no codex_cli
  case). Cut six new tasks: bob-5 (multi-CLI detection +
  cmd_override), martin-7 (Settings dropdown redesign), martin-8
  (model picker in inspector with persistence), martin-9
  (persistent tool/activity log via richer tool turns), martin-10
  (banner + session preservation), and the targeted martin-11.
- 2026-05-15 Alice: user reported codex missing from the assistant
  dropdown. Traced to `AssistantInspectorBody.svelte` which only
  handles claude_cli + gemini_cli in active-provider resolution
  (line 128-129), picker rows (146-147), model resolution
  (393-394), and the model `<select>` branches (579-595).
  bob-1's chan-core 0.11 migration added codex_cli on the server
  but the inspector UI was never updated. `displayAgentName` in
  `agentBanner.ts` is also missing the codex_cli case. Cut
  martin-11 as a targeted small fix so the user can keep
  testing while the broader martin-7/8/9/10 work lands later.
- 2026-05-15 Bob: started bob-5 per-CLI readiness endpoint and
  command override persistence.
- 2026-05-15 Bob: completed bob-5; added `/api/llm/cli_detection`,
  `cmd_override` preference round-trip with validation through
  chan-llm CLI detection, and focused Rust tests; verified build,
  test, fmt, and clippy.
- 2026-05-15 Bob: completed bob-4; removed dead HTTP backend,
  keychain, and model-catalog server routes plus stale frontend API
  callers/types, then verified Rust and web checks.
- 2026-05-15 Martin: completed martin-6; removed dead frontend
  `/api/llm/keys*` client methods/types and scoped SettingsPanel's
  backend helpers to CLI rows only; verified `npm run check`,
  `npm test -- --run`, and the `/api/llm/keys` grep gate.
- 2026-05-15 Alice: verified martin-6 independently. `grep -rn
  "/api/llm/keys" web/src/` returns 0 matches (gate PASSES); same
  for `llmKeysStatus` / `setAnthropicKey` / `clearAnthropicKey` /
  `setGeminiKey` / `clearGeminiKey` / `LlmKeysStatus`. `npm run
  check` clean (3906 files, 0 errors), `npm test -- --run` green
  (79/79 across 5 files). bob-4 unblocked; can now run in parallel
  with the in-flight bob-5.
- 2026-05-15 Martin: completed martin-11; added Codex CLI to the
  assistant inspector picker/model field and `displayAgentName`,
  including the missing `X` banner glyph; verified `npm run check`
  and `npm test -- --run`.
- 2026-05-15 Martin: completed martin-10; empty-state assistant
  banner now maps Claude/Gemini/Codex/Ollama from the active backend
  and backend switching only updates `default_backend`, preserving
  existing conversation state; verified `npm run check` and
  `npm test -- --run`.
- 2026-05-15 Martin: completed martin-7; Settings assistant section
  now uses a single active-CLI dropdown, selected-backend readiness
  from `/api/llm/cli_detection`, and a per-backend binary override
  input with rejected-save retry suppression; verified `npm run
  check` and `npm test -- --run`.
- 2026-05-15 Martin: completed martin-8; model selection now lives
  in the assistant inspector, persists through the existing
  preferences save path per CLI backend, and no model/default/toggle
  controls remain in Settings; verified `npm run check` and
  `npm test -- --run`.
- 2026-05-15 Martin: completed martin-9; tool activity now folds
  into persistent tool turns with args, partial args, output,
  explicit error/status, and timestamps, while the chat renders
  stacked tool cards with output preview/expand; added reducer
  coverage and verified `npm run check` plus `npm test -- --run`.
- 2026-05-15 Alice: verified bob-4, bob-5, martin-10, martin-11
  independently. Backend: `cargo build`, `cargo test` (25 chan +
  67 chan-server = 92 pass, was 64 in chan-server before so +3
  net), `cargo clippy --all-targets -- -D warnings` all green;
  `grep -rn "AssistantBackendKind::{Claude|Ollama|Gemini}|
  ProviderPrefsView|OllamaPrefsView|api_llm_{anthropic,gemini,
  ollama}_models|/api/llm/keys" crates/chan-server/src/` returns
  zero (bob-4 fully cleaned). bob-5: `/api/llm/cli_detection`
  endpoint exists with `CliDetectionResponse`/`CliDetectionView`
  (routes/llm.rs:120-176, registered in lib.rs:784);
  `cmd_override: Option<String>` on `CliPrefsView` (line 92);
  `validated_cmd_override` helper (line 406) writes through to
  per-CLI `cmd` fields. Frontend: `npm run check` clean (3906
  files / 0 errors), `npm test -- --run` 79/79 across 5 files.
  martin-10: agentBanner.ts has Codex case + ALPHABET expanded
  with `X` glyph; backend switch path no longer resets
  conversation state. martin-11: codex_cli wired into
  AssistantInspectorBody at lines 109/125/208/256-267 and
  agentBanner.ts:150. Both critical-path blockers for manual
  testing are now resolved.
- 2026-05-15 Alice: martin-7 fully unblocked (both bob-5 and
  martin-6 are DONE). Queue stands at: martin-7 → martin-8
  (Settings redesign + model picker move), martin-9 (persistent
  tool log) is independent.
