# chan-llm: design

`chan-llm` is the cross-platform LLM layer for chan. It owns the
backends, the prompts, the tool sandbox, and the API-key
resolution policy. This document is the canonical design
reference; update it in the same commit as any change that
affects the public API or the FFI shape.

## Why a separate crate (and a separate repo)

Two consumers, neither contains the other:

- `chan-server` (in `chan-writer/chan`) wraps `LlmSession` in axum
  routes and forwards events over WebSocket to the web frontend.
- Native shells (iOS / Android, future) link this crate via uniffi
  alongside `chan-drive` and receive events through callback
  objects implemented in Swift / Kotlin.

If chan-llm lived inside `chan-writer/chan`, native shells would
either drag in axum / tower / tokio's HTTP stack to consume the
LLM logic, or reimplement it in their native language. Both are
worse than a small extra repo.

`chan-llm` depends on `chan-drive` (for `Drive` and `SearchOpts`).
It does NOT depend on `chan-server` or `chan`. That's the
inversion: the LLM layer is "lower" than the HTTP layer, even
though the HTTP layer is the more visible consumer today.

## Crate layout

```
src/
  lib.rs           public façade, re-exports
  error.rs         LlmError + Result<T>
  config.rs        LlmConfig + load/save TOML
  keys.rs          env -> keychain -> file resolver
  prompts.rs       SYSTEM_PROMPT + tool descriptions
  tools.rs         StandardTool + ToolContext + execute
  session.rs       LlmSession + SessionListener + types
  backends/
    mod.rs         BackendKind + default models
    anthropic.rs   stub
    gemini.rs      stub
    ollama.rs      stub
```

## Public API

```text
LlmConfig            { backend, models, auto_apply_writes, keys }
                     load() / save() at <config>/chan/llm.toml,
                     mode 0600 on Unix.

KeyStatus            Env | Keychain | File | Missing.
keys::resolve(kind, &config) -> (Option<String>, KeyStatus)
keys::set(kind, key)        -> writes to OS keychain only
keys::clear(kind)           -> drops from keychain only

StandardTool         ReadFile | WriteFile | ListFiles |
                     SearchContent.
ToolContext          { drive: Arc<Drive>, auto_apply_writes: bool }
ToolOutcome          Ok(json) | Pending { tool, args }
tools::execute(name, args, &ctx) -> Result<ToolOutcome>

LlmSession::new(drive, config)
LlmSession::send(message, Arc<dyn SessionListener>)

trait SessionListener: Send + Sync {
    on_delta(Delta)
    on_tool_call(ToolCall)
    on_tool_result(ToolResult)
    on_done(StopReason)
    on_error(String)
}
```

## On-disk layout

```
<config>/chan/llm.toml      mode 0600. Backend selection, model
                            overrides, auto_apply flag, on-disk
                            key fallback.
```

`<config>` follows `dirs::config_dir`:

  - macOS: `~/Library/Application Support`
  - Linux: `$XDG_CONFIG_HOME` or `~/.config`
  - Windows: `%APPDATA%`

iOS and Android callers pass an explicit path via `load_from` /
`save_to` since their sandbox dir isn't `dirs::config_dir`.

The chan-drive registry at `~/.chan/config.toml` is a separate
file with a separate purpose; chan-llm doesn't read or write it.

## Tool sandbox

Four built-in tools dispatched by name. Every tool routes through
`chan_drive::Drive`, so the filesystem invariants apply
automatically: path sandbox (no `..` escapes, no mid-path symlinks
out of the drive), special-file refusal (no FIFOs, sockets,
devices), atomic writes, the `.md` / `.txt` editable-text gate.

```text
read_file(path)         -> { path, content }
write_file(path,        -> { path, bytes_written }   (auto_apply on)
           content)        Pending { tool, args }    (auto_apply off)
list_files()            -> [{ path, is_dir, mtime, size }, ...]
search_content(query,   -> SearchResults
              limit?)
```

Adding a new built-in tool:
  1. Add a variant to `StandardTool`.
  2. Wire `name()` and `from_name()`.
  3. Add an `exec_*` arm in `tools::execute`.
  4. Add a `<NAME>_DESC` constant in `prompts.rs`.

## Streaming model

`LlmSession::send` is fire-and-forget. The session spawns work onto
its internal tokio runtime (when real backends land); deltas, tool
calls, and the stop reason fire into the listener as they arrive.

Why not return `impl Stream` or a channel: uniffi can't cross those
boundaries without a costly bridge layer. Callback objects work
identically in Swift, Kotlin, and Rust. Same pattern as
`chan_drive::Drive::watch`.

## Prompt sourcing

Initial commit uses placeholder prompt constants. The real prompts
port from `fiorix/chan/crates/chan-core/src/llm/` once the public
API contract here stabilizes (so prompt iteration doesn't churn
the surface other repos depend on). Prompts are the highest-
leverage shared thing in this crate; bumping any of them changes
the assistant's behavior across web, CLI, and future native shells
in a single commit.

## Backends (planned port)

Three backend modules ship as stubs in this commit. The real ports
follow:

| Backend | Source | Notes |
|---------|--------|-------|
| Anthropic | `claude.rs` (321 LOC) | SSE streaming, tool use round-trips |
| Gemini | `gemini.rs` (568 LOC) | server-side tool exec, function-calling format |
| Ollama | `ollama.rs` (373 LOC) | local server, no key, custom function-calling |

Each backend will:

- Build wire-format requests (system prompt + history + tools +
  user message).
- Drive the streaming response, translating chunks into chan-llm's
  `Delta` / `ToolCall` events.
- Map vendor stop reasons into `StopReason::{EndOfTurn, MaxTokens,
  StopSequence, ToolUse, Error}`.
- NOT touch the filesystem. Tool execution is the host's job (via
  `tools::execute`), and tool results come back as
  `on_tool_result` callbacks.

## FFI plan

uniffi bindings ship in a follow-up. The public API is shaped to
make that mechanical:

- No lifetimes on public types.
- Owned `String` / `PathBuf` everywhere.
- Handles are `Arc<Self>`-able.
- Errors are a single tagged enum with primitive payloads.
- Streaming via `Arc<dyn SessionListener>` (uniffi callback objects
  on the foreign side).
- No public `async fn`; async stays inside the runtime.

## What's NOT here yet

- Real backend implementations (stubs only).
- The internal tokio runtime that drives backend HTTP. The `tokio`
  dep is in Cargo.toml so callers don't get surprised by a feature
  flag flip later, but no runtime is constructed today.
- uniffi bindings. Crate is shaped for them; bindings produce when
  the first native shell lands.
- CI workflow file. Cross-repo auth between two private repos
  (chan-llm depending on chan-drive via path) was the open issue;
  resolved when chan-llm was folded into the chan-core workspace.
  The workspace-level CI now covers this crate; the pre-push hook
  mirrors it locally.
- An assistant chat history schema. The session is stateless on
  this crate's side; consumers persist whatever conversation
  state they want however they want.
