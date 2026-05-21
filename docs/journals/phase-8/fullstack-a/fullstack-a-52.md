# fullstack-a-52 — Graph overhaul G10 + G9: filter toolbar + depth slider semantic

Owner: @@FullStackA
Cut: 2026-05-21 by @@Architect
Status: queued (sequenced AFTER fullstack-a-43; can run parallel with -a-49 / -a-50)

## Goal

Two pieces shipped as one task:

1. **G10 filter toolbar**: node-type filters (files,
   documents, contacts, hashtags + existing language
   filter consolidates) gate what plots at depth N.
2. **G9 depth slider re-impl**: depth N reveals
   node-type-dependent forward content from the
   current root.

## Background

Locked design at
[`../architect/graph-overhaul-plan.md`](../architect/graph-overhaul-plan.md)
§"Architecture overhaul" — G9 + G10 + the
"Refinement: depth semantic is node-type-dependent +
filter toolbar (G10)" section.

@@Alex 2026-05-21 directives:

> the depth slider seems not to be working at all..
> that slider should reveal forward nodes as the
> depth increases

> if the node is a directory, depth shows other
> directories and their files and documents - we
> need filters for files, documents, contacts,
> hashtags so that we can decide what to plot e.g.
> when the directory is a node and depth increases

> we do not need the filter for 'links' to show/hide
> edges, does not make sense to me

## Acceptance criteria

### Depth slider (G9)

Reveals forward content N hops out from the current
root. "Forward" semantic is node-type-dependent:

* Root is a **directory**: depth N reveals
  subdirectories + their files + documents N hops
  down the filesystem tree (honouring the filter
  toolbar).
* Root is a **file / document**: depth N reveals
  outgoing markdown-link targets N hops out (per G5).
* Other roots (language, hashtag, mention): depth N
  reveals the type's natural outgoing edges:
  * language → directories containing files of that
    language.
  * hashtag → files / documents tagged with it.
  * mention → contacts referenced.

The current slider may be entirely broken; audit
first. If the current implementation isn't reusable,
remove it + implement fresh as part of this task.

### Filter toolbar (G10)

* Visible toolbar in the graph view (placement:
  implementer picks; suggested: a horizontal strip at
  the top of the graph viewport or in the inspector).
* Filters in v1:
  * Files (Regular).
  * Documents (Markdown).
  * Contacts.
  * Hashtags.
  * Language (consolidates the existing language
    filter).
* Default states (TBD at fan-out; suggest):
  * Files ON, Documents ON, Contacts OFF, Hashtags
    OFF, Language OFF.
* Filter changes apply live to the rendered graph.
* Filters are NODE-type only. There is NO filter for
  edges/links — edge visibility is implicit (edge
  renders iff both endpoints render under the current
  filter + depth).

### Persistence

* Filter state + depth slider value persist per-
  session (URL hash or session store; implementer
  picks).
* Reasonable defaults persist across drive switches.

### Tests

* Depth slider reveals + hides forward content
  correctly for each root-type.
* Filter toolbar toggles render + hide nodes
  correctly.
* Combined depth + filter interactions.
* Pre-push gate green.

## How to start

1. Audit existing depth slider — likely in
   `GraphPanel.svelte` or sibling; confirm whether
   it's reusable.
2. Audit existing language filter — likely a sibling
   state value; consolidate into the new filter
   toolbar.
3. Design the filter toolbar UI placement.
4. Implement depth + filter logic with the
   node-type-dependent semantics.
5. Tests + verify gate.

## Coordination

* SPA-only.
* Can run parallel with G2 (`-a-49`) + G3 (`-a-50`)
  since it touches different layers.
* Append "Commit readiness" + poke @@Architect when
  ready.

### Sequencing constraint — HARD prereqs

* `fullstack-a-43` (Task A) — needed for the back-
  side component shape.
* Optionally pairs with G2 (`-a-49`) for the
  filesystem-rooted layout context, but the filter +
  depth controls can implement against the existing
  graph rendering for an interim state and swap-in
  the G2 spine when it lands.

## Numbering

This is `-a-52`. See `-a-45` for broader wave
numbering note. Subsequent graph overhaul tasks
(G4 + G5 + G7 + G8) cut after this sub-wave lands.
