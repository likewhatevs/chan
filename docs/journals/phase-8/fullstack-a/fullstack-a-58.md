# fullstack-a-58 — Graph parent-edge invariant: every node has a parent unless folder filter is OFF (audit-then-fix)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Restore the graph invariant @@Alex spec'd: with folder
filter ON (default), every node has an inbound `contains`
edge from a parent directory. No orphans except when
folder filter is OFF.

Two empirical manifestations of the same gap:

1. **File-scope** (graph-from-here on a file): parent dir
   not connected to the file. User can't click-up via
   the graph.
2. **Drive-scope**: orphan markdown nodes scattered around
   the dense cluster.

## Reference

[`../phase-8-bugs.md`](../phase-8-bugs.md) "File-scope
graph doesn't include the parent directory node (spec'd;
missing); drive-scope shows orphan markdown nodes —
likely same root cause" — full bug body with the general
rule + investigation hooks + acceptance criteria.

## Audit-then-fix shape

The bug entry calls for audit-first to determine
whether the gap is chan-server-side (emit missing) or
SPA-side (data present but unrendered).

### Step 1 — Empirical audit

1. Spin up a test server against the chan repo seed.
2. `curl http://127.0.0.1:<port>/api/graph?scope=file&path=CLAUDE.md`
   — inspect JSON for parent dir node + `contains` edge
   from drive-root → CLAUDE.md.
3. `curl .../api/graph?scope=drive` + count file nodes
   with no inbound `contains` edge. Sample 5-10 of
   them; trace whether their parent dir exists in the
   node set + whether the edge is in the edges array.
4. Verdict at task tail: data present / data partial /
   data missing.

### Step 2 — Fix per audit outcome

* **If data PRESENT but unrendered**: SPA fix in
  `GraphCanvas.svelte` rendering logic. Audit which
  condition gates `contains` edge visibility. Fix +
  test pins.
* **If data PARTIAL** (some edges present, some missing):
  hybrid — SPA likely needs filter relaxation + chan-
  server may need emit completion. Fire scope poke
  with the partial findings; @@Architect routes the
  chan-server piece to @@Systacean if needed.
* **If data MISSING wholesale**: fire scope poke. Lane
  re-routes to @@Systacean for chan-server `merge_filesystem_layer`
  / `build_fs_graph` work; this task closes as
  audit-only.

## Acceptance

After fix lands (SPA OR cross-lane):

1. **File-scope** (graph-from-here on any file): the
   file's parent directory renders as a node + `contains`
   edge from parent → file, UNLESS folder filter is OFF.
2. **Drive-scope**: every file node has an inbound
   `contains` edge from its parent directory, UNLESS
   folder filter is OFF.
3. **Folder filter OFF**: parent-dir nodes don't render;
   files float without parent-dir edges (user's choice).
4. **Click parent-dir node**: opens directory inspector
   with "Graph from here" affordance (already wired per
   `-a-50`; just verifying composition).

### Tests

Vitest pin: file-scope graph response contains a
`contains` edge inbound to the file when folder filter
is on. Vitest pin: parent-dir node is rendered + visible
in the canvas (DOM/canvas assertion).

If the fix is chan-server-side (re-route case), the
acceptance pins live in @@Systacean's lane.

### Gate

* `npm test -- --run` green.
* `npm run check` 0e/0w.
* `npm run build` clean.

## Coordination

* @@FullStackA primary (SPA + audit lead).
* Possible cross-lane to @@Systacean if audit
  reveals chan-server gap.
* Atomic-audit-commit discipline.

## Authorization

**Yes** for SPA changes (`GraphCanvas.svelte` + tests +
task tail + outbound). If chan-server changes are
needed: scope-poke + route through @@Architect (don't
expand unilaterally).

## Numbering

This is `-a-58`.

## Out of scope

* Graph hit-radius polish (separate bug; could ride
  if you happen to touch the same render path).
* Filter chip extensions (separate task `-a-57`).
* New graph features beyond the parent-edge invariant.
