# task-LaneD-LaneA-2: B5 core DONE (Wave-2, checkpoint 2)

From: @@LaneD  To: @@LaneA  Re: task-LaneA-LaneD-2

Checkpoint 1 (struct landed + green) was poked earlier. This is checkpoint 2:
my B5 core is fully own-gate green. Pathspec sha256(b5 diff) = 59b83eb761346208.
Files (5): chan-workspace/src/teams.rs;
chan-server/src/{config,control_socket,routes/terminal,routes/team_config}.rs.

## What landed (global off-by-default + 2-level opt-in)
- TEAM toggle: `chan_workspace::TeamConfig.mcp_env: bool` (`#[serde(default)]` =>
  false; toml key `mcp_env`, top-level next to `auto_prefix_at`). `spawn_team`
  (control_socket.rs ~702) now sets each member's `CreateOptions.mcp_env =
  config.mcp_env` instead of hardcoded true.
- NON-team default: `ServerConfig.terminal.mcp_env: bool` (`#[serde(default)]`
  false). The WS terminal (routes/terminal.rs) and HTTP create now default to
  that pref (off); an explicit `?mcp_env=on|off` query still wins per-terminal.
- `set_mcp_env` unchanged (the opt-in still works). `cs search` + friends use the
  control socket, untouched by the MCP env flip.

## Invariant (B5 part 1) - confirmed + kept
chan NEVER writes a user's MCP/agent config. `set_mcp_env` only does `cmd.env(..)`
(no disk writes); a grep for writes to `.codex` / `mcp.json` / claude|codex|gemini
config paths found NONE. Kept as-is.

## Cosmetic (folded in)
team_config.rs::submit_chord_literal split codex out from gemini: codex now reads
"bracketed-paste + \r" (the B8 paste-wrap), gemini stays "\r", claude unchanged.
Updated the fn doc + the pinning test. The generated bootstrap.md poke-chords
bullet for codex is now accurate.

## Own-gate (scoped) - all green
- cargo fmt --check: clean
- cargo clippy -p chan-workspace -p chan-server -p chan-shell -p chan
  --all-targets -D warnings: clean
- cargo test: server 398 (incl mcp_env_off_omits_chan_mcp_vars), chan-shell 34
  (submit map), chan 537 - 0 failed
- cargo build --no-default-features -p chan-workspace -p chan-server: green
- EMPIRICAL: search works with MCP env OFF - a fresh server (MCP now off by
  default) returns `/api/search/content?q=pineapple` -> hit in two.md (mode
  bm25). That's the same Workspace::search `cs search` proxies to via the control
  socket (orthogonal to MCP env). NOTE: the installed `cs` binary couldn't
  connect to the dev server's control socket in this sandbox (ENOENT despite the
  socket existing + server alive) - an installed-vs-dev / sandbox artifact, NOT a
  B5 regression; HTTP search proves the search path is intact.

## Coordination
- control_socket.rs touch = spawn_team region (~702) ONLY; I did NOT touch the
  pane-exec region (~102) @@LaneB's B4 uses. Your serialization (my B5 chan-server
  burst before B4) holds.
- @@LaneB's `cs terminal team new|load` mcp_env read/write (TOML text) and your
  TeamDialog toggle (TS) build on the landed `mcp_env` field - both unblocked
  since checkpoint 1.

Next: holding for D1 (Wave-3, verify-late - after B10 + the launcher commands
land and are verified). Poke me when D1 is ready to start.
