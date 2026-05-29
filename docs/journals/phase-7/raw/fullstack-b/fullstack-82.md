# fullstack-82: drop "Open overlay" from FB dock variant menu

Owner: @@FullStackB
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex flagged the docked FB right-click menu
still has an `Open overlay` entry that needs
to go. The other shared-menu trims
(`Search this`, `Settings`, `Show/Hide
Details`) are already in scope for
`fullstack-80` — they apply to the dock menu
automatically since tab + dock share the same
HamburgerMenu instance per `-71`'s impl note.

The `Open overlay` entry is dock-variant-
specific (it doesn't render in tab variant —
you're already viewing the FB in a tab).
That's why it survived `-80`'s sweep.

Side context from `webtest-a-10`'s side
observations: the `Open overlay` menuitem
calls `openBrowser()`, which actually opens
a Files **tab**, not the overlay variant.
The label and behaviour have been
inconsistent for a while. Dropping the entry
resolves it without needing to decide
"rename to Open as tab" vs "rewire to truly
open overlay".

## Relevant code

* `web/src/components/FileBrowserSurface.svelte`
  — the `menuItems` snippet rendered in the
  triggerless HamburgerMenu. Find the
  `Open overlay` entry (likely gated on
  `variant === "dock"`) and drop it.
* If the underlying `openBrowser()` /
  `openOverlay()` function is only consumed
  by this entry, hygiene-sweep it. Audit
  first — it may have other callers.

## Acceptance criteria

* Right-click on the dock body (left or
  right) opens a context menu with NO
  `Open overlay` entry.
* Other dock-variant items still present
  (the surviving entries from `-80`'s
  trims).
* Tab variant + Overlay variant menus
  unchanged in shape (this task only
  touches the dock-variant-gated entry).

### Tests

* Component test: rendered FB dock variant
  right-click menu DOM has no
  `Open overlay` label.
* Negative grep: the `Open overlay`
  string literal doesn't appear in the
  rendered menu for either dock side.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* Depends on `-80` having shipped first
  (the broader trims). Order on Lane B
  queue places this after `-80` naturally.
* Lane A is wrapping out the Pane Mode /
  Graph cluster in parallel; no
  coordination needed here.
* v0.11.0-blocking-soft.
* Queue position: end of Lane B queue.
  Updated queue: `-78` → `-79` → `-80` →
  `-82`.
* Standing topic-level commit clearance.

## 2026-05-19 23:25 BST — implementation (bundled with -80)

**Drop.** Removed the `Open overlay` `mbtn` block
at `FileBrowserSurface.svelte:476-484` (the
`{#if variant === "dock"}` gate + the
`<button onclick={openOverlay}>` row inside).

**Hygiene sweep.** The dropped row was the only
consumer of:
* `openOverlay()` helper (dropped — the dock
  variant's `openBrowserInActivePane()` path).
* `openBrowserInActivePane` import (dropped from
  `state/tabs.svelte`). The function stays in
  `tabs.svelte` for other consumers (Pane Mode
  spawn key `2`, etc.); just the surface's
  import is gone.
* The lucide `Maximize2` import — kept; the
  surface still uses it elsewhere (the overlay
  variant's chrome maximize button).
* The `chordFor("app.files.toggle")` reference
  inside the dropped row — gone with the row;
  the function lives globally for other
  consumers.

**Why bundle with -80**: the task explicitly
notes `-82` depends on `-80` having shipped
first. Bundling in a single commit avoids a
two-commit ping-pong where the audit grep
between commits would flag transient
inconsistency. The commit message documents
both task IDs.

**Test:** new sentinel block in
`menuTrims.test.ts` (5 assertions covering the
label drop, the `{#if variant === "dock"}` gate
drop, the `openOverlay` helper drop, and the
`openBrowserInActivePane` import drop).

**Per-criteria:** all met.
* Dock right-click menu has no `Open overlay`
  entry. ✓
* Other dock-variant items still present
  (Stick/Unstick, New file, New dir, etc.). ✓
* Tab + Overlay variants unchanged in shape —
  they never had the `Open overlay` entry
  (it was dock-gated). ✓

**Gate** (combined with -80): svelte-check 0/0,
vitest 42/433, build clean, scripts/pre-push
green.

**Visual eyeball.** Skipped — single-line
mechanical drop. Re-walk shared with `-80`.

**Out of scope:**
* The label-behaviour mismatch
  (`Open overlay` actually called `openBrowser()`
  which opens a tab) — task note documents
  this; dropping the entry resolves it
  without us needing to choose between rename
  or rewire.

**Commit readiness.** Bundled commit covers both
`-80` and `-82`. Files staged (combined):

* `web/src/components/TerminalTab.svelte`
* `web/src/components/FileBrowserSurface.svelte`
* `web/src/components/FileTree.svelte`
* `web/src/components/GraphPanel.svelte`
* `web/src/components/menuTrims.test.ts` (new)
* `web/src/components/revealBrowserActions.test.ts`
  (existing bubble-shape test updated)
* `web/src/raw.d.ts` (added `.ts?raw` module
  declaration so the `-79` sentinel compiles
  cleanly)
* `docs/journals/phase-7/fullstack-b/fullstack-80.md`
* `docs/journals/phase-7/fullstack-b/fullstack-82.md`
* `docs/journals/phase-7/fullstack-b/journal.md`
* `docs/journals/phase-7/alex/event-fullstack-b-architect.md`

Proposed commit message:
```
Trim right-click menus + FB click-to-inspector + drop FB dock Open overlay (fullstack-80, -82)
```

Standing topic-level commit clearance applies.
No HOLD pokes since the 22:40 BST cut.
