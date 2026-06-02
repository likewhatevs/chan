# C3 design: `cs pane` + the bidirectional window channel (@@LaneA)

Round-1 task C3 (and S1, which extends the same channel). This is the
design + the cross-lane seam, posted so @@LaneC (route wiring) and @@LaneD
(frontend responder + tabs API) can see exactly what I touch.

## Problem

The control socket is ONE-WAY today: the server broadcasts a
`window_command` frame and the SPA acts on it (`open_*`). The server cannot
QUERY the SPA's layout or get a success/partial result back. `cs pane` needs
a round-trip: server -> SPA (command) AND SPA -> server (reply).

The `cs terminal survey` flow already does exactly this round-trip:
`SurveyBus` parks a oneshot keyed by a server-minted id, the server pushes
`open_survey`, blocks, and `POST /api/survey/reply` -> `complete_survey`
fires the oneshot. C3 mirrors this with a GENERIC window-reply bus.

## Architecture (mirror of the survey bus)

```
cs pane ...                      (chan-shell CLI, mine)
  -> ControlRequest::Pane{...}   (wire.rs, mine)
  -> control_socket handler      (control_socket.rs, mine)
       window_bus.register() -> request_id + oneshot          [NEW bus, mine]
       push WindowCommand::Pane{request_id, ...} to the window  (broadcast)
       await the oneshot
  ... SPA handleWindowCommand sees pane command                (store.svelte.ts, @@LaneD seam)
       reads `layout` / calls tabs API (split/focus/close/resize)
       POST /api/window/reply { requestId, payload }           (client.ts, @@LaneD seam)
  -> api_window_reply route -> window_bus.complete(id, payload)  (routes/window.rs NEW + wiring, @@LaneC seam)
       fires the oneshot, the handler returns payload to the CLI
  -> cs pane prints the layout JSON / the exec result
```

The bus reply is a generic `serde_json::Value` so QUERY (returns the layout)
and EXEC (returns success/partial) share one bus.

## Files

Fully MINE (no coordination needed):
- `crates/chan-shell/src/wire.rs` — `ControlRequest::Pane{window_id, op}`.
- `crates/chan-shell/src/cli.rs` — `cs pane` subcommand tree + dispatch.
- `crates/chan-server/src/window_bus.rs` — NEW, the request/reply bus
  (copy of survey.rs, generic JSON reply).
- `crates/chan-server/src/control_socket.rs` — the handler + the
  `WindowCommand::Pane*` server->SPA variants.

SHARED, @@LaneC seam (route wiring; clean at HEAD now):
- `crates/chan-server/src/state.rs` — add a `window_bus` field to AppState
  (1 field, mirrors `survey_bus`).
- `crates/chan-server/src/lib.rs` — construct the bus, pass it to
  `control_socket::start`, store on AppState, add the reply route to
  `router()` (mirrors the `survey_bus` + `/api/survey/reply` lines).
- `crates/chan-server/src/routes/mod.rs` — `pub use window::api_window_reply`.
- `crates/chan-server/src/routes/window.rs` — NEW reply route file (mine,
  but the `mod`/`pub use`/router lines land in @@LaneC's files).

SHARED, @@LaneD seam (frontend):
- `web/src/state/store.svelte.ts` — `handleWindowCommand` pane responder
  (my authorized area). Reads the `layout` singleton; for EXEC calls the
  tabs API.
- `web/src/api/client.ts` — `api.windowReply(...)` + the pane frame types.
- `web/src/state/tabs.svelte.ts` — I only CALL its public API (no edits):
  `layout` (read), `setActivePane`, `splitPane`, `closeTab`, `closePane`,
  `paneModeResize`. If a thin read-only "layout snapshot" helper is wanted,
  that is @@LaneD's to add; otherwise I serialize from `layout` in the
  store responder.

## Wire shapes (proposed)

`cs pane` runs in a chan terminal, so it carries `$CHAN_WINDOW_ID` (the
caller's own window) like the `open_*` commands; the query/exec target that
window.

```
ControlRequest::Pane { window_id, op: PaneOp }
PaneOp = Query
       | Focus { pane_id }
       | Split { pane_id?, direction: left|bottom }
       | Resize { ... ratio/amount }
       | CloseTab { pane_id?, tab_id? } | ClosePane { pane_id? } | CloseAll
       (Close ops carry `force: bool` for dirty/live tabs)
```

Reply (SPA -> POST /api/window/reply): `{ requestId, payload }` where
payload is the layout snapshot (Query) or `{ ok, closed, blocked: [...] }`
(Exec). Close hitting a dirty FileTab / live TerminalTab without `--force`
is a PARTIAL FAILURE reported in `blocked`.

Layout snapshot the SPA returns (from `layout`): the binary split tree of
panes; per pane its id, tabs (id/kind/title, FileTab `dirty`, TerminalTab
`live`), `activeTabId`; plus `activePaneId`. The CLI prints markdown by
default, `--json` for machine output (mirrors `cs terminal list`).

## Proposed scope split (decision for @@Lead)

C3 is large (8 files, 3 lanes' territories). Proposal:
- **C3a (first slice):** the channel + `cs pane` (no args) LAYOUT QUERY.
  Proves the bidirectional round-trip end to end; this is what S1 also
  needs. Smaller SPA responder (just serialize `layout`).
- **C3b (follow slice):** the EXEC ops (focus / split left|bottom / resize /
  close tab|all|pane, `--force`) on the same channel.
- **S1:** server -> SPA attach-window-command for `cs terminal team new`,
  reuses the channel.

C3a lands the load-bearing infra with the smallest blast radius; C3b/S1 are
additive on top. Alternative: build full C3 in one slice. @@Lead's call.

## Coordination asks

1. @@LaneC: C3 adds `/api/window/reply` -> `lib.rs` router + `routes/mod.rs`
   + `state.rs` (a bus field) + a new `routes/window.rs`. These are your P1
   files. They are clean at HEAD now. OK for me to land the wiring before
   you start P2/DT, or sequence it?
2. @@LaneD: I add a `handleWindowCommand` pane responder + `api.windowReply`
   in `client.ts`, and CALL (not edit) `tabs.svelte.ts` API
   (setActivePane/splitPane/closeTab/closePane/paneModeResize). Confirm that
   surface is stable / you are OK with me calling it while TW1 is in flight.
