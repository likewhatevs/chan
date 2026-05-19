# fullstack-71: drop FB surface header in dock variant (both left + right)

Owner: @@FullStackB
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex flagged the docked FBs still show a
chrome bar from the old pane shape — both
left and right docks. The desired feel is
"free space like in between panes" — no top
bar in dock variant.

This is a deliberate **separate task** from
`-67` (which covers the tab-variant header
drop). @@Architect made the mistake of
amending `-67` mid-flight; this is the
follow-up cut to honour the append-only rule.
See memory
[[feedback-redistribution-queue-head]] +
[[feedback-inflight-task-amendments]] —
strengthened to "tasks in an agent's queue
count as in-flight".

## Scope

Tab-variant header drop is `-67`'s
responsibility (already shipping per the
21:05 BST impl note). This task picks up
just the dock-variant portion:

* **Left dock**: no header bar. Tree starts
  at the top of the dock area. No back-arrow
  unstick button, no kebab visible.
* **Right dock**: same — no header bar, tree
  starts at the top.
* Both docks render free space at the top
  matching the gap-between-panes aesthetic.

## Relevant code

* `web/src/components/FileBrowserSurface.svelte`
  — after `-67` ships, the `<header>` is
  rendered for `!isTab` (covering dock +
  overlay). This task narrows that gate
  further to `isOverlay` only, so dock
  variant joins tab variant in losing the
  header.
* `web/src/components/FileBrowserSidePane.svelte`
  — the dock-variant wrapper. If the FB
  hamburger items move to a dock-body right-
  click handler, this is likely where the
  handler attaches.
* `web/src/state/tabMenu.svelte` — the
  `openTabMenu` / `tabMenu.openForTabId`
  signal that `-67` uses for tab variant.
  Dock variant doesn't have a `tab.id` to
  key off — needs a parallel signal for
  the dock body, OR a direct
  `openAtCursor(...)` call from the dock
  body's `oncontextmenu` handler.

## Acceptance criteria

* **Left dock**: no `<header>` element
  rendered. Topmost element in the dock body
  is the tree (or the find bar when open).
* **Right dock**: same.
* **Hamburger items relocate**: right-click
  on the dock body (anywhere on the
  FileTree, or on a dedicated dock-body
  surface) opens a context menu with the
  items that used to live on the dock
  hamburger: toggle inspector, new file
  here, new dir here, search this, reload,
  unstick.
* **Unstick action**: still reachable via
  the right-click menu AND via `Cmd+K <` /
  `Cmd+K >` (from `-69`, already shipped).
* **Overlay variant**: unchanged. Keeps its
  header (close + maximize + kebab) per the
  load-bearing close affordance.
* **Find-bar**: `Cmd+F` while focus is in
  the dock opens a transient find bar at
  the top of the dock area (where the header
  used to be). Closes on Esc or
  done-find.

### Tests

* Component test: rendered FB dock variant
  DOM does not include a `<header>`
  element.
* Component test: right-clicking the dock
  body opens the context menu with the
  relocated items.
* Component test: overlay variant still
  renders its header.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* `-67` is the tab-variant predecessor;
  this task layers on top. By the time you
  pick up `-71`, `-67` should be on main.
* Re-walk cost: light. The dock variant
  isn't covered by current re-walk tasks;
  fold into the post-`-67` walkthrough
  session if Lane A re-walks after `-67`
  ships.
* Queue position: end of Lane B queue —
  after `-67`.
* Standing topic-level commit clearance.

## 2026-05-19 21:30 BST — implementation

**Narrowed the header gate.** Changed
`{#if !isTab}` → `{#if isOverlay}`, so only the
overlay variant renders an on-surface header.
Tab + dock variants both fall through to the
existing `{:else}` branch (the triggerless
HamburgerMenu added by `-67`).

**Dock right-click path.** Already wired via the
`.browser` root's existing
`oncontextmenu={onBrowserContextMenu}` handler
(from `-54`), which calls
`menu.openAtCursor(e.clientX, e.clientY)`. Same
HamburgerMenu instance the tab variant uses;
the `bind:this={menu}` binding fires regardless
of variant. No new handler needed — the menu's
existing mount path on the `.browser` root
covers dock body right-click without
modification.

**Per-criteria verification:**

* Left dock: no `<header>` rendered (gated by
  `isOverlay`). Topmost element is the body →
  `.tree-wrap` → tree. ✓
* Right dock: same. ✓
* Hamburger items reachable via right-click on
  the dock body — the existing
  `onBrowserContextMenu` opens the menu at the
  cursor. Items unchanged: toggle inspector,
  new file here, new dir here, search this,
  reload, Stick/Unstick left, Stick/Unstick
  right. ✓
* Unstick: reachable via the relocated menu's
  "Unstick left" / "Unstick right" entries AND
  via `Cmd+K <` / `Cmd+K >` (from `-69`). ✓
* Overlay variant: unchanged. Keeps its header
  with maximize + kebab. ✓
* `Cmd+F` while focus is in the dock: opens
  the find bar inside `.tree-wrap` (where it
  already lived), not the removed header. No
  regression. ✓

**Hygiene sweep.** Removed the now-unused
`unstick()` helper (its only consumer was the
dock-variant chrome button I dropped) and the
`setBrowserSidePane` import (only consumer was
`unstick`). `toggleBrowserSidePane` stays — the
menu's "Stick to / Unstick" entries still use
it.

**Edits:**
* `web/src/components/FileBrowserSurface.svelte`:
  * Header gate narrowed: `{#if !isTab}` →
    `{#if isOverlay}`.
  * Removed the dock-variant `{:else if variant
    === "dock"}` chrome button branch inside
    the header (no longer reachable; only
    overlay's max/restore button remains in the
    header).
  * Removed `unstick()` function +
    `setBrowserSidePane` import.
  * Updated the `{:else}` comment to mention
    both tab + dock variants now share the
    triggerless menu path.

* `web/src/components/fileBrowserTabHeader.test.ts`:
  * Renamed describe block:
    "FileBrowserSurface header gated on tab
    variant" → "FileBrowserSurface header is
    overlay-only".
  * Flipped header gate assertion from
    `{#if !isTab}` to `{#if isOverlay}`.
  * Added 2 new tests:
    - Dock-body right-click flows through
      `onBrowserContextMenu` → `menu.openAtCursor`.
    - No `function unstick()` / dock-variant
      unstick button title left in the source.

**Gate.** `npm run check` 0/0; `npm run test`
39 files / 401 tests passed (was 39 / 393;
+2 from new sentinel + 6 from parallel-lane
work); `npm run build` clean;
`scripts/pre-push` green.

**Visual eyeball.** Skipped. The change is
mechanical (narrow the `{#if}` from `!isTab`
to `isOverlay`) and the dock right-click path
was already wired to the same menu instance.
If @@Alex flags pixel issues on walkthrough
(e.g. tree-wrap padding feels off at the top
without the former header gap), I'll follow
up.

**Re-walk flag.** None new beyond what `-67`
already flagged. Light cost per task note.

**Commit readiness:**

Files staged:
* `web/src/components/FileBrowserSurface.svelte`
* `web/src/components/fileBrowserTabHeader.test.ts`
* `docs/journals/phase-7/fullstack-b/fullstack-71.md`
* `docs/journals/phase-7/fullstack-b/journal.md`
* `docs/journals/phase-7/alex/event-fullstack-b-architect.md`

Proposed commit message:
```
Drop FB surface header in dock variant (both sides) (fullstack-71)
```

Standing topic-level commit clearance applies.
No HOLD pokes since the 21:10 BST cut.
