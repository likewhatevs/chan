# Graph overhaul plan — Round 2

Author: @@Architect
Date: 2026-05-21

Status: **planning artifact**. Captures @@Alex's graph
overhaul scope conversation 2026-05-21 + the 5 decisions
locked in the same exchange. Tasks NOT YET DISPATCHED;
sequencing notes below.

Source for the user-side framing: chat with @@Alex
2026-05-21 around "do we have upcoming tasks for fixing
stuff in the graph?" — turned out we did not (only Task
D's Hybrid Graph legend grid was queued, which is a
back-side affordance, not architecture).

## Locked decisions (all 2026-05-21)

| # | Decision | Lock |
|---|----------|------|
| 1 | Sequencing — no v0.11.3 hotfix | v0.11.2 stays as-is; everything bundles into the next release wave (Round 2, possibly extending to Round 3) |
| 2 | chan-reports settings home | Hybrid FB back-side (option b) — folds into Task F's scope; the FB back becomes the **Search / Indexing / Reports** settings surface |
| 3 | Markdown-link expansion semantics | Link targets in "Graph from here" rooted at a markdown doc plot WITHOUT parent-dir edges; clicking "Graph from here" on a link target THEN plots THAT target's ancestor-chain to drive root |
| 4 | G2-G8 wave timing | Round 2 if possible (push to land before Round 3); architecture overhaul does NOT defer to Round 3 unless bandwidth forces it |
| 5 | Depth slider "forward" semantic | Outgoing-edge direction only (targets the current root points TO); NOT full-bidirectional radius |

## 9-task decomposition

### Bugs / regressions

| # | Scope | Class | Lane |
|---|-------|-------|------|
| G1 | chan-reports settings toggle restoration | **regression bug** | folds into Task F per decision #2 |
| G9 | Depth slider broken — outgoing-direction reveal | **regression bug** | investigate; @@FullStackA if pure-SPA, @@Systacean if server-side depth gate |

### Architecture overhaul

| # | Scope | Lane | Size | Dependencies |
|---|-------|------|------|--------------|
| G2 | Filesystem-hierarchy as graph spine — every plotted node sits under its ancestor-chain to the drive root | @@FullStackA (SPA layout) + @@Systacean (graph route may need ancestor data; today's route returns flat edges) | substantial | foundational; gates G3-G8 |
| G3 | Directory nodes as first-class graph entities + FB-style directory inspector with aggregated chan-reports stats | @@FullStackA (inspector) + @@Systacean (chan-report cross-dir aggregation — NEW feature; today's crate is per-file with per-language roll-ups, no per-directory aggregation) | substantial | prereq Task F (reports toggle present + ON) + G2 + chan-report's new cross-dir aggregation |
| G4 | File inspector — Open (if openable, e.g. plaintext) + Graph-from-here (if markdown) | @@FullStackA | small-medium | G2 |
| G5 | Markdown-link plotting semantics — link targets plotted without parent-dir edges; user-driven "Graph from here" on a link target plots that target's ancestor-chain | @@FullStackA | medium | G2 + G4 |
| G6 | Colour scheme — markdown=orange, source code=royalblue, binary=grey, media=purple | @@FullStackA (palette) + couples with Task D (legend renders the new palette); @@Systacean if file-class buckets need chan-report classification | small-medium | couples with Task D |
| G7 | Language nodes with edges to **visible** directories (when language filter is on + reports available) | @@FullStackA (SPA filter logic) + @@Systacean (graph route adds language↔dir aggregation) | medium | G2 + G3 + Task F (reports on) |
| G8 | Graph-from-language plots first-depth dirs containing that language | @@FullStackA + @@Systacean (graph route adds "first-depth dirs containing language X" query) | small-medium | G7 + chan-report classification |

## Cross-cutting prereqs

* **Task F** (Hybrid back-side Search / Indexing / Reports
  settings) — required for G3's aggregated stats to have a
  toggle to gate on. Task F is already in the Hybrid
  back-side wave queue.
* **chan-report cross-directory aggregation** — NEW feature
  in `chan-report`; described as a candidate Round-3
  extension in
  [`report-extensions-ideas.md`](report-extensions-ideas.md)
  "Other ideas worth considering". G3 pulls this forward
  into Round 2. @@Systacean lane; new sub-task in chan-
  report's per-drive `.chan/report.jsonl` (or extension).
* **chan-report file-classification buckets** —
  markdown / source code / binary / media classification
  may need explicit buckets in `chan-report` for G6's
  colour scheme + G7/G8's language-dir relationships.
  Today's `chan-report` does language detection (tokei)
  but the markdown/source/binary/media split isn't an
  explicit dimension. Audit + extend.
* **Task D** (Hybrid Graph legend grid) — couples with G6;
  the legend renders the new colour scheme. Cut + commit
  together.

## Sequencing within Round 2

Recommended dispatch order:

1. **Hybrid back-side wave first** — Tasks B/C/E/F (the
   non-Graph back-sides) land. Task F brings the reports
   toggle, closing G1.
2. **Graph overhaul kick-off — G2 + G6 + Task D as a
   coherent first sub-wave.** G2 establishes the
   filesystem-hierarchy spine + ancestor plotting; G6
   lands the colour scheme; Task D's legend ships with
   the new palette. Tight visual + structural story.
3. **G3 + chan-report cross-dir aggregation extension**.
   Pair @@Systacean's `chan-report` work with @@FullStackA's
   directory-inspector SPA work.
4. **G4 + G5 + G7 + G8 + G9**. Smaller pieces; sequencing
   between them open (no strong inter-deps once G2/G3 land).
   G9 is a regression bug — investigate during this wave;
   could land standalone if the wire bug is obvious + fits
   between bigger items.

## Why "Round 2 wave-3 instead of Round 3"

@@Alex's decision #4: "do before 3 if possible". The
graph is a load-bearing surface for daily use; deferring
the overhaul to Round 3 means another full release cycle
before the filesystem-hierarchy + directory inspectors
land. Push for Round 2 inclusion.

If wave-3 bandwidth runs out (FullStackA is the lane
heavy on this overhaul; their queue already includes
Tasks B/C/D/E/F + relocated G About + the new `-a-44`
drag-to-rearrange):

* Drop G7 + G8 (language-node features) to Round 3
  first — they're enhancements on top of the spine.
* G3's chan-report cross-dir aggregation lands in
  @@Systacean's lane; parallel to FA. Lower risk of
  bandwidth conflict.
* G9 depth-slider fix is small + standalone; keep in
  Round 2 regardless.

## Numbering note

Tasks cut at dispatch get the next-available `-N` slot
per lane per the "highest committed `<agent>-N` + 1"
rule. The graph overhaul does not pre-claim a numbering
range; numbers assign at fan-out, mechanical.

## Cross-references

* [`round-2-plan.md`](round-2-plan.md) §"Hybrid back-side
  revisited" — Task F updated 2026-05-21 to absorb the
  chan-reports toggle.
* [`../phase-8-bugs.md`](../phase-8-bugs.md) — G1 + G9
  entries filed 2026-05-21 (NOT YET DISPATCHED; folds
  into this overhaul).
* [`report-extensions-ideas.md`](report-extensions-ideas.md)
  "Other ideas worth considering" — the cross-directory
  aggregation idea is now load-bearing for G3.
* `fullstack-a-33` — graph-from-here as default +
  ancestor breadcrumb in inspector (landed v0.11.1). The
  breadcrumb work is preserved; G2 layers the visual
  ancestor-chain plotting on top.
* `crates/chan-report/README.md` — current chan-report
  scope (per-file language detection + SLOC + COCOMO);
  read before extending in G3.

## What this plan is NOT

* A commit-grouping plan. That gets cut at Round-2 close
  per the existing `commit-plan-v<X>.md` shape.
* A dispatch wave. Tasks fan out when bandwidth opens
  + Hybrid back-side wave settles.
* A scope-creep gate. New graph asks that surface during
  the wave land here as appends; this artifact stays
  current.

## 2026-05-21 — Refinement: depth semantic is node-type-dependent + filter toolbar (G10)

@@Alex 2026-05-21 follow-up:

> if the node is a directory, depth shows other
> directories and their files and documents - we need
> filters for files, documents, contacts, hashtags so
> that we can decide what to plot e.g. when the
> directory is a node and depth increases

Two refinements on top of the locked decision #5
("forward = outgoing-edge direction").

### Depth semantic is node-type-dependent

When the rooted node is a **DIRECTORY**:

* depth N reveals subdirectories + their files +
  documents N hops down the filesystem tree.
* Sub-content (files, documents, contacts, hashtags)
  honours the filter toolbar (below).

When the rooted node is a **FILE / DOCUMENT**:

* depth N reveals outgoing markdown-link targets
  N hops out (per decision #3 + G5).

For **OTHER node types** (language, hashtag, mention),
"forward" maps to the type's natural outgoing edge:

* language → directories containing files of that
  language (per G7 + G8).
* hashtag → files / documents tagged with it.
* mention → contacts referenced.

### Filter toolbar — G10 (NEW)

| # | Scope | Lane | Size | Dependencies |
|---|-------|------|------|--------------|
| G10 | Filter toolbar for graph node-type plotting — files, documents, contacts, hashtags (+ existing language filter consolidates here) | @@FullStackA (SPA filter UI + state) + @@Systacean (graph route honours filter set in response shape) | medium | G2 + G3 |

Filters multiply with depth: at depth N from a
directory root, only the enabled node-type filters
render. Default-on / default-off state per filter TBD
at fan-out (likely defaults: files ON, documents ON,
contacts OFF, hashtags OFF — implementer can pick;
Settings panel may persist the defaults later).

The legend grid (Task D, Hybrid Graph back-side) already
enumerates the node types; the filter toolbar is the
gating control. Task D + G10 share the node-type
taxonomy + colour palette as the single source of truth.

### G9 (depth slider) reframed

G9 stops being a "fix the broken slider wire" bug and
becomes "implement the depth slider's correct semantic
(node-type-dependent forward reveal)". The current
slider may not be reusable at all — depending on the
audit, G9's scope may shrink to "remove dead slider
component" and the actual reveal logic ships in G2 +
G10 + G5.

### Sequencing update

G10 + reframed G9 land with the first graph sub-wave
(G2 + G3 + G6 + Task D + G10). The visual + structural
+ interactional rework ships as one coherent unit.

### Architecture shape after this refinement

Cleaner picture of how the pieces relate:

| Piece | Role |
|-------|------|
| G2 | Layout spine — filesystem-hierarchy + ancestor plotting |
| G3 | Directory nodes + inspector + reports stats |
| G6 | Colour scheme (markdown / source code / binary / media) |
| G10 | Filter toolbar — gates which node types render |
| G9 | Depth slider — controls how far forward to reveal (folds into G2 + G10's implementation, may not be standalone) |
| Task D | Hybrid Graph back-side legend grid (shares taxonomy with G6 + G10) |
| G4 | File inspector — Open + Graph-from-here |
| G5 | Markdown-link plotting semantics (no parent-dir edges; user-driven dive) |
| G7 | Language nodes with edges to visible directories |
| G8 | Graph-from-language plots first-depth dirs containing language |

The first sub-wave (G2 + G3 + G6 + G10 + Task D, plus
G9 absorbed into G2/G10) is the load-bearing structural
+ visual rework. G4 + G5 + G7 + G8 are smaller layered
additions that follow.

## 2026-05-21 — Clarification: filters are for NODE types only, never edges/links

@@Alex 2026-05-21: "we do not need the filter for 'links'
to show/hide edges, does not make sense to me".

Confirming: G10's filter toolbar covers NODE-type
visibility (files, documents, contacts, hashtags,
language). There is NO filter for graph edges/links.
Edge visibility is implicit: an edge renders iff both
endpoints render under the current node-type filter
set + depth.

The "Search OVERLAY (Cmd+K F)" and the global Search
UX are out-of-scope for this plan entirely; the
"Search settings" name on the Hybrid FB back is about
the INDEXING / discovery settings, not about an
edges/links visibility control.

No edge-filter UI to design or build.
