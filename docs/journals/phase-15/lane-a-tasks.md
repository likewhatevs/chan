# Lane A — two-face flip + Dashboard carrousel

## Bootstrap prompt

You are **@@LaneA**. Read `bootstrap.md`, then this file, then
`plan-round-1.md`, then `coordination.md` (do not read `roadmap-round-1.md` —
it is @@Alex's and already decomposed here). You own the frontend core: the
hybrid
flip and the Dashboard redesign. Mission: re-architect the hybrid flip to a
true two-face card, then redesign the Dashboard carrousel into per-slot
front/back surfaces, relocating the chan-reports / semantic-search /
embedding-model / search-Index sections into the Dashboard slot backs. You
may run two subagents: **A-core** (edits the entangled existing files,
serially) and **A-helper** (builds new standalone components in isolation).
Coordinate directly with @@Alex; cut tasks to @@LaneB / @@LaneC for anything
inside their files. Confirm your understanding and surface open decisions to
@@Alex, then wait for the go to start editing. Do not start before @@Alex
says go.

**You own:** `Pane.svelte`, `EmptyPaneCarousel.svelte`, `DashboardTab.svelte`,
`HybridDashboardConfig.svelte`, `HybridFileBrowserConfig.svelte`,
`FileInfoBody.svelte`, `InspectorBody.svelte`, new components you create, and
the `DashboardTab` type + `flipHybrid` region of `tabs.svelte.ts` (remove the
`paneFlip` bus). You *read but do not edit* `SearchStatusOverlay.svelte`
(copy the Index widget out; @@LaneB deletes the file).

## Tasks (see plan-round-1.md for root causes + file:line refs)

- **A1 (A-core).** True two-face card flip (BUG-1). Render front+back on a
  3D-rotated card, transition driven by `pane.showingBack`. Remove the
  `paneFlip` bus + rAF/`flipActive` logic. Front face `pointer-events:none` +
  `aria-hidden` when flipped; preserve focus-follow. **Smoke Cmd+, on every
  hybrid tab type both directions, focused and unfocused.** Lands before A2.
- **A2 (A-core).** Carrousel per-slot front/back + slot picker on both faces
  (back force-paused) + per-slot Dashboard back. Move sections per
  plan-round-1: About back (Appearance + Screen lock + preview + theme
  relabel), Workspace back (chan-reports), Search back (Index widget +
  Semantic + Embedding); FB settings -> placeholder; Search legend
  conditional. **Reaches `CK-INDEX`** when the Index widget renders in the
  Dashboard Search back -> tell @@Alex + @@LaneB.
- **A3 (A-core).** Dashboard tab right-click menu: per-tab slot on/off
  checkboxes (>=1, default all-checked, serialized beside `carouselSlide`) +
  Settings(Cmd+,); unchecked slots skipped in auto-rotation.
- **A4 (A-core).** Search-slot inspector buttons (BUG-2): drop Upload here,
  add Show Directory + New Terminal.
- **A5 (A-helper).** Build `MatrixRainPreview.svelte` (param'd from
  `MatrixRain.svelte`, reuse `drawStaticMatrix`) + per-slot config bodies as
  standalone components. **Reaches `CK-COMPONENTS`** -> hand to A-core.

## Coordination checkpoints

- A1 -> A2 (internal sequence).
- `CK-COMPONENTS`: A-helper -> A-core.
- `CK-INDEX`: A -> @@LaneB (gates @@LaneB's overlay deletion).

## Open decisions to raise with @@Alex
- Metadata-archive section new home (proposed: Workspace slot back).

## Decisions (resolved 2026-05-30, @@Alex)
- **Metadata archive home:** Workspace slot back (beside chan-reports).
- **Third slot label:** relabel front title + slot-picker entry "Indexing"
  -> "Search". Indexing graph stays as the front content; `DashboardTab`
  doc comment for slide 2 updates to "Search".
- **Slot picker shape:** keep the dot row + prev/next + play/pause as-is;
  add slot labels on hover (front) and a text label row (back). Back is
  force-paused. No segmented-tab replacement.

## Progress
- **A1 DONE + browser-validated + COMMITTED f5c773c5 (2026-05-30).** True
  two-face card flip in `Pane.svelte` (+ `tabs.svelte.ts` paneFlip-bus
  removal + test updates). Smoked across all 5 hybrid surfaces, both
  directions, focused + multi-pane. Full results in `lane-a-journal.md`.
  Committed solo via filtered `git apply --cached` (staged split from
  @@LaneC's live tabs.svelte.ts; their tree untouched).
- **A5 part 1 DONE (A-helper), NOT yet committed.**
  `screensaver/MatrixRainPreview.svelte` + extracted `screensaver/
  matrixRain.ts`; `MatrixRain.svelte` refactored to import the helper.
  svelte-check 0/0, 23 screensaver tests green. Ready for A2 wiring + its
  own commit.
- **Next: A2** (per-slot Dashboard back). Plan: do the A-only files first
  (Pane.svelte back arm, EmptyPaneCarousel, Hybrid configs) and defer the
  A3 tabs.svelte.ts slot-enable serialization until @@LaneC commits, to
  avoid re-mixing the shared file. CK-INDEX fires mid-A2 -> ping @@LaneB.

## Lane-A-side calls (flagged to @@Alex, not blocking)
- **Always-mounted back lifecycle:** two-face card keeps both faces in the
  DOM; gate the Index poller (`/api/indexing/state`) + the screensaver
  `MatrixRain`/`MatrixRainPreview` rAF loop on `pane.showingBack` so nothing
  animates/polls while front-facing. Components stay mounted; only the loops
  start/stop.
- **A3 enabled-slots serialization:** store only *disabled* slot ids beside
  `cs` in the session hash; omit the key when all slots enabled (pre-release,
  no migration). If the persisted `carouselSlide` points at a disabled slot
  on load, clamp to the first enabled slot.
