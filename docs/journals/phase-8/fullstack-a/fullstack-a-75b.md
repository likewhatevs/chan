# fullstack-a-75b — Carousel relocate to Infographics tab; welcome pane becomes static spawn grid

Owner: @@FullStackA
Cut: 2026-05-23 by @@Architect
Status: dispatched
Round: 2 wave-3 (follow-up)

## Goal

Per @@Alex's route on the `-a-75` walk: the
carousel widget moves from the welcome surface
(back of empty pane) into the Infographics tab.
The welcome pane becomes a static spawn grid only.

## Reference

@@Alex (`d4a3fc8`): "This is correct we will no
longer have the carousel in the back of the pane
and it will only live in the tab from now on."

@@WebtestA's walk + addendum (`7cc48a0` + `2dded48`)
flagged the UX gap: Infographics shipped as a static
page; carousel rotation lived on welcome. @@Alex's
direction inverts the placement.

## Scope

1. **Move carousel component** from EmptyPane /
   welcome surface to InfographicsTab body. Preserve
   rotation + play/pause + pagination UX end-to-end.
2. **Strip back-of-pane carousel**. Welcome surface
   reduces to:
   * 5-tile static spawn grid.
   * Footer hint ("Each pane's visible tab is part of
     the scope for Graph.").
3. **Infographics tab default slide order**:
   Shortcuts first; future slides appended.

## Acceptance

1. Empty pane: spawn grid + hint only; no carousel
   widget, no rotation, no play/pause.
2. Infographics tab opened from anywhere: hosts the
   full carousel widget with rotation + pagination +
   play/pause + slide ordering.
3. No regression on the spawn-tile click handlers
   (Cmd+T / Cmd+O / etc.).

### Tests

Vitest pins:
* Welcome surface markup contains spawn-grid +
  hint; no carousel component import.
* InfographicsTab body imports + mounts the
  carousel.
* Carousel rotation + play/pause behaviors
  preserved (existing pins migrate if needed).

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA. SPA-only.
* Autonomous-commit per batch dispatch standing
  auth.
* Atomic-audit-commit.

## Authorization

Yes for `web/src/components/EmptyPane.svelte` (or
welcome component) + `InfographicsTab.svelte` +
carousel component file move + tests + task tail +
outbound.

## Numbering

This is `-a-75 slice 2` (filename `-75b` for
clarity; slice 1 was the initial Carousel +
Infographics ship at `ba381f6`).

## Out of scope

* Re-styling the spawn tiles.
* Adding new slides to the carousel.
* Multi-pane carousel sync (if any was hypothetical).

## 2026-05-23 — slice 1 ready

SPA-only. Per @@Alex's route on the slice-1
walk: carousel widget relocates from the
welcome surface to the Infographics tab; the
welcome reduces to a static spawn grid.

### Shape applied

**New `EmptyPaneWelcome.svelte`**

* Static welcome surface: logo + drive name +
  5-tile spawn grid (New Draft / Terminal /
  FB / RP / Graph) + Infographics tile +
  footer hint. No carousel chrome, no
  rotation, no play/pause.
* Spawn entries mirror
  `Pane.svelte::spawnActions`.
* Clicks dispatch `chan:command` events —
  same chord-routed handlers the carousel's
  old slide 1 used.

**Carousel reshape**

* Slide 1 (Welcome) replaced with Slide 1
  (Shortcuts) — renderTable + monospace
  `<pre>` inside `.slide-shortcuts` block.
* `renderTable` import restored.
* Dropped:
  - `spawnEntries` / `secondaryEntries` /
    `SpawnRow` / `dispatchSpawn` / `chordLabel`.
  - Welcome chrome (placeholder-mark /
    dashboard-header / spawn-row / spawn-sep /
    spawn-row-secondary).
  - Now-unused lucide icons (BarChart2,
    FilePlus, Folder, MessageSquare, Network,
    Terminal).
  - SHORTCUTS / formatChord (no longer used
    here).
* Slides 2 + 3 (Metadata + Indexing graph)
  unchanged.

**InfographicsTab.svelte**

* Body now imports + mounts
  `<EmptyPaneCarousel />`. The earlier static
  ASCII table block (from `-a-75` slice 1)
  retires.

**Pane.svelte**

* Empty-pane placeholder swap:
  `<EmptyPaneCarousel>` → `<EmptyPaneWelcome>`.
* `EmptyPaneCarousel` import dropped (owned
  by InfographicsTab now).

### Files touched

* `web/src/components/EmptyPaneWelcome.svelte`
  (new) — static welcome surface.
* `web/src/components/EmptyPaneCarousel.svelte`
  * Imports + module-init clean-up.
  * Slide 1 rewritten as Shortcuts.
  * CSS: dropped welcome / spawn-row styles;
    added `.slide-shortcuts` +
    `.shortcuts-table`.
* `web/src/components/InfographicsTab.svelte`
  * Body collapses to a single
    `<EmptyPaneCarousel />` mount.
* `web/src/components/Pane.svelte`
  * Import swap + placeholder mount swap.
* `web/src/components/EmptyPaneCarousel.test.ts`
  * "renders the welcome slide by default
    with three dots" → "renders the
    Shortcuts slide by default with three
    dots".
  * Dot-click + arrow-key tests updated for
    the new slide-1 selector.
* `web/src/components/infographicsTabAndCarousel.test.ts`
  * Slice-1 pins for spawn entries +
    welcome chrome flipped from REQUIRE to
    FORBID; new pins for the Shortcuts
    slide + InfographicsTab carousel mount
    + EmptyPaneWelcome shape + Pane import
    swap.

### Decisions

* **5-tile spawn grid in welcome** (matches
  `Pane.svelte::spawnActions` post-`-a-67
  slice 2`). The grid-template-columns
  bumped to `repeat(5, …)` from the prior
  4-up.
* **Infographics tile stays in the secondary
  band**, not the primary spawn row.
  Discoverable from welcome without
  competing with the first-class spawn
  targets.
* **Footer hint preserved** verbatim ("Each
  pane's visible tab is part of the scope
  for Graph.") — single-line spec text.
* **Carousel forwarder kept** (`oncontextmenu`
  prop on EmptyPaneCarousel) for symmetry
  with the prior mount site, even though
  the new mount inside InfographicsTab
  doesn't wire it.

### Gate

* `svelte-check` → 0/0.
* `vitest` → **1251 / 1251** (+1 net from
  `-a-67e` slice 2's 1250).
* `npm run build` → clean.
* `cargo fmt --check` + `clippy
  --all-targets -- -D warnings` → clean
  (no Rust delta).

### Suggested commit subject

```
Carousel relocates to Infographics tab; welcome static spawn grid (fullstack-a-75b)
```

### Files (per-path)

* `web/src/components/EmptyPaneWelcome.svelte` (new)
* `web/src/components/EmptyPaneCarousel.svelte`
* `web/src/components/InfographicsTab.svelte`
* `web/src/components/Pane.svelte`
* `web/src/components/EmptyPaneCarousel.test.ts`
* `web/src/components/infographicsTabAndCarousel.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-75b.md`

Autonomous-commit mode. No clearance held.
Picking up `-a-79`/`-a-80` (unblocked by
systacean-41) next.
