# Orchestration with chan

`chan` is the local notes-and-editor host. It also doubles
as an orchestration host: a single-machine surface where
agents (claude, codex, gemini, custom) can be spawned into
named terminal tabs, exchange typed events with each
other and with the user through an fsnotify-driven
watcher, and reach chan's MCP server without manual
configuration.

This SKILL documents the contracts external authors need
to integrate with that surface.

## Quick paths

* "I want my agent to send events to other agents
  routed through chan" → [atomic-writes.md](./atomic-writes.md).
* "I want chan to spawn an agent CLI for me" →
  [spawn-protocol.md](./spawn-protocol.md).
* "I want claude / codex / gemini to discover chan's MCP
  server automatically" → see `systacean-14` task notes;
  per-agent discovery shapes land here once
  characterised.

## What chan provides

* **Loopback HTTP server** at `127.0.0.1` with bearer-
  token auth (per-launch token printed on stderr; appended
  to the URL when chan launches a browser).
* **Per-terminal-session fsnotify watcher** that ingests
  typed event files written into a user-chosen directory,
  dispatches `poke\n` to the matching agent's PTY, and
  surfaces events to the rich-prompt bubble overlay.
* **In-process MCP server** exposed over a Unix-domain
  socket; chan-launched terminals get `CHAN_MCP_*` env
  vars to find it.
* **HTTP control channel** for programmatic terminal-tab
  creation, naming, command execution, restart, and
  close.

## What chan does NOT provide

* Multi-user collaboration. Single-user, single-machine,
  by design.
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
