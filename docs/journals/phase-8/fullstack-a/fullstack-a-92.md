# fullstack-a-92 — Broadcast survey-reply echo fan-out (SPA intercept; option 2 routed)

Owner: @@FullStackA (primary; cross-lane to @@Systacean for chan-server WS frame)
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

When rich-prompt broadcast input is ON, the survey-
reply echo (`poke + chord` bytes) fans to all
selected broadcast targets, NOT just the originating
terminal session.

## Reference

[`../phase-8-bugs.md`](../phase-8-bugs.md) §"Broadcast
survey-reply echo doesn't fan to broadcast targets"
(line ~401).

Root cause: `dispatch_agent_event` writes the
`poke + chord` to a SINGLE session via
`send_input` — the session that originally owned
the survey event. The broadcast layer is SPA-side
+ only runs on rich-prompt SUBMIT.

## Routing decision: option 2 (SPA-side intercept)

The bug-list entry surfaces 2 architectures:

1. **Server-side fan-out**: server tracks broadcast
   target lists; `dispatch_agent_event` fans
   directly to all PTYs.
2. **SPA-side intercept**: server emits a WS frame
   instead of writing direct; SPA receives + fans
   per its existing broadcast layer.

**Routed option 2** — leverages the existing SPA
broadcast layer (one source of truth for broadcast
targeting); avoids new server state tracking SPA
selection changes.

## Scope

### chan-server side (cross-lane)

* `dispatch_agent_event` in
  `crates/chan-server/src/terminal_sessions.rs:502`:
  STOP writing the echo directly. Instead emit a
  WS frame describing the intended echo
  (target session + payload bytes).
* WS frame shape: `{type: "agent_event_echo",
  session_id, payload_bytes}`.

### SPA side (primary)

* WS handler receives `agent_event_echo` frames.
* If broadcast input is ON for the session: fan
  the payload to all broadcast target PTYs (uses
  existing fan-out path from `-a-31`).
* If broadcast is OFF: write to the single
  originating session (current behavior).

### Connection-drop edge

Bug-list flagged the SPA-PTY connection-drop case
between reply landing + WS echo. Mitigation:
chan-server retries the WS emit on reconnect OR
queues the echo briefly. Implementer's call on the
retry shape; document the chosen approach.

## Cross-lane scope

* @@FullStackA primary (SPA WS handler + fan-out
  wiring).
* @@Systacean chan-server `dispatch_agent_event`
  swap + WS frame emit.

@@FullStackA fires scope-poke once the SPA shape
is stable; @@Systacean lands the chan-server piece.

## Acceptance

1. **Broadcast ON + survey reply**: clicking
   reply option fans `poke + chord` to all
   broadcast targets.
2. **Broadcast OFF + survey reply**: echoes to
   the single originating session (no regression).
3. **Connection-drop**: graceful (per chosen
   mitigation).

### Tests

* SPA: WS-frame handler + fan-out dispatch.
* chan-server: `dispatch_agent_event` emits frame
  instead of writing direct.

### Gate

`cargo` + `npm` gates green.

## Authorization

Yes for SPA WS handler + `crates/chan-server/src/terminal_sessions.rs`
+ tests + task tail + outbound.

## Numbering

This is `-a-92`.
