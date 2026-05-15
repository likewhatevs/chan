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
chan-llm = "0.9"

# Optional: stdio MCP server module + the `chan-llm-mcp` binary.
# Pulls rmcp + schemars; off by default.
chan-llm = { version = "0.9", features = ["mcp"] }
```

## Backends

```
Backend     Status   Notes
----------  -------  -----------------------------------------------
ClaudeCli   ready    drives a local `claude` subprocess; v2 routes
                     writes through chan-llm's MCP server
GeminiCli   ready    drives a local `gemini` subprocess; v2 rewrites
                     `GEMINI_CLI_HOME` and bridges real auth files
CodexCli    ready    drives local `codex exec --json`; v2 injects
                     chan MCP config with per-run `-c` overrides
```

## Public API

```text
LlmConfig            backend, models, auto_apply_writes,
                     mcp_image_max_bytes,
                     claude_cli, gemini_cli, codex_cli.
                     load() / save() at chan-drive's config dir.

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

## Contacts and the graph

chan-drive maintains a sqlite link graph next to every drive
(nodes, edges, headings, tags, contacts). The editor uses it for
the `[[` link picker, the `@` contact picker, chip rendering, and
the graph view. None of that surface is exposed through chan-llm.

This is intentional. A contact file is just a `.md` with YAML
frontmatter (`chan.kind: contact`, plus name / email / phone
fields); the body holds free-form notes. From the agent's
perspective:

  - **"Find Alice"**: `search_content "Alice"` (BM25 over bodies
    and frontmatter).
  - **"What contacts do I have?"**: `list_files` with a prefix
    that matches the contacts directory, then `read_file` on
    interesting hits.
  - **"What links to this contact?"**: `search_content` for the
    filename, or read candidate notes directly.

The model parses YAML frontmatter from `read_file` output without
needing a typed surface. Adding `list_contacts`, `backlinks`, or
`neighbors` MCP tools would buy efficiency (avoid the
list+read+search loop on large drives) but not new capability;
that's tracked as
[chan-writer/chan-core#3](https://github.com/chan-writer/chan-core/issues/3),
not a v1 omission.

The frontend reading the same files renders chips and the @ picker
straight from chan-drive's `GraphView::contacts_filtered`, but
nothing about that path crosses chan-llm.

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
