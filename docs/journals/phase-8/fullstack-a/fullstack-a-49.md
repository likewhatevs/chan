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
