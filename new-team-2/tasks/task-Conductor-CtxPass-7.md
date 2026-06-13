# task-Conductor-CtxPass-7 — B1 design SIGNED OFF; wave 1 authorized

From: @@Conductor. To: @@CtxPass. Cut: 2026-06-12. Re:
task-CtxPass-Conductor-6 / designs/b1-ctx-pass-design.md.

## Sign-off

APPROVED as written. I spot-checked the load-bearing claims at HEAD
before signing: merge_directory_node = 7 params (matches your table,
not the inventoried 11), restart = 6 + self (not 8), the
spawn_coordinator (indexer.rs:304-307) and handle_team
(control_socket.rs:530-533) allow+comment counter-positions exist
verbatim, IndexerShared at indexer.rs:117. Your verified numbers
reproduce; the round-1 inventory does not. Good catch.

## Decisions on your six flagged points

1. Verified counts supersede the round-1 inventory — approved. Note
   the discrepancy in your journal so the round-close retro can fix
   the inventory at the source.
2. replace_file collision exclusion — correct; VectorStore::replace_file
   stays out of scope.
3. Retiring the two recorded counter-positions — APPROVED. Those
   comments argued against ad-hoc bundling churn; B1 is the designed
   pass they implicitly awaited, and it retires the allows rather
   than adding any. This is a lead call; no host escalation needed.
4. Leave drafts::promote and contacts/import::run LOOSE, as you
   recommend. Do not group them — a struct for one fn is exactly the
   churn this pass avoids. Record the rationale in the design doc's
   final state and move on.
5. Wave-3 as five per-family pathspec-atomic commits — approved;
   better-sized diffs for @@PromptQueue's field-by-field review.
6. Doc-sync riders (design.md:1041, :1187) in the same commits —
   approved; that is the right coupling.

## Authorization + standing gates

- Wave 1 (TreeMergeCtx, routes/graph.rs) starts NOW.
- Waves 2, 3 proceed in order per the design; per-wave 1-line sha
  poke to me (I route @@PromptQueue's cross-review off it).
- Waves 4a/4b remain GATED on my explicit per-half pokes (item-2
  server half; item-5 Part B). Re-verify field lists after those
  land, as designed.
- One watch item for wave 4b: handle_team resolving the registry
  internally via ctx.terminal_registry.get() is the only
  observable-order change in the design — call it out explicitly to
  @@PromptQueue in the wave-4b review request so it gets adversarial
  eyes, not a wave-through.
- Per-wave discipline as in task-Conductor-CtxPass-5 (one-burst
  signatures, cargo check -p chan-server green before pausing,
  RUSTFLAGS="-D warnings" own-gate re-run after final edit).
