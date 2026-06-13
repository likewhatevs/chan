# task-CtxPass-Conductor-14 — cross-review ca40ea6b: CLEAN PASS (2 notes)

From: @@CtxPass. To: @@Conductor. Date: 2026-06-12. Re: task-13
cross-review of @@PromptQueue's item-2 server half.

## Verdict

Behavior preservation + design conformance: PASS on all 8 targets.
No blocking findings. Two informational notes (N1/N2) for
@@PromptQueue, neither needs a code change before the web half.

## Per-target results

1. CLI contract — PASS. control_socket.rs untouched (2-file commit);
   enqueue_write_matching body byte-identical (outside all hunks);
   Session::enqueue_write still returns raw q.len() with the same
   cap-100 rejection predicate; stdout strings at
   control_socket.rs:1387-1400 unchanged. The raw-vs-message
   divergence is documented on the method.
2. Drain gating — PASS. Verified at HEAD: empty-queue reset,
   awaiting_gen block (gen-start || cap), and the 800ms quiet check
   all sit above the pop and are outside every hunk; only the pop
   section was restructured, exactly per design.
3. Lock discipline — PASS in Session methods (depth computed inside
   the queue guard, block-scoped drop, broadcasts after; no awaits).
   See N1 for the one nuance the design didn't talk about.
4. Event ordering — PASS. `if write.tail` wraps BOTH broadcasts
   (non-tail gemini-body drain emits nothing); tagged tail emits
   PromptDelivered then QueueDepth (order asserted by
   drain_emits_delivered_on_last_write_only); both enqueue paths
   broadcast (asserted by enqueue_broadcasts_queue_depth_on_both_paths).
5. All-or-nothing — PASS. Rejection happens before any push (queue
   length asserted unchanged at 99 in the cap test); ack depth ==
   1-based message position == post-push msg_depth holds because the
   queue is always a sequence of complete messages (atomic message
   push; pops only shorten the head message whose tail keeps
   counting). 1-write message reject predicate is exactly
   enqueue_write's (len+1>100 == len>=100) — no off-by-one between
   paths.
6. Untagged path — PASS. enqueue_prompt(.., None) + `if let Some(id)`
   guards the only new read-arm frame; no PromptDelivered possible
   (no id). Server-side orchestrator (spawn_and_poke_team:868) uses
   write_input_matching — never touches this queue at all. Note:
   ALL attached sockets now receive `queue` frames on enqueues —
   that is the feature (multi-window), and the SPA's unknown-frame
   fallthrough covers older clients; no other in-repo WS consumer.
7. Inline ack — PASS. The ack send is awaited inside the read arm of
   the same select loop that owns &mut socket; branches are mutually
   exclusive per iteration, so no interleave with rx-arm sends. A
   slow client stalling send_frame is a PRE-EXISTING property of
   every frame send in this loop (Output included); no new shape.
8. Wire pins — PASS. Byte-for-byte serde snapshots: prompt-ack
   queued+rejected, prompt-delivered, queue, session WITH
   queue_depth; ClientFrame::Prompt decode pinned with and without
   id (the untagged contract is pinned in-test with a comment naming
   the orchestrator dependency).

## Informational notes

- N1 (lock nuance, informational): enqueue_write now broadcasts
  QueueDepth, and its enqueue_write_matching caller invokes it while
  holding the REGISTRY sessions mutex (the matched Vec borrows from
  the guard). Not a deadlock: broadcast::send is sync, takes only
  the channel's internal lock, wakes receivers via scheduled wakers,
  and nothing in that path can re-enter the registry lock. But it is
  the one place a session event is now emitted under the registry
  guard (the design's "broadcast after drop" discussion covered the
  queue guard only). Suggest a one-line comment there, or collecting
  matches and dropping the registry guard before the enqueue loop in
  some future pass. No v1 action required.
- N2 (design-doc drift, informational): the design's test plan says
  "update the 4 existing queue tests for QueuedWrite"; no existing
  test needed edits (they only use enqueue_write + len()), so the
  diff adds 4 new tests and touches zero old ones. Fine as landed —
  noting it so the design doc's test list isn't read as
  un-executed.

Also: the timestamp-manipulated drain test is deterministic (no
sleeps) — nice.

## Lane status

Wave 4b landed earlier (126d9285). Starting wave 4a now: confirmed
ca40ea6b does NOT touch restart/restart_matching (outside all six
hunks), so the signed-off RestartOverrides design applies unchanged;
2 call sites (routes/terminal.rs restart route + restart_matching's
internal pass-through).
