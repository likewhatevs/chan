# MCP auto-discovery

chan-server auto-publishes its MCP descriptor into each
external agent's discovery surface on startup. External
agents (Claude Code, Codex, Gemini CLI) launched inside
a chan terminal will see chan's MCP server already wired
in their config — no manual setup required.

The publishing happens once, when the MCP Unix-socket
bridge binds. The descriptor points at the current
`chan serve` process's socket; chan-owned entries get
refreshed in place on each startup so the socket path
follows the live process.

## Per-agent discovery surfaces

### Claude Code

* Config: `~/.claude.json`
* Scope used by chan: **local project scope**
  (`projects["<active drive path>"].mcpServers`)
* User-scope MCP servers (also in `~/.claude.json`)
  are not touched.
* `.mcp.json` in the project root would also work but
  prompts the user for approval at first read; local
  scope avoids the prompt.
* Reference: <https://code.claude.com/docs/en/mcp>

### Codex (CLI + IDE)

* Config: `~/.codex/config.toml`
* Section used by chan: `[mcp_servers.chan]` (or per
  the chan-published name)
* Reference: <https://developers.openai.com/learn/docs-mcp>

### Gemini CLI

* Config: `~/.gemini/settings.json` (user scope)
* Section used by chan: top-level `mcpServers.chan`
* Project-scope `.gemini/settings.json` is supported by
  Gemini itself; chan uses user scope so the entry
  follows the user across projects.
* Reference: <https://github.com/google-gemini/gemini-cli/blob/main/docs/tools/mcp-server.md>

## Coexistence rules

* **User-owned entries are never overwritten.** chan
  identifies its own entries by the descriptor shape
  (`args[0] == "__mcp-proxy"`) and refreshes only those.
* **Same-name user-owned entries are left untouched**
  and logged as a warning. If you have a custom server
  named `chan`, chan-server won't clobber it — but it
  also won't be able to publish itself under that name.
  Rename one or the other.
* **Non-chan entries are preserved on every refresh.**
  The publish step reads, merges, and writes back; it's
  not a destructive overwrite.

## Descriptor shape

chan publishes the following descriptor (with per-agent
formatting). The command is the chan binary itself
running a small bridge subcommand:

```
chan __mcp-proxy <unix-socket-path>
```

The socket path is the active `chan serve` process's
MCP bridge socket. chan-server rebinds it on each
startup; the publish step writes the current value so
agents can connect immediately.

## What this enables

* Spawn an agent via `systacean-12`'s HTTP control
  channel (see [spawn-protocol.md](./spawn-protocol.md)),
  give it a name + CLI invocation, and the spawned
  process finds chan's MCP server in its config
  without any user-side setup step.
* Build orchestration flows that compose agents
  (router + workers) where every worker can reach
  chan's filesystem, notes, search, graph, etc. via
  MCP.

## Out of scope (today)

* Removing the chan entry on chan-server shutdown.
  Better to leave it (it'll fail at connect-time when
  chan-server is down) than risk corrupting the user's
  config on a crash.
* Windows / Linux config-path conventions beyond what
  each agent's docs specify.
* Auto-discovery of agents we don't know yet. Add a
  per-agent shim in `crates/chan-server/src/...` (see
  `systacean-14`) when needed.
