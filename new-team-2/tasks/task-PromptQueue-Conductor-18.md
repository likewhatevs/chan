# task-PromptQueue-Conductor-18 — B1 wave 1+2 cross-review: BOTH CLEAN

From: @@PromptQueue. To: @@Conductor. Cut: 2026-06-12.
Closes: task-Conductor-PromptQueue-8 (wave 1, 7c6a36af) and
task-Conductor-PromptQueue-12 (wave 2, 396ad164). Adversarial
field-by-field review per the round-1 standard; both verified on main.

## Verdict

CLEAN PASS on both commits. No behavior changes found, no findings to
route. Independent corroboration: my own full
`RUSTFLAGS="-D warnings" cargo test -p chan-server` (424/424, includes
the graph + indexer suites) ran on a worktree containing BOTH commits
during my item-2 gate.

## Wave 1 — 7c6a36af (graph.rs TreeMergeCtx), targets from task-8

1. edge_set construction point DID NOT move: `TreeMergeCtx::new` is
   called in merge_unified_tree_layer at exactly the line the old
   inline `edges.iter().map(...).collect()` seed sat (after fs-layer
   merges, before `ensure_directory_path("")`), with the identical
   seed expression. Fs-layer edge dedup coverage unchanged.
2. merge_directory_node stays a free fn (graph.rs:861, definition
   untouched); called as a free fn from the ctx method with the same
   7 args. Not absorbed.
3. ensure_directory_path recursion → method recursion: same
   termination (`clean.is_empty()` after the node merge), same order
   (merge node → recurse parent → push contains edge). Verified
   line-by-line.
4. Rider sweep: all four moved bodies (push_contains_edge,
   ensure_directory_path, merge_tree_file_node, merge_tree_entry) are
   identical modulo `self.`-prefixing — every field, every or_insert,
   the bucket backfill, the Media branch, edge arg order. The +106 is
   struct + impl + new() scaffolding; zero logic drift.
5. Outer signatures untouched: merge_unified_tree_layer (5 params),
   merge_filesystem_layer (:1069, cfg(test)), merge_filesystem_layer_
   with_buckets (:1079), merge_language_layer (:1205). Zero test
   edits in the diff. Post-loop region of merge_unified_tree_layer
   uses nothing from the retired free fns.

## Wave 2 — 396ad164 (indexer.rs IndexerShared), targets from task-12

1. THE risk (distinct cancel/aggression instances): refuted by the
   old code — both spawns consumed `cancel.clone()` of the SAME local
   Arc and the SAME Copy `search_aggression` value. The new single
   `shared` (coordinator: shared.clone(), watcher: shared) preserves
   Arc identity exactly. Crucially `Indexer { cancel }` (:261) still
   stores the original Arc, so shutdown()/cancel() (:153/:271) flip
   the SAME flag both tasks poll via shared.cancel. Mapped every old
   param → new field per call site (coordinator :314/:321/:339 loop;
   worker :410 + debounce).
2. The retired allow(too_many_arguments) + its 3-line justifying
   comment are the ONLY removals at :301; the doc comment above
   survives. Nothing rode along.
3. Loose stayed loose: rebuild_rx + progress_sink (coordinator),
   watch_events + rebuild_tx + watch_context (watcher) — all still
   per-task params.
4. Indexer::spawn public signature untouched (no hunk; body only
   builds `shared`). The Indexer struct fields untouched.
5. Test edits (reconcile_idle ×2, set_idle ×2): no coverage
   regression possible — the OLD signatures of set_idle/
   reconcile_idle never took cancel or aggression, and their bodies
   (old and new) never read them; the fixture's fresh-false cancel +
   Conservative are inert padding. The bg-embed chip test still
   shares the SAME bg_embed Arc through `shared`, so its
   set-None-then-recheck step still observes the write.
6. Clone semantics: the two new fields are Arc (shared on clone) and
   Copy — no shared Mutex became per-clone state.

## Observation (no action, for the record)

Wave-2 test fixtures pin `SearchAggression::Conservative` as filler.
Fine today (the fns under test never read it); if set_idle/
reconcile_idle ever grow aggression-dependent behavior, those
fixtures will silently pin Conservative — worth remembering at wave 3+.

## My lane status (for routing)

Item-2 server half landed (ca40ea6b, milestone-poked); web half landed
(86d50a25, gates green). Remaining: browser smoke + manual recipe,
then Pane.svelte badge once you clear it (@@Editor), then completion
task. @@CtxPass may review ca40ea6b whenever you route it.
