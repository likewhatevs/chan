# systacean-14: MCP auto-discovery for external agents

Owner: @@Systacean
Cut by: @@Architect
Date: 2026-05-19

## Goal

Make chan's MCP server auto-discoverable by external
agents (claude, codex, gemini) launched inside a chan
terminal — without manual user setup. Today we export
`CHAN_MCP_SERVER_JSON` + companion `CHAN_MCP_*` env
vars, which are chan-namespaced and external agents
don't read. We need to publish our descriptor into each
agent's actual discovery surface (config file or
unprefixed env name).

Pairs naturally with `systacean-12` (agent spawning):
when chan launches `claude` in a freshly-spawned tab,
the MCP server is already wired.

## Relevant links

* @@Alex's intent:
  [../request.md](../request.md) — "Auto-configure
  external agents" enhancement bullet.

## Acceptance criteria

### Per-agent discovery shapes

Investigate and document each agent's MCP discovery
path before designing the wire. Likely shapes:

* **claude**: reads MCP config from
  `~/.config/claude/mcp.json` or similar; the
  Anthropic SDK has documented discovery semantics.
* **codex**: ?
* **gemini**: ?

Document each in the orchestration SKILL
(`architect-1`'s output) once nailed down.

### Hard constraints

* **Coexist additively**. Never overwrite or replace
  an existing user MCP setup. We append our server
  entry; their existing entries stay untouched.
* **Land where they actually read**. Verify each agent
  picks up chan's MCP server descriptor after our
  publish step. A descriptor in a file the agent
  doesn't read is worse than nothing.

### Behavior

* On `chan serve` startup (or on the first
  `systacean-12` spawn within a session), chan-server
  detects each agent's config path and appends its
  MCP descriptor if missing. Idempotent — re-running
  doesn't duplicate.
* Removal on chan-server shutdown is optional —
  better to leave the entry (it'll just fail at
  connect time when chan-server is down) than to
  risk corrupting the user's config on a crash.
* Per-agent shims live in
  `crates/chan-llm/src/discovery/` (or similar
  module). Each shim knows: where to write, what
  shape to write, how to detect "already there".

### Tests

* Per-agent shim has unit tests that operate on a
  tmp config file.
* "Coexist additively" is a property test: take an
  existing config with random entries, run our
  publish step, assert the random entries are
  untouched and our entry is present.

## Out of scope

* Implementing the MCP server's actual functionality
  (already exists in `chan-llm`).
* Cross-platform discovery beyond what each agent
  documents.
* Removing entries on shutdown.

## How to start

1. Read each agent's docs for MCP discovery shape.
   Claude SDK first, then codex, then gemini.
2. Sketch the per-agent shim API; one trait + three
   impls is probably right.
3. Wire it into the `chan serve` startup or into
   `systacean-12`'s spawn path.

## Hand-off

Standard. Pre-push gate green. Coordinate with
@@Architect on the orchestration SKILL — the per-agent
shapes you document feed directly into the SKILL's
external-user setup guide. Ping via
`alex/event-systacean-architect.md`.
