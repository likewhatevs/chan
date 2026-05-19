# fullstack-35: empty-pane infographic carousel (metadata + indexing graph)

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-19

## Goal

Replace / extend the empty-pane welcome content with a
carousel-style widget surfacing chan's metadata + the
indexing state visually. The existing welcome content
(Chan logo, dashboard placeholder, keyboard shortcuts)
stays as one slide of the carousel; new slides bring
in infographics. @@Alex called this out as a
visual-experimentation surface — small bets here are
fine.

## Relevant links

* @@Alex's chat note 2026-05-19 05:30 BST.
* Empty-pane welcome surface — currently in
  `Pane.svelte` (or sibling), rendered when
  `pane.tabs.length === 0`.

## Acceptance criteria

### Carousel structure

* The empty-pane content becomes a horizontal carousel
  with at least these slides, in order:
  1. **Welcome** — current content (Chan logo +
     dashboard placeholder + keyboard shortcuts).
     Unchanged; just becomes slide 1 of N.
  2. **Metadata** — see below.
  3. **Indexing** — see below.
* Carousel navigation: subtle dot indicators +
  arrow affordances.
* **Auto-rotate every 5 seconds**, starting from
  slide 1 (Welcome). The user sees the welcome
  content first by default and the carousel cycles
  through the other slides automatically.
* Auto-rotate pauses on pointer hover and on focus
  within the carousel; resumes after the pointer
  leaves / focus exits.
* Keyboard left/right when the carousel is focused
  navigates manually (and pauses auto-rotate while
  focused).
* Right-click on the carousel background still opens
  the pane hamburger menu (current behavior; don't
  break it).

### Slide 2 — Metadata infographic

* Show chan's metadata size and a breakdown.
  Examples of data available in chan-server today:
  drive root path, drive size on disk, indexer size
  (BGE-small model + indices), per-known-area
  breakdown if any. Make a visually-experimental
  infographic — bar / pie / sparkline — that
  communicates "how much space is what".
* Numbers can be approximate; this is a UX-
  experimentation surface. Don't over-engineer.

### Slide 3 — Indexing-state graph

* A directory-only graph rooted at the drive root.
  Files are NOT plotted; just directories.
* Each directory node renders in one of three states:
  * **Grey** — not indexed yet.
  * **Orange** — indexing in flight.
  * **Green** — fully indexed.
* Orange nodes **pulsate subtly** (slow alpha or
  scale animation) to signal in-flight work. Other
  states are static.
* Same label rule as the main graph (`fullstack-32`):
  labels render for the selected node + 1 depth
  neighbors.
* If a dir has substructure, hover / click expands /
  contracts (reuse the main graph machinery if
  possible).
* Backend signal: chan-server's indexer already
  tracks per-directory state for the BGE-small
  embedding index. Surface that via a small API
  (`GET /api/indexing/state` returning a tree of
  `{path, state}` nodes) — coordinate with
  @@Systacean for the endpoint shape.

## Out of scope

* Reworking the indexer itself.
* Cross-pane / shared carousel state.
* Click-through actions from the indexing graph
  (just visualization for this pass).
* Metadata breakdowns that require new chan-server
  endpoints (only surface what's already available
  or trivially derivable).

## How to start

1. Inventory empty-pane render code in `Pane.svelte`.
   Refactor into a `<EmptyPaneCarousel>` component
   with N slides.
2. Slide 1: paste in existing welcome content
   verbatim.
3. Slide 2: query whatever metadata APIs already
   exist; render a small infographic. If new
   metadata is needed, coordinate with @@Systacean.
4. Slide 3: pair with @@Systacean on
   `GET /api/indexing/state` (or whatever endpoint
   shape they prefer). Render the dir-only graph;
   reuse the main graph component if you can.
5. Pulsating orange: CSS `@keyframes pulse` with
   2-3s cycle, animation-iteration-count infinite,
   applied only to `state === "indexing"` nodes.

## Hand-off

Standard. Pre-push gate green. Coordinate with
@@Systacean on the indexing-state API + any metadata
endpoint shape before implementing. Ping via
`alex/event-fullstack-architect.md`.
