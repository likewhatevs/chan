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

## 2026-05-19 10:35 BST — phase 1 landed (@@FullStackB)

Carousel scaffolding + slides 1 (Welcome) + 2 (Metadata)
land in this pass. Slide 3 (Indexing graph) ships once
@@Systacean's `GET /api/indexing/state` endpoint lands;
for now it renders a clearly-flagged stub that still
reflects the current `indexStatus` value (`indexing N/M`
/ `reindexing` / `idle`) so the carousel-shape
walkthrough works end-to-end.

Files:

* `web/src/components/EmptyPaneCarousel.svelte` (new)
  * 3-slide carousel: dot indicators + chevron arrows
    at the bottom, 5 s auto-rotate starting on slide 1.
  * Auto-rotate pauses on pointer hover AND on
    focus-within; both signals are tracked separately
    and OR'd into a `paused` derived. Resume fires
    automatically when both clear.
  * `ArrowLeft` / `ArrowRight` when the carousel is
    focused nudges manually (and pauses auto-rotate via
    the focus signal until focus leaves).
  * `oncontextmenu` is a forwarded prop; the empty
    pane's right-click welcome menu still opens from
    anywhere on the carousel surface.
  * Slide 1 lifts the pre-existing placeholder content
    verbatim (chan-mark, drive-summary dashboard
    header, "scope-for-graph" hint, shortcut table).
  * Slide 2 builds a stacked horizontal bar of
    file-kind counts from `tree.entries` (document /
    contact / text / media / binary), a legend with
    per-kind counts + dots, and a footer with total
    directories + bytes-on-disk. Empty drives render a
    placeholder "empty drive" bar segment.
  * Slide 3 is the indexing-state stub.
* `web/src/components/EmptyPaneCarousel.test.ts` (new)
  * Asserts default slide is Welcome with 3 dots.
  * Dot clicks navigate to each slide.
  * `oncontextmenu` is forwarded to the parent handler.
  * `ArrowLeft` / `ArrowRight` nudge the active slide.
* `web/src/components/Pane.svelte`:
  * Replaced the old placeholder block. Single-pane
    lone-pane case now renders `<EmptyPaneCarousel>`;
    multi-pane empty case still shows the bare chan
    mark (unchanged rhythm — extra panes during
    workspace setup don't need the full carousel).
  * Dropped now-dead imports + derivations:
    `drive`, `indexStatus`, `renderTable`,
    `driveSummary`, `indexLabel`, `shortcutTable`.
    The carousel owns its own copies.
  * Dropped now-orphan CSS rules:
    `.placeholder-shortcuts`, `.placeholder-hint`,
    `.dashboard-header`, `.dashboard-name`,
    `.dashboard-stats`, `.dashboard-index`.
    `.placeholder`, `.placeholder-stack`, and
    `.placeholder-mark` stay for the multi-pane
    bare-logo branch.

Verification (10:35 BST):

* `npm run test` → 32 files / 281 tests pass.
* `npm run check` → 0 errors / 0 warnings.
* `npm run build` → clean (existing chunk-size +
  dynamic-import warnings unchanged).
* `scripts/pre-push` → green
  (fmt + clippy + cargo test + no-default-features
  build).

Out of scope for this pass:

* Slide 3 dir-only graph (needs `/api/indexing/state`
  from @@Systacean). The stub will be swapped out
  once the endpoint lands; the `<EmptyPaneCarousel>`
  surface is the only file that needs to change for
  that follow-up.

Commit message proposed:
`Empty-pane carousel scaffolding + slides 1+2 (fullstack-35 phase 1)`.
