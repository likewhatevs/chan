# martin-11: Codex CLI visibility in assistant inspector

Owner: Martin. Depends on: nothing. Independent fix.

## Why

User reports the assistant inspector's backend dropdown does not
show Codex CLI. Grep confirms `AssistantInspectorBody.svelte` was
written for `claude_cli` and `gemini_cli` only; the `codex_cli`
case was never added:

```
AssistantInspectorBody.svelte:33   // docstring says "claude_cli / gemini_cli" only
AssistantInspectorBody.svelte:128  if (a.claude_cli.enabled) return "claude_cli";
AssistantInspectorBody.svelte:129  if (a.gemini_cli.enabled) return "gemini_cli";
                                   (no codex_cli branch)
AssistantInspectorBody.svelte:146  { kind: "claude_cli", ... },
AssistantInspectorBody.svelte:147  { kind: "gemini_cli", ... },
                                   (codex_cli missing from picker rows)
AssistantInspectorBody.svelte:393  if (kind === "claude_cli") return a.claude_cli.model
AssistantInspectorBody.svelte:394  if (kind === "gemini_cli") return a.gemini_cli.model
                                   (codex_cli model never resolved)
AssistantInspectorBody.svelte:579  {:else if activeProvider === "claude_cli"} ...
AssistantInspectorBody.svelte:589  {:else if activeProvider === "gemini_cli"} ...
                                   (no codex_cli branch in model <select>)
```

bob-1's chan-core 0.11 migration added the codex_cli backend on
the server but this UI was never updated.

Also: `agentBanner.ts::displayAgentName` (line 131-147) has cases
for `claude` / `claude_cli` / `gemini` / `gemini_cli` / `ollama`
but no case for `codex_cli`. It falls through to the raw uppercase
fallback. martin-10 covers the banner overhaul, but you can fix
this one-liner here while you're already in agentBanner.ts.

## Files to touch

- `web/src/components/AssistantInspectorBody.svelte`
- `web/src/components/agentBanner.ts` (one-liner)

## Required changes

### AssistantInspectorBody.svelte

Add `codex_cli` everywhere `claude_cli` and `gemini_cli` already
appear:
- Active-provider resolution (line 128): add
  `if (a.codex_cli.enabled) return "codex_cli";` before whatever
  fallback exists.
- Picker rows array (line 146): add
  `{ kind: "codex_cli", label: "Codex CLI", on: editing.assistant.codex_cli.enabled }`.
- Model resolution (line 393): add
  `if (kind === "codex_cli") return a.codex_cli.model ?? null;`.
- Model `<select>` branch (after line 589): add a
  `{:else if activeProvider === "codex_cli"}` branch matching the
  existing two. Curate the model shortlist by checking what
  chan-llm exposes for codex CLI (look at
  `~/dev/github.com/chan-writer/chan-core/crates/chan-llm/src/backends/codex_cli.rs`
  if there's a published model list; otherwise leave the shortlist
  as a free-text input with placeholder "default" until martin-8
  redoes this surface).
- Update the docstring at line 33-34 to include codex_cli.

### agentBanner.ts

Add `codex_cli` to `displayAgentName`'s switch:

```ts
case "codex_cli":
  return "CODEX CLI";
```

Note that the ASCII alphabet at the top of the file does not
include `X` or `_`. If the banner renders for codex_cli the
result will be "CODECLI" with the X and _ silently dropped. This
is ugly but not blocking. Either add `X` to the alphabet (six-row
ANSI Shadow style; match the existing glyphs) or accept the
truncated banner and leave a note. Use judgement; this is small.

## Acceptance criteria

1. `npm run check` and `npm test -- --run` are green.
2. Manual: open the assistant inspector with codex_cli enabled in
   prefs; the picker shows Codex CLI as a selectable option.
3. Selecting Codex CLI in the inspector renders a model field
   (free-text or shortlist).
4. `displayAgentName("codex_cli")` returns "CODEX CLI".

## Out of scope

- The broader inspector / model-persistence overhaul. martin-8
  takes that on; this task is the smallest possible fix to make
  Codex visible.
- The banner-by-backend overhaul. martin-10 owns that;
  `displayAgentName` is touched here only because adding one
  case is a one-line fix.

## Done means

Post an update to `tasks/journal.md` (status DONE for martin-11,
plus a one-line log entry).
