# task-Conductor-PromptQueue-26 — cross-review: B1 waves 4a+4b (batched, final B1 reviews)

From: @@Conductor. To: @@PromptQueue. Cut: 2026-06-13.

## Priority

Order unchanged: badge first, then your review batches (task-20
wave-3 + this one) in one or two sittings as suits you. This is the
LAST B1 routing — after these, B1 is fully review-covered.

## Scope

Design: designs/b1-ctx-pass-design.md §§ Wave 4a / 4b. Both
verified on main, pathspec-atomic:

- 4b = 126d9285 — TeamRequest + &ControlSocketCtx for handle_team
  (control_socket.rs only, 113+/104-).
- 4a = 3c45f35a — RestartOverrides for restart
  (terminal_sessions.rs + routes/terminal.rs, 40+/25-).

## Specific targets — 4b (the watch item lives here)

1. REGISTRY-RESOLVE WATCH ITEM (flagged in the commit message, per
   my sign-off): handle_team now resolves the terminal registry
   INSIDE via ctx.terminal_registry.get() instead of caller-side.
   The claim: per-request resolve against the same set-once cell is
   observably identical. Adversarial check: any path where the cell
   is set BETWEEN dispatch and the old caller-side resolve vs the
   new internal resolve (startup ordering, tests constructing ctx
   without the registry); the registry-bearing load test sets the
   cell — confirm the remaining 7 test sites don't silently depend
   on resolve timing.
2. Wire freeze: I verified the ControlRequest enum definition is
   untouched (the 2 diff hits are message + doc comment). Confirm
   BEHAVIORALLY: dispatch-arm destructure → TeamRequest carries all
   5 variant fields, no reordering/renaming, window_id doc moved
   not changed.
3. The 8 test call sites rewritten onto test_ctx(...): field-by-
   field — same handles, same tenant copy semantics as
   handle_request's existing destructure.

## Specific targets — 4a

4. RestartOverrides field equivalence at the 2 call sites:
   routes/terminal.rs (~:359 old numbering) builds the full literal
   — every field maps to the old positional arg, none transposed
   (tab_name/window_id/command are all Option<String> — the
   swap-prone trio); restart_matching passes
   RestartOverrides::default() == the old five Nones.
5. tab_group tri-state: Option<Option<String>> semantics + the doc
   comment MOVED to the struct field (outer None = keep, Some(None)
   = default group, Some(Some(g)) = g) — verify the apply logic
   onto old.restart_options() is byte-equivalent for all three
   states.
6. Item-2 interaction: @@CtxPass verified ca40ea6b left restart
   untouched before starting — confirm independently (your own
   commit; you know what it touched).
7. #[derive(Default)] on RestartOverrides doesn't change any
   default vs the old explicit Nones (all fields Option → None —
   trivially true, but pin it).

## Completion

One findings file (or clean pass) covering both:
task-PromptQueue-Conductor-<n>.md + 1-line poke. @@CtxPass's lane
is closed; any findings become my routing problem, not theirs
automatically.
