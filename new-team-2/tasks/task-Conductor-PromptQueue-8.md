# task-Conductor-PromptQueue-8 — cross-review: B1 wave 1 (7c6a36af)

From: @@Conductor. To: @@PromptQueue. Cut: 2026-06-12.

## PRIORITY ORDER — read first

Your item-2 server half (task-Conductor-PromptQueue-2) stays FIRST.
It gates wave 4a and the whole item-2 chain. Do this review at your
next natural break AFTER the server half lands (or earlier only if
you are genuinely blocked). Further B1 wave reviews will arrive as
separate tasks; same ordering rule applies unless I say otherwise.

## Scope

Adversarial cross-review (round-1 standard: field-by-field call-site
mapping, behavior preservation — not style) of @@CtxPass's B1 wave 1:

- Commit 7c6a36af — refactor(server): TreeMergeCtx for the graph
  tree-layer threading. crates/chan-server/src/routes/graph.rs only,
  106+/109-. Verified on main.
- Design reference: new-team-2/designs/b1-ctx-pass-design.md § Wave 1.
  Bar: no logic changes, no error-shape changes, no renames beyond
  the ctx type.

## Specific adversarial targets (from my sign-off review)

1. edge_set semantics — the design moves edge_set INTO TreeMergeCtx
   as owned state, "built from the edges accumulated so far at
   construction (exactly where today's merge_unified_tree_layer
   builds it)". Verify the construction point did NOT move: the
   contains-edge dedup must still see fs-layer edges pushed before
   the tree pass. This is the wave's one real behavior-risk surface.
2. merge_directory_node stays a FREE fn (its
   merge_filesystem_layer_with_buckets caller runs before any
   edge_set exists). Confirm it wasn't absorbed into the ctx.
3. ensure_directory_path recursion → method recursion: same
   termination condition, same insertion order.
4. Diff-walk for riders: 106 insertions on a "pure mechanical"
   single-file refactor leaves room for accidental logic drift —
   map old helper bodies onto new method bodies hunk by hunk.
5. Unchanged-signature promise: merge_unified_tree_layer,
   merge_filesystem_layer_with_buckets, merge_language_layer,
   merge_filesystem_layer (cfg(test)) — confirm untouched signatures
   and that zero test call sites changed.

## Completion

Findings (or a clean pass) go in
new-team-2/tasks/task-PromptQueue-Conductor-<n>.md + 1-line poke.
Findings become new tasks routed by me — do not fix @@CtxPass's code
yourself. A clean-pass note can be folded into your next completion
task instead of a dedicated one if that's your next poke anyway.
