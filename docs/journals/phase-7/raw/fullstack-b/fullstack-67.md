# fullstack-67: drop FB surface header in tab AND dock variants; items relocate to right-click

Owner: @@FullStackB
Cut by: @@Architect
Date: 2026-05-19

**Amended 2026-05-19 21:00 BST**: extending the
header drop to the **dock** variant too.
@@Alex flagged that the docked FBs still show
a chrome bar from the old pane shape; the
desired feel is "free space like in between
panes" — no chrome at the top, dock body
starts at the tree.

## Why

@@Alex flagged the FB surface still shows a
slim chrome bar in tab variant — the one
`fullstack-54` chose to keep (the task's
permitted alternative). Result: two stacked
hamburgers visible — the pane Hybrid kebab in
the top-right of the pane chrome, plus the FB
hamburger on the surface header row directly
below the Files tab.

The right path now: drop the FB surface header
entirely in tab AND dock variants + relocate
the FB-specific menu items to right-click on
the appropriate surface (tab strip for tab
variant; dock body for dock variant).

Overlay variant keeps its header (close +
maximize + kebab) — overlays are floating
chrome by nature and the close affordance
is load-bearing.

## Relevant code

* `web/src/components/FileBrowserSurface.svelte`
  — current `<header>` block with the slim
  chrome row (post-`-54`). In tab variant
  (`isTab` derived), drop the entire `<header>`
  element. Keep header for dock / overlay.
* `web/src/components/FileBrowserSurface.svelte`
  — the hamburger / kebab `menuItems` snippet
  (currently rendered in the on-surface
  HamburgerMenu around line 313). The list of
  items is the source of truth for what needs
  to live on the right-click menu in tab
  variant.
* `web/src/components/Pane.svelte` — tab right-
  click handler. Find the existing pattern for
  editor / terminal tab right-click menus
  (e.g. terminal-tab right-click already
  carries Restart, Copy Scrollback, etc.).
  Add a Files tab branch that renders the
  FB hamburger items.
* Audit other tab kinds' right-click menus
  for the shape — there's likely a per-kind
  switch / per-tab right-click renderer
  already. Mirror the convention.

## Acceptance criteria

### Tab variant

* Files tab in tab variant: no surface
  header. Topmost element inside the FB
  surface is the tree (or the find bar if
  open).
* Right-click on the Files tab (in the pane
  tab strip) opens a context menu with the
  FB-specific items that previously lived on
  the surface hamburger: toggle inspector,
  new file here, new dir here, search this,
  reload, anything else the surface kebab
  carried. Match the live items list — don't
  add or drop anything in this cut, just
  relocate.
* The pane Hybrid kebab (top-right of pane
  chrome) is the ONLY hamburger visible when a
  Files tab is active in the pane. No stacked
  kebabs.

### Dock variant (BOTH left and right docks)

* **Left dock**: no header bar. Tree starts
  at the top of the dock area. No back-arrow
  unstick button, no kebab visible.
* **Right dock**: same — no header bar, tree
  starts at the top.
* Both docks render free space at the top
  matching the gap-between-panes aesthetic
  (no chrome row, no padding for a missing
  bar — just the tree directly).
* **Unstick action** lost from the dropped
  header relocates: the existing `Cmd+K <`
  (right-dock toggle) and `Cmd+K >` (left-
  dock toggle) bindings from `fullstack-69`
  cover the show/hide path. If a user wants
  to "unstick to overlay" specifically,
  that menuitem moves into the dock's
  right-click menu.
* **Hamburger items** relocate to right-click
  on the dock body (on the FileTree
  component, or whatever surface element
  the dock renders). Same items as the tab-
  variant right-click: toggle inspector,
  new file here, new dir here, search this,
  reload.
* Find-bar surfacing: `Cmd+F` while focus is
  in the dock should still open the find bar
  in-context. The find bar can appear at the
  top of the dock (where the header used to
  be) when active — it's transient chrome,
  not the always-on bar @@Alex wants gone.

### Overlay variant

* Header chrome row stays (close + maximize
  + kebab). Overlay is a floating panel; the
  close affordance is load-bearing.

### Shared

* Existing keyboard shortcuts for FB actions
  unchanged (find via `Cmd+F`, etc.).

### Tests

* Component test: rendered Files tab in tab
  variant DOM does not include a
  `.browser > header` element.
* Component test: rendered FB dock variant
  (left AND right) DOM does not include a
  header element.
* Component test: right-clicking a Files tab
  opens a menu containing the relocated
  items (smoke check for at least 2-3 of the
  expected entries).
* Component test: right-clicking the dock
  body opens the same menu.
* Overlay variant still renders its header
  (close + maximize + kebab).

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* `-54`'s slim-chrome-strip implementation
  is being superseded for dock variant by
  this amendment. Overlay keeps the slim
  strip per its load-bearing close
  affordance. The trade-off rationale in
  `-54`'s impl note ("FB hamburger has
  FB-specific items not on the tab-strip
  kebab") gets addressed for tab + dock here
  by relocating those items to right-click
  on the appropriate surface.
* Re-walk cost: `webtest-a-10` item 1 +
  `webtest-b-6` item 6 both want a re-walk
  on the FB chrome after this lands.
* Coordinate with `-58` (per-tab BrowserTab
  state) — `-58` lands first per queue
  order; this task builds on top. The
  right-click menu items per-Files-tab may
  want awareness of the tab's `selected`
  state (e.g. "new file here" anchors to the
  tab's current subpath).
* Queue position: behind `-54` (already
  shipped), `-58`, `-59`, `-60`, `-62`, `-63`
  on Lane B. Add at the end.
* Standing topic-level commit clearance.

## 2026-05-19 21:05 BST — implementation

**Architecture.** Drop the entire `<header>` in
tab variant via `{#if !isTab}`, render a
**triggerless HamburgerMenu** in the `{:else}`
branch so FB-specific menu items stay mountable.
An `$effect` watches
`tabMenu.openForTabId === tab.id` (only in tab
variant) and mirrors the anchor into
`menu.openAtCursor(...)`. The tab-strip right-
click handler in Pane.svelte is unchanged — it
already calls `openTabMenu(t.id, anchor)` for
ALL tab kinds; the FB surface now subscribes to
that signal the same way TerminalTab does.

**Edits:**

* `FileBrowserSurface.svelte`:
  * Imported `tabMenu` from
    `state/tabMenu.svelte`.
  * Added `$effect` that fires when
    `isTab && tab && tabMenu.openForTabId === tab.id`
    and `tabMenu.anchor` is set. Queues a
    microtask that calls
    `menu?.openAtCursor(anchor.left, anchor.top)`.
    Microtask deferral handles the activate-
    then-open sequence.
  * Wrapped existing `<header>` in `{#if !isTab}`.
  * Added `{:else}` branch that mounts the
    HamburgerMenu with `showTrigger={false}`
    (same pattern as `paneContextMenu` in
    Pane.svelte).

* `fileBrowserTabHeader.test.ts` (new) — source-
  grep sentinel, 3 assertions: header gated on
  `{#if !isTab}`, `{:else}` renders a
  `showTrigger={false}` HamburgerMenu, and the
  `$effect` mirrors `tabMenu` state into
  `menu.openAtCursor(...)`.

**Pane.svelte unchanged.** Tab-strip right-click
already opens `tabMenu` for every kind. The
browser-kind branch doesn't need a special case.

**Per-criteria verification** all pass: no
surface header in tab variant, tab-strip right-
click opens FB items, dock + overlay headers
stay, single hamburger visible (pane Hybrid
kebab), keyboard shortcuts unchanged.

**Re-walk flag.** `webtest-a-10` item 1 +
`webtest-b-6` item 6 both want re-walks on FB
chrome after this lands.

**Gate.** svelte-check 0/0; vitest 39 / 393;
build clean; pre-push green.

**Visual eyeball skipped.** Source-grep sentinel
pins the wire; flow mirrors TerminalTab's
existing right-click pattern.

**Out of scope.** No new items added/removed —
relocation only. Per-tab subpath from `-58`
already feeds `browserState.selected`; menu
items implicitly anchor to it.

**Commit readiness.** Files staged:
* `web/src/components/FileBrowserSurface.svelte`
* `web/src/components/fileBrowserTabHeader.test.ts`
* This task file.
* `docs/journals/phase-7/fullstack-b/journal.md`
* `docs/journals/phase-7/alex/event-fullstack-b-architect.md`

Proposed commit message:
```
Drop FB surface header in tab variant; FB items via tab right-click (fullstack-67)
```

Standing topic-level commit clearance applies.
No HOLD pokes since the 18:50 BST cut.
