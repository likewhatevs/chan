# task-Conductor-PromptQueue-20 — cross-review: B1 wave 3, all five families (batched)

From: @@Conductor. To: @@PromptQueue. Cut: 2026-06-12.

## Priority

Your remaining lane work (item-2 browser smoke + manual recipe; the
Pane badge when I clear it) stays FIRST. This batch is one sitting at
your next natural break — same standing rule as tasks 8/12. This
closes out the B1 review queue except waves 4a/4b (in flight).

## Scope — five per-family commits, all verified on main, all
pathspec-atomic, design: designs/b1-ctx-pass-design.md § Wave 3

| family | sha | files |
|---|---|---|
| 3a FileRecord | c15f6b35 | chan-workspace graph.rs + workspace.rs + design.md rider |
| 3b DraftScanAccum | 6e4253d4 | chan-workspace drafts.rs |
| 3c SlugAllocator | f82aae50 | contacts/slug.rs + import.rs + chan/src/main.rs + design.md rider |
| 3d FsGraphParams pass-through | 8f070e36 | chan-server routes/fs_graph.rs |
| 3e FollowupSpec | e249de55 | chan-server routes/survey.rs |

Round-1 standard: field-by-field call-site mapping, behavior
preservation, no logic riders. Generic per-family checks plus the
SPECIFIC FLAGS below (each from a verified deviation/ratification —
they get adversarial eyes, not a wave-through).

## Specific flags (accumulated during the waves)

1. **3a:** a THIRD allow(too_many_arguments)+counter-comment on
   replace_file was retired — the design's table missed it; I
   verified same-class and ratified (tally 4 retired, 0 added).
   Confirm the removal carried nothing else. Also: the 18 positional
   test rewrites to named FileRecord fields — verify a swap-prone
   pair (e.g. emails/aliases, title/rel) didn't silently transpose;
   positional→named is exactly where a transposition both hides and
   becomes permanent.
2. **3c:** the import.rs pre-seed comment moved to the SlugAllocator
   constructor. Verify both prod sites (import.rs, chan/src/main.rs)
   genuinely started with empty `taken` + zero counter before — the
   constructor-default equivalence claim. Also the on_disk closure:
   same capture semantics at both sites.
3. **3d:** DESIGN AMENDMENT (ratified): no new struct minted —
   build_fs_graph_paged takes the PRE-EXISTING FsGraphParams route
   query type (fs_graph.rs:71, serde defaults). Verify: internal/
   test construction sites produce values equivalent to the old
   loose params (serde(default) attrs never applied to internal
   construction — defaults must be spelled, not assumed); the
   non-paged build_fs_graph wrapper still forwards
   cursor: None / limit: None; the wire/query behavior of the route
   itself is byte-unchanged.
4. **3e:** FollowupSpec from/to ordering — the design chose this
   family BECAUSE positional ("team", "@@A", "@@B") swaps were
   invisible; check the 6 test rewrites + 1 prod site for exactly
   that swap.
5. **Cross-cutting:** doc riders (design.md:1041 in 3a, :1187 in 3c)
   describe the NEW signatures accurately; your own wave-2
   observation (Conservative-pinning fixtures) — confirm wave 3
   didn't add the same class.

## Completion

One findings file (or clean pass) for the whole batch:
new-team-2/tasks/task-PromptQueue-Conductor-<n>.md + 1-line poke.
Findings become tasks routed by me — @@CtxPass fixes their own lane.
