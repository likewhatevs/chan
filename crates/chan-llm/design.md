# chan-llm: design reference

Canonical design notes for the `chan-llm` crate. Update this file
in the same commit as any change that affects the public API or
the FFI shape.

## 1. Problem and scope

chan needs one assistant layer that survives across two consumers
that look very different from outside:

- `chan-server` (in `chan-writer/chan`) wraps `LlmSession` in axum
  routes and forwards streaming events to the web frontend over
  WebSocket.
- Native shells (iOS / Android, future) link this crate via uniffi
  alongside `chan-drive` and receive events through callback
  objects implemented in Swift / Kotlin.

A single crate keeps the system prompt, the tool schemas, the
edit-control rules, and the auto-apply policy in one place; both
consumers move in lockstep when any of those bump. Native shells
avoid pulling axum, tower, and tokio's HTTP stack and never have
to reimplement the assistant logic in Swift or Kotlin.

In scope:

- Backends (HTTP and subprocess) with a uniform `Backend` trait.
- Embedded system prompt + tool descriptions.
- The four built-in tools (`read_file`, `write_file`,
  `list_files`, `search_content`), all routed through
  `chan_drive::Drive`.
- API key resolution (env, keychain, file fallback).
- `LlmSession` orchestration loop (assistant turn, tool round-
  trip, pending-write resume, cancel).
- Optional stdio MCP server (`feature = "mcp"`) for external MCP
  clients and for v2 ClaudeCli / GeminiCli wiring.

Out of scope:

- HTTP / WebSocket transport for chan's web UI (lives in
  `chan-server`).
- Conversation history persistence (the consumer owns the
  transcript and passes it on every `send`).
- Any direct filesystem access. All file I/O goes through
  `chan-drive`.

`chan-llm` depends on `chan-drive`. It does NOT depend on
`chan-server` or `chan`. That inversion (the LLM layer is "lower"
than the HTTP layer, even though the HTTP layer is the more
visible consumer today) is the point.

## 2. Architecture overview

```
                        +---------------------+
   host transcript ---> |     LlmSession      |
                        |  (Arc, sync facade) |
                        +----------+----------+
                                   | spawns onto internal
                                   | tokio runtime
                                   v
                        +---------------------+      +----------+
                        |  Backend trait impl | ---> | upstream |
                        |  Anthropic / Gemini |      |  HTTP /  |
                        |  Ollama / ClaudeCli |      | subproc  |
                        |  / GeminiCli        |      +----------+
                        +----------+----------+
                                   | tool_call events
                                   v
                        +---------------------+
                        |   tools::execute    |
                        |  StandardTool       |
                        +----------+----------+
                                   | every op
                                   v
                        +---------------------+
                        |  chan_drive::Drive  |
                        |  path sandbox,      |
                        |  atomic writes,     |
                        |  editable-text gate |
                        +---------------------+

                             ^
                             |
   SessionListener  <--------+   on_delta / on_tool_call /
   (Send + Sync trait)           on_tool_result / on_done /
                                 on_error
```

The optional MCP server in `mcp.rs` is a second entry into the
same `tools::execute` foundation, this time over rmcp's stdio
transport. It exists for one reason: external agentic CLIs
(`claude`, `gemini`) bring their own tool loop, and if we let
that loop call the CLI's native filesystem tools it would touch
the user's notes directly, bypassing the path sandbox, the
editable-text gate, atomic writes, and the `auto_apply_writes`
confirmation contract. The MCP server re-projects chan-llm's
tools (`read_file`, `write_file`, `list_files`,
`search_content`) over JSON-RPC on stdio so the CLI's tool loop
can be allowlisted onto chan-llm's tools and only chan-llm's
tools, while still routing every operation through
`tools::execute` and `chan_drive::Drive`. The flow:

```
   user message
        |
        v
   +----------------+      stdin/stdout JSON-RPC      +---------+
   | claude / gemini| <----------------------------> |  MCP    |
   |  CLI subproc   |                                |  server |
   +----------------+                                +----+----+
                                                          |
                                                          v
                                                  +---------------+
                                                  | tools::execute|
                                                  +-------+-------+
                                                          |
                                                          v
                                                  +---------------+
                                                  |   Drive       |
                                                  | (sandbox,     |
                                                  |  atomic, gate)|
                                                  +---------------+
```

`auto_apply_writes` is honoured on the MCP path the same way it
is on the in-process path: the MCP server takes the flag at
construction (`Server::new(drive, auto_apply_writes)`). The v2
ClaudeCli / GeminiCli wiring spawns the MCP subprocess with the
host-binary `__mcp` subcommand and passes `--auto-apply` only
when the user has opted in. When it's off, `write_file` returns
a deferred error back to the CLI's tool loop (the host-approval
side-channel for resuming the CLI mid-call after the user
confirms is tracked as chan-llm issue #1). Either way, the
chan-drive gates apply: the CLI subprocess can only see what
chan-llm's tools expose, never the underlying filesystem
directly. The `chan-llm-mcp` standalone binary and chan's
hidden `__mcp` subcommand both run the same server with the
same dispatch.

## 3. Components

`LlmConfig` (`config.rs`)

- Persisted at `chan_drive::paths::config_dir().join("llm.toml")`.
  On desktop that resolves to `~/.chan/llm.toml`, co-located with
  chan-drive's registry; iOS / Android sandboxes pass an explicit
  path through `load_from` / `save_to`.
- File mode 0600 on Unix, written via an atomic-rename helper
  that mirrors `chan_drive::fs_ops::atomic_write` (tempfile in
  the same dir, fsync, rename, fsync parent).
- Fields: `backend`, `models`, `urls`, `max_tokens`,
  `auto_apply_writes`, `mcp_image_max_bytes`, `keys`, `claude_cli`,
  `gemini_cli`. Empty sub-tables and `None` scalars are skipped on
  serialization so a fresh install doesn't grow noise.

Key resolver (`keys.rs`)

- Three tiers: env -> keychain -> file fallback.
- Service name `chan` in the OS keychain (macOS Keychain,
  Windows Credential Manager, Linux Secret Service / kwallet).
- `resolve(kind, &config) -> (Option<String>, KeyStatus)`.
- `keychain_lookup(kind)` is a public probe for hosts that need
  to verify a write actually landed (macOS Security.framework
  silently no-ops on unsigned dev binaries).

Prompts (`prompts.rs`)

- `SYSTEM_PROMPT` and `SYSTEM_PROMPT_NO_TOOLS` (the latter for
  Ollama models that don't support tool calling).
- Per-tool descriptions (`READ_FILE_DESC`, `WRITE_FILE_DESC`,
  `LIST_FILES_DESC`, `SEARCH_CONTENT_DESC`, `READ_IMAGE_DESC`)
  referenced from the tool schema and re-exposed verbatim in the
  MCP server. `READ_IMAGE_DESC` is MCP-only: the in-process
  backends don't have a multimodal-content slot today, so it
  isn't surfaced via `tools::standard_tool_schemas()`.

Tool sandbox (`tools.rs`)

- `StandardTool` enum, `ToolContext { drive, auto_apply_writes }`,
  `ToolOutcome { Ok(Json) | Pending { tool, args } }`.
- `tools::execute(name, args, &ctx)`. Soft caps:
  `READ_FILE_CAP_BYTES = 256 KiB`, `LIST_FILES_CAP_ENTRIES = 2000`,
  `SEARCH_CONTENT_MAX_LIMIT = 100`, default limit 20.
- `standard_tool_schemas()` returns OpenAI-shaped
  `{name, description, parameters}` objects backends translate
  to their wire format.

Session (`session.rs`)

- `LlmSession::new(drive, config)`, `send(history, listener) ->
  CancelHandle`, `approve_pending(history, call_id) ->
  Vec<Message>`.
- Stateless on the crate's side: the host passes the full
  transcript on each `send` call. Pending-write placeholders use
  `PENDING_STATUS` / `REJECTED_STATUS` / `FAILED_STATUS` strings
  in the Tool message body; `apply_resume` swaps the placeholder
  for a typed result.
- Stop reasons: `EndOfTurn`, `MaxTokens`, `StopSequence`,
  `ToolUse`, `Error`, `Cancelled`.

MCP server (`mcp.rs`, `feature = "mcp"`)

- rmcp-based stdio transport. `Server::new(drive,
  auto_apply_writes).with_max_image_bytes(n).serve_stdio().await`.
- Five tools: `read_file`, `write_file`, `list_files`,
  `search_content` (text, via `tools::execute`) and `read_image`
  (binary, via `Drive::read` + base64 + rmcp `Content::image`).
  chan-drive's path sandbox and regular-file gate apply to all
  five; the editable-text gate fires for the text tools, the
  image-extension allowlist (`is_supported_image`: png/jpg/jpeg/
  webp/gif) fires for `read_image`.
- `read_image` is capped per call. The cap defaults to
  `DEFAULT_MCP_IMAGE_MAX_BYTES` (10 MiB) and is overridable via
  `Server::with_max_image_bytes`. Embedded callers (chan-server,
  the `__mcp` subcommand) thread it from
  `LlmConfig::mcp_image_max_bytes`; the standalone binary takes
  `--max-image-bytes <N>`.
- Standalone binary `chan-llm-mcp` builds when the feature is
  on; in chan's CLI the same code path runs in-process via the
  hidden `__mcp` subcommand.

Backends (`backends/`)

- One module per provider plus `retry.rs` for shared retry policy.
- `Backend` trait is async + `Send + Sync`. `run` translates one
  HTTP / subprocess exchange and returns an `Outcome` so the
  session-level loop can decide whether to dispatch tool calls.
- Backends never emit `on_tool_call`, `on_tool_result`, or
  `on_done` themselves; that's the orchestration loop's concern.

## 4. Public API surface

The crate's headline types, all sync, all FFI-shaped:

```text
LlmConfig            { backend, models, urls, max_tokens,
                       auto_apply_writes, mcp_image_max_bytes,
                       keys, claude_cli, gemini_cli }
                     load() / save()
                     load_from(&Path) / save_to(&Path)

MaxTokens, Models, Urls, Keys, ClaudeCli, GeminiCli
                     each ::is_empty() and ::for_backend(kind)
                     where applicable.

BackendKind          Anthropic | Gemini | Ollama
                     | ClaudeCli | GeminiCli

KeyStatus            Env | Keychain | File | Missing
keys::resolve(kind, &config) -> (Option<String>, KeyStatus)
keys::status(kind, &config)  -> KeyStatus
keys::set(kind, key)         -> Result<()>
keys::clear(kind)            -> Result<()>
keys::keychain_lookup(kind)  -> Option<String>

StandardTool         ReadFile | WriteFile | ListFiles
                     | SearchContent
ToolContext          { drive: Arc<Drive>, auto_apply_writes: bool }
ToolOutcome          Ok(Json) | Pending { tool, args }
tools::execute(name, args, &ctx) -> Result<ToolOutcome>
tools::standard_tool_schemas()   -> Vec<ToolSchema>

LlmSession::new(drive, config) -> Self
LlmSession::backend()          -> Option<BackendKind>
LlmSession::send(history, listener) -> CancelHandle
LlmSession::approve_pending(history, call_id) -> Result<Vec<Message>>

CancelHandle         cancel(), is_cancelled()
Role                 System | User | Assistant | Tool
Message              { role, content, tool_call_id, tool_calls }
Delta                { text }
ToolCall             { id, name, args }
ToolResult           { id, output }
ResumeOutcome        Applied(Json) | Rejected { reason }
                     | Failed { error }
StopReason           EndOfTurn | MaxTokens | StopSequence
                     | ToolUse | Error | Cancelled

LlmError             single tagged enum, primitive payloads.
                     Typed passthroughs from chan-drive:
                     WriteConflict { current_mtime_ns },
                     WriteTooLarge { kind, size, limit },
                     ListingTooLarge { observed, limit },
                     PathRefused(String). Plus
                     NotImplemented, BackendNotConfigured,
                     MissingApiKey, ConfigDecode/Encode, Http,
                     BackendError { status, message }, Tool, Core,
                     Io, Keychain, Mcp, Resume.

PENDING_STATUS, REJECTED_STATUS, FAILED_STATUS
is_pending_placeholder(&Message) -> bool
apply_resume(history, call_id, outcome) -> Result<Vec<Message>>
```

MCP module (`feature = "mcp"`):

```text
mcp::Server::new(drive, auto_apply_writes) -> Server
mcp::Server::with_max_image_bytes(n: u64)  -> Server
mcp::Server::serve_stdio().await           -> Result<()>
mcp::DEFAULT_MCP_IMAGE_MAX_BYTES           -> u64 (10 MiB)
mcp::is_supported_image(rel: &str)         -> Option<&'static str>
```

## 5. Invariants and trust boundaries

Key resolution is policy, not opinion. Env beats keychain beats
file: per-shell overrides keep working over SSH and inside CI;
the keychain is the desktop default; the file is the headless-
server fallback for boxes without a session bus. Writes only ever
go to the keychain; the file fallback (`LlmConfig.keys`) is read-
only from chan-llm's perspective. A user-managed TOML stays user-
managed.

`auto_apply_writes` is the user's contract. When false,
`write_file` returns `ToolOutcome::Pending` and never touches
disk. Never silently flip it to true. The host shows confirmation
UI for `Pending`, then either calls `write_file` again with the
flag effectively on for that single call (via the orchestrator's
approve-pending path) or records a rejection through
`apply_resume`.

`llm.toml` mode 0600 on Unix is the trust boundary for fields
that change subprocess behavior:

- `[claude_cli] cmd`: full path or PATH-resolved binary used to
  spawn the agentic CLI. A user-edited entry here can replace
  `claude` with any other binary.
- `[claude_cli] extra_args`: appended verbatim after chan-llm's
  own claude flags. A maliciously edited entry can pass
  `--mcp-config /tmp/evil.json` and override our own
  `--mcp-config` because cli arg-parsers take last-wins. We
  accept this: the user owns `llm.toml`, and 0600 keeps other
  local accounts from editing it. On Windows there is no
  equivalent gate; users on shared machines should treat
  `llm.toml` as a secret.
- `[gemini_cli] cmd` / `extra_args`: same shape, same trust
  story. gemini-cli has no per-invocation `--mcp-config` flag,
  so v2 mode rewrites `GEMINI_CLI_HOME` to a tmpdir we own and
  lays out a synthetic `~/.gemini/` (`settings.json` +
  `policies/chan.toml`). A user-edited `extra_args` cannot
  override settings the way it can override a CLI flag, but it
  can still pass other gemini flags that alter behavior.
  Redirecting `GEMINI_CLI_HOME` blocks gemini from reading the
  user's real `~/.gemini` auth, so v2 forwards the chan-llm-
  resolved Gemini API key via `GEMINI_API_KEY` on the subprocess
  env. v2 launches with no chan-llm-stored key surface an auth
  error from gemini.
- `mcp_command` is `serde(skip)` on both `[claude_cli]` and
  `[gemini_cli]`: a malicious TOML cannot set them. Hosts inject
  programmatically, so they are part of the host binary's trust
  profile, not the config file's.

## 6. Streaming model

`LlmSession::send` is fire-and-forget. The session owns an
internal tokio runtime, spawns the orchestration loop onto it,
and dispatches into the host-supplied `Arc<dyn SessionListener>`
as deltas, tool calls, tool results, and the final stop reason
arrive. The returned `CancelHandle` is a cheap clone of an
`Arc<AtomicBool>`; flipping it stops the in-flight session at
the next checkpoint (between SSE / NDJSON chunks, between tool
iterations, between subprocess reads).

Why not return `impl Stream` or a `tokio::sync::mpsc::Receiver`:
uniffi can't carry either across an FFI boundary cleanly. A
trait object with sync callbacks works identically in Swift,
Kotlin, and Rust, and matches what `chan_drive::Drive::watch`
already does, for the same reason.

There is a per-turn cap on accumulated assistant text
(`ASSISTANT_TEXT_CAP_BYTES = 10 MB`). Backends abort the stream
when they cross it; silently truncating would corrupt the
transcript fed back to the model on the next turn.

## 7. Tool sandbox details

Four built-in tools dispatched by name. Every tool routes through
`chan_drive::Drive`, so the filesystem invariants apply
automatically: path sandbox (no `..` escapes, no mid-path symlinks
out of the drive), special-file refusal (no FIFOs, sockets,
devices), atomic writes, the `.md` / `.txt` editable-text gate.

```text
read_file(path)         -> { path, content, size, mtime_ns?,
                             truncated?, note? }
write_file(path,        -> { path, bytes_written }   (auto_apply on)
           content,        Pending { tool, args }    (auto_apply off)
           expected_mtime_ns?)
list_files(prefix?)     -> { entries, count, total,
                             truncated?, note? }
search_content(query,   -> SearchResults
              limit?)
```

`expected_mtime_ns` is the optimistic-concurrency token. The
assistant gets `mtime_ns` back from `read_file`; passing it to
`write_file` makes the write a compare-and-swap against the
file's current mtime. On conflict chan-drive returns
`WriteConflict`, which surfaces as `LlmError::WriteConflict`
with the typed payload so hosts can render a "file changed,
re-read" prompt without string-matching.

Adding a new built-in tool:

1. Add a variant to `StandardTool`.
2. Wire `name()` and `from_name()`.
3. Add an `exec_*` arm in `tools::execute`.
4. Add a `<NAME>_DESC` constant in `prompts.rs`.
5. Add a `ToolSchema` entry in `standard_tool_schemas()`.
6. If MCP exposure is wanted, add a `#[tool]` handler in
   `mcp.rs` that routes through the same `tools::execute` call.

## 8. Backends

```
Backend     Notes
----------  ----------------------------------------------------
Anthropic   SSE streaming, tool round-trips. Default model
            `claude-opus-4-7`, default max_tokens 4096.
Gemini      Function-calling format, server-side tool exec.
            Default model `gemini-2.5-pro`.
Ollama      Local server, no key, custom function-calling shape
            (no native tool-use; uses SYSTEM_PROMPT_NO_TOOLS for
            models without tool support). URL precedence:
            `OLLAMA_HOST` env > `urls.ollama` > localhost:11434.
ClaudeCli   Drives a local `claude` CLI subprocess. v1 runs
            claude as a black-box agent against the drive root.
            v2 (host-injected `mcp_command`) writes a temp
            `--mcp-config` pointing at chan-llm's MCP server,
            allowlists chan-llm's tools plus claude's read-only
            tools, and drops `--permission-mode bypassPermissions`,
            so writes still stage through `auto_apply_writes`.
GeminiCli   Drives a local `gemini` CLI subprocess. Same v1/v2
            split as ClaudeCli. v2 rewrites `GEMINI_CLI_HOME` to
            a tmpdir we own (gemini-cli has no per-invocation
            `--mcp-config`), lays out a synthetic `~/.gemini/`
            (`settings.json` advertising chan-llm's MCP server,
            `policies/chan.toml` deny-policy for native edit /
            shell tools), and passes
            `--allowed-mcp-server-names chan`. Forwards the
            chan-llm-resolved Gemini key via `GEMINI_API_KEY`
            since redirecting the home dir blocks gemini from
            reading the user's real `~/.gemini` auth.
```

Each backend:

- Builds wire-format requests (system prompt + history + tools +
  user message).
- Drives the streaming response, translating chunks into
  chan-llm's `Delta` events plus the in-progress assistant
  text buffer.
- Maps vendor stop reasons into `StopReason::{EndOfTurn,
  MaxTokens, StopSequence, ToolUse, Error, Cancelled}`.
- Does NOT touch the filesystem. Tool execution is the
  orchestration loop's job; tool results come back as
  `on_tool_result` callbacks on the listener.

The agentic CLI backends are the deliberate exception: the CLI's
own tool loop runs against the drive directly (v1) or against
chan-llm's MCP server (v2). The session-level loop sees empty
`tool_calls` from these backends since the CLI has already
executed them.

Per-call `max_tokens` resolves user override > backend default
(`config.max_tokens.for_backend(kind)`). ClaudeCli and GeminiCli
omit this knob: the CLIs pick their own ceilings.

## 9. On-disk layout

```
~/.chan/llm.toml      mode 0600 on Unix. Backend selection,
                      model overrides, URL overrides,
                      auto_apply_writes flag, on-disk key
                      fallback, [claude_cli] / [gemini_cli]
                      subprocess settings.
```

Path resolution routes through `chan_drive::paths::config_dir`,
so chan-llm's config sits beside chan-drive's registry
(`~/.chan/config.toml`) on every desktop platform. iOS and
Android callers pass an explicit path via `load_from` /
`save_to` since their sandbox dir isn't the same as desktop.

The chan-drive registry at `~/.chan/config.toml` has a separate
purpose; chan-llm doesn't read or write it.

Key fallback in `[keys]` is the headless-server escape hatch.
Env and keychain take precedence; the file is read-only from
chan-llm's perspective and only the user can populate it.

## 10. Consumers

Today:

- `chan-server` (in `chan-writer/chan`) wraps `LlmSession` in
  axum routes. It implements `SessionListener` to forward
  `Delta` / `ToolCall` / `ToolResult` / `StopReason` events as
  WebSocket frames to the web frontend, owns the conversation
  transcript, and surfaces the auto-apply confirmation UI.
  `chan-server` also calls `keys::resolve`, `keys::set`,
  `keys::clear`, and `keys::keychain_lookup` to back the
  settings-page key flows, and uses
  `backends::anthropic::list_models` / `gemini::list_models`
  for the model picker.
- `chan` (the CLI in `chan-writer/chan`) depends on `chan-llm`
  with the `mcp` feature so its hidden `__mcp` subcommand can
  spin up `chan_llm::mcp::Server` in-process. That is the
  binary chan-llm's v2 ClaudeCli / GeminiCli wiring spawns as a
  subprocess via `mcp_command`, so the chan binary is both the
  user-facing CLI and the MCP bridge an external agentic CLI
  talks to. Pulling it in directly (instead of relying on a
  separate `chan-llm-mcp` companion) keeps chan a single static
  binary at the cost of `rmcp + schemars` in the dependency
  graph.

Future:

- Native iOS / Android shells linked through uniffi alongside
  `chan-drive`. They construct `LlmSession` directly, implement
  `SessionListener` in Swift / Kotlin, and receive streaming
  events without a network hop. The public API is already shaped
  for this: no lifetimes, owned `String` / `PathBuf`, `Arc`-able
  handles, callback-based streaming, primitive-payload errors.

## 11. FFI plan

uniffi bindings ship in a follow-up. The public surface is
shaped to make that mechanical:

- No lifetimes on public types.
- Owned `String` / `PathBuf` everywhere.
- Handles are `Arc<Self>`-able (`LlmSession`, `Drive`).
- Errors flatten into the `LlmError` tagged enum with primitive
  payloads. Backend / chan-drive errors are mapped through
  `From` impls; nothing non-uniffi leaks.
- Streaming via `Arc<dyn SessionListener>` with sync methods
  (uniffi callback objects on the foreign side).
- No public `async fn`. The tokio runtime stays inside
  `LlmSession`. The MCP `serve_stdio` is async because its only
  caller is the `chan-llm-mcp` binary's own tokio main; native
  shells don't link the MCP module.

## 12. What's NOT here yet

- uniffi bindings. The crate is shaped for them; bindings land
  when the first native shell does.
- Conversation history schema. The session is stateless; consumers
  persist whatever transcript shape they want however they want.
- MCP `resources/` (browse-style discovery). Tracked separately.
  Image reads are covered by the `read_image` tool; resources
  would add a list / read surface scoped to the drive's media
  directory (the model picks files without first calling
  `list_files`).
