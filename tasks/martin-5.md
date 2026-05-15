# martin-5: SettingsPanel CLI-backend UI (unblock manual testing)

Owner: Martin. Depends on: martin-3 (DONE).

## Why this exists

The user tried to manually test martin-3 against a running `chan
serve` and could not select a working backend. After bob-1's
chan-core 0.11 migration the only backends chan-llm still supports
are the three CLI shell-executors (`ClaudeCli`, `GeminiCli`,
`CodexCli`). The HTTP-API backends (anthropic, gemini, ollama) were
dropped from chan-llm; `AssistantBackendKind::{Claude, Ollama,
Gemini}` still exist in the server enum as dead options that map to
`None` via `to_chan_llm` (see
`crates/chan-server/src/routes/preferences.rs` lines 152-161).

But `SettingsPanel.svelte` still surfaces only the legacy
HTTP/keychain flow: it has a `KeychainProvider = "anthropic" |
"gemini"` (line 90), saving an anthropic key auto-sets
`default_backend = "claude"` (line 145), and there is no UI to
toggle or default the three CLI backends. Net effect: the user can
type API keys all day and the assistant stays disabled because
those backends route to `None`.

This task replaces the assistant section of SettingsPanel with one
that lets the user enable / select / configure the three CLI
backends, which is what actually works post-migration.

## Files to touch

- `web/src/components/SettingsPanel.svelte` only.

## Required UI

### 1. Backend list (top of assistant section)

Show one row per CLI backend (`claude_cli`, `gemini_cli`,
`codex_cli`). Each row:

- Display name (e.g. "Claude CLI", "Gemini CLI", "Codex CLI"). No
  emojis.
- An `enabled` toggle bound to the matching
  `editing.assistant.<backend>.enabled` field. These already come
  from the server (see `AssistantPrefsView` lines 202-213 in
  `routes/preferences.rs`).
- A model input bound to `editing.assistant.<backend>.model`.
  Optional; placeholder "default" when empty.
- A "set as default" radio bound to
  `editing.assistant.default_backend`. Values are `"claude_cli"`,
  `"gemini_cli"`, `"codex_cli"`.

When the user picks one as default, the existing save flow already
persists it (the server's existing
`AssistantBackendKind::{ClaudeCli, GeminiCli, CodexCli}` round-trip
works). No new endpoints required.

### 2. Status indicator per row

Read the active backend's status from `/api/llm/status` (already
called on mount; see the existing `loadStatus` logic). For the
currently-default backend, show the `ready` / `reason` fields:

- `ready === true`: green dot, "ready".
- `ready === false`: red dot with the `reason` text inline (the
  server already formats it as `"\`<cmd>\` not found or rejected.
  Install the <name> CLI, or set its cmd in llm.toml."`). Render
  it verbatim; this is the right error UX already.

For the other two rows, no detection status is fetched in v1 (the
existing endpoint only reports the active backend). That is fine
for unblocking the user; we add a multi-backend detection endpoint
later if needed.

### 3. Drop the legacy keychain UI

Remove the `anthropic` / `gemini` keychain input rows and the
`saveKeychain` / `removeKeychain` flows for them. The chan-llm
backends those keys talked to no longer exist; keeping the UI
implies functionality that's gone.

Specifically:
- Delete `KeychainProvider` type, the `keychainInput`,
  `keychainBusy`, `keychainError` records, and the two functions.
- Remove the JSX rows that render anthropic / gemini key inputs.
- Remove the `editing.assistant.default_backend = "claude"` /
  `"gemini"` assignment in the save path. Those values are dead.

### 4. Don't touch the unrelated branches

- Tunnel-public neutralization, `effective_enabled`, the master
  switch UI, theme / pane / editor settings: leave alone.
- Do NOT add UI for the dead `claude`, `ollama`, `gemini` enum
  variants. They exist in the server enum only because removing
  them from the disk schema is a separate cleanup (see follow-ups
  below).

## Acceptance criteria

1. `pnpm exec svelte-check` / `npm run check` is green.
2. `pnpm exec vitest run` is green.
3. Manual: launch `chan serve`, open Settings, see three CLI rows
   with enabled toggles and a default-backend radio; pick one,
   save, reload page, selection persists.
4. With the selected CLI not installed on disk, the status row
   shows the server-provided "not found" message verbatim.
5. With the CLI installed, status flips green and InlineAssist
   submits land at the CLI.
6. No anthropic / gemini API-key inputs remain in the UI.

## Out of scope (cut as follow-ups, do not do here)

- Server-side cleanup of the dead `AssistantBackendKind::{Claude,
  Ollama, Gemini}` enum variants and their empty stub fields. This
  is a Bob task; will be a `bob-4` if/when we cut it. Not blocking
  manual testing.
- A `/api/llm/cli_detection` endpoint that reports presence for
  all three CLI backends in one call. Today the active-backend
  status endpoint is enough.
- Per-CLI install instructions in the UI. The reason string from
  the server already names the binary; that's enough for now.

## Hints

- The save path lives in `SettingsPanel.svelte` around the
  `saveKeychain` / "Save" handler. Wire the new CLI rows into the
  same `editing.assistant.*` object so the existing PUT call
  picks them up unchanged.
- The server's `LlmStatus.backend` tag string for the active CLI
  is `claude_cli` / `gemini_cli` / `codex_cli` (see `backend_tag`
  in `routes/llm.rs`). Match on that, not on display name.
- Take care to preserve existing accessibility and keyboard
  navigation in the panel. Radios should be in a single `<fieldset>`
  so screen readers announce the group correctly.

## Done means

Post an update to `tasks/journal.md` (status DONE for martin-5, plus
a one-line log entry).
