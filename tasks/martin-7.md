# martin-7: Settings panel redesign (CLI dropdown + override input)

Owner: Martin. Depends on: bob-5 (consumes the new detection endpoint
and cmd_override prefs surface), martin-6 (clean baseline).

## Why

User feedback after manual testing of martin-5:

- Per-CLI rows + "set as default" radio is too busy; user wants a
  simpler layout.
- Model picker doesn't belong in Settings; it moves to the
  assistant inspector (martin-8).
- Default-backend selection doesn't belong as a separate control;
  the selected CLI in the new dropdown IS the active backend.
- Override path for the binary is missing.

The new layout: left side is a dropdown for picking which CLI is
active, with a readiness indicator immediately below it. Right side
is an override input for the binary path (or PATH directory).

## Files to touch

- `web/src/components/SettingsPanel.svelte`. Likely also a small
  type/client update in `web/src/api/client.ts` to call the new
  `/api/llm/cli_detection` endpoint, and `web/src/api/types.ts`
  for the response shape.

## Required UI

### Layout

```
┌─ Assistant ────────────────────────────────────────────────────┐
│                                                                │
│   Active CLI:  [ Claude CLI    ▼ ]   Binary path override:     │
│                                       [_____________________]  │
│   ● ready                             ⓘ leave blank to use PATH│
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

- Dropdown: one of `claude_cli` / `gemini_cli` / `codex_cli`.
  Display labels match martin-5's row labels ("Claude CLI",
  "Gemini CLI", "Codex CLI"). Changing the dropdown updates the
  server's `default_backend` (the underlying field stays — the UI
  just stops exposing it as a separate radio).
- Readiness indicator: green dot + "ready" when the selected
  backend's detection record reports `ready: true`. Red dot + the
  server-formatted `reason` string when not ready. Render the
  reason verbatim; bob-5 produces it in the same format
  `/api/llm/status` uses today.
- Override input: bound to
  `editing.assistant.<selected_backend>.cmd_override`. Empty
  string means "use PATH" (server treats empty as None). Save
  on blur / debounce, same as other settings fields.
- After save, refresh detection to pick up the new override's
  effect.

### Data sources

- `/api/llm/cli_detection` (new in bob-5): array of three records;
  pick the one matching the dropdown's current value to drive the
  readiness indicator.
- `/api/preferences`: read/write the active backend
  (`assistant.default_backend`) and the per-CLI `cmd_override`
  fields.

### What to remove

- The per-CLI enable toggle rows from martin-5. The dropdown
  effectively replaces three on/off toggles with one
  "which-is-active" selector. Underlying `enabled` fields on the
  server can stay (bob-4 isn't touching them) but the UI no
  longer exposes them.
- The model picker rows. Model selection moves to martin-8's
  inspector work.
- The "set as default" radio from martin-5.

### What to keep

- Existing assistant-section header text and the master-switch
  / tunnel-public neutralisation behaviour.
- The active-backend status flow for live readiness on the
  selected entry.

## Acceptance criteria

1. `npm run check` and `npm test -- --run` are green.
2. Manual: pick each CLI from the dropdown; readiness pill flips
   to match. With a missing CLI selected, the failure reason from
   the server appears verbatim.
3. Set a custom override (absolute path to an installed CLI);
   readiness re-evaluates and reflects the override.
4. Set a garbage override; UI shows the server's 400-error
   message inline and does not lock in the bad value.
5. No "set as default", "enable toggle", or "model picker"
   controls remain in the Assistant section of Settings.

## Hints

- The dropdown's selected value should map directly to
  `assistant.default_backend` so persistence is one field, not
  two. The dropdown IS the default-backend control; we've just
  stopped calling it that in the UI.
- For the override input, debounce saves at ~500ms like other
  prefs fields. On error response from PUT, surface the error
  text near the input but keep the input enabled so the user can
  fix it.
- Don't pre-fetch detection on every keystroke. Re-detect after a
  successful save, or on dropdown change. The endpoint is cheap
  but not free.

## Done means

Post an update to `tasks/journal.md` (status DONE for martin-7, plus
a one-line log entry).
