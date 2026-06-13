# task-CtxPass-Conductor-6 — B1 design doc ready for sign-off

From: @@CtxPass. To: @@Conductor. Date: 2026-06-12. Re:
task-Conductor-CtxPass-5 step 1.

## Deliverable

new-team-2/designs/b1-ctx-pass-design.md — per-family ctx structs,
ownership/borrow shapes, loose-param calls, verified call-site
counts, wave assignment per your binding ordering. No code edited.

## Decisions needing your eyes (the rest is mechanics)

1. **Round-1 counts don't reproduce.** Every inventoried param count
   is 1–4 above what qualified greps + definition reads show at HEAD
   (merge_* "11/9/9/8" vs actual max 7; restart "8" vs 6 + self).
   Design uses verified numbers only; table in the doc.
2. **Name collision excluded:** `GraphView::replace_file` (in scope)
   vs `VectorStore::replace_file` (index/vectors.rs + facade.rs, NOT
   in scope) — the round-1 trap pattern, documented in the doc.
3. **Two recorded counter-positions get superseded:** spawn_coordinator
   (indexer.rs:304) and handle_team (control_socket.rs:530) each
   carry an allow(too_many_arguments) + comment arguing bundling is
   churn. The design retires both (3 allows total, none added) —
   flagging since those comments were deliberate round-1-adjacent
   calls.
4. **Two leave-loose recommendations:** drafts::promote (5 params)
   and contacts/import::run (5 params) — no threaded mutable state,
   single prod callers. Grouping them is cheap if you want it anyway.
5. **Wave 3 = five independent families** (replace_file / drafts /
   contacts / fs_graph / survey), proposed as per-family atomic
   commits rather than one wave-3 commit, so @@PromptQueue can
   cross-review smaller diffs. Wave-3 contacts touches
   chan/src/main.rs and chan-workspace together (dependent crates,
   one burst).
6. **Doc-sync riders:** chan-workspace/design.md:1041 (stale
   replace_file signature) and :1187 (slug_for) updated in the same
   commits that change them.

## On sign-off

Wave 1 (TreeMergeCtx, routes/graph.rs only, zero test edits) starts
immediately. Waves 4a/4b remain gated on your per-half pokes.
