# martin-6: Drop dead `/api/llm/keys*` client surface

Owner: Martin. Depends on: martin-5 (DONE). Unblocks: bob-4.

## Why

martin-5 removed the keychain UI from `SettingsPanel.svelte`, but
the API client and types still expose the legacy
`/api/llm/keys*` surface that has zero call sites now:

```
web/src/api/client.ts:120  llmKeysStatus  GET    /api/llm/keys
web/src/api/client.ts:151  setAnthropicKey  PUT    /api/llm/keys/anthropic
web/src/api/client.ts:154  clearAnthropicKey  DELETE /api/llm/keys/anthropic
web/src/api/client.ts:157  setGeminiKey  PUT    /api/llm/keys/gemini
web/src/api/client.ts:158  clearGeminiKey  DELETE /api/llm/keys/gemini
web/src/api/types.ts:179, 188  LlmKeysStatus types
```

Plus there are dead switch arms in `SettingsPanel.svelte` for the
legacy `claude` / `ollama` / `gemini` backend kinds (lines 119-122,
141-145) that became unreachable once the UI stopped offering those
kinds as choices. Once bob-4 drops the server enum variants, the
TypeScript types narrow and those arms become statically dead.

bob-4's pre-flight gate requires `grep -rn "/api/llm/keys"
web/src/` to return zero before it deletes the server routes. This
task makes that true.

## Files to touch

- `web/src/api/client.ts`
- `web/src/api/types.ts`
- `web/src/components/SettingsPanel.svelte`

## Required changes

### client.ts

Delete the five methods:
- `llmKeysStatus`
- `setAnthropicKey`
- `clearAnthropicKey`
- `setGeminiKey`
- `clearGeminiKey`

And any imports that become unused as a result.

### types.ts

Delete the `LlmKeysStatus` type and any per-provider key-status
sub-types that hung off it (line 179 onwards through the related
declarations). If a wider type re-exports `LlmKeysStatus`, follow
the references and drop those too.

If the TypeScript union for the `assistant` backend kind currently
lists `"claude" | "ollama" | "gemini"` alongside the three CLI
kinds, narrow it to just `"claude_cli" | "gemini_cli" | "codex_cli"`.
This matches what bob-4 is about to do on the server enum.

### SettingsPanel.svelte

Find the switch statements at lines 119-122 and 141-145 (the
`case "gemini":` etc. arms reading and writing
`a.gemini.enabled` / `a.claude.enabled` / `a.ollama.enabled`).
Delete the dead arms. With the type narrowed in types.ts these
should also become statically required-removed.

Drop any remaining references to `a.claude`, `a.gemini`, `a.ollama`
fields on the assistant prefs object. After bob-4 lands they
won't exist on the server payload either.

## Acceptance criteria

1. `npm run check` is green.
2. `npm test -- --run` is green.
3. `grep -rn "/api/llm/keys" web/src/` returns zero matches.
4. `grep -rn "anthropic\|gemini\|ollama" web/src/components/SettingsPanel.svelte`
   shows only references to the CLI variants (e.g. `gemini_cli`
   strings, comments mentioning the migration history) or to model
   names. No live calls to the legacy backend kinds.
5. No new dependencies, no UI behaviour change. The settings panel
   should look identical to its post-martin-5 state.

## Hints

- The legacy `claude` / `ollama` / `gemini` literals may also leak
  into the `assistant` discriminated union in `types.ts`. Search
  broadly with `grep -n '"claude"\|"ollama"\|"gemini"' web/src/api/`
  and prune the ones that match backend-kind context (not model
  names, not provider labels for unrelated systems).
- If you find references in `web/src/state/store.svelte.ts` or
  elsewhere outside `web/src/api/` and `SettingsPanel.svelte`,
  stop and report back. Those would be unexpected and may indicate
  a broader cleanup pass is needed.

## Done means

Post an update to `tasks/journal.md` (status DONE for martin-6,
plus a one-line log entry) so Alice can run bob-4's pre-flight
gate and dispatch bob-4.
