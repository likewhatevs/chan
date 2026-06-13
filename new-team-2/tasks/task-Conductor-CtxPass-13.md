# task-Conductor-CtxPass-13 — wave 4a RELEASED + cross-review: item-2 server half (ca40ea6b)

From: @@Conductor. To: @@CtxPass. Cut: 2026-06-12.

## Gate release

Wave 4a (terminal_sessions::restart → RestartOverrides) is RELEASED:
@@PromptQueue's item-2 server half landed at ca40ea6b (verified on
main; terminal_sessions.rs + routes/terminal.rs only). Both wave-4
gates are now open. Binding order unchanged: finish your wave-3
family commits first.

## Ordering within your lane (recommended, not binding)

Do the cross-review below BEFORE starting wave 4a — it forces the
at-HEAD read of terminal_sessions.rs that the design already
requires of you ("field list re-verified after item-2 lands; new
restart inputs become fields, not extra params"). One read, two
deliverables.

## Cross-review scope: ca40ea6b (pairing per round plan)

Adversarial, behavior preservation + design conformance.
Design: designs/item-2-prompt-queue-visibility.md (wire contract +
server changes sections). 2 files, 387+/49-.

Specific targets:

1. CLI contract: enqueue_write's raw-length return + EnqueueOutcome
   (control_socket.rs:1374) + `cs terminal write` stdout + cap 100 —
   byte-for-byte. control_socket.rs is claimed untouched; verify the
   CALLERS' observable behavior, not just the file diff.
2. Drain gating: everything ABOVE the pop in try_drain_one (800ms
   output-quiet, gen-start await, cap) untouched — the team poke bus
   rides this; any drift hits every team session.
3. Lock discipline: depth computed inside the std-mutex guard,
   broadcasts AFTER drop, no awaits added in Session methods (the
   design pins this; a sync broadcast::send inside the guard is the
   deadlock-adjacent failure shape).
4. Event ordering: tagged tail drain → PromptDelivered THEN
   QueueDepth; non-tail (gemini body) drains emit NOTHING; both
   enqueue paths broadcast QueueDepth.
5. All-or-nothing: rejection at cap leaves the queue byte-identical
   (no partial push); ack depth == 1-based message position ==
   post-push msg_depth.
6. Untagged path: ClientFrame::Prompt without id (team orchestrator
   lead-identity poke) is fire-and-forget, zero new frames to that
   client — the one in-repo contract the design says must not break.
7. Inline prompt-ack on the submitting socket inside the read arm:
   confirm it cannot interleave/deadlock with the rx-arm frame sends
   (same socket, select loop).
8. Wire pins: serde snapshots cover prompt-ack (both variants),
   prompt-delivered, queue, session.queue_depth.

## Completion

Findings (or clean pass) →
new-team-2/tasks/task-CtxPass-Conductor-<n>.md + 1-line poke. May be
folded into your wave-3 sha pokes if that's your next poke anyway.
Findings become tasks routed by me — @@PromptQueue fixes their own
lane.
