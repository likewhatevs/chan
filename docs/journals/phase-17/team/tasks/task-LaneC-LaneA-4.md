# task-LaneC-LaneA-4: lang graph disconnected-files - FIXED (server-side)

From: @@LaneC (phase-17)  To: @@LaneA  Re: task-LaneA-LaneC-4 + followup-LaneA-LaneC-4

Fixed in graph.rs (your authorization). Stayed in graph.rs - per-file language
was derivable from the full report, NO chan-workspace change needed. Gate green.
Implemented by a subagent under my direction; I verified (read the diff,
re-ran the Rust gate independently, reconciled the repro). No commit/push.

## Reproduce (curl, before-fix)

Seeded a drive (linked.md w/ [[links]]+#tags, notes/sub.md, plain.md +
notes/lonely.md plain prose, src/hello.py, src/main.rs); curled /api/graph.
- scope=workspace: 6 file nodes / 6 language edges - NO floating. So @@Alex's
  "files with no links/tags float" FRAMING does not trigger in workspace scope:
  plain .md IS tracked by the report (tokei classifies it Markdown, never
  empty). The framing was a red herring; the real defect is below.
- scope=directory&path=notes: 4 file nodes / only 2 language edges. linked.md +
  plain.md FLOAT. WHY = not-in-scoped-report: merge_unified_tree_layer pulls
  root spine / link-target files into the namespace as NODES, but the language
  edges came from report_for_prefix("notes") (verified: /api/report/prefix
  path=notes -> 2 files). So out-of-prefix file nodes had no language edge.
- The language LENS (@@Alex's lang=Markdown) is a client BFS over this same
  scoped data, so it inherits the scope-restricted-report vs full-namespace-
  nodes mismatch -> floating file nodes.

## Root cause

graph.rs: file NODES come from merge_unified_tree_layer (full File-Browser
namespace); language EDGES came from a SCOPE-RESTRICTED report
(scoped_report_files -> report_for_prefix/report_for_files). The two sets
differ, so file nodes outside the scoped prefix had no language edge and floated.

## Fix (graph.rs merge_language_layer, ~1233-1340)

Drive the language edges off the SAME namespace as the nodes: read the FULL
workspace.report() (not the scope-restricted one), build path->language /
path->code maps, then iterate the FILE NODES already in the graph and emit one
Language->File edge per node the report classifies. Per-file language comes from
ReportFileStats::language (tokei) - derivable in graph.rs, so NO chan-workspace
change. Media/binary (separate node kinds, absent from the report map) get NO
spurious edge; non-language files get none either. Removed the now-unused
scoped_report_files helper; `p` -> `_p` (scope no longer narrows the report).

## After-fix (curl, re-confirmed)

- workspace: 6/6, directory(notes): 4 file nodes / 4 language edges (linked.md +
  plain.md now connect to language:Markdown), file(src/hello.py): 5/5. 0 floating
  in every scope. A file outside a scope's language (main.rs vs a python scope)
  gets NO orphaned edge.

## Gate: GREEN (independently re-run by me)

- cargo fmt --check: 0
- cargo clippy -p chan-server --all-targets -D warnings: 0 (clean)
- cargo test -p chan-server: 400 passed / 0 failed (incl. both language-layer tests)
- make web-check: built (client unchanged - no client edit needed; verified the
  language lens + visibleEdges render in-scope language edges, so the new edges
  paint).
Footprint: ONLY crates/chan-server/src/routes/graph.rs (chan-workspace untouched).

## Caveat (please relay to @@Alex)

I could not VISUALLY browser-smoke the lang=X lens: the browser nav to my
throwaway server was permission-denied earlier and I did not retry it. The fix
is empirically confirmed at the DATA layer (curl before/after, all scopes) and
the client render path is read-verified, so the lens will connect every
report-classified file node. Suggest @@Alex re-open his lang=Markdown graph to
visually confirm no floaters. One narrow residual (NOT @@Alex's md symptom): a
`file`-kind node the report does not track at all would still float - that would
be a chan-workspace report-coverage matter (@@LaneD), out of scope here.

## Pathspec sha (uncommitted WIP; HEAD 92fdf17e)

  2d9b600480a3400dd280b2275f1c2cf12b45a460  crates/chan-server/src/routes/graph.rs
