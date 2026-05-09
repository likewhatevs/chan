# chan-llm

LLM backends, embedded prompts, and the tool sandbox the chan
assistant uses to read and edit chan drives, in one crate. Public
API is FFI-shaped (no lifetimes, owned types, callback-based
streaming) so the same crate backs the chan HTTP server today and
native iOS / Android shells over uniffi later. Filesystem access
routes through `chan-drive`, so the path sandbox, special-file
refusal, atomic writes, and editable-text gate apply to every
tool call.

## Add to your project

```toml
[dependencies]
chan-llm = "0.7"

# Optional: stdio MCP server module + the `chan-llm-mcp` binary.
# Pulls rmcp + schemars; off by default.
chan-llm = { version = "0.7", features = ["mcp"] }
```

## Backends

```
Backend     Status   Notes
----------  -------  -----------------------------------------------
Anthropic   ready    SSE streaming, tool round-trips
Gemini      ready    function-calling, server-side tool exec
Ollama      ready    local server, custom function-calling shape
ClaudeCli   ready    drives a local `claude` subprocess; v2 routes
                     writes through chan-llm's MCP server
GeminiCli   ready    drives a local `gemini` subprocess; v2 rewrites
                     `GEMINI_CLI_HOME` and forwards `GEMINI_API_KEY`
```

## Public API

```text
LlmConfig            backend, models, urls, max_tokens,
                     auto_apply_writes, keys, claude_cli, gemini_cli.
                     load() / save() at chan-drive's config dir.

KeyStatus            Env | Keychain | File | Missing.
keys::resolve(kind, &config) -> (Option<String>, KeyStatus)
keys::set(kind, key)        -> writes to OS keychain only
keys::clear(kind)           -> drops from keychain only

StandardTool         ReadFile | WriteFile | ListFiles | SearchContent.
ToolContext          { drive: Arc<Drive>, auto_apply_writes: bool }
ToolOutcome          Ok(json) | Pending { tool, args }
tools::execute(name, args, &ctx) -> Result<ToolOutcome>

LlmSession::new(drive, config)
LlmSession::send(history, Arc<dyn SessionListener>) -> CancelHandle
LlmSession::approve_pending(history, call_id) -> Vec<Message>

trait SessionListener: Send + Sync {
    fn on_delta(&self, Delta);
    fn on_tool_call(&self, ToolCall);
    fn on_tool_result(&self, ToolResult);
    fn on_done(&self, StopReason);
    fn on_error(&self, String);
}
```

## Build and test

```bash
cargo build -p chan-llm
cargo test  -p chan-llm
cargo build -p chan-llm --features mcp
```

## Design reference

See [`design.md`](design.md) for problem framing, architecture,
invariants, the tool sandbox, per-backend notes, on-disk layout,
the FFI plan, and the current consumer set.

## License

Apache-2.0.
