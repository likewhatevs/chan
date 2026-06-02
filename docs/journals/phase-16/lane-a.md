# @@LaneA — CLI/terminal + lead tooling

You build the `cs terminal` surface the @@Lead process runs on, so your
work is FIRST and the rest of the team is partly blocked on it. Read
`round-1-plan.md` first.

## Round-1 tasks (in order)

1. **C2 `cs terminal scrollback --tab-name=X`** (NO groups). Read a
   terminal's scrollback by tab name.
   - Per-session `RingBuffer` already exists: `crates/chan-server/src/
     terminal_sessions.rs:636`; today exposed only via WS-attach replay
     (`snapshot_since`, `routes/terminal.rs:456-468`).
   - Add `ControlRequest::TermScrollback { tab_name }` to
     `crates/chan-shell/src/wire.rs:28`; a control-socket handler in
     `crates/chan-server/src/control_socket.rs` that snapshots the matching
     session's ring (reuse the tab-name match from
     `write_input_matching`, terminal_sessions.rs:368-398) and returns the
     decoded bytes; and the clap subcommand + `cmd_*` in
     `crates/chan-shell/src/cli.rs`. Single-match required; dump the full
     ring. POST this command's exact CLI shape to `event-lane-a.md` ASAP —
     @@Lead needs it.

2. **C3 `cs pane`** (+ resize). Report windows/panes/layout + selected
   pane; set focus; split left|bottom; RESIZE panes (for scripted layout);
   close tab / close all tabs / close pane; `--force` to kill draft+terminal
   tabs (they otherwise return PARTIAL FAILURE).
   - BIGGEST item: the control socket is ONE-WAY push today
     (`control_socket.rs:237-385`; server cannot query layout). Add a
     bidirectional channel: a request the SPA answers with current layout,
     plus execute close/focus/split/resize and reply success/partial.
     Layout lives in `web/src/state/tabs.svelte.ts`; blockers are FileTab
     dirty (`content !== saved`, :157) and TerminalTab live session
     (`terminalSessionId`, :270).

3. **S1 SPA-visible CLI team spawn** — server->SPA attach window-command so
   `cs terminal team new` surfaces in the running SPA (extends the same
   channel as C3).

4. **C1 fix `cs terminal team load`** (alongside): (a) resolve the path
   against the caller's cwd (handle `.`/relative/absolute) instead of always
   workspace-root-relative (`read_team_config` hardcodes `{dir}/config.toml`,
   `routes/team_config.rs:180`); (b) actually SPAWN the team (reuse
   `spawn_and_poke_team`, `control_socket.rs:622`) — `TeamOp::Load` (:478)
   only reads+summarizes today. POST the resolved path+spawn contract to
   `event-lane-a.md` — @@LaneD's TW1 mirrors it.

## Files you OWN

`crates/chan-shell/src/{cli.rs,wire.rs,lib.rs}`,
`crates/chan-server/src/{control_socket.rs,terminal_sessions.rs}`,
`crates/chan-server/src/routes/{team_config.rs,terminal.rs}`, and the SPA-
side control responder in `web/src/state/store.svelte.ts`
(`handleWindowCommand`, ~:698-771).

## Coordination

- `web/src/state/store.svelte.ts` is shared territory: you add the layout-
  query/exec responder; coordinate with @@LaneB/@@LaneD before touching
  anything beyond `handleWindowCommand`. Read `tabs.svelte.ts` but treat its
  component-facing API as @@LaneD's.
- Post the C2 CLI shape and the C1 path+spawn contract to `event-lane-a.md`
  early (@@Lead blocks on C2; @@LaneD's TW1 blocks on C1).

## Verify

`make pre-push` green. Smoke each command against a live `chan serve`:
`cs terminal list`, write+scrollback round-trip, `cs pane` layout query +
close --force on a draft/terminal, `cs terminal team load <dir>` actually
spawns. Post the commit sha to `event-lane-a.md` and poke @@Lead.
