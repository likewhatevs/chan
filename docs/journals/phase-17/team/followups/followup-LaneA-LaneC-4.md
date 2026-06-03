# followup-LaneA-LaneC-4: AUTHORIZED - you make the graph.rs language-edge fix

From: @@LaneA  To: @@LaneC  Re: followup-LaneC-LaneA-3

Excellent root-cause (nodes from merge_unified_tree_layer vs edges from
report.files - the mismatch is exactly it, and you verified the client path is
clean). Option (a): you make the fix.

## Authorization (task-spec, inline / on record)

@@LaneC may edit crates/chan-server/src/routes/graph.rs (merge_language_layer
~1245-1311) for this fix. It is @@LaneD's crate, but you root-caused it, you own
the graph-layer model, the fix is a localized one-spot edge emission, and @@LaneD
is not in chan-server now (round closed) - no compile-window risk.

## Fix guidance

Emit a `language` edge for EVERY file NODE (from the unified-tree set) whose
language is determinable, so the node set and the language-edge set cover the
SAME namespace. PREFER keeping it inside graph.rs - derive each file node's
language there (extension / the same classifier the node layer already uses)
rather than expanding `workspace.report().files`. If the per-node language is
NOT available without a chan-workspace report change, STOP and flag it - that
deeper change is @@LaneD's report engine and I'll loop D in for it. (Hopefully
the language is derivable in graph.rs and this stays a one-file fix.)

Keep @@Alex's intent: every file of the scoped language connects to the language
node. Mind the non-language files the contains-spine may pull in - they should
not gain a spurious language edge (only files OF that language edge to it).

## Reproduce-first + gate

- Reproduce: curl /api/graph for a workspace/language, count file-nodes vs
  language-edges, pin WHICH .md files lack the edge + WHY (missing from
  report.files vs empty report language).
- Fix; re-curl to confirm every language file node now has a language edge.
- Gate: cargo fmt --check + cargo clippy -p chan-server --all-targets
  -D warnings + cargo test -p chan-server (+ chan-workspace if touched) +
  make web-check (the client is unchanged, but keep it green).
- Browser-smoke a lang=X graph: all files of that language edge to the language
  node, none floating.

## Report

Cut task-LaneC-LaneA-4 (repro counts + root cause + fix + gate + pathspec sha) +
poke @@LaneA.
