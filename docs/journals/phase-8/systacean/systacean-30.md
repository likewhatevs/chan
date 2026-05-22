# systacean-30 — chan-drive Team config schema + storage + list/load/duplicate API (addendum-b foundation)

Owner: @@Systacean
Cut: 2026-05-22 by @@Architect
Status: dispatched
Round: addendum-b wave-1

## Goal

Foundation layer for the Team feature: chan-drive
primitive for team config + workspace storage under
`Drafts/team-{name}/`. Parallels the `-24` Drafts
foundation.

## Reference

[`../alex/addendum-b.md`](../alex/addendum-b.md)
§"The Team Feature" + §"Clarifications".

## Scope (chan-drive backend)

### Team workspace layout

Each team lives at `Drafts/team-{name}/` with:

* `config.toml` — team config (schema below).
* `events/` — per-team event channel files (watched
  by chan-server per `-31`).
* `docs/` — generalised process docs (per `-a-81`'s
  generalisation).
* Arbitrary additional files (user-pasted, agent
  output, etc.).

### Config schema (TOML)

```toml
team_name = "marketing"
host_name = "Alex"
host_handle = "@@Alex"
auto_prefix_at = true
created_at = "2026-05-22T13:57:00Z"

[[members]]
handle = "@@Architect"
command = "claude"
env = { CHAN_TAB_NAME = "@@Architect" }
is_lead = true
position = { row = 0, col = 0 }

[[members]]
handle = "@@FullStackA"
command = "claude"
env = { CHAN_TAB_NAME = "@@FullStackA" }
is_lead = false
position = { row = 0, col = 1 }
```

Position is the airplane-style grid coordinate; can be
`null` for tabs-in-current-Hybrid layout.

### Filesystem primitive

* `Drive::create_team(team_name, config) -> Result<TeamRef>` —
  creates `Drafts/team-{team_name}/` with config.toml +
  empty events/ + docs/.
* `Drive::list_teams() -> Result<Vec<TeamRef>>` —
  enumerates teams in Drafts.
* `Drive::load_team(team_name) -> Result<TeamConfig>` —
  reads + parses config.toml.
* `Drive::duplicate_team(source_team_name, new_team_name) -> Result<TeamRef>` —
  **verbatim copy** with only the team name changed
  in config + dir name. All paths inside relative.
* `Drive::team_dir(team_name) -> Result<&Path>` —
  resolves the team workspace dir.
* `Drive::team_events_dir(team_name) -> Result<&Path>` —
  for `-31`'s watcher targeting.

### Atomic-write parity

Config writes use existing `atomic_write_in` on the
drafts cap-std handle. Same gates as other Drafts
content (per `-26`).

## Acceptance

1. `create_team` creates dir + config + empty
   events/docs.
2. `list_teams` enumerates.
3. `load_team` round-trips config.toml.
4. `duplicate_team` produces verbatim copy with only
   name changed; all internal paths still resolve.
5. `team_dir` / `team_events_dir` return correct paths.

### Tests

* Round-trip: create + load + verify config.
* Duplicate: verbatim content (assert files match
  byte-for-byte except config.toml's team_name + dir
  name).
* List enumerates correctly.

### Gate

`cargo fmt / clippy / test`; `RUSTFLAGS="-D warnings"
cargo build --no-default-features` green.

## Coordination

* @@Systacean lane (chan-drive primary).
* Consumed by `systacean-31` (watcher) +
  `fullstack-a-79` (bootstrap orchestrator) +
  `fullstack-a-80` (load flow).

## Authorization

Yes for `crates/chan-drive/src/*.rs` (new `teams.rs`
module or extend `drafts.rs`) + tests + task tail +
outbound.

## Numbering

This is `-30`.
