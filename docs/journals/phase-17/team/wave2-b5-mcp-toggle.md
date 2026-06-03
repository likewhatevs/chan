# Wave-2 design: B5 MCP env off-by-default + team toggle

Lead prep (@@LaneA). NOT dispatched yet - Wave-2, after Wave-1 lands and the
two-team collision is resolved. Cross-lane (D + B + A); the config schema is
the single coordination point.

## Decisions (from @@Alex)

- MCP env vars start OFF by default for ALL agents (claude + codex + gemini).
  Opt-in to turn them back on. (@@Alex survey: "Global off, opt-in toggle".)
- chan NEVER writes a user's MCP/agent config files (B5 part 1 - confirm + keep
  the invariant; recon found no such writes).
- `cs search` and friends still work with MCP env OFF (they use the control
  socket, not the MCP env descriptor).
- The opt-in lives at TWO levels:
  1. TEAM level: a per-team toggle in the team setup dialog + `cs terminal team
     new/load`, persisted in the team `config.toml`. (@@Alex: "the team setup
     dialog and cs terminal team new/load should include config for whether to
     turn on/off mcp env vars for the team".)
  2. NON-team default: a server-config/preference opt-in for plain
     `cs terminal new` / server-spawned terminals (B5 original).

## Config schema (the coordination point - agree before editing)

- Team `config.toml`: add a top-level `mcp_env` bool, DEFAULT false (off).
  Lives next to team_name / host_name / tab_group / auto_prefix_at.
- The team-spawn path reads `config.mcp_env` and sets each member terminal's
  CreateOptions.mcp_env accordingly (default off when the field is absent -
  pre-release, no migration).
- Keep the Rust team-config struct (chan-shell), the server's team-config parse
  (chan-server), and the TS team-config type (TeamDialog) in lockstep on the
  new field.

## Lane split (Wave-2)

- @@LaneD (B5 core): flip CreateOptions.mcp_env default to false at the ~20
  spawn sites incl control_socket.rs team spawn (~702); add the server pref for
  the non-team default; wire the team-spawn path to read config.mcp_env;
  set_mcp_env stays. Confirm + keep the no-user-config-writes invariant.
- @@LaneB (chan-shell): `cs terminal team new`/`load` gains the mcp_env field
  (cli.rs + wire.rs team config); read/write it in config.toml. Coordinate the
  config struct shape with @@LaneD (shared chan-server crate for the spawn read).
- @@LaneA (me): TeamDialog.svelte - a "MCP env vars" toggle in the team setup
  dialog (default off), threaded through wireToDialog/dialogToWire to the new
  config field.

## Sequencing note

B5 shares the chan-server crate with @@LaneB's B4 (pane-exec region) - the lead
sequences those so the shared-crate compile window does not overlap. The team
config struct is the only NEW cross-lane signature; land it in one burst
(struct + parse + cs read/write + spawn read) and re-check
`cargo check -p chan-server -p chan-shell` green before pausing.
