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

## 2026-05-22 — implementation complete; ready for smoke

Picked up `-30` per the ADDENDUM-B WAVE-1 dispatch poke. Parallels the `-24` Drafts foundation pattern.

### What landed

**1. New module `crates/chan-drive/src/teams.rs`** (~330 LOC + 6 module-level tests):

* `TEAM_DIR_PREFIX = "team-"` const + `TeamRef { name, abs }` handle with `config_path() / events_dir() / docs_dir()` helpers.
* `TeamConfig` struct: `team_name + host_name + host_handle + auto_prefix_at + created_at + members: Vec<Member>`. `auto_prefix_at` has a `#[serde(default)]` so legacy configs default to true; `members` defaults to empty.
* `Member { handle, command, env, is_lead, position }` with `Position { row, col }` for airplane-grid layout.
* `create(drafts_dir, &TeamConfig)` — creates `team-{name}/`, persists config.toml atomically, creates empty `events/` and `docs/`.
* `load(drafts_dir, team_name)` — reads + parses config.toml.
* `list(drafts_dir)` — enumerates `team-*` directories (filters regular drafts out).
* `duplicate(drafts_dir, source, new_name)` — recursive byte-for-byte copy + rewrites `team_name` in the new config.toml.
* `validate_name` — empty / `..` / `/` / `\` rejected.

**2. Drive plumbing** (`crates/chan-drive/src/drive.rs`):

* `Drive::create_team / list_teams / load_team / duplicate_team` — thin wrappers around the module functions.
* `Drive::team_dir(name)` — absolute path; validates via `load_team` first so non-existent teams error cleanly.
* `Drive::team_events_dir(name)` — absolute path to the team's `events/` subdir (consumed by `-31`'s per-team watcher).

**3. lib.rs re-exports**: `Member, Position, TeamConfig, TeamRef`.

### Tests (+8)

Module-level (6) in `teams.rs::tests`:

1. `create_then_load_roundtrips` — full round-trip with non-trivial config (2 members + nested env).
2. `list_filters_to_team_prefix_and_skips_drafts` — `list_teams` ignores `untitled-N` + `rich-prompt-N`.
3. `duplicate_copies_verbatim_then_rewrites_team_name` — pasted sentinel files in `docs/` + `events/` round-trip byte-for-byte; config.toml's team_name is the only field overwritten.
4. `duplicate_rejects_same_name_and_existing_target`.
5. `create_rejects_invalid_names_and_existing` — empty / `..` / `a/b` rejected; second `create` with same name rejected.
6. `load_rejects_missing_team`.

Drive-level (2):

7. `teams_create_list_load_duplicate_through_drive` — full Drive-method exercise.
8. `teams_dont_appear_in_list_drafts_under_team_prefix` — documents that teams + drafts share the metadata dir; `list_drafts` enumerates both (with `team-*` prefix), `list_teams` filters.

### Acceptance criteria status

| # | Criterion | Status |
|---|-----------|--------|
| 1 | `create_team` creates dir + config + empty events/docs | ✓ |
| 2 | `list_teams` enumerates | ✓ |
| 3 | `load_team` round-trips config.toml | ✓ |
| 4 | `duplicate_team` produces verbatim copy with only name changed | ✓ |
| 5 | `team_dir` / `team_events_dir` return correct paths | ✓ |

### Pre-push gate

* `cargo fmt --check`: clean.
* `cargo clippy --all-targets -- -D warnings`: clean (2 rustdoc list-item warnings fixed during gate).
* `cargo test -p chan-drive --lib`: **459 passed; 0 failed; 2 ignored** (was 451; +8 new).
* `cargo test` workspace: all crates green.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`: green.

### Files

| File                                       | +    | -   |
|--------------------------------------------|------|-----|
| `crates/chan-drive/src/teams.rs` (new)     | +325 | 0   |
| `crates/chan-drive/src/drive.rs`           | +136 | 0   |
| `crates/chan-drive/src/lib.rs`             | +2   | 0   |

Plus task tail + outbound poke. 5 paths total.

### Suggested commit subject

```
chan-drive: Team workspace primitive (config + create/list/load/duplicate/team_dir/team_events_dir) (systacean-30)
```

### Smoke plan

`gh workflow run ci.yml --ref systacean-30-smoke`. Expected: all Rust jobs green. Web side: `-29`'s pre-existing `BubbleOverlay.test.ts` TS-drift might still be in HEAD; if so, re-fire after FullStackA's fix lands. NOT a `-30` issue.

### What this unblocks

`-31` (multi-team watcher) — consumes `Drive::team_events_dir(team_name)` to attach a per-team `WatchHandle` via `-25`'s `WatchRoot` primitive.

`fullstack-a-79` (bootstrap orchestrator) + `fullstack-a-80` (load flow) — consume `create_team` / `list_teams` / `load_team` via chan-server route.

Per architect's pre-authorization in the dispatch poke, proceeding to commit + push + smoke.
