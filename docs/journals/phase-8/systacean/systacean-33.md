# systacean-33 — chan-server dispatch_agent_event: emit agent_event_echo WS frame (cross-lane piece of -a-92)

Owner: @@Systacean
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Swap `dispatch_agent_event` from direct PTY write to
WS frame emit. The SPA side (`-a-92 SPA-side` shipped
in `c99f7dd`) decodes + routes through its existing
broadcast layer.

## Reference

@@FullStackA's scope-poke at the tail of
[`../fullstack-a/fullstack-a-92.md`](../fullstack-a/fullstack-a-92.md).

Routing option 2 (SPA intercept) is in HEAD on the
SPA side; this task closes the cross-lane piece.

## Scope

`crates/chan-server/src/terminal_sessions.rs::dispatch_agent_event`
(line ~527):

1. Compute the `bytes` Vec as today (poke_text + chord).
2. Replace `session.send_input(&bytes);` with a
   WS-frame emit:
   ```rust
   session.send_text_frame(json!({
     "type": "agent_event_echo",
     "payload_b64": base64::encode(&bytes),
   }))
   ```
   (or equivalent — match the existing JSON-frame
   emit shape used by "ready" / "session" / "cwd").
3. **Connection-drop handling**: buffer the frame
   for ~5s + emit on reconnect, OR document a
   different mitigation. SPA side is mitigation-
   shape-agnostic.

## Acceptance

1. `dispatch_agent_event` emits the new frame
   instead of calling `send_input`.
2. SPA receives + decodes + routes through
   broadcast layer (per `-a-92`).
3. Connection-drop window handled per chosen
   mitigation.

### Tests

* Unit: `dispatch_agent_event_emits_agent_event_echo_frame`.
* Backward-compat: connection-drop mitigation.

### Gate

`cargo fmt / clippy / test`; smoke green.

## Coordination

* @@Systacean lane.
* @@WebtestA empirical walks after this lands
  closes the full `-a-92` saga.

## Authorization

Yes for `crates/chan-server/src/terminal_sessions.rs`
+ tests + task tail + outbound.

## Numbering

This is `-33`.
