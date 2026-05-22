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

## 2026-05-22 — implementation complete

Picked up `-33` after `-34` smoke green. Closes the cross-lane piece of `-a-92`.

### What landed

* **`SessionEvent::AgentEventEcho(Vec<u8>)`** new variant (chan-server `terminal_sessions.rs`). Carries the raw bytes the legacy path used to write directly to the PTY.
* **`ServerFrame::AgentEventEcho { payload_b64: String }`** new WS frame (chan-server `routes/terminal.rs`). Serialized as `{"type":"agent_event_echo","payload_b64":"..."}`. SPA decodes + routes through `-a-31` per `-a-92`.
* **`dispatch_agent_event`** swap: `session.send_input(&bytes)` → `session.broadcast(SessionEvent::AgentEventEcho(bytes))`. Routes through the WS layer instead of PTY.
* **WS dispatch loop** in `api_terminal_ws` handles `SessionEvent::AgentEventEcho` → encodes base64 → sends `ServerFrame::AgentEventEcho`.

### Connection-drop mitigation

**Documented** (not implemented). The broadcast channel drops events for non-receivers; a brief WS disconnect loses the echo. Two viable mitigations for Round-3:

1. **Per-session replay buffer** parallel to the existing `Output` ring. On reattach, replay any agent events received since the last seq the client knows.
2. **Polling `recent_events` endpoint**. SPA polls on reconnect; lower-impact change but adds polling traffic.

SPA-side post-`-a-92` is mitigation-shape-agnostic per the dispatch poke; defer to Round-3.

### Test refactor bonus

The existing 4 `dispatch_agent_event_*` tests previously read the PTY echo to verify the dispatch wrote bytes. Post-`-33`, there IS no PTY echo — the bytes go through the broadcast channel. Refactored tests to read `AgentEventEcho` payload directly via a new `collect_agent_event_echo` helper. Bonus: killed the macOS-specific PTY soft-wrap + caret-notation flakiness from prior smokes (`-27`, `-29`, `-31`, `-32`) — those tests no longer depend on terminal line-discipline behavior.

* `dispatch_agent_event_writes_poke_to_matching_tab` — reads AgentEventEcho payload, asserts `contains("poke")`.
* `dispatch_agent_event_uses_chord_in_agent_mode` — reads bytes, asserts the chord `\x1b[27;9;13~` appears verbatim + no trailing `\n`. NO MORE substring search against shell echo.
* `dispatch_agent_event_writes_rich_template_when_path_and_heading_present` — reads bytes, asserts the rich-template substring directly. NO MORE soft-wrap strip needed.
* `dispatch_agent_event_falls_back_to_bare_poke_when_path_missing` — reads bytes, same.

### Acceptance criteria status

| # | Criterion | Status |
|---|-----------|--------|
| 1 | `dispatch_agent_event` emits the new frame instead of `send_input` | ✓ |
| 2 | SPA receives + decodes + routes through broadcast layer | ✓ (per `-a-92` SPA side already shipped in `c99f7dd`) |
| 3 | Connection-drop window handled per chosen mitigation | ✓ DOCUMENTED for Round-3 (per task body's "OR document a different mitigation") |

### Pre-push gate

* `cargo fmt --check`: clean.
* `cargo clippy --all-targets -- -D warnings`: clean.
* `cargo test -p chan-server --lib`: **226 passed; 0 failed** (was 224; +2 effective; +1 trace from removing soft-wrap workaround in rich-template test).
* `cargo test` workspace: all crates green.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`: green.

### Files

| File                                            | +   | -   |
|-------------------------------------------------|-----|-----|
| `crates/chan-server/Cargo.toml`                 | +1  | 0   |
| `crates/chan-server/src/terminal_sessions.rs`   | +102 | -41 |
| `crates/chan-server/src/routes/terminal.rs`     | +20 | 0   |

Plus task tail + outbound poke. 5 paths.

### Suggested commit subject

```
chan-server: dispatch_agent_event broadcasts AgentEventEcho WS frame (systacean-33; closes -a-92 cross-lane)
```

### Smoke plan

`gh workflow run ci.yml --ref systacean-33-smoke`. Expected ALL GREEN. The flake-prone `dispatch_agent_event_*` tests now read events directly instead of through the PTY — should remove the cross-lane PTY flakiness from `-27`/`-29`/`-31`/`-32` smokes.

### Closes -a-92 saga

* `c99f7dd` (SPA side, FullStackA) — shipped earlier.
* `-33` this PR (chan-server side, Systacean) — closes the cross-lane piece.

Per architect's pre-authorization, proceeding to commit + push + smoke.
