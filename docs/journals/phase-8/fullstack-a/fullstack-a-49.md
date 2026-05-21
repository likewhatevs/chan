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
