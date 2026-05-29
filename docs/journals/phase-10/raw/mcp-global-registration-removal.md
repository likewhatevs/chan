# Stop Global Chan MCP Registration

Date: 2026-05-26.

Scope: `chan-server` startup behavior and MCP discovery documentation.

## Summary

`chan serve` no longer publishes Chan's MCP bridge into global external-agent
configuration files.

Removed behavior:

- No write to `~/.codex/config.toml`.
- No write to `~/.claude.json`.
- No write to `~/.gemini/settings.json`.

Kept behavior:

- The in-process MCP bridge still starts when `chan serve` can bind its Unix
  socket.
- Chan terminal sessions still receive `CHAN_MCP_*` discovery variables.
- `CHAN_MCP_SERVER_JSON`, `CHAN_MCP_SOCKET`, `CHAN_MCP_COMMAND`, and
  `__mcp-proxy` behavior are unchanged.
- Existing stale global config entries are not cleaned up automatically.

## Reason

The old startup path had a durable side effect outside the drive and outside
Chan's scoped terminal environment. Starting a local notes server could update
Codex, Claude, and Gemini user config files.

The supported discovery path is now only the scoped terminal environment that
Chan controls for PTYs launched inside the app.

## Code Changes

- Removed `mod mcp_discovery` from `crates/chan-server/src/lib.rs`.
- Removed the `mcp_discovery::publish_for_agents(...)` call after successful
  MCP bridge startup.
- Deleted `crates/chan-server/src/mcp_discovery.rs`, including its config
  writer helpers and tests.

No `desktop/` files were touched by this change.

## Docs Changes

Updated these docs to stop advertising global MCP config publication:

- `docs/agents/orchestration/mcp-discovery.md`
- `docs/agents/orchestration/README.md`
- `docs/templates/team-process/orchestration/mcp-discovery.md.tpl`
- `docs/templates/team-process/orchestration/README.md.tpl`
- `docs/templates/team-process/README.md`

The new docs state that Chan does not write the external-agent config files and
that `CHAN_MCP_*` variables are the supported discovery contract.

## Verification

Commands run:

```bash
cargo fmt --check
cargo test -p chan-server
git diff --check -- crates/chan-server/src/lib.rs docs/agents/orchestration/README.md docs/agents/orchestration/mcp-discovery.md docs/templates/team-process/README.md docs/templates/team-process/orchestration/README.md.tpl docs/templates/team-process/orchestration/mcp-discovery.md.tpl
```

Results:

- `cargo fmt --check` passed.
- `cargo test -p chan-server` passed: 316 tests.
- `git diff --check` passed for the touched files.

## Open Items

- Manual cleanup remains user-owned for stale entries created by older Chan
  versions, such as `[mcp_servers.chan]` in `~/.codex/config.toml`.
- Concurrent desktop in-process registry work is tracked separately in
  `docs/journals/phase-10/desktop-in-process-registry.md`.
