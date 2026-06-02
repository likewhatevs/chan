# @@LaneB — Graph

Read `round-1-plan.md` first. Round-1 = G1 (the dir-spine fix). G2
(incremental large-graph loading) is round-2; do NOT start it this round.

## Round-1 task

**G1 — directory spine on filtered lenses.** When the graph is filtered by
`lang=X` (and similarly by hashtag and by mention), files render but the
directory spine back to the workspace root is omitted, leaving edgeless
files. Plot the spine so NO file lacks an edge to a directory.

- The workspace-scope graph ALREADY has the spine: `crates/chan-server/src/
  routes/graph.rs` `merge_filesystem_layer_with_buckets` (:1804) ->
  `merge_unified_tree_layer` -> `ensure_directory_path` (:959) walks parents
  to root. Reuse this.
- The gaps: the language overview `/api/graph/languages`
  (`build_language_graph`, :712-811) and the lang/hashtag/mention lenses do
  NOT merge that fs spine. Add it there.
- Target shape = the dashboard/search "spine" @@Host likes: files around
  directories, spine bottom-up, languages/hashtags/contacts at the top. It
  is the SAME unified `/api/graph` shape; the fs layer IS the spine.
- Render is Cytoscape.js + fcose (`web/src/components/GraphPanel.svelte`);
  verify the added edges lay out as a spine, not a hairball.

## Files you OWN

`crates/chan-server/src/routes/graph.rs` (and `fs_graph.rs` if you reuse the
tree walk), `web/src/components/GraphPanel.svelte`,
`web/src/state/graphData.svelte.ts`.

## Coordination

- `web/src/state/store.svelte.ts` belongs to @@LaneA this round — don't edit
  it. If you need a graph-data change there, poke @@Lead.

## Verify

`make pre-push` green. Browser-smoke a `lang=` lens, a hashtag lens, and a
mention lens on a real workspace: confirm every file node has at least one
edge up to a directory and the layout reads as a spine. Post the commit sha
to `event-lane-b.md` and poke @@Lead.
