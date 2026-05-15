# alice-1: Triage user feedback after CLI assistant manual testing

Owner: Alice. Depends on: martin-6 / bob-4 cleanup context. Unblocks:
new Bob/Martin follow-up tasks.

## Why

The user manually tested the CLI-assistant settings and inline
assistant stream work and reported several product/architecture issues
that need splitting across frontend and backend before implementation.

This is an architect triage task, not an implementation task. Alice
should turn the points below into concrete Bob/Martin task briefs with
clear dependencies.

## User feedback to preserve

1. Settings readiness:
   - The assistant settings page only shows a green `ready` indicator
     for Claude.
   - Non-ready assistants must clearly explain what is wrong.
   - Current `/api/llm/status` only reports the active/default backend,
     so complete per-backend readiness likely needs backend support.

2. Settings layout redesign:
   - Left side: dropdown to pick the assistant CLI.
   - Under that dropdown: readiness indicator for the selected CLI.
   - Right side: override setting for user-configured `PATH` or direct
     path to the binary.
   - The binary/path override must be verified with chan-core's hardened
     path checking code.
   - Remove model picking from Settings.
   - Remove default-backend selection from Settings.
   - In the assistant overlay inspector, the model selector should always
     persist the last model used, similar to pane-size persistence.

3. Inline tool/activity log:
   - Tool/activity updates currently appear one after another instead of
     stacked as a stable list/log.
   - They do not show enough detail: tool name, parameters, output, etc.
   - They disappear over time; they should remain in the chat as a log.
   - This probably changes martin-3's transient `assistantStream.activity`
     rendering into persistent conversation turns or a separate persisted
     activity-log turn type.

4. Assistant banner/session switching:
   - Selecting Gemini still prints the Claude banner.
   - The banner must always match the selected assistant.
   - Switching assistant must not clear the current session.

## Suggested decomposition

- Bob task: backend CLI detection/config endpoint.
  - Report readiness for all supported CLI backends, not just the active
    one.
  - Support user-configured command/path override.
  - Validate override paths through chan-core hardened path checking.
  - Preserve enough config shape for the Settings dropdown UI.

- Martin task: Settings redesign.
  - Replace backend cards with left-side CLI dropdown + readiness status
    and right-side binary/PATH override controls.
  - Remove model/default controls from Settings.
  - Consume the backend readiness/config surface from Bob's task.

- Martin task: assistant inspector model persistence.
  - Persist last-used model from the assistant overlay inspector per CLI
    backend, using existing settings/session persistence conventions.

- Martin task: persistent activity/tool log.
  - Render tool/activity events as a stacked persistent log in chat.
  - Include tool details, args/partial args, output/error summaries, and
    timestamps where useful.
  - Avoid duplicating the existing structured tool_call/tool_result chips
    unless the new log replaces them deliberately.

- Martin task: banner and session switch behavior.
  - Ensure banner text/branding is derived from the selected assistant,
    not hard-coded Claude state.
  - Preserve current session/conversation when switching assistants.

## Notes

- Do not fold this into bob-4. bob-4 is cleanup of dead HTTP/keychain
  surface and should stay narrow.
- Current manual testing should use the Vite frontend at
  `http://127.0.0.1:5173/`; backend-served `:8787` may show stale
  embedded assets until the web bundle is rebuilt.

## Done means

Update `tasks/journal.md` with the split follow-up tasks and their
dependency order.
