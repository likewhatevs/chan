# Phase-15 round-4 - @@LaneC journal (`cs terminal team` CLI)

## Wave 1 - DONE, gated-green, merged to main (local)

Built the `cs terminal team new | load` surface + the `--script`
design-driver, the control-socket handler, and the shared bootstrap
generator. The whole config path stays server-side (the wire stays
chan-workspace-free); the bootstrap text is never regenerated client-side.

### What landed (my files only)

- `crates/chan-shell/src/wire.rs`: new `ControlRequest::TerminalTeam { dir,
  op, config_toml, script }` + a `TeamOp { New, Load }` enum. The config
  travels as RAW TOML text (not a typed `TeamConfig`) so chan-shell keeps
  its serde-only, chan-workspace-free footprint; the server owns the parse
  / validate / generate. Wire tag `terminal_team` + op strings `new`/`load`
  pinned by a round-trip test.
- `crates/chan-shell/src/lib.rs`: export `TeamOp`.
- `crates/chan-shell/src/cli.rs`: `TerminalAction::Team { New | Load }`.
  `new <dir>` takes the config from `--config <file>` XOR `--stdin`;
  `load <dir>` takes only the dir. `--script` on either prints the
  paste-and-run bootstrap to STDOUT (captured artifact, like `survey`); the
  write ack / load summary go to stderr. `TEAM_AFTER_HELP` worked examples
  on `cs terminal team --help` (sample config.toml + the three flows), in
  the round-3 survey `--help` style.
- `crates/chan-server/src/routes/team_config.rs`: made
  `validate/read/write_team_config` + `generate_bootstrap_md` `pub(crate)`
  (shared, no duplication). Added `ensure_created_at` (server stamps RFC
  3339 UTC when the input omits it), `generate_bootstrap_script` (the
  `--script` body), `identity_prompt`, and the `sh_squote` / `ansi_c_escape`
  quoting helpers. The script is: shebang + `set -euo pipefail` + `mkdir -p`
  the tree + a quoted heredoc for `config.toml` + a quoted heredoc for the
  server-generated `bootstrap.md` + lead-first per-agent
  `cs terminal new` + command launch + an identity poke
  (`--submit=<agent>`). Shell members (no agent) are launched but never
  poked. ASCII-only, no em dashes.
- `crates/chan-server/src/routes/mod.rs`: `team_config` is now
  `pub(crate) mod` so the sibling `control_socket` can reuse it.
- `crates/chan-server/src/control_socket.rs`: the `TerminalTeam` arm +
  `handle_team`. `new` parses -> stamps created_at -> validates ->
  (`--script`) emits the script OR writes config.toml + the regenerated
  bootstrap.md + the tree through the Workspace sandbox. `load` reads +
  validates `{dir}/config.toml` -> (`--script`) script OR a one-line
  summary. The workspace is resolved LAZILY: `new --script` is a pure
  generator with no filesystem I/O.
- `crates/chan-workspace/src/teams.rs`: `#[serde(default)]` on
  `TeamConfig.created_at` (one line) so a hand-written CLI config can omit
  the timestamp; the SPA still always sends it. This is the only change
  outside chan-server/chan-shell, within the lane doc's "reuse teams.rs".

### Input-shape decision (mine to make, documented in --help)

A team is specified by its on-disk `config.toml` shape (the same
`TeamConfig` the dialog persists), passed via `--config <file>` or
`--stdin`, plus the workspace-relative `<dir>` it lands in. Rationale:
`TeamConfig` is a rich nested struct (1-9 members, each with an env map +
agent type); a flag-per-field surface would be unwieldy, and the TOML form
round-trips byte-for-byte with what `{dir}/config.toml` already stores.
`created_at` is optional (server-stamped) so a hand-written spec stays
minimal.

### Gate (all green)

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test` (exit 0; +12 new unit tests: 8 in team_config, 5 in
control_socket handle_team, 1 wire round-trip, 2 cli parse, 1 input
resolution - all green), `cargo build --no-default-features`, web
`svelte-check` (0 errors) + `npm run build`. Web untouched by this lane;
ran the web gate anyway since it is the standing gate.

### Empirical smoke (scoped ad-hoc serve, torn down)

Renamed binary copy (`/tmp/csmoke-lanec`) + throwaway drive
(`/tmp/chan-test-lanec`), pkill scoped to my own pid; drive removed from
the registry + temp files cleaned at the end.

- `cs terminal team new ... --script` emits a script that passes
  `bash -n` (valid, self-contained, paste-and-run). created_at auto-stamped
  when omitted.
- WRITE path (`new`, no --script): materializes config.toml + bootstrap.md
  + tasks/journals/followups through the Workspace sandbox.
- REPRODUCIBILITY: with a pinned created_at, the script-written config.toml
  is BYTE-IDENTICAL to the handler-written one; bootstrap.md differs ONLY
  in the embedded dir name (the intended parameter) - same dir => identical.
  This is the Wave-2 "script reproduces the same team as direct new" check,
  already passing for the FILES half.
- LOAD: summary + `--script` regeneration both work.

## Wave 2 - scope + the open question for @@Architect

Wave 2 is the non-`--script` `new` RUNNING the bootstrap (lead-first
terminal spawn) + tests + a LIVE smoke (the emitted script reproduces the
same team's terminals as direct `new`).

OPEN QUESTION (spawn mechanism): the `--script` form launches each agent by
writing its command into a fresh `cs terminal new` shell, then poking the
identity prompt after a `sleep 3` boot grace. This stays 100% in-lane (no
SPA edit) and honors "no cross-lane seam", but the script-driven spawn is
inherently async/best-effort (the `cs terminal new` window_command is
fire-and-forget; the live dialog flow awaits `api.spawnTerminal`).

The faithful, non-racy alternative is to teach `cs terminal new` a
`--command` / `--env` (mirroring `api.spawnTerminal({command, env})`). That
would make BOTH the script and the Wave-2 live `new` robust, but it needs a
small additive SPA edit (`open_term_new` -> `openTerminalInActivePane` ->
`spawnTerminal` must carry command+env), which is OUTSIDE my listed files
(web/ is unowned this round). I will NOT half-wire it (a green build with an
SPA that ignores the new fields is the gate-blind-wire-rename trap).

Recommendation: for the Wave-2 LIVE `new`, orchestrate the spawn
server-side via the terminal Registry (lead first, full command+env, submit
chords) so it is robust without the script's async fragility, and keep
`--script` as the auditable/portable best-effort form. Will confirm the
exact approach with @@Architect at the Wave-2 refresh and smoke it on a live
server (needs `navigate`/live terminals re-allowed, or a terminal-only
smoke via `cs terminal list`).

## Wave 2 - DONE, gated-green + live-smoked, committed 626593e9 (local)

ACCEPTED by @@LaneA (2026-06-01): clean + gated, server-side spawn per the
decision. The SPA-visibility limitation is a ROUND-5 BACKLOG item (by
design - no SPA edit this round). Wave-2 barrier then waited on B; C
recycles at the next refresh.

Architect resolved the Wave-1 open question: orchestrate the spawn
SERVER-SIDE via the terminal Registry (no SPA edit); keep `--script` as
the auditable form. Implemented exactly that.

### What landed (my files only; commit 626593e9)

- `crates/chan-server/src/control_socket.rs`: `handle_team` is now async +
  takes the terminal registry. The `new` write path, after writing
  config.toml + the regenerated bootstrap.md, calls `spawn_team`
  (`resolve_team_group` -> `-N` collision against live registry groups,
  spawn lead-first with full command + env + tab-name + group via
  `Registry::create`), then `spawn_and_poke_team` waits a boot grace and
  pokes each AGENT member its identity prompt + submit chord through
  `write_input_matching`. Shell members (no agent) spawn but are NOT
  poked. `team_spawn_summary` is a pure response builder (errors when
  nothing came up). +5 new unit tests (group collision, lead-first spawn
  + agent-only pokes, shell-team skips the wait, summary wording x2); the
  5 prior `handle_team` tests converted to `#[tokio::test]` + the new
  registry arg.
- `crates/chan-server/src/routes/team_config.rs`: extracted
  `team_base_group` + `lead_first_order` as shared `pub(crate)` helpers
  and exposed `identity_prompt`, so the `--script` generator and the
  server-side spawner agree on the group, order, and poke text (one
  source of truth). +2 helper unit tests.
- `crates/chan-shell/src/{submit.rs,lib.rs}`: the submit-chord map now
  compiles WITHOUT the `client` feature (only the `ValueEnum` `--submit`
  parse impl stays `client`-gated), so chan-server reads the chord bytes
  without linking clap. Added `SubmitAgent::from_agent_name`. This avoids
  a THIRD duplicate of the runtime-critical chord bytes in chan-server
  (team_config already had the human-readable literal form). chan-shell
  is wholly mine this round, no seam.

### Spawn mechanism (server-side, per the architect decision)

`Registry::create` per member, lead first, with the member's command
(blank -> default shell), env, tab-name = handle, and the resolved group.
The non-`--script` `new` BLOCKS a boot grace (`TEAM_SPAWN_POKE_GRACE` =
3s, the script's `sleep 3` mirror) then delivers each agent's poke, so
the CLI returns only once the pokes land (the same inline ordering the
script runs). The poke bytes are
`apply_submit_chord(identity_prompt(...), agent)` - byte-identical to the
`--script` form's `cs terminal write --submit=<agent>`.

### Gate (all green, bare exit codes verified)

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo build --no-default-features`, `cargo test` (full workspace, exit
0). chan-shell tested in BOTH feature modes (`--no-default-features`
confirms the clap-free chord path). Web untouched (no web edit this
wave).

### Live smoke (scoped: unique-named binary `cssrv-lanecL` + throwaway
drive `/tmp/csL-drive`, server torn down, drive unregistered)

All confirmed against a live server via the control socket (no browser):
- `cs terminal team new smoketeam --config ...` ->
  `team "smoketeam" spawned in group "smoketeam": 3 member(s) up, poked
  2 agent(s)`; `real 0m3.041s` confirms the boot-grace block.
- `cs terminal list` shows @@Lead, @@Hand, @@Sh in group `smoketeam`,
  lead first.
- POKE DELIVERY (capture members `cat > file`): lead PTY received
  `# Team work / You are @@Lead on team "smoketeam" (a team of 3; host
  @@Neo, lead @@Lead).`; worker received the full personalized variant
  ending `... wait for @@Lead to assign your task.`
- CHORD SEMANTICS confirmed live: the codex CR (`\r`) flushed its line
  through the PTY (canonical CR->NL); the claude CSI chord correctly does
  NOT flush (special key, not a newline) - exactly why claude needs
  modifyOtherKeys. The chord BYTES are unit-tested.
- Shell member @@Sh (`sleep`, no agent) spawned but produced no capture
  -> NOT poked.
- Group collision: a second `new` -> `group "smoketeam-2"`.
- `--script` regression: still emits valid bash (`bash -n`, 132 lines).

### Visibility limitation (flagged for @@Architect / a future SPA round)

The registry-spawned team is real + listable (`cs terminal list`) +
pokeable (`cs terminal write`) + broadcast-capable, but the panes do NOT
auto-surface in an SPA window: the SPA mounts panes only for sessions it
itself spawned (`api.spawnTerminal` -> `openTerminalInPane`), and there
is NO server->SPA "attach existing session to a pane" window-command.
Adding one is an SPA edit (web/ unowned this round = the gate-blind-wire
trap). This matches the architect's "no SPA edit" decision and the
lane's "automatable, CLI-driven team" purpose; a SPA-visible CLI spawn is
a clean future-round item. `cs terminal team new` is thus the headless/
automatable contract; the Cmd+P dialog remains the SPA-visible path.

## Carryover / notes

- The `sleep 3` boot grace is now exercised live: the agents' compose
  boxes accept the poke after ~3s; codex submitted (CR), claude's chord
  is a non-flushing special key (correct). Kept at 3s. Tunable via
  `TEAM_SPAWN_POKE_GRACE`.
- SPA-visible CLI team spawn (a server->SPA attach window-command) is the
  natural next step if the headless spawn should mirror the dialog's
  panes; needs a web edit, out of scope this round.
- No back-compat paths added (pre-release). The created_at serde default
  and the chan-shell submit un-gating are additive, not migrations.
