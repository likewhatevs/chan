# chan-llm

Cross-platform LLM layer for chan. Owns the backends, the prompts,
the tool sandbox, and the API-key resolution policy. This file is
the canonical design reference; update it in the same commit as any
change that affects the public API or the FFI shape.

## Why a separate crate

Two consumers, one set of prompts and tool gates:

- `chan-server` (in `chan-writer/chan`) wraps `LlmSession` in axum
  routes and forwards events over WebSocket to the web frontend.
- Native shells (iOS / Android, future) link this crate via uniffi
  alongside `chan-drive` and receive events through callback
  objects implemented in Swift / Kotlin.

A single crate keeps the system prompt, tool schemas, edit-control
rules, and auto-apply policy in one place; both consumers move in
lockstep when any of those bump. Native shells avoid pulling axum /
tower / tokio's HTTP stack and never have to reimplement the
assistant logic in Swift or Kotlin.

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
  mcp.rs           stdio MCP server (feature = "mcp")
  backends/
    mod.rs         BackendKind + default models
    anthropic.rs   SSE streaming + tool round-trips
    gemini.rs      function-calling + server-side tool exec
    ollama.rs      local server, custom function-calling
    claude_cli.rs  drives a local `claude` CLI subprocess
    gemini_cli.rs  drives a local `gemini` CLI subprocess
  bin/
    chan-llm-mcp.rs  MCP server binary (feature = "mcp")
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

The system prompt and the per-tool descriptions live in
`src/prompts.rs`. They are the highest-leverage shared thing in
this crate: bumping any of them changes the assistant's behavior
across web, CLI, and future native shells in a single commit. Host
apps that need a different prompt pass their own; the constants
here are the default a host gets when it doesn't.

## Backends

Four backend modules ship today, all wired through the same
`SessionListener` callback shape:

```
Backend       Notes
------------  ----------------------------------------------------
Anthropic     SSE streaming, tool use round-trips.
Gemini        Function-calling format, server-side tool exec.
Ollama        Local server, no key, custom function-calling shape
              (no native tool-use; uses SYSTEM_PROMPT_NO_TOOLS).
Claude CLI    Drives a local `claude` CLI subprocess; tool
              dispatch routes through chan-llm's MCP server so
              writes still stage through `auto_apply_writes`.
```

Each backend:

- Builds wire-format requests (system prompt + history + tools +
  user message).
- Drives the streaming response, translating chunks into chan-llm's
  `Delta` / `ToolCall` events.
- Maps vendor stop reasons into `StopReason::{EndOfTurn, MaxTokens,
  StopSequence, ToolUse, Error}`.
- Does NOT touch the filesystem. Tool execution is the host's job
  (via `tools::execute`), and tool results come back as
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

## Trust boundaries

A few config fields can elevate or change subprocess behavior;
write access to `llm.toml` is the trust boundary. The file is
created mode 0600 on Unix so only the owner can edit it.

  - `[claude_cli] cmd`: full path or PATH-resolved binary used to
    spawn the agentic CLI. A user-edited entry here can replace
    `claude` with any other binary.
  - `[claude_cli] extra_args`: appended verbatim after chan-llm's
    own claude flags. A maliciously edited entry here can pass
    `--mcp-config /tmp/evil.json` and override our own
    `--mcp-config` because cli arg-parsers take last-wins. We
    accept this: the user owns `llm.toml`, and 0600 keeps other
    local accounts from editing it. On Windows there is no
    equivalent gate; users on shared machines should treat
    `llm.toml` as a secret.
  - `[gemini_cli] cmd` / `extra_args`: same shape as `[claude_cli]`,
    same trust story. Gemini's headless contract differs in two
    places. First, gemini-cli has no per-invocation
    `--mcp-config` flag, so v2 mode rewrites `GEMINI_CLI_HOME` at
    a tmpdir we own and lays out a synthetic `~/.gemini/`
    (`settings.json` + `policies/chan.toml`); a user-edited
    `extra_args` cannot override settings the way it can override
    a CLI flag, but it can still pass other gemini flags that
    alter behavior. Second, redirecting `GEMINI_CLI_HOME` blocks
    gemini from reading the user's real `~/.gemini` auth, so we
    forward the chan-llm-resolved Gemini API key via the
    `GEMINI_API_KEY` env var on the subprocess. v2 launches with
    no chan-llm-stored key surface an auth error from gemini.
  - `mcp_command` is `serde(skip)` on both `[claude_cli]` and
    `[gemini_cli]`: a malicious TOML cannot set them. Hosts
    inject programmatically, so they are part of the host
    binary's trust profile, not the config file's.

## What's NOT here yet

- uniffi bindings. The crate is shaped for them; bindings land when
  the first native shell does.
- An assistant chat history schema. The session is stateless on
  this crate's side; consumers persist whatever conversation
  state they want however they want.
