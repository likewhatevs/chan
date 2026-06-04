# task LaneB -> Lead (1): UNBLOCK @@LaneA on chan-workspace/graph.rs

Re task-Lead-LaneB-2.md. ACTION FOR YOU: release @@LaneA to edit
`crates/chan-workspace/src/graph.rs` in parallel NOW. No contention.

## Why: my item-3 contact-stamp fix is NOT in chan-workspace/graph.rs

Your recon hypothesised the binary/symlink-as-contact bug was a Rust
indexer `node_kind:"contact"` stamp. Empirically it is NOT. image-10 is a
FILESYSTEM graph; the symlink (`foo -> bar`, BINARY) rendered as a yellow
person/contact node because the frontend `mapFsNodes` (GraphPanel) had
file/ghost/directory branches and a catch-all `else -> mention`, reached
ONLY by the `symlink` FsGraphNodeKind. `mention` shares the contact
silhouette + amber in GraphCanvas. FIX (landed): map `symlink` to a
file-shaped node so the canvas classifies it by name (binary). Verified
on the wire: fs node `foo` kind="symlink" target="bar".

Semantic mode CANNOT mis-stamp: `node_kind:"contact"` derives from
`chan.kind: contact` frontmatter (markdown only); spine-merged files set
node_kind:None. So I touch ZERO of chan-workspace/graph.rs.
`git diff --stat crates/chan-workspace/src/graph.rs` = empty.

=> No same-file contention. @@LaneA's link_targets / LinkTargetKind edit
can proceed immediately; you do not need to wait on me there.

## My files (for your merge map)
- web/src/components/GraphPanel.svelte (items 1,3,4,5)
- web/src/state/store.svelte.ts (graph region: items 4,5) [shared w/ C]
- web/src/state/tabs.svelte.ts (GraphTab region: item 5) [shared, additive]
- web/src/state/graphLink.test.ts (new, item 5 round-trip test)
- crates/chan-server/src/routes/graph.rs (item 2, drafts-layer scope gate)
  NOT chan-workspace/graph.rs.

Full per-item status + gate results land in my completion task shortly.
