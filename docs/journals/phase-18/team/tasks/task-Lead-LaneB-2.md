# task Lead -> LaneB (2): heads-up + chan-workspace/graph.rs sequencing

@@Alex resolved the `[[` survey as "both" (additive), so @@LaneA will ADD
workspace-path candidates to chan-workspace `link_targets` (graph.rs:1308) +
`LinkTargetKind` (graph.rs:1429). Two boundary facts for you:

1. A is NOT touching your chan-server `routes/graph.rs`. The `api_link_targets`
   handler there is a pure passthrough; A's change is confined to chan-workspace
   + the editor client. Your `crates/chan-server/src/routes/graph.rs` wire-kinds
   work stays yours alone.

2. SAME-FILE CONTENTION on `crates/chan-workspace/src/graph.rs`:
   - You: NodeKind / GraphNode / contact-stamp (~80-210, plus wherever the
     binary/symlink-as-contact misclassification is set).
   - A: link_targets / LinkTargetKind (~1308-1480).
   Far-apart regions, but a `.rs` file is NOT interleave-safe. You have PRIORITY
   (your contact-stamp is a bug, already in flight; A's is a gated enhancement).
   ASK: land your chan-workspace graph.rs portion (the contact-stamp fix) in ONE
   burst, re-`cargo check -p chan-workspace` + `cargo test -p chan-workspace`
   GREEN, then poke me: "chan-workspace graph.rs stable". I release A's
   link_targets edit only after that. A is holding its graph.rs write meanwhile.
   - If your contact-stamp fix lands in a DIFFERENT file (e.g. the indexer /
     index build path, not graph.rs), tell me - then there's no same-file
     contention and I release A to edit graph.rs in parallel with you.

This is additive to your task-Lead-LaneB-1.md; your 5 items are unchanged.
No action needed beyond the one poke when your chan-workspace graph.rs is stable.
