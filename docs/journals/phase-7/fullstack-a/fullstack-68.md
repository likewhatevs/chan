# fullstack-68: kill Graph bar; filter chips → right-click; hamburger → tab right-click

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Why

`fullstack-64` drops the maximize button +
scope-selector dropdown and reworks the title.
After that lands, the Graph tab still has a
chrome bar carrying the filter chips and the
hamburger menu. @@Alex flagged: kill that bar
entirely, mirror the FB-tab `-67` pattern.

* Filter chips → right-click context menu on
  the Graph tab. Pattern reference: terminal-
  tab right-click has the broadcast items at
  the bottom — similar "secondary controls
  attached to the tab" treatment.
* Hamburger menu items → right-click on the
  Graph tab itself, matching the file-browser
  / editor / terminal tab convention.

End state: Graph tab has no chrome bar at all.
Only the tree (graph canvas) renders inside
the tab body. Discoverability via tab right-
click.

## Relevant code

* `web/src/components/GraphPanel.svelte` —
  after `-64` lands, the header still carries
  filter chips + hamburger. Drop the entire
  `<header>` (or whatever the bar element is
  called after `-64`'s trim).
* `web/src/components/GraphPanel.svelte` —
  the filter-chips render block (filter
  toggles like folder / link / tag / contact
  / language / media etc.; chips are the
  per-tab state surfaced via the `gf:` URL
  hash key per `webtest-a-9` item 1 verdict).
  This block moves into the right-click menu
  payload.
* `web/src/components/GraphPanel.svelte` —
  the hamburger menu items snippet. List of
  items relocates to the right-click menu.
* `web/src/components/Pane.svelte` — tab
  right-click handler. Add a Graph-tab branch
  that renders both the filter chips section
  AND the hamburger items. Pattern reference:
  terminal-tab right-click bottom section
  (broadcast items live there).

## Right-click menu shape

Suggested layout (your call within the
constraint):

```
[Hamburger items at the top]
(separator)
Filters:
  ☑ folder
  ☑ link
  ☑ tag
  ☑ contact
  ☑ language
  ☑ media
```

Filter chips are toggles — clicking flips the
chip state, persisted to `gf:` per the
existing serialization. Per-tab state from
the existing schema; no new state plumbing.

## Acceptance criteria

* Graph tab in pane: no chrome bar / header
  visible. The graph canvas is the only
  rendered surface inside the tab body.
* Right-click on the Graph tab opens a
  context menu with:
  * The hamburger items that previously
    lived on the bar.
  * The filter chips, as toggles, persisted
    to `gf:` hash key on click.
* Filter chip click in the right-click menu
  produces the same state change as the
  current chip click in the chrome bar (the
  walker verified the per-tab persistence
  works; only the surface changes here).
* Hamburger items in the right-click menu
  fire the same actions as before.
* No regression on the graph canvas itself
  — pan / zoom / node click / inspector
  surface unchanged.
* Coordinate with `-64`: `-64` drops the
  maximize button + scope selector + smart
  title. `-68` drops the rest of the bar +
  relocates surviving items. They land in
  sequence on Lane A; this task assumes
  `-64` has shipped first.

### Tests

* Component test: rendered Graph tab DOM
  does not include the bar / header
  element (whatever class name `-64` leaves
  behind, this task removes it entirely).
* Component test: right-click on a Graph
  tab opens a menu containing both the
  hamburger items and the filter chip
  toggles.
* Filter chip click in the right-click
  menu produces the same `gf:` hash mutation
  as the current chrome-bar click.
* Per-tab filter state still round-trips
  via URL hash (regression check on
  `webtest-a-9` item 1's verdict).

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* Re-walk: `webtest-a-9` item 1 (multi
  Graph tabs) wants a re-walk after this
  ships — confirm chip persistence still
  works through the new right-click
  surface.
* The `selectedScope` / `scopeOptions`
  cleanup from `-64` should already be in
  place; this task only deals with the
  bar removal + chip / hamburger relocation.
* Queue position: behind `-64` on Lane A.
  Updated Lane A queue:
  `-66` → `-64` → `-68` → `-61` → `-65`.
* Standing topic-level commit clearance.

## 2026-05-19 16:56 BST — @@FullStackA implementation note

Implementation:

* `GraphPanel.svelte`: extracted the filter-chips render
  block into a `filterChips` snippet. Wrapped the
  existing `<div class="bar">` (chips + hamburger) in
  `{#if !tab}` so it stays visible in the overlay
  variant but disappears in the tab variant. Tab body
  is now canvas-only per the spec.
* `GraphPanel.svelte`: added a `<svelte:window>`
  listener (`onTabMenuKeydown` + `onTabMenuPointerDown`)
  + `tabMenuOpen` / `tabMenuPos` derived state mirroring
  FileEditorTab's pattern. When `tabMenu.openForTabId
  === tab.id`, a `<div class="tab-menu-bubble">`
  renders just below the tab strip via `clampMenu`;
  bubble re-uses the existing `menuItems` snippet at
  the top and the new `filterChips` snippet at the
  bottom under a "Filters" label. One source of truth
  for both surfaces — chip clicks in the overlay bar
  AND in the tab bubble mutate the same
  `graphState.filters` via the existing `bind:checked`
  on the chip checkbox.
* CSS: added `.tab-menu-bubble`, `.bubble-list`,
  `.bubble-filters`, `.bubble-filters-label` rules
  matching the file-editor / terminal-tab bubble
  chrome (border, shadow, z-index 50, max-height
  with overflow). Reused `.tab-menu-bubble .filters`
  for the chip layout inside the bubble.
* Imports: `clampMenu` from `./menuClamp`, `tabMenu` +
  `closeTabMenu` from `../state/tabMenu.svelte`. No
  new dependencies.

`fullstack-64`'s `synthesizeScope()` fallback +
`graphTitle()` rewrite are already on `main`
(`d8ee2e8`); this task layers on top with the chrome
removal.

The Pane.svelte tab strip already wires
`oncontextmenu` → `openTabMenu(t.id, ...)` for graph
tabs (the same dispatcher used by file editor /
terminal tabs), so no Pane.svelte change is needed —
existing right-click on the tab now opens the
GraphPanel-owned bubble.

Tests added in `revealBrowserActions.test.ts`:

* `GraphPanel hides the chrome bar when rendered as a
  tab` — verifies the `{#if !tab}` gate around the
  bar.
* `GraphPanel renders a tab-menu-bubble carrying
  menuItems + filterChips` — verifies the bubble
  contains both snippets, gated on `tab &&
  tabMenuOpen`.

Per-tab filter state (`gf:` URL hash key) still
round-trips since the chip checkboxes still write
into `graphState.filters` via `bind:checked` — the
plumbing is unchanged.

Gate green:

* `npm run test -- revealBrowserActions` (17 passed),
* `npm run test` (365 passed),
* `npm run check` (0 errors / 0 warnings),
* `npm run build`,
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`
  (green).

Visual eyeball worth doing: open a Graph tab,
verify body is canvas-only (no bar). Right-click
the tab name; bubble shows reload / depth slider /
details toggle / settings on top + filter chips at
the bottom. Toggle a chip from the bubble; verify
the canvas updates AND the `gf:` URL hash changes.

Proposed commit message:

> Kill Graph tab chrome bar; chips + menu items move to tab right-click (fullstack-68)
>
> The Graph tab no longer renders a chrome bar; the
> body is canvas-only. The filter chips and the
> former hamburger menu items relocate to a
> tab-menu-bubble that opens on tab-strip right-
> click. Overlay variant keeps the bar (no tab-strip
> right-click available). Chip toggles in either
> surface mutate the same `graphState.filters`, so
> per-tab `gf:` URL hash persistence is unchanged.
> Mirrors the file-editor / terminal-tab right-click
> bubble pattern (clampMenu positioning, Esc /
> pointer-down outside dismiss).
