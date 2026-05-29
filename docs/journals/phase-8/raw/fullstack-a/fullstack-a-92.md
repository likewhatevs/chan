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

## 2026-05-22 — SPA-side ready for review (chan-server side scope-poke routed)

Two-file change. SPA-only. Cross-lane piece
(chan-server `dispatch_agent_event` swap)
scope-poked to architect for @@Systacean
routing.

### What landed (SPA)

`web/src/components/TerminalTab.svelte`:

* `ServerFrame` discriminated union
  extended with
  `{ type: "agent_event_echo"; payload_b64:
  string }`. Doc-comment cites the
  broadcast-layer reuse (`-a-31`) + base64
  framing rationale (chord may include
  non-UTF8 bytes per `-b-13`'s submit-mode
  chord).
* WS handler branch on
  `frame.type === "agent_event_echo"`:
  decode → `sendUserInput(payload)` →
  existing broadcast layer fans
  automatically.
* New `decodeAgentEventEcho(payload_b64)`
  helper. Returns null on malformed b64 so
  the WS handler short-circuits without
  throwing (a corrupt echo still passes
  JSON.parse + type discriminator).

`web/src/components/agentEventEchoFanout.test.ts`
(new): 5 raw-source pins:
* ServerFrame union extended.
* Rationale comment cites broadcast +
  base64.
* WS branch decodes + calls
  `sendUserInput`.
* `decodeAgentEventEcho` uses `atob` +
  null-soft on malformed.
* `sendUserInput` preserves the broadcast
  fan-out pair.

### Architecture rationale (option 2 routed)

The SPA-side intercept (vs server-side
fan-out) leverages the existing `-a-31`
broadcast layer + keeps the targeting state
in ONE place (the SPA's
`tab.broadcastEnabled` +
`terminalBroadcastMemberIds`). Server-side
fan-out would have required new state
tracking SPA selection changes.

Routing the payload through
`sendUserInput` (instead of `sendInput`
alone) is the key insight:
* `sendInput(data)` → WS frame to server
  → PTY write (origin session).
* `broadcastTerminalInput(tab, data)` →
  fan to selected broadcast targets (no-op
  when broadcast is off for this session).

When broadcast is OFF, behaviour matches
today (single-session echo). When ON, the
same bytes reach every target.

### Connection-drop edge

The bug-list flagged this. Mitigation:
the SPA's existing reconnect logic in
`connect()` (line ~448) sends a new
`session` frame with `lastSeq` so the
server replays missed bytes. If a
`agent_event_echo` arrived during the
WS gap, the server's
`dispatch_agent_event` retry would need to
fire on reconnect.

**Implementer's call**: chan-server should
buffer `agent_event_echo` frames briefly
during WS-disconnected windows + emit on
reconnect. Documenting the contract here;
@@Systacean lands the actual buffering
shape.

### Cross-lane scope-poke (TO @@Systacean via architect)

`crates/chan-server/src/terminal_sessions.rs`
`dispatch_agent_event` (line ~527):
1. Compute the `bytes` Vec as today
   (poke_text + chord).
2. Replace
   `session.send_input(&bytes);`
   with a WS-frame emit:
   `session.send_text_frame(json!({
     "type": "agent_event_echo",
     "payload_b64": base64::encode(&bytes),
   }))` (or equivalent — match the
   existing JSON-frame emit shape used by
   "ready" / "session" / "cwd").
3. On connection-drop window: buffer the
   frame for ~5s + emit on reconnect (or
   document a different mitigation; the
   SPA side is mitigation-shape-agnostic).

Server-side tests: add a unit test
asserting `dispatch_agent_event` emits the
new frame instead of calling
`send_input`.

### Acceptance

1. **Broadcast ON + survey reply**: echoes
   to all selected broadcast targets ✓
   (SPA mechanism via tests; @@WebtestA
   walks after the chan-server piece
   lands).
2. **Broadcast OFF + survey reply**:
   echoes to single originating session ✓
   (no-op fan-out path).
3. **Connection-drop**: graceful per
   @@Systacean's chosen mitigation.

### Gate

* vitest **1019 / 1019** (+5 net from
  `-a-66 slice e`'s 1014).
* svelte-check 0 errors / 0 warnings across
  4036 files.
* npm build clean.
* Rust gate not re-run (chan-server side
  pending @@Systacean).

### Decisions

* **Option 2 (SPA intercept)** per the
  bug-list routing — leverages existing
  broadcast layer.
* **Base64 framing** — payload includes
  non-UTF8 bytes (modifyOtherKeys chord);
  base64 round-trips cleanly through JSON
  WS frames without escape-string
  contortions.
* **`atob` + null-soft decode** — malformed
  echoes don't throw; they no-op.
* **Single-helper route via `sendUserInput`**
  — broadcast fan-out is automatic + the
  source of truth for targeting stays in
  the SPA.

### Suggested commit subject

```
Terminal: SPA-side agent_event_echo handler (broadcast fan-out via existing -a-31 layer) (fullstack-a-92 SPA-side)
```

Single commit. Frame discriminator + handler
branch + decoder + 5 test pins. Chan-server
side scope-poked separately.

### Files for `git add` (per-path discipline)

* `web/src/components/TerminalTab.svelte`
* `web/src/components/agentEventEchoFanout.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-92.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance + the
@@Systacean chan-server-side landing that
closes the cross-lane piece.
