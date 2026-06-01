# Phase-15 round-4 - @@LaneC (`cs terminal team` CLI)

You are @@LaneC. Read `round-4-bootstrap.md` -> `round-4-status.md` -> this
file -> `round-4-plan.md` (grounded anchors). You build the CLI equivalent of
the Cmd+P team setup/load dialog.

## Goal

`cs terminal team new | load` + a `--script` flag. Build `--script` FIRST: it
is the design-driver that forces the public `cs` surface to express team
bootstrap end-to-end, and the direct `new` then runs the same sequence (one
source of truth). This is the public, automatable contract for create / load /
run-with-teams.

## Your files (no other lane edits these)

- `crates/chan-shell/src/cli.rs` (`TerminalAction::Team` + subcommands + args)
- `crates/chan-shell/src/wire.rs` (a new `ControlRequest::TerminalTeam`
  variant, if using the control socket)
- `crates/chan-server/src/control_socket.rs` (the team handler)
- `crates/chan-server/src/routes/team_config.rs` (refactor
  `generate_bootstrap_md` to a shared fn so the CLI/handler reuses it)
- reuse `crates/chan-workspace/src/teams.rs` (the `TeamConfig`/`Member` types)

Disjoint from D (routes/search.rs) and B (build infra). No cross-lane seam.

## Grounded state (see round-4-plan.md for full anchors)

- Team config: `teams.rs:9-64` (`TeamConfig`/`Member`; 1-9, one lead, agent
  types). Validation `team_config.rs:75-109`. On-disk `{team-dir}/config.toml`
  + `bootstrap.md` (server-regenerated) + `tasks|journals|followups/`, all via
  `Workspace::{read_text,write_text,create_dir}`.
- bootstrap.md gen: `team_config.rs generate_bootstrap_md` ~223 (roster +
  per-agent poke chords + the 1-liner format). REFACTOR to a shared fn.
- HTTP routes: `POST /api/team-config/{read,write}` (`lib.rs` ~911).
- The orchestration to mirror: `teamOrchestrator.svelte.ts runTeamBootstrap`
  ~339-441 (write config -> resolve group [collision -N] -> spawn LEAD first ->
  drop placeholder -> spawn workers -> place prompt -> seed submit mode ->
  broadcast).
- `cs terminal` surface: `chan-shell/src/cli.rs:161` `TerminalAction`
  dispatching `ControlRequest` (`wire.rs`); handlers `control_socket.rs` ~224.

## Your work scope, by wave

### Wave 1 - `--script` + the CLI surface + the config handler

- `cs terminal team new --script` (and `load --script`) emit a runnable shell
  script of the WHOLE bootstrap: `mkdir -p {dir}/{tasks,journals,followups}`;
  `cat <<'EOF' > {dir}/bootstrap.md` heredoc with the generated bootstrap;
  then per agent (LEAD first) `cs terminal new --tab-name=<handle>
  --tab-group=<team>` + `cs terminal write --tab-name=<handle> --submit=<agent>
  $'<identity/bootstrap prompt>\x1b[27;9;13~'`. The script must be valid +
  self-contained (paste-and-run). Any gap in the public `cs` surface (a
  missing flag, an unscriptable step) is the API to fix THIS wave.
- The config path: `new` validates + writes `{dir}/config.toml` + the
  server-regenerated `bootstrap.md` + the subdirs (reuse the HTTP write logic
  via a new `ControlRequest::TerminalTeam` handler, or call the route). `load`
  reads + validates `{dir}/config.toml`.
- Input shape for `new`: decide how a team is specified on the CLI (flags vs a
  TOML file path) - keep it scriptable. Document it in `cs terminal team
  --help` (worked examples, like the round-3 survey --help).
- Gate your files; poke @@Architect "wave 1 done". (Terminal SPAWN
  orchestration is Wave 2.)

### Wave 2 - lead-first spawn orchestration + tests + smoke

- The non-`--script` `new` RUNS the bootstrap: spawn the lead first, then the
  workers, with the right tab-names / group / submit chords (mirror
  runTeamBootstrap's sequencing; the placeholder pane never goes empty in the
  UI, but the CLI spawns fresh tabs). Tab-group collision detection (append
  -N).
- Tests: round-trip `new` + `load`; verify `bootstrap.md` is server-
  regenerated (not client-side); verify the `--script` output, when executed,
  reproduces the same team as the direct `new` (diff the two results).
- Browser/terminal smoke on a test server: `cs terminal team new` spawns the
  team; the lead reads bootstrap.md; pokes round-trip.
- Gate; poke @@Architect "wave 2 done".

## Completion (each wave)

Drive your files to gated-green + merge (pathspec commits), write your journal
(`round-4-lane-c-journal.md`), poke @@Architect.
