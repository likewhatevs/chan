# task-LaneA-LaneD-2: B5 - MCP env off-by-default + team toggle

From: @@LaneA  To: @@LaneD  Wave: 2

B11 + B10 landed - thanks, both thorough. (B10 async-watch + B11 searchable
decisions: I'm handling separately - see your report follow-ups; do NOT block B5
on them.) Now B5.

## Full design

Read docs/journals/phase-17/team/wave2-b5-mcp-toggle.md (I pre-staged it). @@Alex
confirmed: GLOBAL off by default for ALL agents + an opt-in toggle. Summary:

- Confirm + KEEP the invariant: chan NEVER writes a user's MCP/agent config
  files (recon found none - keep it).
- Default CreateOptions.mcp_env = false at the ~20 spawn sites incl
  control_socket.rs team spawn (~702) + terminal_sessions.rs (~84). set_mcp_env
  stays. `cs search` + friends must still work with MCP env OFF.
- Opt-in at TWO levels:
  1. TEAM: a top-level `mcp_env` bool (DEFAULT false) in the team config.toml,
     read by the team-spawn path.
  2. NON-team default: a server-config/preference opt-in for plain
     `cs terminal new` / server-spawned terminals.

## CRITICAL sequencing - land the config struct FIRST, then poke me

The team `mcp_env` field is a 3-lane feature: YOU add the field + spawn read +
default-off; @@LaneB adds `cs terminal team new/load` read/write (chan-shell);
I add the TeamDialog toggle (TS type). The struct is the coordination point.

-> Land the team-config `mcp_env` field (chan-shell wire/struct + chan-server
   parse) + the default-off core in ONE burst, re-check
   `cargo check -p chan-server -p chan-shell` GREEN, then POKE me with the exact
   field name + type. I will then release @@LaneB's cs-surface + my TeamDialog
   toggle against your landed struct. Do NOT leave the struct half-applied
   (shared-crate compile window - @@LaneB is about to do B4 in chan-server too;
   I am SERIALIZING: your B5 chan-server burst lands BEFORE B4's control_socket
   touch).

## Also fold in (1 line, cosmetic)

@@LaneB flagged routes/team_config.rs::submit_chord_literal - a 3rd doc-only
mirror of the submit chord map; its "(chord \r)" parenthetical is now stale
(codex needs the bracketed-paste wrap, see B8). It feeds the generated
bootstrap.md poke-chords bullet. Since you're in chan-server + team config,
update that one parenthetical to reflect codex's paste-wrap (or drop the
parenthetical). Non-blocking.

## Gate

- cargo fmt --check + cargo clippy -p chan-server -p chan-shell -p chan
  --all-targets -D warnings + cargo test (the crates you touch).
- cargo build --no-default-features for the crates you touch.
- `cs search` still works with MCP env off (empirical).
- Browser-visual (team dialog toggle) is mine once I add it.

## Report

Two checkpoints: (1) poke me when the config struct + default-off core compiles
green (so B + I add surfaces); (2) cut task-LaneD-LaneA-2 when B5 fully
own-gate-green + poke. Then D1 (Wave-3, verify-late).
