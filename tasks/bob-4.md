# bob-4: Remove dead HTTP-backend and keychain plumbing

Owner: Bob. Depends on: martin-6 (cleans up the API client surface
that martin-5 left untouched; bob-4's pre-flight grep gate requires
zero `/api/llm/keys` references in `web/src/`).

## Why

chan-llm 0.11 dropped the HTTP-API backends (Anthropic HTTP, Gemini
HTTP, Ollama HTTP). The only supported backends are the three local
shell-executor CLIs: `ClaudeCli`, `GeminiCli`, `CodexCli`. Those
CLIs handle their own credentials, so the OS-keychain plumbing the
server used to expose for storing anthropic / gemini API keys has
no consumer.

bob-1 deliberately kept the dead surface as a minimal no-op layer
so the migration commit stayed small. With martin-5 removing the
frontend that called these endpoints, the server-side cleanup is
now safe.

## Files to touch (expected)

- `crates/chan-server/src/routes/preferences.rs`
- `crates/chan-server/src/routes/llm.rs`
- `crates/chan-server/src/lib.rs` (route table registrations)
- `crates/chan-server/src/error.rs` (comment about keychain at line 48)
- Possibly `crates/chan-server/src/state.rs` if any field becomes
  unused after this pass.

## Required changes

### 1. preferences.rs

- Delete `AssistantBackendKind::Claude`, `::Ollama`, `::Gemini`
  variants (lines 135-137). Adjust `to_chan_llm` to return
  `BackendKind` directly instead of `Option<BackendKind>` (every
  remaining variant maps to a real backend).
- Delete `ProviderPrefsView` (line 94) and `OllamaPrefsView`
  (line 114) structs.
- Delete the `claude`, `ollama`, `gemini` fields on
  `AssistantPrefsView` (lines 65-67) and their default
  initialisers in `preferences_view` (lines 186-188 and 199-201).
- Anywhere else that pattern-matches on the dead variants, drop
  the dead arms. The compiler will tell you.

### 2. routes/llm.rs

- Delete handlers and their route registrations:
  - `api_llm_keys_status` (line 976) - `/api/llm/keys` GET
  - `api_llm_set_anthropic_key` (line 991) - `/api/llm/keys/anthropic` POST
  - `api_llm_clear_anthropic_key` (line 1002) - `/api/llm/keys/anthropic` DELETE
  - `api_llm_set_gemini_key` (line 1006) - `/api/llm/keys/gemini` POST
  - `api_llm_clear_gemini_key` (line 1017) - `/api/llm/keys/gemini` DELETE
  - `api_llm_anthropic_models` (line 1102) - whatever route registers it
  - `api_llm_gemini_models` (line 1112)
  - `api_llm_ollama_models` (line 1133)
  - Anything else that wraps a keychain call. `grep -n "keychain"
    crates/chan-server/src/routes/llm.rs` is your friend.
- Remove the `keychain_available: bool` field from `LlmKeyView`
  (line 75) and its initialisers (lines 105, 143). Adjust the doc
  comment block at 72-74 accordingly.
- Re-evaluate `LlmKeyView` itself. Today its `set` / `source` /
  `path` fields describe "is the API key resolved?". For CLI
  backends the question is "is the CLI binary present?" which
  `LlmStatus.ready` + `reason` already answers. If `LlmKeyView`'s
  remaining fields are all redundant, fold them away and shrink
  `LlmStatus` accordingly. If you keep `LlmKeyView`, at minimum
  rename its doc so it talks about CLI presence, not API keys.
- Drop any helper functions that exist solely for keychain key
  resolution if they have no other callers. Don't delete chan-llm
  imports that are still used elsewhere.

### 3. lib.rs route table

The route registrations in `router()` (or wherever the axum
`Router::new().route(...)` chain lives) must lose the entries for
the deleted handlers. Grep for `/api/llm/keys`, `/anthropic`,
`/gemini`, `/ollama` in lib.rs.

### 4. error.rs:48

The comment block there talks about "keychain backends are all
attached to the owner's machine". Update or remove to match the
new reality: CLI backends sit on the same machine, the security
property the comment was protecting is still true, but the framing
is stale.

### 5. Tests

- Remove or adapt any tests that exercise the deleted endpoints.
- The lifecycle-frame tests bob-3 added are unaffected.

## Acceptance criteria

1. `cargo build` passes.
2. `cargo test` passes.
3. `cargo fmt --all -- --check` and `cargo clippy --all-targets --
   -D warnings` pass.
4. `grep -rn "keychain" crates/chan-server/src/` returns no
   substantive references (a stray comment about the OS-keychain
   feature that may come back later is fine, but no live code).
5. `grep -rn "AssistantBackendKind::\(Claude\|Ollama\|Gemini\)\b"
   crates/chan-server/src/` returns nothing.
6. `grep -rn "anthropic\|gemini\|ollama" crates/chan-server/src/
   --include='*.rs'` shows only references in comments / model
   names / unrelated contexts. No live API routes.

## Sequencing safety check

Before deleting `/api/llm/keys*`, `grep -rn "/api/llm/keys"
web/src/` MUST return zero matches. martin-5's job is to make that
true. If it doesn't, the frontend is still calling the endpoint
and we are NOT safe to delete it yet. In that case, stop and
report back; do not break the SPA.

## Out of scope

- A future `/api/llm/cli_detection` endpoint that reports presence
  for all three CLI backends at once. martin-5 lives without it.
- Migration of on-disk `llm.toml` to drop stored anthropic /
  gemini API keys. chan-llm 0.11 ignores them; users can clean up
  their own config files. If we want to do it, that's a separate
  task with a versioned schema migration.
- Removal of the `claude`, `gemini`, `ollama` strings from the
  default model names in `llm.toml`, if any. Those are model
  identifiers and may still be valid as CLI-side model selectors.

## Done means

Post an update to `tasks/journal.md` (status DONE for bob-4, plus a
one-line log entry summarising what got removed).
