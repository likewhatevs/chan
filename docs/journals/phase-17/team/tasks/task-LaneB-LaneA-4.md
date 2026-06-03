# task-LaneB-LaneA-4: cs team mcp_env surface DONE (B5 CLI half)

From: @@LaneB  To: @@LaneA  Re: task-LaneA-LaneB-4

## Result: `cs terminal team new --mcp-env <on|off>` sets TeamConfig.mcp_env in
   the written config.toml. Empirically verified end-to-end. chan-shell ONLY.

Flag name: `--mcp-env <on|off>` (value enum). Default when omitted: OFF
(leaves the config's value / @@LaneD's serde default).

## Design (why an injection, not a constructed TeamConfig)

The CLI forwards a RAW config TOML string to the server
(`config_toml: Option<String>`); chan-shell has no toml parser and no
TeamConfig type. So `--mcp-env` injects the top-level `mcp_env` key into the
forwarded TOML client-side. I added `toml` to chan-shell behind the `client`
feature (the server still links wire-types only - verified
`--no-default-features` builds) and a parse + set-root-key + re-serialize
(`set_team_mcp_env`), so `mcp_env` lands at the document root BEFORE the
`[[members]]` tables (a naive string append can't guarantee that). The server
re-parses + regenerates config.toml from the result, so re-serialization is
safe (and is how default OFF still writes `mcp_env = false`).

Did NOT touch routes/team_config.rs (your/@@LaneD cosmetic) or
control_socket.rs (B4, held).

## Implemented

- `McpEnvToggle` ValueEnum (on|off) + `--mcp-env <on|off>` on `team new`
  (Option; omitted -> no injection).
- `set_team_mcp_env(config_toml, bool)`: toml::Table insert + re-emit.
- Help text: default OFF, what it controls (MCP env vars for the team's
  terminals; agents still reach `cs search` with it off), overrides input.
- `team load`: NO CLI change needed - serde round-trips the field. Verified
  empirically it preserves `mcp_env`.
- Tests: clap on/off/bogus parse; set_team_mcp_env sets + overrides at root +
  preserves [[members]].

## Files changed (chan-shell only)

  crates/chan-shell/Cargo.toml   blob cabb8056671f6fe8e3c6945214b573448338082f
        (toml dep behind the `client` feature)
  crates/chan-shell/src/cli.rs   blob 9e2ae7673fa73181762e70e9b2bb0bde2c19a475

## Own-gate (scoped) - GREEN

  cargo fmt -p chan-shell --check                          PASS
  cargo clippy -p chan-shell --all-targets -D warnings     PASS
  cargo test -p chan-shell                                 PASS (37)
  cargo build -p chan-shell --no-default-features          PASS (toml stays
        client-gated; server links wire-only, no clap/toml)

## Empirical (fresh binary :8793, shell-only `command=true` config so NO real
   agents spawn)

  team new teamON  --mcp-env on   -> config.toml  mcp_env = true
  team new teamDEF (no flag)       -> config.toml  mcp_env = false
  team new teamOFF --mcp-env off   -> config.toml  mcp_env = false
  team load teamON --script        -> round-trips (config still mcp_env = true;
                                      3493-byte bootstrap emitted)
  team new --help                  -> shows flag, OFF default, [possible
                                      values: on, off]
Torn down: server killed by PID, chan remove, rm temp. @@LaneD's :8810 + my
prior servers untouched (no broad pkill).

## Note for you

`--script` emits the bootstrap SPAWN script (the `cs terminal` commands), not a
config dump - so `mcp_env` does not appear literally in `--script` output. It
is consumed by the server's spawn-options (@@LaneD's half: team spawn reads
config.mcp_env). That matches your task line ("--script emits it if it emits
the other team fields" - it doesn't emit team config fields as such).

## Status

Done. B4 still HELD per your sequencing (B4 + @@LaneD's B5 chan-server serialize
the crate gate). Holding for your B4 release once @@LaneD cuts B5-done.
