# chan-llm design

`chan-llm` is now the MCP-facing tool sandbox for chan drives. It
does not own an in-app chat session, transcript persistence, local
agent subprocess management, or app settings. Those surfaces were
removed in phase 5 with the editor Agent UI.

## Scope

In scope:

  - Shared prompt and tool descriptions for chan drive access.
  - Direct tool dispatch through `tools::execute`.
  - MCP stdio / async-I/O hosting behind the optional `mcp` feature.
  - Image reads for MCP clients, capped by server policy.
  - Typed error passthroughs for chan-drive write conflicts,
    write-size limits, listing limits, and refused paths.

Out of scope:

  - HTTP routes, WebSocket events, and frontend state.
  - API key storage and model/provider configuration.
  - Agent transcript/history storage.
  - Spawning or supervising model CLIs.

## Architecture

```
MCP client / host
        |
        v
chan_llm::mcp::Server  -- feature "mcp"
        |
        v
chan_llm::tools::execute
        |
        v
chan_drive::Drive
```

`mcp::Server` owns a `ToolContext`, which is just an `Arc<Drive>`.
Each tool call goes through `tools::execute` unless it is the MCP-only
`read_image` path. This keeps the same path sandbox, regular-file
checks, editable-text checks, atomic writes, and graph/search access
that the editor uses.

`serve_stdio` is used by the standalone `chan-llm-mcp` binary and by
`chan __mcp`. `serve_io` is used by chan-server's MCP bridge: the
server already holds the drive lock, so it hosts the MCP service
in-process over a Unix-domain socket and lets child processes proxy
stdio to that socket.

## Tools

Text tools are defined as `StandardTool` and dispatched by name:

  - `read_file`
  - `write_file`
  - `list_files`
  - `search_content`
  - `repo_report`
  - `graph_neighbors`
  - `graph_tags`
  - `graph_files_with_tag`

`read_image` is exposed only by the MCP server because it returns an
MCP image content block rather than a JSON text result. Supported
extensions are `.png`, `.jpg`, `.jpeg`, `.webp`, and `.gif`.

Writes are full-file replacements. `write_file` accepts
`expected_mtime_ns` for compare-and-swap semantics and maps
chan-drive conflicts into `LlmError::WriteConflict`.

## Configuration

The library has no model/provider config. MCP image size is server
policy:

  - default: `DEFAULT_MCP_IMAGE_MAX_BYTES` (10 MiB)
  - override: `Server::with_max_image_bytes(bytes)`
  - standalone binary: `--max-image-bytes <N>`

`chan-llm-mcp --config <path>` points at the chan-drive registry
config, not an LLM settings file.

## Error Boundary

`LlmError` is intentionally small:

  - `Tool`
  - `Core`
  - `WriteConflict`
  - `WriteTooLarge`
  - `ListingTooLarge`
  - `PathRefused`
  - `Io`
  - `Mcp`

Public error variants stay matchable for hosts while preserving the
original chan-drive display text for user-facing messages.
