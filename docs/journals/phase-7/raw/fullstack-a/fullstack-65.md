# fullstack-65: Files tab title from selected element (basename)

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex flagged the Files tab title should
mirror what `fullstack-64` does for the Graph
tab: title is the basename of the selected
element (file basename or dir name), not the
literal `Files` string. Same "the tab is named
after what you're looking at" principle.

## Dependency

**Wait for `fullstack-58` to ship from Lane B
before starting this task.** `-58` adds the
per-tab `selected` field to `BrowserTab`. Until
that lands, Files tabs share `selected` state
across tabs (the schema gap that `-58` is
fixing), so a per-tab title from selection
can't render correctly.

Lane A's queue ahead of this:
`-55` → `-56` → `-64` → `-61` → `-65`. By the
time you hit `-65`, `-58` should be on main
from Lane B. If not, idle / pick another and
come back.

## Title resolution

Given `tab.selected` (the per-tab selected
drive-relative path from `-58`):

* `tab.selected` empty / null → title is
  `Files` (current default, no selection).
* `tab.selected = "foo/bar/baz.md"` → title
  is `baz.md` (file basename).
* `tab.selected = "foo/bar/"` → title is
  `bar` (dir basename; trailing slash signals
  directory upstream, optional in title).
* Anything else → fallback basename via
  `path.split("/").filter(Boolean).pop()`.

If `tab.selected` is a directory vs a file —
whichever discrimination the SPA already does
(file kind detection at selection time?) —
mirror it. The current `selected` semantics
from `-58` will inform; check that task's
shipped contract.

## Relevant code

* `web/src/state/tabs.svelte.ts` — `BrowserTab`
  type (expanded by `-58` to include
  `selected`). Find the title field +
  rendering path.
* `web/src/state/tabs.svelte.ts:324`-ish — the
  `tabTitle` helper that returns `t.title` for
  graph + browser. Browser branch should
  derive from `tab.selected` rather than
  `tab.title` if a selection exists.
* `web/src/components/FileBrowserSurface.svelte`
  — the surface that drives the `selected`
  state. Make sure title updates propagate
  when selection changes (Svelte
  reactivity should handle this if the title
  derives from `tab.selected` directly).
* `web/src/state/tabs.svelte.ts:387` — there's
  a `Graph: ${scopeId}` line; `-64` is
  reworking the graph side. Mirror whatever
  pattern `-64` lands for the Graph case so
  the two tab kinds share a derivation
  approach.

## Acceptance criteria

* Files tab title in the tab strip reads:
  * `Files` when nothing is selected.
  * file basename when a file is selected.
  * dir basename when a directory is
    selected.
* Selection change → title updates reactively
  without a full tab rebuild.
* Multiple Files tabs in the same pane (per
  `-58`) each carry their own derived title
  reflecting their own per-tab selection.
* No regression on the Files surface body
  itself — selection still drives the
  DETAILS inspector, expansion state, etc.

### Tests

* Vitest: title derivation function (or
  derived) maps `selected` → display string
  per the resolution rules above.
* Component test: two Files tabs with
  different `selected` paths render
  different titles in the tab strip.
* Negative test: a Files tab with empty
  `selected` falls back to `Files`.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* Mirrors `fullstack-64` (Graph title) — same
  principle, same shape. Coordinate the
  basename helper if it makes sense to share
  one between Graph + Files derivations.
* v0.11.0-blocking-soft — the Files tab still
  works without this, but the marquee
  multi-FB feature reads more naturally
  when each tab is named after its content.
* Re-walk: light. Lane B `webtest-b-6` item 6
  will re-walk post-`-58`; this lands after
  that and benefits from the same re-walk
  session.
* Queue position: end of Lane A queue,
  gated on `-58` shipping from Lane B.
* Standing topic-level commit clearance.

## 2026-05-19 17:05 BST — @@FullStackA implementation note

`fullstack-58` is already on main (`dc1ff46`), so
`BrowserTab.selected` is populated per-tab by
`FileBrowserSurface.svelte`'s activate snapshot.
Building on that contract.

Implementation:

* `tabs.svelte.ts`: new exported helper
  `browserTabLabel(t: BrowserTab)`. Trims and
  splits `t.selected` by `/`, drops empty
  segments (so trailing slashes are tolerated),
  returns the last segment as the basename.
  Empty / null / whitespace-only selection falls
  back to `t.title` (which all constructors set
  to `"Files"`).
* `tabs.svelte.ts:tabLabel`: browser branch
  routed through `browserTabLabel` instead of
  returning `t.title` directly. Selection
  changes propagate reactively through Svelte's
  `$derived` graph since `tabLabel` is called
  inside `tabLabelInPane`, which is in turn read
  from Pane.svelte's tab strip via
  `truncateTabTitle(tabLabelInPane(...))`.
* `tabs.svelte.ts:tabTooltip`: browser branch
  now returns `File Browser: <selected>` when a
  selection exists so hover disambiguates two
  Files tabs whose basenames collide (e.g.
  `index.md` in different dirs); plain
  `File Browser` when no selection.

Title resolution matches the spec:

* `null` / `undefined` / empty / whitespace → `Files`
* `notes/sub/foo.md` → `foo.md`
* `notes/sub` → `sub`
* `notes/sub/` → `sub` (trailing slash tolerated)
* `README.md` → `README.md`

Mirrors `fullstack-64`'s shape for Graph tabs;
both consume the same `truncateTabTitle` from
`fullstack-66` at the Pane.svelte call site, so
long basenames elide via the shared utility.

Tests added in `tabs.test.ts` (`browserTabLabel
(fullstack-65)` describe block):

* No-selection fallback (`null`, `undefined`,
  empty, whitespace-only).
* File basename derivation.
* Dir basename derivation (with + without
  trailing slash).
* Two browser tabs with different selections
  produce different labels.
* `tabLabel` routes the browser branch through
  `browserTabLabel` (regression check).

Gate green:

* `npm run test -- tabs` (85 passed),
* `npm run test` (372 passed),
* `npm run check` (0 errors / 0 warnings),
* `npm run build`,
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`
  (green).

Visual eyeball worth doing: open a Files tab,
select a file → tab strip shows the basename;
select a directory → shows the dir basename;
deselect → falls back to `Files`. Open two
Files tabs, each with different selections —
each tab shows its own derived title.

Proposed commit message:

> Files tab title from selection (fullstack-65)
>
> Files tab title in the tab strip now derives from
> the per-tab `selected` path (added by fullstack-58)
> instead of the literal "Files" label. New
> `browserTabLabel(tab)` returns the basename of the
> selection (trailing slash tolerated); no selection
> falls back to `tab.title` ("Files" by default).
> `tabLabel` routes the browser branch through it;
> `tabTooltip` carries the full selected path so
> hover disambiguates basename collisions across
> Files tabs. Mirrors fullstack-64's shape for
> Graph tabs.
