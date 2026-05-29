# fullstack-a-49 — Graph overhaul G2: filesystem-hierarchy as graph spine

Owner: @@FullStackA
Cut: 2026-05-21 by @@Architect
Status: queued (sequenced AFTER fullstack-a-43 + Tasks B/C/E/F land)

## Goal

Every plotted graph node sits under its ancestor-chain
to the drive root. The graph stops being a flat node
soup and becomes a filesystem-rooted hierarchy by
default; markdown / hashtag / mention / language nodes
layer on top of that spine.

## Background

Locked design at
[`../architect/graph-overhaul-plan.md`](../architect/graph-overhaul-plan.md)
§"Architecture overhaul" — G2. @@Alex 2026-05-21:

> the graph is now primarily showing a filesystem-based
> hierarchy and it must; whatever we are plotting, we
> always start from a parent directory which if we
> click we get an inspector for that directory like we
> get in the file browser

Plus: "if the plotted node is a file, we should always
plot its own ancestor and stop at the drive". Plus the
markdown-link semantic from G5: link targets plot
WITHOUT parent-dir edges until the user clicks "Graph
from here" on them.

## Acceptance criteria

* Every file node in the graph renders with its full
  ancestor-chain (directories) to the drive root.
* Directory nodes are first-class (foundation for G3).
* "Graph from here" rooted at any node uses the
  filesystem-hierarchy spine as the layout backbone.
* Markdown-link targets (G5) DO NOT carry parent-dir
  edges initially; user-driven "Graph from here" on a
  target plots that target's ancestor-chain.
* Tests cover: ancestor-chain renders for a file 3+
  dirs deep; drive-root stop boundary; layout
  stability across "Graph from here" re-rootings.
* Pre-push gate green.

## How to start

1. Audit current graph layout in the SPA — the
   `GraphPanel.svelte` + companion state stores.
2. Audit current graph-route shape in chan-server
   (`crates/chan-server/src/routes/graph.rs`); confirm
   ancestor data is already in the response OR add to
   it.
3. Design the layout transform: input is the existing
   nodes + edges; output is the filesystem-rooted
   tree.
4. Implement the layout shape.
5. Wire tests + verify gate.

## Cross-lane consumption

* `systacean-15` (cross-dir aggregation) provides per-
  directory stats used by G3 for the inspector body.
  Not required for G2's layout but available.
* `systacean-16` (file-classification buckets)
  provides the markdown / source / binary / media
  classification used by G6 + G7/G8.

If the chan-server graph route needs ancestor-data
changes, fire a scope question — that'd be a
@@Systacean follow-up.

## Coordination

* SPA-primary; chan-server cross-pollination possible.
* Append "Commit readiness" + poke @@Architect when
  ready.

### Sequencing constraint — HARD prereq

Depends on `fullstack-a-43` + Tasks B/C/E/F landing
in HEAD. The Hybrid back-side wave settles first,
then the graph overhaul wave kicks off.

## Numbering

This is `-a-49`. See `-a-45` for the broader wave
numbering note. G3 (`-a-50`), G6+TaskD (`-a-51`),
G10+G9 absorbed (`-a-52`) fan out alongside as the
first graph overhaul sub-wave.

## 2026-05-21 — audit findings + scope-check poke

Audit of the current graph route + SPA layout changes
my read of G2's surface area significantly from the
task body's framing. Firing this as a scope-check so
@@Architect can route before I dig into a substantial
layout-algorithm design.

### What's already in place

* **chan-server** `/api/graph` already emits filesystem
  hierarchy data. `crates/chan-server/src/routes/graph.rs:1131`
  calls `merge_filesystem_layer(&drive, &p, ...)` which
  populates the response with:
  - `GraphNodeView::Directory { id, label, path,
    path_class, files, code }` nodes (one per directory).
  - `GraphEdgeView { kind: "contains" }` edges connecting
    each file to its parent directory + each subdir to
    its parent.
  - This runs unconditionally; the FsGraphScope variant
    (Drive / Directory / File) controls the depth.
* **SPA** `GraphPanel.svelte` already CONSUMES these.
  Lines 491 (`if (kind === "contains") return show.folder`),
  543 (counting contains), 789 (`node.kind === "directory"`),
  1003-1004 (contains edge rendering). The Directory
  nodes are rendered + the contains edges are drawn.

The task body says: "today's route returns flat edges
(@@Systacean: graph route may need ancestor data)." This
is incorrect for the current HEAD — the route emits the
hierarchy. The reading of "flat edges" may be a misread
of an earlier commit OR a forward-looking concern about
incomplete depth.

### What G2 actually needs

The Directory + contains data is THERE but the SPA
renders the graph via **d3-force force-directed
simulation** in `web/src/components/GraphCanvas.svelte`
(1133 lines). All nodes are equal participants in the
simulation; "ancestor-chain to the drive root" emerges
visually only by accident of the contains-edge attraction.

For G2's "filesystem-rooted hierarchy by default"
semantic, the layout TRANSFORM in GraphCanvas.svelte
needs to change — likely:

* A directory-depth-based Y-axis force (`forceY` per
  depth level) so files sit BELOW their parent dir.
* A parent-anchored X-axis force pulling children
  toward their parent's X position.
* OR: switch to a hybrid layout — `d3-hierarchy`'s
  `tree()`/`cluster()` for the directory backbone +
  d3-force for markdown-link overlay forces.

This is a substantial layout-algorithm design effort.
The task body's "Design the layout transform: input is
the existing nodes + edges; output is the filesystem-
rooted tree" reads as a small "step 3" but it's the
load-bearing piece.

### Scope question for @@Architect

Two options:

**(A) -a-49 as-spec'd**: I design + ship the full
hierarchical layout transform in `GraphCanvas.svelte`.
Substantial; the d3-force tuning + the hybrid layout
strategy + the markdown-link overlay semantics + the
"Graph from here" re-rooting all need to compose. Likely
the biggest single SPA commit of this session.

**(B) -a-49 split**: cut into 2-3 smaller pieces.
- `-a-49a`: data-shape audit + Directory inspector
  wiring (subset of G3; foundational view of the
  existing Directory nodes).
- `-a-49b`: layout transform (the hierarchical layout
  algorithm).
- `-a-49c`: markdown-link overlay semantics (G5).
Maybe more digestible commits + clearer review
boundaries.

**(C) Scope-down -a-49 to just the layout shape +
defer the markdown-link / G5 piece to its own task**.
This is the original `-a-49` minus the line in
acceptance criteria "Markdown-link targets (G5) DO NOT
carry parent-dir edges initially" which crosses into
G5's scope.

### My lean

(C). The G5 line in the acceptance criteria reads as
a teaser; G5 has its own task slot in the queue
(`-a-N` later). Keeping `-a-49` focused on the layout
TRANSFORM alone keeps the commit boundary clean.
Markdown-link semantics can land separately once the
hierarchical layout is in place.

### What I'm doing now

Holding `-a-49` here until you route the scope. The
`-a-55` "committed as 7cf6f8e" trailing append is in
my working tree (not committed yet); will bundle with
whichever shape of `-a-49` lands first.

Standing by. Queue behind `-a-49` is `-a-50..52`
(further graph overhaul) → `-a-42` (About; A+B+C+F
all in HEAD).

## 2026-05-21 — ready for review (option C — layout transform only)

Two-file change. SPA-only; no Rust touched. Scope
locked to the filesystem-hierarchy layout transform
per your routing; markdown-link overlay / G5 stays
deferred.

### Layout strategy picked

**Strategy (1)** — d3-force with depth-anchored
`forceY` + parent-anchored custom `forceX`. Lowest
blast radius; composes with the existing simulation;
keeps all the existing interaction model intact (pan,
zoom, drag, selection, refit).

Why not (2) or (3):

* **(2) hybrid d3-hierarchy.tree() + d3-force
  overlay** would be architecturally cleaner but
  introduces a second layout engine that has to
  reconcile positions with the force model. The
  existing simulation already runs every tick;
  adding a tree() pre-pass + overlay would mean
  re-running the tree on every node-set change and
  managing position handoff. Significantly higher
  blast radius for marginal visual improvement.
* **(3) full d3-hierarchy tree() (no force)** drops
  the force-based interaction model entirely — no
  more dragging nodes apart to investigate clusters,
  no more "release node to let it relax." The user
  loses the existing affordances. Architect's note
  flagged this trade-off explicitly.

(1) is the conservative implementer call. If a
future pass wants to migrate to (2)/(3), the
hierarchy fields on `DNode` (depth + parentId) +
the helper (`nodeHierarchy`) carry forward
cleanly.

### What landed

`web/src/components/GraphCanvas.svelte`:

* `DNode` extended with `depth: number` +
  `parentId: string | null` fields. Initialized
  from a new `nodeHierarchy(node)` helper.
* `FORCE` config gains three new knobs:
  - `hierarchyYSpacing: 90` — vertical pixels per
    depth level. Roughly matches the existing
    `linkDistance: 70` + a bit of room.
  - `hierarchyYStrength: 0.45` — strong-ish pull
    toward the depth band so the tree shape
    holds against the link springs.
  - `parentXStrength: 0.18` — weaker pull toward
    parent's X so siblings cluster but can
    individually drift to avoid collisions.
* `nodeHierarchy(n)` helper derives depth + parentId
  from the node's kind + path:
  - tag / mention / language: `depth: -1,
    parentId: null` (exempt from hierarchy
    forces; keep floating on the center force).
  - folder with id "" or path "": drive root, depth
    0, parentId null.
  - folder with path "docs/journals": depth 2,
    parentId "directory:docs".
  - file with path "docs/foo.md": depth 2,
    parentId "directory:docs".
  - file at drive root ("README.md"): depth 1,
    parentId "" (drive root).
* `rebuildWorkingSet` populates depth + parentId
  on both the existing-node mutate branch + the
  fresh-node construct branch.
* `buildSim` replaces the centered `forceY<DNode>(0)`
  with a depth-aware variant: hierarchical nodes
  (depth >= 0) target `depth * hierarchyYSpacing`
  with `hierarchyYStrength`; non-hierarchical
  (depth < 0) keep `centerStrength` at y=0.
* `buildSim` adds a new `"parentX"` force via the
  custom `parentXForce(strength)` factory.
* `parentXForce`: per-tick velocity push toward
  parent's X position. Skips non-hierarchical
  nodes (depth < 0) + nodes with null parentId +
  nodes whose parent isn't in the working set
  (filtered out by visibility). Velocity push
  proportional to `strength * alpha` so the
  simulation converges as alpha decays.
* `parentXForce.initialize(nodes)` wires the
  node array per the d3-force `Force` interface.

`web/src/components/GraphCanvas.test.ts` (new):

* 11 raw-source pins for the wiring shape:
  - DNode carries depth + parentId.
  - FORCE config has the three new knobs.
  - nodeHierarchy: non-hierarchical → -1; drive
    root → 0/null; folder path derivation; file
    path derivation.
  - rebuildWorkingSet propagates depth +
    parentId on both branches.
  - buildSim wires depth-aware forceY.
  - buildSim registers the parentX force.
  - parentXForce skips non-hierarchical + null
    parent + missing parent.
  - parentXForce.initialize wires the node array.

### Visual behavior

After this lands:

* Drive root sits at y=0 (depth 0).
* Top-level directories (`docs`, `crates`, `web`)
  sit at y=90 (depth 1).
* Second-level dirs (`docs/journals`) at y=180.
* Third-level (`docs/journals/phase-8`) at y=270.
* Files render at their parent dir's depth + 1.

Per the architect's acceptance criteria refinement:
`docs/journals/phase-8/` renders BELOW `docs/`
renders BELOW the repo root. Markdown files within
a directory render BELOW the directory node. ✓
(force-relaxed; siblings spread laterally via
charge + collide.)

### Out-of-scope reminder

* Markdown-link semantics (G5) — link targets
  WITH parent-dir edges in the chan-server
  response will get the depth-anchored layout
  treatment. The G5 task is the one that decides
  whether link targets keep their parent-dir
  edges initially or not; this task doesn't gate
  the data shape.
* "Graph from here" re-rooting — works because
  the focal-id positioning in `rebuildWorkingSet`
  pins the focal at (0,0), and the depth layout
  is RELATIVE to the visible node set. Re-rooting
  on a deep file naturally shows just that
  file's ancestor chain (depth re-calculates
  based on the new visible set's contents).

### Gate

* vitest **658 / 658** (+11 net from `-a-55`'s
  647).
* svelte-check 0 errors / 0 warnings across
  3990 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Empirical verification recommendation

Vitest pins lock the WIRING shape; the visual
layout HAS to be verified empirically against a
chan source drive. The architect's acceptance
criterion check —
`docs/journals/phase-8/` BELOW `docs/` BELOW the
repo root — needs a manual eye on a running
chan-desktop or chan-serve with the chan repo
mounted. Recommend a `webtest-a-6` walk with this
specific verification + the broader graph layout
sanity check.

### Suggested commit subject

```
Graph layout: filesystem-hierarchy as backbone (fullstack-a-49)
```

Single commit. State extension + helper +
sim-build wiring + custom force + tests are all
tightly coupled around the same layout
transform.

### Files for `git add` (per-path discipline)

* `web/src/components/GraphCanvas.svelte`
* `web/src/components/GraphCanvas.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-55.md`
  (`-a-55` "committed as 7cf6f8e" trailing
  append; bundled per the established pattern)
* `docs/journals/phase-8/fullstack-a/fullstack-a-49.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

Push held — multi-agent tree commit discipline.
Standing by for clearance.

## 2026-05-21 — committed as 5685be4 (CROSS-AGENT INCIDENT)

Cleared by @@Architect with all decisions accepted.
Committed as `5685be4 Graph layout: filesystem-
hierarchy as backbone (fullstack-a-49)`.

### Cross-agent commit-hygiene incident — peer-staged stowaways

**Pre-commit `git diff --staged --stat`** showed exactly
5 files (the cleared set minus `fullstack-a-55.md` which
was already in HEAD via `8be1bfc`):

```
docs/journals/phase-8/alex/event-fullstack-a-architect.md   | 156 ++
docs/journals/phase-8/fullstack-a/fullstack-a-49.md         | 187 ++
docs/journals/phase-8/fullstack-a/journal.md                |  98 ++
web/src/components/GraphCanvas.svelte                       | 119 ++
web/src/components/GraphCanvas.test.ts                      | 111 ++
5 files changed, 669 insertions(+), 2 deletions(-)
```

**Post-commit `git show --stat HEAD`** shows **18 files**
— a 13-file delta swept in between my audit and my
commit. The stowaway set is @@Systacean's `-19` work:

* `crates/chan-drive/src/drive.rs`
* `crates/chan-drive/src/index/facade.rs`
* `crates/chan-drive/src/indexer.rs`
* `crates/chan-drive/tests/contacts_import.rs`
* `crates/chan-drive/tests/file_types.rs`
* `crates/chan-drive/tests/remove_cleanup.rs`
* `crates/chan-drive/tests/smoke.rs`
* `crates/chan-server/src/indexer.rs`
* `crates/chan-server/src/routes/graph.rs`
* `crates/chan-server/src/routes/inspector.rs`
* `crates/chan-server/src/routes/search.rs`
* `docs/journals/phase-8/alex/event-systacean-architect.md`
* `docs/journals/phase-8/systacean/systacean-19.md`

### Root cause

Multi-agent staging race. The architect's clearance
round 17 cleared three lanes simultaneously (`ci-13` +
`systacean-19` + `fullstack-a-49`). Between my
`git diff --staged --stat` audit and my `git commit`,
@@Systacean's session ran `git add <-19 files>` which
expanded the staged set silently. My explicit per-path
`git add` was clean; the race expanded the index after
my audit.

### Net state

* `-a-49` code + docs landed in HEAD verbatim (the 5
  files I explicitly added are byte-identical to my
  staged set).
* `-19` work ALSO landed in HEAD verbatim — under a
  commit subject that doesn't reflect its content.
  Same regression shape as `a8e991a` `-a-44` incident
  but in reverse: this time I'm the lane whose commit
  swept up the peer's work.

### Process lesson

The `feedback_shared_worktree_commits` memory rule's
"pre-commit `git diff --staged --stat` + post-commit
`git show --stat HEAD`" works as designed — the
discrepancy IS the incident signal. The gap is that
the discipline doesn't atomicize the audit + commit
into one shell line; the harness's bash boundary lets
other lanes add to the index between them.

**Fix for future commits**: collapse the audit + commit
into ONE bash invocation:
`git add <paths> && git diff --staged --stat && git commit -m "..." && git show --stat HEAD`.
Single bash line, no inter-command race window.

### Routing to @@Architect

Same options as the `-a-44` incident:
(a) history rewrite (declined last time; same risk
profile here — `-19` is on top of `-a-49` so a
rewrite would touch both commits).
(b) audit-trail correction (this append + symmetric
append in @@Systacean's `-19` task tail).
(c) follow-up empty commit naming `systacean-19`.

I lean (b) + (c): audit-trail correction here + a
small follow-up empty commit with the `systacean-19`
grep anchor. Architect calls.

Standing by.
