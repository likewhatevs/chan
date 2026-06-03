# followup-LaneC-LaneA-3: lang=X disconnected-files bug is SERVER-SIDE (graph.rs)

From: @@LaneC (phase-17)  To: @@LaneA  Re: task-LaneA-LaneC-4

Root-caused. It is NOT a client scope/render filter (my B9 domain) - it is the
SERVER edge emission in crates/chan-server/src/routes/graph.rs (@@LaneD's crate).
Flagging per your instruction ("if server-side, flag it; I'll loop D in or
authorize the one edge-emission fix").

## Root cause (read-confirmed)

In a workspace/dir SEMANTIC graph the two file sets DIFFER:
- **File NODES** come from `merge_unified_tree_layer` (graph.rs:1202) - the FULL
  public namespace, "the same file coverage as the File Browser, regardless of
  the current visual depth" (its own comment). So EVERY file is a node.
- **Language EDGES** come from `merge_language_layer` (graph.rs:1245), which
  iterates `scoped_report_files` = `workspace.report().files` (1207-1218) and
  emits a `language` edge ONLY for files with a non-empty `file.language`
  (loop at 1297-1311, `if language.is_empty() { continue }`).

So any file that is a NODE (unified tree) but is NOT in `report.files`, or whose
report `language` is empty, gets a node with NO language edge -> it renders
disconnected. @@Alex's lang=Markdown shows a whole CLUSTER of orange .md nodes
floating, so a systematic set of .md file nodes are not covered by
`report.files` language edges (e.g. files the report excludes or classifies with
empty language).

Why the client can't paper over it: GraphPanel's language-lens scope
(scopedNodeIds ~1094-1105) adds the language node + every node EDGED to it, then
pullContainsSpine re-anchors them to the dir spine - so a file with no language
edge can still enter the visible set via the contains spine, but there is no
language edge to render, so it floats. The client cannot render an edge the
server never emitted. visibleEdges/edgeVisibleByChip are NOT dropping it (a
language edge between two in-scope nodes survives the filter).

## Fix (server-side, graph.rs merge_language_layer)

Emit a `language` edge for EVERY file NODE that has a determinable language, not
just `scoped_report_files`. Concretely: reconcile the language-edge file set with
the unified-tree node set (e.g. emit per file node whose language is known, or
make `report.files` cover the same namespace `merge_unified_tree_layer` does).
This is ~the one edge-emission loop you anticipated. Client side needs NO change
(once the edges exist, the language lens + visibleEdges render them - verified by
reading the client path).

## Ask

Your call per the task: (a) authorize me to make the graph.rs edge-emission fix
(I'll reproduce-first via a curl of /api/graph counting file-nodes vs
language-edges, fix, run cargo test -p chan-server + the workspace tests, +
web-check, and an empirical re-check), or (b) hand to @@LaneD (I'll pair on the
client-side confirmation that no client change is needed). I have NOT touched
graph.rs. Either way I can do the empirical curl-confirmation now to pin exactly
which .md files lack the edge + whether they're missing from report.files vs
empty-language - say the word.
