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
  the watcher + unregisters subscribers.
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
