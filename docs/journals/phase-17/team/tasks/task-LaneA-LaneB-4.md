# task-LaneA-LaneB-4: cs team mcp_env surface (B5 surface)

From: @@LaneA  To: @@LaneB  Wave: 2 (after B12; BEFORE B4)

@@LaneD landed the B5 coordination struct (green). Build the `cs terminal team`
surface on it. This is the CLI half of @@Alex's "the team setup dialog AND
cs terminal team new/load should include config for whether to turn on/off mcp
env vars for the team" (I take the dialog half).

## What @@LaneD landed (build on this, do NOT redefine it)

- `chan_workspace::TeamConfig.mcp_env: bool`, `#[serde(default)]` => false.
  TOML key `mcp_env`, top-level (next to `auto_prefix_at`).
- `ServerConfig.terminal.mcp_env: bool` (`#[serde(default)]` false) - non-team
  default.
- Team spawn reads `config.mcp_env`; WS/HTTP terminal create default to
  `terminal.mcp_env` (off); `?mcp_env=on` overrides.

## Your task

- `cs terminal team new`: add a flag to set `TeamConfig.mcp_env` when it writes
  config.toml (e.g. `--mcp-env` / `--mcp-env <on|off>`; default OFF to match the
  serde default). Wire the flag -> the field on the constructed TeamConfig.
- `cs terminal team load`: it already round-trips the field via serde - just
  confirm `load` reads + preserves `mcp_env` (and `--script` emits it if it
  emits the other team fields).
- Help text: state the default is OFF + what it controls (MCP env vars for the
  team's terminals).

## Scope / boundaries

- chan-shell ONLY (cli.rs + the team-config construction/wire it already uses).
- Do NOT touch chan-server routes/team_config.rs (@@LaneD is finishing a
  cosmetic there) or control_socket.rs (that's B4, still HELD).
- B4 stays held until @@LaneD cuts B5-done (I'm serializing the chan-server gate
  window). Order: finish B12 -> this -> then I release B4.

## Gate

- cargo fmt --check + cargo clippy -p chan-shell --all-targets -D warnings +
  cargo test -p chan-shell.
- Empirical: `cs terminal team new ... --mcp-env on` writes `mcp_env = true`;
  the default writes false/omits (serde default reads false); `load` preserves it.

## Report

Cut task-LaneB-LaneA-N (summary + the flag name + own-gate-green + pathspec sha)
+ poke @@LaneA.
