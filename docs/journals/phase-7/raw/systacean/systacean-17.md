# systacean-17: rename + restart doesn't update CHAN_TAB_NAME on the second cycle

Owner: @@Systacean
Cut by: @@Architect
Date: 2026-05-19

## Goal

Two related symptoms in the rename-terminal-then-restart
flow:

1. The "rename pending — restart needed" warning fires
   on the FIRST rename of a tab but not on subsequent
   renames in the same session.
2. After restart, the new PTY's `CHAN_TAB_NAME` env var
   stays at the OLD tab name (the value at first
   spawn), not the current tab name.

@@Alex's repro: tab originally `@@FullStack`, renamed
to `@@FullStackA`, warning fired, restart picked up the
new name. Renamed AGAIN to `@@FullStackB`; no warning
fired AND `echo $CHAN_TAB_NAME` after restart still
shows `@@FullStack`. The screenshot shows the tab strip
saying `@@FullStackB`, the menu Name field saying
`@@FullStackB`, the terminal "connected - 125x43", but
the env says `@@FullStack`.

This breaks spawned-agent routing (`systacean-12` matches
events by tab name; if the env is stale the agent thinks
its name is something else) and `chan open` window
discovery (`systacean-1` uses `$CHAN_TAB_NAME`).

## Note on the binary @@Alex tested on

@@Alex flagged they're "1 release behind" — likely on
the binary before `fullstack-26` onwards. Reproduce on
current `main` before deciding what's already fixed.

## Acceptance criteria

* Every rename of a tab whose PTY is alive triggers the
  rename-pending warning (the indicator landed in
  `fullstack-17` polish). Not just the first.
* When the user restarts the PTY after a rename, the
  new PTY launches with `CHAN_TAB_NAME` set to the
  current display name, regardless of how many times
  the tab has been renamed in this session.
* Same guarantee for `CHAN_DRIVE_NAME` and any other
  per-tab env vars chan-server injects on PTY launch
  (audit during the fix).
* Regression test: a sequence of (rename → restart →
  echo CHAN_TAB_NAME → rename again → restart →
  echo CHAN_TAB_NAME) lands the right values on every
  step.

## Likely seams

* `crates/chan-server/src/terminal_sessions.rs` — the
  PTY spawn / respawn path. The env block probably
  captures the tab name at first spawn into a cached
  value, and the restart machinery doesn't re-read
  the current tab name when it relaunches.
* `Registry::restart` (added in `systacean-12`) — does
  it re-derive env or reuse the original?
* The rename-pending indicator path on the frontend:
  if it's gated by a "dirty since first spawn" flag
  that doesn't reset on restart, that explains why
  subsequent renames don't trigger it.

## Out of scope

* Other rename-related UX (already handled by
  `fullstack-17` polish + the indicator).
* Changes to how chan-server tracks tab names —
  whatever the source of truth is, the fix is to
  read it at restart time, not at first spawn time.

## How to start

1. Reproduce on current `main` binary. Confirm the
   second-rename-no-warning + stale-env behavior.
2. Inspect `terminal_sessions.rs` for cached env
   values on the spawn path.
3. Inspect the restart endpoint (`POST
   /api/terminals/<session>/restart`) to confirm
   whether it re-reads tab name.
4. Frontend indicator: trace whether the dirty flag
   resets on restart.

## Hand-off

Standard. Pre-push gate green. Coordinate with
@@FullStackA/B if any frontend state needs to reset on
restart. Ping via
`alex/event-systacean-architect.md`.

## 2026-05-19 10:00 BST - ready to land

Reproduced by code inspection on current `main`: controlled terminal
restart reused `Session::tab_name`, which was captured at first spawn,
and the frontend restart call sent no current tab name. The frontend
env-name marker also only refreshed when a session id changed, so a
same-id restart could not make a second rename stale again.

Implemented:

* `POST /api/terminals/:session/restart` accepts optional `name` and
  `window_id` JSON fields.
* chan-server restart uses those overrides when respawning the PTY, so
  `CHAN_TAB_NAME` is derived from the current tab title supplied by the
  frontend.
* Controlled-terminal restart sends `terminalTabName(tab)` and
  `sessionWindowId()`.
* After a successful restart request, frontend state records the
  current tab name as the active env name, so the next rename triggers
  the warning again.

Audit: no `CHAN_DRIVE_NAME` injection exists in the current PTY env
path. The per-tab injected values on this path are `CHAN_TAB_NAME` and
`CHAN_WINDOW_ID`; `CHAN_CONTROL_SOCKET` and MCP env are server-scoped
and already recomputed on each spawn.

Regression coverage:

* chan-server route test proves restart can update
  `CHAN_TAB_NAME` for the respawned PTY.
* frontend state test proves rename staleness resets after restart and
  fires again on a subsequent rename.

Verification:

* `cargo test -p chan-server api_restart_terminal_updates_chan_tab_name_env --no-default-features`
* `npm run test -- src/state/tabs.test.ts`
* `cargo test -p chan-server --no-default-features`
* `cargo clippy -p chan-server --all-targets --no-default-features -- -D warnings`
* `npm run check`
* `npm run build`
* `scripts/pre-push`

Known risk: did not drive the browser manually; coverage exercises the
two failing seams directly.

Proposed commit message:

`Refresh terminal env name on restart`
