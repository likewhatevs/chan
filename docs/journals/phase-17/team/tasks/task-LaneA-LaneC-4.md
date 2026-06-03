# task-LaneA-LaneC-4: BUG - language graph drops file<->language edges

From: @@LaneA  To: @@LaneC  (post-round hotfix; @@Alex reported live)

## @@Alex's report (verbatim)

"plotting the graph from lang=x and I expect to see every single file with an
edge to the language node.. but instead I see markdown document nodes (orange)
flying around disconnected from the spine."

Screenshot: a `lang=Markdown` semantic graph (status bar "semantic graph",
40/43 nodes, 66/67 edges). The language node (pink `{ }`) + tags (green `#`) +
SOME document nodes + dirs form a connected cluster; but a whole cluster of
ORANGE document nodes (top-left) float with NO edge to anything. @@Alex expects
EVERY file of that language to have a language edge to the language node.

## Expected (your own doc confirms it)

store.svelte.ts openGraphForLanguage (~2090): "The language node is a bubble
connected to EVERY file of that language; depth does not apply to language
lenses, so depth:0". So the intent is: language node -> language edge -> every
file of that language. The disconnected files violate that.

## Entry + likely areas (re-verify; this is your B9 domain)

- store.svelte.ts openGraphForLanguage: mode="semantic", scopeId=
  `language:${language}`, depth:0.
- The semantic-graph scope + the language edges: who emits the file<->language
  edges, and does the scope keep them for files that have NO OTHER edges (the
  disconnected ones look like files with only a language edge - no links/tags)?
  - Server: crates/chan-server/src/routes/graph.rs (LanguageGraph* +
    EdgeKind "language", ~363/384/449-470) - are language edges emitted for
    EVERY file, or only files already in the link/tag spine?
  - Client scope: the semantic scope by `language:X` + B9's
    scopedNodeIds/layer model - does it include a file node but drop its
    language edge (so the node floats)?
  - Render: GraphCanvas.svelte RenderedEdgeKind "language" (~42-56, 992) - is
    "language" in the rendered-edge set for this scope, or filtered by a lens?

## Hypothesis to check first

A file with ONLY a language edge (no markdown links, no tags) gets its NODE into
the scope but its language EDGE dropped - so it renders disconnected. Likely a
B9 scoping/layer interaction (the spine reachability may not traverse language
edges to leaf files). Confirm by graphing a language whose files have no
links/tags.

## Authorized

Your owned graph files (GraphPanel.svelte, store.svelte.ts) + GraphCanvas.svelte
(authorized) + crates/chan-server/src/routes/graph.rs IF the fix is server-side
(language edges not emitted for all files) - if so, flag it (that's @@LaneD's
crate; I'll loop D in or authorize the one edge-emission fix).

## Gate + report

Reproduce first, root-cause (server emission vs client scope vs render filter),
fix so EVERY file of the language connects to the language node, gate
(web-check + cargo if you touch graph.rs), browser-smoke a lang=X graph (all
files edged to the language node, none floating). Cut task-LaneC-LaneA-4 + poke.
