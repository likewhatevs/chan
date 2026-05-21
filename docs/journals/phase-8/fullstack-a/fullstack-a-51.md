# fullstack-a-51 — Graph overhaul G6 + Hybrid Graph legend grid (Task D combined)

Owner: @@FullStackA
Cut: 2026-05-21 by @@Architect
Status: queued (sequenced AFTER fullstack-a-43 + systacean-16)

## Goal

Two visually-coupled pieces shipped as one task:

1. **G6 colour scheme**: re-stratify graph node colours
   by file-class bucket.
2. **Hybrid Graph back-side legend grid** (Task D from
   the Hybrid back-side wave): renders the new colour
   scheme as `[Node] [Colour]` rows.

## Background

Locked design at
[`../architect/graph-overhaul-plan.md`](../architect/graph-overhaul-plan.md)
§"Architecture overhaul" — G6. @@Alex 2026-05-21:

> the colours: we want orange only for markdown.. all
> other source code files should be the royalblue we
> discussed sometime ago, and binary files grey..
> media remains purple

Plus the Hybrid Graph back-side legend grid from
[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Hybrid back-side revisited" — Task D. The two were
originally separate (Task D was in the Hybrid back-side
wave; G6 is a graph palette change). Bundled because
the legend renders the new palette — they must ship
together for the visual story to be coherent.

## Acceptance criteria

### G6 colour scheme

| Node class | Colour |
|------------|--------|
| Markdown   | orange |
| Source code | royalblue |
| Binary     | grey |
| Media      | purple (kept) |
| Directory  | (TBD — implementer picks; suggest a neutral mid-tone that reads as "container" against the four file-class colours; flag the choice in task tail for @@Alex confirm) |
| Hashtag    | (existing palette) |
| Mention    | (existing palette) |
| Language   | (existing palette) |

Source for the markdown / source / binary / media
classification: `systacean-16`'s new file-class
buckets in chan-report.

### Hybrid Graph legend grid (Task D)

* `HybridGraphConfig.svelte` (the empty body
  placeholder from `-a-43`) populates with a grid:
  rows are node types; each row shows `[Node label]
  [Colour swatch]`.
* Reads colour values from the central palette source
  introduced by G6.
* Rows: all node types currently in the graph
  (Directory, File Markdown, File Source, File
  Binary, File Media, Hashtag, Mention, Language).

### Tests

* Colour-mapping tests for each bucket.
* Legend grid renders all current node types with
  matching swatches.
* Pre-push gate green.

## How to start

1. Confirm `systacean-16`'s file-class API is live.
2. Audit current graph node colour palette — where it
   lives (likely a const map in `GraphPanel.svelte` or
   a sibling module).
3. Re-stratify the palette: markdown / source code /
   binary / media → new mapping. Add the directory
   colour (flag the pick for @@Alex if surprising).
4. Build the legend grid component in
   `HybridGraphConfig.svelte`.
5. Tests + verify gate.

## Coordination

* SPA-primary; consumes `systacean-16`'s file-class
  API for the markdown/source/binary/media split.
* Append "Commit readiness" + poke @@Architect when
  ready.

### Sequencing constraint — HARD prereqs

* `fullstack-a-43` (Task A) — `HybridGraphConfig.svelte`
  needs to exist.
* `systacean-16` (file-classification buckets).

Can land in parallel with G2 (`-a-49`) + G3 (`-a-50`)
since it touches different layers (palette + back-side
component vs layout + inspector).

## Numbering

This is `-a-51`. See `-a-45` for broader wave
numbering note. NOTE: This task absorbs Task D from
the Hybrid back-side wave — Task D does NOT get a
separate slot.
