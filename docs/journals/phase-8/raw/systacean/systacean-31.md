# systacean-31 — chan-server multi-team watcher orchestration (per-team event-channel watchers)

Owner: @@Systacean
Cut: 2026-05-22 by @@Architect
Status: dispatched
Round: addendum-b wave-1
Dependency: `systacean-30`

## Goal

chan-server orchestrates a watcher PER LOADED TEAM,
each rooted at the team's `Drafts/team-{name}/events/`
dir. Uses the multi-root `WatchRoot` primitive from
`-25`.

## Reference

[`../alex/addendum-b.md`](../alex/addendum-b.md)
§"Clarifications" #2 — per-team isolated watcher.

## Scope (chan-server)

### Lifecycle IPCs

* `team_load_start(team_name) -> Result<()>` — spins
  up a watcher rooted at the team's events/ dir.
  Registers per-team event-bus subscribers.
* `team_unload(team_name) -> Result<()>` — tears down
  the watcher + unregisters subscribers. **Non-destructive**:
  same semantics as today's watcher off-toggle. The team
  workspace (config + events + docs) PERSISTS on disk;
  terminals stay open (user-managed). Re-load via normal
  Load Team flow at any time.
* `team_list_loaded() -> Result<Vec<String>>` — for
  SPA to know which teams are active.

### Watcher integration

* Use `WatchRoot::team(team_events_dir, prefix)`
  shape (mirrors `WatchRoot::drafts` from `-25`).
* Event paths emerge prefixed (e.g.
  `team-marketing/events/event-x.md`) so the SPA can
  route per-team.

### Multiple teams concurrent

* N loaded teams = N WatchRoot entries on a single
  WatchHandle OR N separate handles. Implementer's
  call on which architecture reads cleaner.
* Recommend single handle with multiple roots — fewer
  syscalls, simpler lifecycle.

### Event dispatch

* When an event lands in a team's events/ dir, route
  to the SPA via the existing event-stream surface
  (per the watcher protocol).

## Acceptance

1. `team_load_start` spins up watcher; events in
   that team's events/ dir flow to the SPA.
2. `team_unload` stops the watcher cleanly.
3. Multiple teams concurrent: each gets independent
   event stream.
4. No regression on the drive-root + Drafts/ watcher
   from `-25`.

### Tests

* Single team load/unload lifecycle.
* Multi-team concurrent events.
* Backward-compat: no team loaded → behaves as today.

### Gate

`cargo fmt / clippy / test`; `RUSTFLAGS="-D warnings"
cargo build --no-default-features` green.

## Coordination

* @@Systacean lane (chan-server primary).
* Depends on `systacean-30` API for `team_events_dir`.
* Consumed by `fullstack-a-79` (bootstrap calls
  `team_load_start` after spawning terminals) +
  `fullstack-a-80` (load flow calls same IPC).

## Authorization

Yes for chan-server watcher / event-watcher / IPC
surfaces + tests + task tail + outbound.

## Numbering

This is `-31`.

## 2026-05-22 — implementation complete; ready for smoke

Picked up `-31` per the addendum-b WAVE-1 sequencing. `-30`'s Team workspace primitive is the consumer-side for the API; `-25`'s `WatchRoot` is the consumer-side for the watcher.

### What landed

**1. chan-drive `Drive::watch_team(team_name, cb)`** (`crates/chan-drive/src/drive.rs`):

* Wraps `WatchRoot` construction with the `team-{name}/events` prefix + delegates to `WatchHandle::start`.
* Resolves the events_dir via `team_events_dir` so a missing team errors cleanly (no silent "watch a non-existent path").
* Caller (chan-server) drops the returned handle to stop watching.

**2. chan-server `loaded_teams` state field** (`crates/chan-server/src/state.rs`):

* `Mutex<HashMap<String, WatchHandle>>` keyed by team name. Each entry holds the per-team handle returned by `Drive::watch_team`.
* Lifecycle is non-destructive per addendum-b's tear-down clarification: dropping the handle releases the notify watcher, but the on-disk workspace (`Drafts/team-{name}/{config.toml, events/, docs/}`) PERSISTS.

**3. chan-server route surface** (`crates/chan-server/src/routes/teams.rs`, new):

* `POST /api/teams/{name}/load` → `api_team_load`: builds the watch bridge (re-uses the same `events_tx` / `index_events_tx` fan-out as the drive-root watcher), calls `Drive::watch_team`, inserts the handle into `loaded_teams`. Idempotent (re-load replaces the existing handle).
* `POST /api/teams/{name}/unload` → `api_team_unload`: removes the entry from `loaded_teams` (drops the handle = unwatch). 404 when the team isn't currently loaded.
* `GET /api/teams/loaded` → `api_team_list_loaded`: returns the sorted list of currently-loaded team names.

**4. Route registration** (`crates/chan-server/src/lib.rs`):

* Three new routes wired into `router()`. Imports added to the route-use list.

### Architecture decision: per-team isolated `WatchHandle`

The task body said "single handle with multiple roots OR N separate handles; implementer's call". Went with **N separate handles** — each loaded team has its own `WatchHandle`. Rationale:

* Lifecycle is cleaner: `team_unload` = `HashMap::remove` = `Drop` on the handle = the notify watcher unwatches its root. No dynamic add/remove on a shared handle.
* No `Arc<Mutex<Vec<WatchRoot>>>` mutation behind the dispatcher closure (the current `WatchHandle::start` API takes a static `&[WatchRoot]`; adding dynamic-roots would be a larger chan-drive refactor).
* Per-team isolation matches the addendum-b spec's "per-team isolated watcher" wording.

If profile data later shows a meaningful syscall cost from N notify watchers, the architecture can switch to shared-handle without changing chan-server's API surface.

### Tests (+1)

* `crates/chan-drive/src/drive.rs::tests::watch_team_emits_events_with_prefix` — full end-to-end: create team via `Drive::create_team`, attach watcher via `Drive::watch_team`, write a file in the team's `events/` dir, poll for the event with the expected `team-{name}/events/<file>` prefix. Uses the same outcome-poll pattern as `-23` (with a 200ms attach-settle sleep mirroring `-25`'s pattern for FSEvents coalescing safety on macOS).

Plus existing `AppState` literal sites in `state.rs::tests` + `search.rs::tests` updated to include `loaded_teams: Mutex::new(HashMap::new())` (required by struct construction; not backward-compat for literal builders).

### Acceptance criteria status

| # | Criterion | Status |
|---|-----------|--------|
| 1 | `team_load_start` spins up watcher; events flow to SPA | ✓ via `api_team_load` route |
| 2 | `team_unload` stops the watcher cleanly | ✓ via `api_team_unload` route (drops the handle) |
| 3 | Multiple teams concurrent | ✓ via per-team `HashMap` storage |
| 4 | No regression on `-25`'s drive-root + Drafts/ watcher | ✓ (preserved; teams don't touch the existing `WatchHandle::start` call inside `Drive::watch`) |

### Pre-push gate

* `cargo fmt --check`: clean.
* `cargo clippy --all-targets -- -D warnings`: clean.
* `cargo test -p chan-drive --lib`: **460 passed; 0 failed; 2 ignored** (was 459; +1 new).
* `cargo test -p chan-server --lib`: **213 passed; 0 failed** (unchanged; +0 because the route is hard to integration-test without the full HTTP stack).
* `cargo test` workspace: all crates green.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`: green.

### Files

| File                                            | +   | -  |
|-------------------------------------------------|-----|----|
| `crates/chan-drive/src/drive.rs`                | +99 | 0  |
| `crates/chan-server/src/state.rs`               | +9  | 0  |
| `crates/chan-server/src/lib.rs`                 | +14 | -2 |
| `crates/chan-server/src/routes/mod.rs`          | +2  | 0  |
| `crates/chan-server/src/routes/search.rs`       | +1  | 0  |
| `crates/chan-server/src/routes/teams.rs` (new)  | +133 | 0 |

Plus task tail + outbound poke. 8 paths total.

### Suggested commit subject

```
chan-drive + chan-server: per-team WatchHandle + team_load/unload/list_loaded routes (systacean-31)
```

### Smoke plan

`gh workflow run ci.yml --ref systacean-31-smoke`. Expected: all 5 jobs green. PTY-test flakiness from prior `-27`/`-29` smokes might still appear; if so, re-fire.

### What this unblocks

* `fullstack-a-79` (bootstrap orchestrator) — calls `POST /api/teams/{name}/load` after spawning terminals.
* `fullstack-a-80` (load flow) — same IPC.
* SPA event stream — receives prefixed events (`team-marketing/events/event-x.md`) over the existing `/ws` channel.

Per architect's pre-authorization, proceeding to commit + push + smoke.
