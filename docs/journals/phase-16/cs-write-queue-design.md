# `cs terminal write` pending-input detection + queue — feasibility + design (@@LaneA)

DESIGN-FIRST per @@Lead. The @@Host ask: `cs terminal write` should detect
when the target has PENDING INPUT and QUEUE the write instead of delivering
it (return a queue#); queue bound 100; per-process binding dropped on
recycle. @@Lead flagged the crux: DETECTING pending input. This doc assesses
the signal and proposes a design. No code yet.

## The crux: why "pending input" is hard to observe

The server owns the PTY master: it WRITES bytes to the slave
(`Session::send_input`, terminal_sessions.rs:887) and READS the slave's
output into a replay ring (`record_output`, :952). The agents (claude /
codex / gemini) run in RAW MODE inside the ALT SCREEN (`in_alt_screen`,
:660) — a fullscreen TUI that owns the whole grid.

Consequences:
- **No line-discipline buffer to read.** Raw mode means every typed byte is
  delivered straight to the agent; the kernel holds no "typed but not yet
  submitted" buffer for the server to introspect. The compose buffer lives
  in the AGENT PROCESS's heap.
- **The output ring is rendered pixels, not structure.** The server's byte
  stream is the agent's TUI redraw, not a `{compose_len: N}` field. "Has
  unsubmitted text" is only heuristically recoverable from the rendered
  screen, and that rendering is agent-specific.

So "pending input" (unsubmitted compose content) is NOT directly observable
server-side, and only fragilely observable from the rendered screen.

## Signal assessment (the four @@Lead named)

1. **PTY introspection** — NOT VIABLE. Raw mode -> no line-discipline buffer;
   the compose state is in the agent's memory, opaque to the PTY layer.

2. **Keystroke / activity heuristic (server-side)** — PARTIALLY VIABLE, and
   cheapest. The session already tracks `last_activity` (every `send_input`
   + `record_output` bumps it, :888/:956) and emits an `Activity` signal for
   unfocused output (:973). Refine into TWO timestamps:
   - `last_output_at` — the agent is rendering / generating;
   - `last_user_input_at` — a keystroke arrived from the SPA WS (distinct
     from a server `cs`-write; the WS-input path marks it).

   BUSY = output within QUIET_MS OR user-input within TYPING_MS; FREE = quiet
   on both. Reliably detects "agent generating" + "human actively typing."
   KNOWN GAP: a session idle-at-prompt with a half-typed-but-PAUSED compose
   buffer reads FREE (no recent activity), so a delivered poke could land
   mid-buffer. This residual is the genuinely hard case.

3. **SPA-via-C3 (xterm screen introspection)** — RICHER BUT FRAGILE +
   CONDITIONAL. The C3 window channel (just landed) lets the server ask the
   SPA, which hosts xterm.js with the live screen buffer + cursor. The SPA
   could heuristically detect "cursor on a non-empty input line" — but the
   compose box is agent-TUI-specific (claude vs codex vs gemini differ), so a
   generic detector is fragile + high-maintenance. AND it only works when an
   SPA is ATTACHED; a headless CLI-spawned agent (no browser tab) has no
   xterm screen — the server has only the raw ring. NOT RECOMMENDED as the
   primary signal; possible future refinement for attached sessions.

4. **Serialization (FIFO + per-session write lock)** — VIABLE + ORTHOGONAL.
   Does not detect compose state, but guarantees concurrent `cs terminal
   write` calls don't INTERLEAVE bytes into one PTY (poke A's text + poke B's
   text mixing). That is a real race the queue must also solve; the queue IS
   the serialization, independent of the pending-input signal.

## Recommended design

- **Detection = dual server-side quiescence (signal 2).** Add per-session
  `last_output_at` + `last_user_input_at` atomics, extending the existing
  `last_activity` plumbing (the reader thread already calls `record_output`;
  the WS-input path already calls `send_input`). BUSY when either is inside
  its window (start ~ output 400ms, input 1500ms; tune live), else FREE.
- **Queue = bounded FIFO (100) per session.** A sidecar map keyed by
  `session_id` in the terminal registry. `cs terminal write`: if the target
  is FREE, deliver immediately (return "delivered", no #); if BUSY, enqueue
  (return the queue position #). At 100, reject with "queue full".
- **Drain on BUSY->FREE.** A per-session drain (a debounced tick, or hooked
  off `record_output` going quiet) delivers queued writes IN ORDER once the
  target is FREE, each with its submit chord (the existing
  `apply_submit_chord` path).
- **Binding = per-session (PTY process), dropped on recycle.** The queue
  lives with the session; `Registry::{close,restart}` already drop the
  session, so the sidecar queue drops with it. Matches "per-process binding
  dropped on recycle" exactly.
- **Return = queue#.** Extend the `cs terminal write` control response to
  carry either "delivered to N session(s)" (immediate, today's behavior) or
  "queued at position # for <tab>" when the target was busy.

## Honest limitations

- Detects "busy generating / human typing," NOT "compose buffer has
  unsubmitted content." A PAUSED half-typed buffer reads FREE. Fully solving
  it needs either agent cooperation (the agent declaring its compose state —
  not available for external agents) or fragile per-agent TUI parsing
  (signal 3). Recommend shipping the quiescence-based queue (it covers the
  dominant agent->agent poke race + the active-typing race) and treating
  perfect compose-detection as out of scope.
- QUIET_MS / TYPING_MS are empirical; validate against real claude/codex
  streaming + typing before locking the values.

## Open questions for @@Host / @@Lead

1. **Always-queue vs opt-in `--queue`?** I lean OPT-IN: existing pokes that
   want immediate fire keep working; `--queue` (or `--when-free`) adds the
   wait-for-free behavior. An always-on queue changes every poke's timing.
2. **Is the known gap (paused half-typed buffer reads free) acceptable for
   v1?** I believe yes — it is the rare case and the unsolvable-without-agent-
   cooperation one. Flagging so it is a conscious call, not a silent miss.
3. **Drain behavior**: auto-deliver-on-free (recommended) vs hold until the
   caller polls? Auto-deliver is the useful behavior; the queue# is then an
   informational receipt, not a handle the caller must redeem.

## Decisions (from @@Lead 09:38)

- Design APPROVED in shape (dual-quiescence + bounded-100 FIFO + drain-on-
  free + queue-dropped-on-recycle).
- **Q3 DECIDED: auto-deliver on free.** Queued writes flush automatically on
  the BUSY->FREE transition; the queue# is an informational receipt, not a
  handle the caller must redeem.
- **Build HELD** pending @@Host on Q1 (opt-in `--queue` vs always-on) and Q2
  (is the paused-half-typed-buffer gap acceptable for v1).
- **At build time: document the limitation in `--help`.** The `cs terminal
  write` (and/or the `--queue` flag) help text must state that the queue
  waits for the target to be IDLE (not generating / not actively typed-into),
  and that it cannot detect a paused, half-typed compose buffer — so a poke
  may still land mid-buffer in that rare case.

## Footprint (when greenlit)

All in my lane: terminal_sessions.rs (the two timestamps + the per-session
queue + drain), control_socket.rs (the `term_write` queue path + the queued
response), chan-shell wire.rs/cli.rs (`--queue` flag + the queued-response
shape). No @@LaneC seam; no SPA change unless we later add signal 3.

## Decisions (from @@Host, via @@Lead 10:40) -- SUPERSEDES the holds above

@@Host reframed the feature: it is NOT about detecting a human typing into
the terminal. It is about SERIALIZING `cs terminal write` deliveries so
queued messages submit AFTER EACH OTHER automatically. Build is RELEASED.

- **Q1 ANSWERED: ALWAYS-ON.** Every `cs terminal write` enqueues. A FREE
  target drains instantly (queue of one); a BUSY target enqueues + auto-
  drains. No `--queue` flag. Rationale: the intended workflow routes ALL
  input through this API -- including a future FLOATING INPUT BUBBLE whose
  Enter calls the same `cs terminal write`, so messages always enqueue
  properly and submit after each other. Always-on is REQUIRED for that; an
  opt-in flag would let the bubble's writes race.
- **Q2 DROPPED, not deferred: do NOT detect human typing.** Detection narrows
  to ONE signal: IS THE AGENT GENERATING? (output quiescence /
  `last_output_at`). DROP `last_user_input_at` as a gating signal -- the queue
  OWNS the input path, so direct human-into-terminal typing is a non-case in
  the intended workflow, and the paused-half-typed-compose-buffer gap is
  therefore MOOT. Simpler than dual-quiescence: one signal, not two.
- **Drain semantics (the core value):** FIFO; deliver the next queued message
  ONLY when the agent is IDLE (its previous turn's output has quiesced), each
  with its submit chord, so the chain auto-submits one after another. After a
  deliver+submit, AWAIT the agent's generation-START before the next delivery
  (don't fire two messages into one compose during the post-submit, pre-
  generation window -- a brief settle/debounce on `last_output_at`).
- **UNCHANGED:** bounded-100 per-session FIFO, dropped on session recycle,
  auto-deliver (Q3 stands), return queue# when busy / "delivered" when free.
  `--help` documents the queue waits for the TARGET AGENT to finish generating
  before delivering the next message.
- **Footprint shrinks:** only `last_output_at` plumbing (not the second input
  timestamp); same files (terminal_sessions + control_socket + chan-shell).
- **BUILD RELEASED.** No remaining @@Host holds.
