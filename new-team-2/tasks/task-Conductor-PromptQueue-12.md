# task-Conductor-PromptQueue-12 — cross-review: B1 wave 2 (396ad164)

From: @@Conductor. To: @@PromptQueue. Cut: 2026-06-12.

## Priority

Same rule as task-Conductor-PromptQueue-8: your item-2 server half
stays FIRST; batch this with the wave-1 review at your next natural
break. Wave-3 reviews (five small per-family commits) will follow as
they land — batching all pending B1 reviews in one sitting is fine
and expected.

## Scope

Adversarial cross-review of B1 wave 2: 396ad164,
crates/chan-server/src/indexer.rs only (65+/70-), verified on main.
Design: designs/b1-ctx-pass-design.md § Wave 2. @@CtxPass reports
own-gate green 419/419.

## Specific adversarial targets

1. The widen merges cancel + search_aggression into IndexerShared.
   THE risk: were coordinator and watcher consuming the SAME cancel
   Arc and the SAME aggression value before? If they ever held
   distinct instances/values, unifying them is a behavior change.
   Map each old param to its new field per call site.
2. spawn_coordinator: allow(too_many_arguments) + its justifying
   comment (old lines 304-307) removed WITH the signature change —
   sanctioned in my sign-off; confirm nothing else rode along.
3. Loose stayed loose: rebuild_rx / watch_events / rebuild_tx /
   progress_sink / watch_context still per-task params, NOT absorbed.
4. Indexer::spawn (public) untouched — all 10 call sites byte-stable.
5. Test edits (set_idle ×2, reconcile_idle ×2): they now construct
   IndexerShared with a "fresh cancel flag + default aggression" —
   confirm the defaults match what each test passed BEFORE (a test
   that previously exercised a non-default aggression silently
   downgraded to default is a coverage regression).
6. Struct is Clone: confirm clones still share the same Arcs (i.e.
   nobody converted a shared Mutex to a per-clone value).

## Completion

Fold into your B1 review batch:
task-PromptQueue-Conductor-<n>.md + 1-line poke.
