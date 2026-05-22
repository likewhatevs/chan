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
