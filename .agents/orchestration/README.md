# Orchestration with chan

`chan` is the local notes-and-editor host. It also doubles
as an orchestration host: a single-machine surface where
agents (claude, codex, gemini, custom) can be spawned into
named terminal tabs (the Team Work flow) and reach chan's
MCP server through `CHAN_MCP_*` terminal environment
variables.

> Status note: the fsnotify-driven event-coordination
> layer (typed event files -> watcher -> `poke` dispatch ->
> notification bubbles) was REMOVED in the Team Work revamp,
> along with the event-reply / submit-mode endpoints and the
> Spawn-agents dialog. The notification bubble overlay is now
> a frontend-only static stub. Equivalent functionality is
> planned to return in a later phase; the event/watcher
> contracts below are retained as the blueprint for that
> returning implementation.

This SKILL documents the contracts external authors need
to integrate with that surface.

## Quick paths

* "I want my agent to send events to other agents
  routed through chan" → [atomic-writes.md](./atomic-writes.md).
* "I want chan to spawn an agent CLI for me" →
  [spawn-protocol.md](./spawn-protocol.md).
* "I want claude / codex / gemini launched in a chan terminal
  to use chan's MCP server" -> [mcp-discovery.md](./mcp-discovery.md).

## What chan provides

* **Loopback HTTP server** at `127.0.0.1` with bearer-
  token auth (per-launch token printed on stderr; appended
  to the URL when chan launches a browser).
* **Per-terminal-session fsnotify watcher** - REMOVED in
  the Team Work revamp; planned to return. It
  ingested typed event files written into a user-chosen
  directory, dispatched `poke\n` to the matching agent's
  PTY, and surfaced events to the Team Work bubble overlay
  (now a frontend-only static stub).
* **In-process MCP server** exposed over a Unix-domain
  socket; chan-launched terminals get `CHAN_MCP_*` env
  vars to find it.
* **HTTP control channel** for programmatic terminal-tab
  creation, naming, command execution, restart, and
  close.

## What chan does NOT provide

* A networked event bus. Events live on the local
  filesystem; chan watches them.
* Cross-host orchestration. The tunnel relocates the
  HTTP transport, not the agent runtime.

## Contracts in one sentence

* Event files: write atomically (temp + same-dir rename).
* Watcher: reads once on fsnotify; never multi-reads.
* chan-server: never writes back into a directory it
  watches.

Detail in the linked guides.
