# fullstack-a-75 — Drive metadata Carousel redesign + Infographics tab container (Round-2 items 1+4 coupled)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched
Round: 2 wave-3

## Goal

Redesign the empty-pane carousel + introduce the
Infographics tab container per the Round-2 plan
items 1+4 (coupled).

## Reference

[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Surface unification" (line 118+) and §"Empty-pane
carousel slide 1" (line 169+).

## Scope

### Carousel redesign

* Replace the current shortcut table in
  `EmptyPaneCarousel.svelte` slide 1 with the four
  spawn actions (Terminal / File Browser / Rich
  Prompt / Graph).
* Item ordering: Terminal, File Browser, Rich
  Prompt, Graph. Separator. Then existing items
  (highlight colour picker, any others).

### Infographics tab container

* New tab kind: Infographics.
* Move the carousel's shortcut table into the new
  Infographics tab type (per round-2-plan: "the
  carousel's shortcut table will move into the new
  Infographics tab type").
* User can open an Infographics tab from the
  hamburger / right-click menus.

### Surface unification across 3 menus

Three menus must show the same first-class spawn
items:

1. **Empty-pane carousel slide 1** (`EmptyPaneCarousel.svelte`).
2. **Pane hamburger menu** (`Pane.svelte::paneMenu`).
3. **Empty-pane right-click menu** (`Pane.svelte::emptyPaneMenu`).

All three lists the same: Terminal, File Browser,
Rich Prompt, Graph. Separator. Existing items.

### Single commit

Per round-2-plan §"Task": single commit since menus
reference the chord set; splitting would produce a
half-state where hints mismatch the runtime.

## Acceptance

1. **Carousel slide 1** shows 4 spawn actions
   (Terminal/FB/RP/Graph) in order.
2. **Hamburger menu** matches the same 4 entries at top.
3. **Empty-pane right-click menu** matches the same.
4. **Infographics tab** spawnable; contains the
   former carousel shortcut table.
5. **PaneModeHelp + SERVE_LONG_ABOUT** resynced if
   they reference the old shortcut layout (audit at
   pickup).

### Tests

Vitest pins per menu surface + Infographics tab
shape.

### Gate

`npm test` / `npm run check` / `npm run build` green.

## Coordination

* @@FullStackA. SPA-only.
* Atomic-audit-commit. Per-path staging.

## Authorization

Yes for SPA carousel + Pane menus + new Infographics
tab component + tests + task tail + outbound.

## Numbering

This is `-a-75`.

## Out of scope

* Backend changes.
* Existing chord bindings (already migrated in prior
  waves).

## 2026-05-23 — slice 1 (Infographics tab + carousel redesign)

SPA-only. Single commit per the architect's
"single commit, menus reference the chord set"
guidance.

### What landed

**New tab kind: Infographics**

* `InfographicsTab` discriminated-union variant
  in `tabs.svelte.ts` (kind="infographics" + id +
  title). Cloner, serializer (k:"i"), tabLabel,
  `resolveSpawnContext` (no path context →
  drive root) all extended.
* `openInfographicsInPane(paneId)` +
  `openInfographicsInActivePane()` helpers.
* `InfographicsTab.svelte` component hosts the
  ASCII shortcut table (lifted from
  `EmptyPaneCarousel.svelte` slide 1). Renders
  a labeled `<h2>Shortcuts</h2>` header + the
  monospace shortcut table inside an
  `aria-label="Infographics"` region. Open via
  the "Infographics" entry in the pane
  hamburger / empty-pane right-click / carousel
  secondary band.

**Pane.svelte changes**

* Imports: `InfographicsTab` component +
  `BarChart2` lucide icon.
* New `:else if active?.kind === "infographics"`
  render branch alongside the existing
  file/graph/browser branches.
* `emptyPaneExtraActions` gains an Infographics
  row (`app.infographics.open` command).

**Carousel changes**

* Dropped the ASCII shortcut table block
  (`<pre class="placeholder-shortcuts">`).
* Dropped `renderTable` import + `shortcutTable`
  variable.
* `spawnEntries` now mirrors Pane.svelte's
  `spawnActions`: New Draft (slot 0) +
  Terminal / FB / RP / Graph. Pre-`-a-67 slice
  2` the carousel only had the 4 spawn items;
  the missing New Draft was a drift.
* New `secondaryEntries` array with the
  Infographics entry.
* Markup adds a `.spawn-sep` divider + a
  `.spawn-row.spawn-row-secondary` band between
  the primary spawn band and the page-hint.

**App.svelte**

* New `case "app.infographics.open"` in the
  runCommand switch → calls
  `openInfographicsInActivePane()`.
* Imports: `openInfographicsInActivePane` +
  `openInPane` (already needed by
  `materializeStagedDraftEditors` from
  `-a-68 slice 2`).
* `App.svelte`'s autosave-effect tab-walking
  loop extended with an explicit
  `if (t.kind === "browser")` arm + a fall-
  through that touches `t.id` for infographics.
  Without this the TS narrowing breaks: the
  `else` block had to handle `BrowserTab |
  InfographicsTab`, but `inspectorOpen` lives
  only on browser.

### Surface unification across 3 menus

The three first-class spawn surfaces now ship
the same set + ordering:

| Surface              | Primary band                              | Secondary    |
|----------------------|-------------------------------------------|--------------|
| Pane hamburger       | New Draft / Terminal / FB / RP / Graph    | (none)       |
| Empty-pane right-click| New Draft / Terminal / FB / RP / Graph + extras | Infographics |
| Empty-pane carousel  | New Draft / Terminal / FB / RP / Graph    | Infographics |

The pane hamburger keeps its existing tail
(Enter Hybrid Nav + focus-border palette);
Infographics isn't added there in this slice
(it's accessible via empty-pane right-click +
carousel). Slice 2 could add it to the
hamburger if @@Alex wants surface 3/3 to also
carry it.

### Decisions

* **New Draft at slot 0 across all three
  menus**: matches the post-`-a-67 slice 2`
  reality; the architect's `-a-75` spec listed
  4 spawn items but that pre-dated `-a-67
  slice 2`. Flagged inline in the carousel's
  `spawnEntries` comment.
* **Infographics in the secondary band**, not
  the primary spawn set. It's a read-only
  surface, not a workflow target.
* **Cloner + serializer extended**: dropping
  these breaks session save/restore once any
  user opens an Infographics tab. The serializer
  emits `k:"i"`; the restore path doesn't need
  changes today since the unused-kind code path
  silently drops unknown kinds (deserializer
  TODO if a slice 2 grows real state).
* **`BarChart2` icon** for the Infographics
  affordance (charts > shortcuts visually;
  matches the planned future panels: drive
  metrics, broadcast routing, etc.).

### Files touched

* `web/src/state/tabs.svelte.ts`
  * New `InfographicsTab` type + Tab union
    extension.
  * 2 new exports:
    `openInfographicsInPane`,
    `openInfographicsInActivePane`.
  * Cloner + tabLabel + tabTooltip + SerTab
    discriminator + serializer extended.
* `web/src/state/store.svelte.ts`
  * `resolveSpawnContext` switch gains an
    `infographics` arm (returns drive root).
* `web/src/components/InfographicsTab.svelte`
  (new) — body of the new tab kind.
* `web/src/components/Pane.svelte`
  * Import `InfographicsTab` + `BarChart2`.
  * Render branch added.
  * `emptyPaneExtraActions` row added.
* `web/src/components/EmptyPaneCarousel.svelte`
  * Shortcut table block dropped.
  * `renderTable` import + `shortcutTable`
    variable dropped.
  * `spawnEntries` extended with New Draft.
  * New `secondaryEntries` array + markup
    band + CSS.
* `web/src/App.svelte`
  * Import `openInfographicsInActivePane`.
  * New `case "app.infographics.open"`.
  * Autosave-effect tab-walking loop
    extended for the `infographics` arm.
* `web/src/components/Pane.test.ts`
  * `empty pane right-click shows the welcome
    menu` pin updated to include Infographics
    between Search and Settings.
* `web/src/components/infographicsTabAndCarousel.test.ts`
  (new) — 17 architectural pins for the type +
  helpers, render branch, command routing,
  carousel changes, Infographics body.

### Gate

* `svelte-check` → 0/0.
* `vitest` → **1215 / 1215** (+17 from
  `-a-68 slice 2`'s 1198; 17 new pins).
* `npm run build` → clean.
* `cargo fmt --check` + `clippy
  --all-targets -- -D warnings` → clean
  (no Rust delta).

### Suggested commit subject

```
Infographics tab + carousel redesign (fullstack-a-75)
```

### Files (per-path)

* `web/src/state/tabs.svelte.ts`
* `web/src/state/store.svelte.ts`
* `web/src/components/InfographicsTab.svelte` (new)
* `web/src/components/Pane.svelte`
* `web/src/components/EmptyPaneCarousel.svelte`
* `web/src/App.svelte`
* `web/src/components/Pane.test.ts`
* `web/src/components/infographicsTabAndCarousel.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-75.md`

Autonomous-commit mode. No clearance held.
Next: `-a-79`/`-a-80` Team orchestrator
(addendum-b headline).
