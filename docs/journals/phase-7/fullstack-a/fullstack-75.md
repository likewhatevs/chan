# fullstack-75: align Graph tab right-click menu shape; filter chips vertical

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex flagged the Graph tab's right-click
context menu (from `-68`) is visually
inconsistent with other tabs' right-click
menus and the filter chips are laid out
horizontally instead of one-per-line.

Current Graph bubble (per the screenshot):
* `Show Details` toggle button (oversized).
* `Depth` slider with value display
  (different control category, dominates
  visually).
* `Reload`, `Settings` rows.
* `FILTERS` section header.
* Filter chips as horizontal pills:
  `link 5`, `tag 0`, `contact 0`, `media 4`.

Other tabs' right-click menus
(TerminalTab, FileEditorTab,
FileBrowserSurface menus) use the standard
HamburgerMenu vertical-stack pattern:
* Each row is a `<button class="mbtn">`.
* `[icon] [label] [chord-hint]` left to
  right.
* Section dividers between groups.

The Graph bubble needs to align with that
pattern so users hit the same affordance
shape across surfaces.

## Spec

### Visual shape

* Use the same HamburgerMenu structure as
  the other tab right-click menus.
* Each action row (`Show Details`,
  `Reload`, `Settings`) renders as
  `<button class="mbtn">` with icon +
  label + optional chord on the right.
* `Depth` slider keeps its slider control
  (it's a different category) but the row
  matches the visual weight of the other
  menu rows — same row height, same
  padding, label on the left, slider +
  value on the right.

### Filter rows — one per line

Filters become individual `<button class="mbtn">`
rows, one per filter:

```
[●] folder       N
[●] link         5
[●] tag          0
[●] contact      0
[●] language     N
[●] media        4
```

Each row shows:
* Coloured dot indicating the filter's
  category colour (matches current chip
  colour).
* Filter name (`folder`, `link`, etc.).
* Count on the right.
* Checked-state indicator (e.g. checkmark
  on the left, or filled dot vs hollow,
  matching the existing `colour-swatch`
  pattern from the pane hamburger). Match
  whatever shape the other right-click
  menus use for toggle state — keep the
  convention consistent.

Click toggles the filter (same persistence
to `gf:` URL hash key as today; only the
surface changes).

### Section dividers

The existing layout has implicit grouping
(action rows / FILTERS section). Use the
same `.menu-divider` separators the other
hamburger menus use:

```
Show Details                 (button)
Depth          [slider]  N
─────────────
Reload
Settings           Cmd+,
─────────────
folder    N
link      5
tag       0
contact   0
language  N
media     4
```

## Relevant code

* `web/src/components/GraphPanel.svelte` —
  the `tab-menu-bubble` snippet from `-68`.
  Restyle to match the hamburger row
  shape; relocate the filter chips into
  vertical rows.
* `web/src/components/Pane.svelte` /
  other components — pattern reference
  for `<button class="mbtn">` row shape.
  Terminal tab's right-click menu is
  a good template.
* CSS: the `.tab-menu-bubble` /
  `.bubble-filters` / `.bubble-filters-label`
  rules from `-68` need a sweep.
  Replace with the standard `.menu-row`
  / `.menu-divider` styling shared with
  other hamburger menus.

## Acceptance criteria

* Graph tab right-click menu uses the same
  visual row shape as TerminalTab /
  FileEditorTab / FileBrowserSurface
  right-click menus.
* Filter chips render one per row (no
  horizontal pill layout).
* Each filter row visibly indicates its
  on/off state.
* Filter click still toggles persistence
  through `gf:` URL hash; per-tab
  filter state still round-trips.
* `Show Details` toggle still works and
  reflects the inspector-open state.
* `Depth` slider still functions; row
  visual weight matches the other rows.
* `Reload` + `Settings` actions
  unchanged.
* Section dividers separate the action
  group from the filters group.

### Tests

* Component test: rendered right-click
  menu DOM uses `.mbtn` row class (or
  whatever the standard hamburger row
  class is named).
* Component test: filter rows render
  vertically (one per row, not in a
  horizontal `flex` container).
* Filter toggle: clicking a filter row
  in the menu produces the same `gf:`
  hash mutation as today (regression).

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* v0.11.0-blocking-soft. UX consistency
  win. Could slip to v0.11.1 if your
  queue runs short, but the visual gap
  is conspicuous enough that I'd ship
  it in v0.11.0 if possible.
* Re-walk: light. Lane A's 8801 has the
  Graph state preserved from prior
  walks; a quick visual check after this
  lands closes the loop.
* Queue position: end of Lane A queue.
  Updated queue: `-70` → `-72` → `-73` →
  `-74` → `-75`.
* Standing topic-level commit clearance.

## 2026-05-19 18:55 BST — @@FullStackA implementation note

Implementation:

* `GraphPanel.svelte` tab-menu-bubble render
  block: replaced the `@render menuItems()` +
  `@render filterChips()` indirection with
  inline `<button class="mbtn">` rows. Matches
  TerminalTab / FileEditorTab /
  FileBrowserSurface row shape exactly —
  `mbtn-icon` + `mbtn-label` + `mbtn-chord`.
* `Show Details` row uses `ArrowLeft`/`ArrowRight`
  icon to reflect inspector state; `Reload`
  uses the existing `↻` glyph slot; `Settings`
  shows the `Settings` icon + the registered
  chord via `chordFor("app.settings.toggle")`.
* Depth slider keeps its slider control but
  the row chrome uses `.mbtn` styling — label
  on the left, range input + value on the
  right. `.disabled` class greys it out for
  drive / global scopes (same gate as the
  overlay variant).
* Filter chips become per-row `<button
  class="mbtn filter-row">` entries, one per
  kind. Each row carries:
  * `.filter-dot` — kind-coloured filled
    circle when on, hollow ring when off.
  * `.mbtn-label` — kind name (with the same
    filesystem-mode aliasing the chip had:
    `contains` / `symlink` / `hardlink` /
    `directory` / `contact` / `media`).
  * `.filter-count` — node count for the
    kind on the right, right-aligned with
    tabular-nums.
  * `role="menuitemcheckbox"` + `aria-checked`
    for accessibility.
  * Click toggles `graphState.filters[kind]`
    via the same `show` proxy the overlay's
    chips bind into — `gf:` URL hash
    persistence unchanged.
* Section dividers: `.msep` between the
  action group / depth / settings cluster /
  filters cluster, matching the convention
  used by TerminalTab's hamburger menu.
* CSS: replaced `.bubble-list` / `.bubble-
  filters` / `.bubble-filters-label` rules
  with `.mbtn` / `.msep` / `.depth-row` /
  `.filter-row` / `.filter-dot` styling
  modeled on TerminalTab's hamburger CSS so
  the bubble feels like one shape across
  surfaces. Empty-ruleset `.filter-row {}` /
  unused `.mbtn[disabled]` selector trimmed
  per svelte-check.

The overlay variant's bar still uses the
existing `filterChips()` snippet + the
`<HamburgerMenu>` wrapper for `menuItems()`,
so the chrome bar (which only renders when
the panel is NOT a tab) is unchanged. Two
surfaces, one source of truth for the action
list (chip toggles bind into the same
`graphState.filters`), but different visual
shells.

Tests updated in `revealBrowserActions.test.ts`:

* Old "carrying menuItems + filterChips"
  assert flipped to "with mbtn rows +
  vertical filter rows". Source asserts:
  bubble contains `<button class="mbtn"
  onclick={toggleInspector}` + `<button
  class="mbtn filter-row"` with the
  `show[kind] = !show[kind]` toggle. Also
  asserts the bubble does NOT carry the
  horizontal `<div class="bubble-filters">`
  container anymore.

Gate green:

* `npm run test` (410 passed),
* `npm run check` (0 errors / 0 warnings —
  two transient warnings from drafting were
  resolved by trimming the empty ruleset +
  unused `[disabled]` selector),
* `npm run build`,
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`
  (green).

Visual eyeball worth doing: right-click a
Graph tab → bubble opens with the new row
shape; each filter row toggles independently;
the on/off cue reads from the dot fill.
Compare to TerminalTab's hamburger menu —
should feel like the same chrome.

Proposed commit message:

> Align Graph tab right-click rows with hamburger shape (fullstack-75)
>
> Restyle the Graph-tab right-click bubble to use
> the standard `.mbtn` row shape from the other
> tab menus (TerminalTab / FileEditorTab /
> FileBrowserSurface). Inline the action rows
> (Show Details / Depth / Reload / Settings) and
> render the filter chips as per-row toggle
> buttons, one per kind, with a coloured dot for
> on/off state + a count on the right. The
> overlay variant's chrome bar keeps the original
> horizontal chip layout. Filter toggles still
> mutate `graphState.filters`, so the `gf:` URL
> hash round-trips unchanged.
