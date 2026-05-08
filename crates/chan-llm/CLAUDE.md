# CLAUDE.md

Contribution guidelines for Claude Code (claude.ai/code) when
working on `chan-llm`.

## What This Project Is

`chan-llm` is the cross-platform LLM layer for chan: backends
(Anthropic, Gemini, Ollama, Claude CLI), embedded prompts, the
tool sandbox the assistant uses to read and edit chan drives,
and the API-key resolution policy. Two consumer shapes:
`chan-server` over HTTP, and native shells (iOS / Android) via
uniffi.

It lives as a workspace member alongside chan-drive in the
chan-core repo so the LLM layer and the filesystem primitives it
sandboxes through can move in lockstep. chan-server gets an
axum-shaped wrapper; a native shell gets a uniffi-shaped binding.
Both link the same prompts, tools, and edit-control rules.

Build, test, toolchain, and pre-push hook setup are documented
at the workspace root (`../../CLAUDE.md`).

## Project Principles

### One place for assistant behavior

The system prompt, the tool schemas, the edit-control rules, and
the auto-apply policy all live here. Bumping any of them changes
every consumer in lockstep. Don't fork "the prompts the web app
uses" vs "the prompts iOS uses".

### Tools route through chan-drive

Every tool (read_file, write_file, list_files, search_content)
calls into `chan_drive::Drive`. The filesystem invariants (path
sandbox, special-file refusal, atomic writes, editable-text gate)
apply automatically; there is no escape hatch. A backend cannot
invent a tool that bypasses these gates.

### auto_apply_writes is the user's contract

When `LlmConfig.auto_apply_writes` is false, `write_file` returns
`Pending` instead of writing. The host (chan-server, native shell)
shows a confirmation UI and re-issues with the user's approval.
When true, writes go straight to disk. Hard line: never silently
flip from false to true; never write to disk in the false branch.

### Keys: env -> keychain -> file

Three-tier resolution. Writes only go to the OS keychain. The file
fallback (`LlmConfig.keys`) is read-only from chan-llm's
perspective: a user-managed TOML stays user-managed. Env wins so
per-shell overrides keep working over SSH.

### FFI-shaped from day one

Public types: no lifetimes, owned strings only, `Arc`-able handles.
`LlmSession::send` is callback-based via `SessionListener`; async
stays internal so uniffi doesn't have to negotiate a runtime
across the boundary. New public APIs follow the same constraints;
if a method is hard to express through uniffi, that's a signal to
restructure rather than punt.

## Writing Rules

- **No em dashes** in comments or documentation.
- **Tables**: pure ASCII, target 80 columns.
- **Factual**: no marketing language.
- **Comments**: explain WHY, not WHAT.

## Contributor Patterns

- **Backends never touch chan-drive directly**: a backend only
  builds wire-format requests and parses streaming responses.
  Anything filesystem goes through the tool sandbox.
- **Streaming is callback-based**: backends emit
  `on_delta` / `on_tool_call` / `on_tool_result` / `on_done` /
  `on_error` from the runtime's worker thread. Don't return
  `impl Stream` or `tokio::sync::mpsc::Receiver` from public
  methods; that breaks the FFI shape.
- **Errors are uniffi-friendly**: every `LlmError` variant
  carries primitives only. Don't store `reqwest::Error` or
  `chan_drive::ChanError` directly; flatten via `Display`.
- **Tests use a `Collector` listener**: `Vec<Event>` collector
  pattern, see `session::tests`. Don't reach for tokio test
  utilities until the runtime actually does work.

## Documentation

- **Design**: [`design.md`](design.md). Crate layout, public API
  shape, FFI plan, prompt sourcing.
- **chan-drive contract**: `../chan-drive/design.md` (sibling
  crate in the same workspace).
- **chan repo contract**: `../../../chan/design.md` (sibling
  checkout).
- **Issue tracker**: GitHub `chan-writer/chan-core` (chan-llm's
  former repo at `chan-writer/chan-llm` is archived).
