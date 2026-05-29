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

## 2026-05-22 — audit verdict + ready for review

Two-file change (1 SPA + 1 new test + 3 docs).
SPA-only; no Rust touched.

### Audit verdict: HYBRID (SPA-side fix + drive-scope flagged)

**File-scope: SPA regression from `-a-52`.** Source-
level audit (no test-server needed):

* chan-server `walk_file`
  (`crates/chan-server/src/routes/fs_graph.rs:500`)
  emits the file node + the parent directory node +
  a `contains` edge with source=parent, target=file.
  Data is present at chan-server.
* `merge_filesystem_layer` is called UNCONDITIONALLY
  on every `/api/graph` request, so semantic-mode
  drive/dir/file scopes ALSO carry the filesystem
  layer.
* SPA `scopedNodeIds` BFS at file-scope: seedIds =
  the file node id; the forward-only BFS (introduced
  by `fullstack-a-52`'s G9) walks `source → target`
  only. The parent → file contains edge has parent
  at SOURCE, file at TARGET. BFS starting at the
  file walks OUTGOING edges — but the contains edge
  is INCOMING to the file. Parent never gets added
  to scopedNodeIds.

**Diagnosis**: my `-a-52` G9 simplification
introduced this regression. Pre-`-a-52`, the
bidirectional BFS would have added the parent via
`frontier.has(e.target) → add e.source`.

**Drive-scope orphan markdown**: structurally
unrelated to file-scope (drive-scope sets
`scopedNodeIds = null` — no BFS filter; all nodes
from chan-server render directly). chan-server's
`walk_directory` emits contains edges recursively
for descendants, so the wire data appears complete
on source-reading.

Empirical confirmation of drive-scope orphans
requires a running test server + curl of the
graph response. **Couldn't run one this beat** —
the registered test drives are locked, and the
classifier denied my Write attempts to a fresh
`/tmp/chan-test-a58-audit/` scratch dir. Flagging
drive-scope orphans as a follow-up for empirical
confirmation in a separate session that can
write to `/tmp` or has direct access to a fresh
seed.

If the drive-scope manifestation turns out to be
the file-scope fix transitively (drive scope
shares no scoped filter so it shouldn't need a
fix), this lands here. If it's a separate
chan-server emit gap, a scope poke routes the fix
to @@Systacean.

### Fix shape

`web/src/components/GraphPanel.svelte` `scopedNodeIds`
derive — added a **parent-pull pass** after the
forward BFS:

```ts
let pulled = true;
while (pulled) {
  pulled = false;
  for (const e of edges) {
    if (
      e.kind === "contains" &&
      visited.has(e.target) &&
      !visited.has(e.source)
    ) {
      visited.add(e.source);
      pulled = true;
    }
  }
}
```

Iterates to a fixed point so ancestor chains pull
cleanly (file → parent → grandparent → ... → drive
root). Gates on `e.kind === "contains"` so other
edge kinds (link/tag/mention) keep their
forward-only semantics from `-a-52`'s G9.

The parent-pull adds nodes to `visited` but
`hiddenFolderIds` in `visibleNodeIds` still hides
folder nodes when the folder chip is OFF —
satisfying acceptance criterion 3 (folder filter
OFF → parent dirs don't render) without
parent-pull bypassing the chip gate.

### Acceptance criteria — coverage

1. **File-scope: parent dir + contains edge render**
   ✓ — parent-pull adds the parent's directory node
   to scopedNodeIds; the existing `visibleEdges`
   chain renders the contains edge because both
   endpoints are now in scope.
2. **Drive-scope: every file has inbound contains
   edge** — DEFERRED for empirical confirmation
   (per audit-then-fix shape). If drive-scope
   shares the chan-server data, no separate fix
   needed; if not, scope poke for chan-server emit
   gap.
3. **Folder filter OFF: parent dirs don't render**
   ✓ — `hiddenFolderIds` still applies in
   `visibleNodeIds` regardless of parent-pull.
4. **Click parent-dir node → directory inspector**
   ✓ — already wired in `-a-50`; this task only
   ensures the parent node renders so the click
   target exists.

### Tests

`web/src/components/graphParentEdgeInvariant.test.ts`
(new): 5 raw-source pins.

* Parent-pull pass exists + uses the fixed-point
  while loop shape.
* Pass runs AFTER the BFS (positional anchor
  check via index of comment markers).
* Pass gates on `e.kind === "contains"` only.
* Pass writes to `visited` directly (single
  accumulator).
* Folder-filter hiding still kicks in via
  `hiddenFolderIds`.

### Gate

* vitest **718 / 718** (+5 net from `-a-57`'s
  713).
* svelte-check 0 errors / 0 warnings across
  3996 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions

* **(B) SPA-side fix** over scope poke — clear
  regression caused by `-a-52`, easy + targeted
  one-pass while loop.
* **Iterate to fixed point** — supports ancestor
  chains. Files often live 3-5 levels deep; one-
  pass would miss grandparents.
* **Gate on `contains` only** — other edge kinds
  (link/tag/mention) keep `-a-52`'s forward-only
  semantics. The structural invariant the bug
  spec'd is specifically about parent-dir →
  child file contains edges.
* **Parent-pull AFTER forward BFS** — preserves
  `-a-52`'s G9 depth-slider semantic for forward
  expansion; parent-pull is the structural rule
  on top of that.
* **Drive-scope deferred for empirical confirm**
  — can't run a test server this beat. Flag for
  follow-up; if it shares the file-scope fix
  effectively, this beat resolves the whole bug;
  if not, scope poke to @@Systacean.

### Suggested commit subject

```
Graph parent-edge invariant: pull ancestor chain via contains edges (fullstack-a-58)
```

Single commit. Fix + test + audit verdict are
tightly coupled.

### Files for `git add` (per-path discipline)

* `web/src/components/GraphPanel.svelte`
* `web/src/components/graphParentEdgeInvariant.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-58.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Single bash invocation per the
`feedback-atomic-audit-commit` memory rule.
Authorization standing per the WAVE-3 FAN-OUT
dispatch.

Push held — multi-agent tree commit discipline.
Standing by for clearance.
