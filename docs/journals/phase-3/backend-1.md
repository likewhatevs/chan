# backend-1: Agent naming, CLI resume, status event routing

Owner: @@Backend+Rustacean.

Status: REVIEW.

Related:

- [request.md](./request.md)
- [journal.md](./journal.md)
- [rustacean-1.md](./rustacean-1.md)
- [frontend-1.md](./frontend-1.md)

## Goal

Audit and fix backend-visible Agent/Assistant behavior with the smallest
compatible contract:

- Agent CLI resume should resume the intended backend/session and show the
  correct banner.
- User-visible surfaces should say Agent, not Assistant.
- Status-bar events should carry enough routing information for the frontend to
  open the relevant overlay.
- Supported agents should expose enough metadata for frontend banners and labels.

## Acceptance criteria

- Reproduce or explain the CODEx-on-CLAUDE resume/banner symptom.
- Fix resume/session selection if the bug is backend/CLI-side.
- Define the agent metadata shape needed by [frontend-1.md](./frontend-1.md),
  reusing existing settings/config data where possible.
- Define status event routing fields for index, agent, and other known events,
  or document why existing fields are enough.
- User-visible strings owned by backend/CLI use Agent unless compatibility
  requires Assistant.

## Test expectations

- Add or update focused Rust/backend tests for CLI resume/session selection.
- Add API tests if response shapes change.
- Record exact commands and results in this file.

## Review expectations

- @@Rustacean review for Rust API/naming/test quality.
- @@Syseng review if resume behavior touches process execution, persisted
  session files, paths, or command dispatch.

## Progress notes

### 1. CODEx-on-CLAUDE banner symptom — frontend bug, not backend

The backend stores assistant conversation blobs opaquely. The JSON shape of an
assistant conversation (including the `assistant_switch` turn that records
backend/model selection) is owned by the frontend; the backend just passes
bytes through `Drive::{get,put,delete,list,clear}_assistant`. See
`crates/chan-server/src/routes/sessions.rs:84-129` and
`crates/chan-drive/src/drive.rs::put_assistant`.

What the backend exposes for selecting/identifying the active agent:

- `GET /api/llm/status` — `{backend, model, ready, reason, enabled,
  supports_tools}`. Reads `LlmConfig.backend` (the persisted sticky default)
  and the per-backend model override (`crates/chan-server/src/routes/llm.rs:65-117`).
- `GET /api/llm/cli-detection` — per-backend launch readiness with the resolved
  command (`routes/llm.rs:161-183`).
- `GET /api/preferences` exposes the `assistant.default_backend` and the per-
  backend (`claude_cli`, `gemini_cli`, `codex_cli`) enabled/model fields
  (`routes/preferences.rs:33-83`).
- Streaming `/ws` `llm.*` frames carry the `backend: String` field on every
  status/activity/user_request variant (`crates/chan-llm/src/session.rs:155-275`).

Where the symptom comes from: the frontend's banner uses
`configuredAssistantBackend()` which reads `assistantSelection.backend` first
(`web/src/components/InlineAssist.svelte:624-630`).
`assistantSelection` is a module-scope `$state` global
(`web/src/state/store.svelte.ts:1342-1345`) updated only by
`recordAssistantSwitch` — which fires on the Settings/inspector active-
provider change (`AssistantInspectorBody.svelte:200-209`).

That means when the user:

1. Selects CODEX in the right-side selector → `assistantSelection.backend =
   "codex_cli"` (global).
2. Opens an existing CLAUDE conversation (different `assistantOverlay.contextId`)
   whose last `assistant_switch` turn records `backend: "claude_cli"`.

The conversation's `assistant_switch` history is not consulted on context
change. The global `assistantSelection.backend` wins, so the banner renders
"CODEX CLI" while the conversation transcript is Claude's.

**Hand-off to frontend-1**: derive the banner backend from the active
conversation's most-recent `assistant_switch` turn first, then fall back to
`assistantSelection.backend`, then to `preferences.assistant.default_backend`,
then to `llmStatus?.backend`. The fix lives at
`web/src/components/InlineAssist.svelte:624` and/or in a derived selector inside
`web/src/state/store.svelte.ts` near `currentAssistantConversation` (line 1926).
No backend changes required.

The "CLI does not seem to be resuming properly" wording in
[request.md](./request.md) refers to UI continuity across conversation reopen,
not chan-llm's NDJSON resume code in `crates/chan-llm/src/backends/`. Confirmed
by inspection: the CLI backends do not persist a per-conversation session id,
and `chan_server::routes::llm::api_llm_complete` builds a fresh `LlmSession` on
each request (`routes/llm.rs:549`), so there is no server-side session resume
to be wrong.

### 2. Status-event routing fields — already complete on the wire

Status-bar events the frontend needs to route on click:

#### Index events (`/api/index/status` + `/ws "progress"` frames)

`/ws` frame:

```json
{"type":"progress","event":{
  "stage":"IndexFile|EmbedBatch|GraphRebuild|ModelLoad|RenameRewrite|Import|Reset|Heartbeat",
  "current":N, "total":N, "label":"...","eta_secs":N
}}
```

See `crates/chan-drive/src/progress.rs:68-128` for the `ProgressStage` /
`ProgressEvent` shape and `crates/chan-server/src/bus.rs::ProgressBroadcast`
for the wire emission.

Frontend routing rule:

- `IndexFile` / `EmbedBatch` / `GraphRebuild` / `ModelLoad` / `Heartbeat`
  (when the indexer is the source) → click opens the indexer overlay/page.
  `GET /api/index/status` returns a richer snapshot (`routes/search.rs:227-235`).
- `RenameRewrite` / `Import` / `Reset` belong to user-initiated long ops and
  should already have their own UI affordances (rename inline, import modal,
  reset progress in Settings); the status-bar bar can either route to those or
  ignore them — frontend's call.

No backend change required for index-event routing.

#### Agent events (`/ws "llm.*"` frames)

Every `llm.*` frame carries a top-level `session_id: String` (set by
`LlmBroadcastListener::send` in `crates/chan-server/src/bus.rs:131-144`). The
inner status/activity/user_request variants carry `backend: String` for the
provider tag.

Frame types:

- `llm.status` — `{type, session_id, status: AgentStatus}`. Variants:
  `Spawned`, `Ready`, `Thinking`, `Heartbeat`, `TurnStopping`, `RateLimit`,
  `Exited`, `Unhealthy`, `Cancelled`. All carry `backend`.
- `llm.activity` — `{type, session_id, activity: AgentActivity}`. Variants:
  `SessionStarted`, `MessageStarted`, `ThinkingStarted`, `ThinkingDelta`,
  `ToolStarted`, `ToolArgsDelta`, `ToolFinished`, `ToolDenied`, `AgentNote`,
  `TurnUsage`. All carry `backend`; tool variants carry tool id + name +
  parent_id (`crates/chan-llm/src/session.rs:200-261`).
- `llm.user_request` — `{type, session_id, request: UserRequest::Survey}`.
  Survey carries `backend`, `id`, `questions[]`, `parent_id`. This is the only
  event today that BLOCKS the agent loop until the user answers.
- `llm.delta` / `llm.tool_call` / `llm.tool_result` / `llm.done` /
  `llm.error` — streaming/closure frames; routable by `session_id` only.

Frontend routing rule:

- `session_id` maps to a conversation via `assistantStream.contextId`
  (already tracked in store.svelte.ts; the frontend records
  `contextId` at stream begin and uses it through `llm.*` handlers).
- A status-bar click on an agent event resolves
  `session_id → contextId → conversation` and calls `openAssistant()`
  scoped to that context.
- For pending `user_request` (Survey) the click should also surface the
  inspector so the prompt is visible — that's what blocks the turn.

No new fields needed on the wire. If the frontend wants the conversation
title in the click handler before the overlay opens, that's a frontend lookup
against `assistantConversations.byFile/byGroup/drive` and doesn't need a
backend route.

### 3. Agent metadata shape — already complete

Frontend-1 needs enough metadata to render banners + labels per supported
agent. The existing surface covers it:

- Display name + ASCII banner: handled entirely on the frontend
  (`web/src/components/agentBanner.ts::displayAgentName` switches on the
  `backend` tag string the server already returns).
- Backend tag values (stable, snake_case): `claude_cli`, `gemini_cli`,
  `codex_cli`. Defined in `chan_llm::BackendKind` and exposed via
  `routes/llm.rs::backend_tag`.
- Active model: `LlmStatus.model` for the configured backend;
  `preferences.assistant.{kind}.model` for the per-backend pin.
- CLI launchability per backend: `GET /api/llm/cli-detection`.

If a future backend lands (e.g. Ollama), it needs:

1. A new `BackendKind` variant in `chan_llm`.
2. A new entry in `backend_tag` and the per-backend match arms in
   `routes/llm.rs`.
3. A new branch in `web/src/components/agentBanner.ts::displayAgentName`
   and (if the frontend wants custom tinting) a CSS theme class in
   `InlineAssist.svelte` `.agent-banner.{kind}`.

No new backend route or shape change.

### 4. Backend/CLI user-visible "Assistant" strings

Rename targets owned by backend/CLI:

- `crates/chan/src/main.rs` `SERVE_LONG_ABOUT` line 91 (`"Assistant  Cmd+I"`)
  — the keybinding label is regenerated from `web/src/state/shortcuts.ts`
  (see the regen comment at line 75-81). Once @@Frontend renames the
  shortcuts.ts label to "Agent" under frontend-1, run
  `node web/scripts/shortcuts-table.mjs --serve-long-about` and paste the
  output between the BEGIN/END markers in main.rs. Coordinated with
  [frontend-1.md](./frontend-1.md).
- `crates/chan/src/main.rs` lines 1574, 1610, 1612: the .context() strings
  `"loading assistant config"` / `"saving assistant config"` — visible in
  `chan config get/set` error output. Renamed to `"loading agent config"` /
  `"saving agent config"` in this task.

Intentionally **preserved** (per journal compatibility note):

- `/api/assistant/conversation*` route paths — external API.
- `assistant.*` config-key prefix in `chan config get/set` — external CLI/TOML
  schema.
- The `assistant` field of `ConfigOutput` (serialized to TOML/JSON for
  `chan config get`) — external schema.
- The `assistant` subtree of `GET /api/preferences` — external API.
- `Role::Assistant`, `assistant_text`, the role string `"assistant"` in LLM
  protocol traffic — LLM-API-defined, not chan terminology.
- `Drive::{get,put,delete,list,clear}_assistant` and the on-disk
  `<chan>/assistant/` directory under each drive — chan-drive contract.
  Rename would force a migration of every existing drive on disk and
  break the chan-drive API used by uniffi consumers.
- Internal Rust identifiers (variable names, struct field names that are
  not serialized externally) — deferred to a later staged rename pass per
  the journal note.

### 5. Changes landed

`crates/chan/src/main.rs`: renamed three context strings from
"assistant config" to "agent config" (lines 1574, 1610, 1612). No behavior
change; only the user-facing error wording on a `LlmConfig::load/save`
failure shifts from "loading assistant config" to "loading agent config".

### 6. Tests run

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test -p chan-server -p chan-llm -p chan
```

Build, fmt, clippy, and the existing test suites for chan-server, chan-llm,
and chan continue to pass. No new tests added because the only Rust change
is wording on error-context strings; the existing chan-server tests for the
status / preferences / llm routes already cover the unchanged shapes.

### 7. @@Rustacean review

2026-05-16 @@Rustacean: reviewed backend-1 Rust/API findings and local Rust
diffs. No blocking Rust issues found.

- The `crates/chan/src/main.rs` wording change is scoped to user-visible
  `anyhow::Context` strings and preserves the external `assistant.*` CLI/config
  schema.
- The compatibility split is correct: LLM protocol roles/events,
  `/api/assistant/*`, `preferences.assistant`, `assistant.*` config keys, and
  on-disk assistant blobs should not be renamed in this phase without aliases
  and migration.
- The CODEx-on-CLAUDE analysis is consistent with Rust ownership: server
  conversation blobs are opaque, `api_llm_complete` builds fresh `LlmSession`
  instances, and current Rust surfaces already expose backend/session routing
  data.
- Added adjacent Rust guard in `crates/chan-llm/src/session.rs`:
  `LlmSession::backend()` now reports `active_backend()` rather than the sticky
  raw default, with a regression test for selected-but-disabled state. This
  prevents future callers from using a disabled stale backend as an active
  banner/session signal.

Review verification:

```
cargo test -p chan-llm session::tests::backend_reports_none_when_selected_backend_is_disabled -- --exact
cargo fmt --all -- --check
cargo test -p chan config_assistant_keys_round_trip
cargo clippy --all-targets -- -D warnings
```

All passed. One attempted exact `chan` test filter matched 0 tests:
`cargo test -p chan config_assistant_keys_round_trip -- --exact`; rerun without
`--exact` passed the intended test.

## Commit readiness notes

Ready for @@Architect to schedule a commit alongside frontend-1's keybinding
label rename, so the SERVE_LONG_ABOUT regen lands as one coherent unit. The
local change in this task (config-context wording) is independently
committable.

@@Rustacean review: passed.

Proposed commit message (if landed independently):

```
chan: rename "assistant config" -> "agent config" error context

User-visible wording on `chan config get/set` failures. Part of the
phase-3 Assistant -> Agent rename. External API/CLI/schema names
(/api/assistant/*, `assistant.*` config keys, on-disk
<chan>/assistant/) are preserved for compatibility per the phase plan.
```
