# fullstack-a-50 — Graph overhaul G3: directory nodes + FB-style inspector with aggregated reports stats

Owner: @@FullStackA
Cut: 2026-05-21 by @@Architect
Status: queued (sequenced AFTER fullstack-a-43 + Tasks B/C/E/F + systacean-15 + Task F)

## Goal

Make directory nodes first-class graph entities. Clicking
a directory opens an FB-style inspector body with
aggregated chan-reports stats for that directory.

## Background

Locked design at
[`../architect/graph-overhaul-plan.md`](../architect/graph-overhaul-plan.md)
§"Architecture overhaul" — G3. @@Alex 2026-05-21:

> whatever we are plotting, we always start from a
> parent directory which if we click we get an
> inspector for that directory like we get in the file
> browser, with the directory aggregated stats for
> chan-reports

The FB-style inspector is the same shape as the file-
browser side panel today (directory name + path + a
body of metadata + actions). Body contents for the v1
inspector:

* Directory path + name.
* File / subdirectory count.
* Aggregated stats from chan-report (file count by
  bucket markdown / source / binary / media; SLOC by
  language; COCOMO summary scoped to this directory).
* "Graph from here" action (re-root the graph at this
  directory).

## Acceptance criteria

* Directory nodes are clickable + open the inspector
  body.
* Inspector renders the aggregated stats from
  `systacean-15`'s new chan-report endpoint.
* "Graph from here" action on a directory inspector
  re-roots the graph at that directory.
* Tests cover: inspector mount on directory click;
  aggregated stats render correctly; "Graph from
  here" re-rooting behaviour.
* Pre-push gate green.

## How to start

1. Confirm `systacean-15` cross-dir aggregation API is
   live + accessible from the SPA.
2. Confirm Task F's chan-reports toggle exists + can
   be ON (without the toggle ON, the aggregated stats
   aren't computed; inspector body shows a
   "Enable chan-reports in FB settings to see
   aggregated stats" placeholder).
3. Design the inspector body shape — mirror the FB
   side-panel rendering pattern.
4. Wire the "Graph from here" action to the existing
   re-rooting code path (from `-a-33`).
5. Tests.

## Coordination

* SPA-primary; consumes `systacean-15`'s API.
* Append "Commit readiness" + poke @@Architect when
  ready.

### Sequencing constraint — HARD prereqs

* `fullstack-a-43` (Task A) — directory node
  rendering depends on the back-side architecture
  shape.
* Tasks B/C/E/F (Hybrid back-side wave) — Task F
  brings the chan-reports toggle which gates
  aggregated stats.
* `systacean-15` (cross-dir aggregation API).
* G2 (`-a-49`) — directory nodes need the
  filesystem-hierarchy spine to render meaningfully.

## Numbering

This is `-a-50`. See `-a-45` for broader wave
numbering note.
