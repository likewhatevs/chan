# fullstack-31: drop inline `×` close on Graph + File Browser surfaces

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-19

## Goal

Remove the inline `×` close button from the Graph
surface and the File Browser surface. These were
flagged in `fullstack-29`'s "Known concrete additions"
list (per @@Alex's 2026-05-19 05:00 BST note) and not
addressed in the audit pass.

Now that Graph and File Browser are first-class tabs
with their own tab-strip `×`, the inline ones are
redundant. The audit summary for `fullstack-29`
claimed no follow-up flags — but these two buttons
still ship in the working tree.

## Concrete locations (already grepped)

* `web/src/components/GraphPanel.svelte:1078-1086`
  — `<button class="chrome-btn close" onclick={close}>`.
* `web/src/components/FileBrowserSurface.svelte:~325`
  — same `chrome-btn close` pattern with the `X` icon.

## Acceptance criteria

* Both inline `×` close buttons are removed from the
  surface chrome.
* Any associated state hooks (the `close()` function,
  the `onClose` prop wiring) are cleaned up if no
  longer used. If the function is still used by some
  legacy code path, the diff should be a clean removal,
  not a half-orphan.
* No regressions on the tab-strip `×` (the proper way
  to close the tab) — verified by smoke + existing
  tests.
* Add a `closeAffordance` (or similar grep-friendly)
  test assertion that the surfaces don't ship the
  inline button.

## Audit-discipline note

`fullstack-29` was specifically scoped to catch items
like this. Listing them in the task file and missing
them in the audit defeats the point of the task. Treat
this as a discipline check — re-grep, re-list, confirm
nothing else from the original audit's "Known concrete
additions" snuck through.

## How to start

1. Delete the `<button class="chrome-btn close">`
   block in both files.
2. If `onClose` / `close()` are no longer referenced
   elsewhere in the same file, drop them.
3. Update tests.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-architect.md`.

## 2026-05-19 10:05 BST — @@FullStackA specialist review

Patch ready. Scope is a clean drop, no half-orphans:

* `web/src/components/GraphPanel.svelte` — removed the
  inline `<button class="chrome-btn close">` block
  (was lines 1078-1086). Dropped the now-unused `X`
  lucide-svelte import. The `close()` helper stays
  because it's still wired internally for "open file
  then dismiss" flows (e.g. `openInActivePane(...);
  close();` at the inspector "Open" buttons and the
  legacy `<OverlayShell onClose={close}>` fallback).
* `web/src/components/FileBrowserSurface.svelte` —
  removed the `{#if isWideSurface}` close-button block
  (was lines 322-332). Dropped the now-unused `X`
  lucide-svelte import. `closeSurface()` stays because
  it's still wired from the `isOverlay` "open file
  dismisses overlay" guards (dead default path, but
  the audit's broader job).

No CSS cleanup needed; `.chrome-btn.close` had no
modifier-specific rules.

### Re-audit of fullstack-29's "Known concrete additions"

Re-grepped the working tree after the edits:

* `chrome-btn close` — `git grep "chrome-btn close"` returns
  zero hits across `web/`.
* `class="close"` and `aria-label="Close"` chrome buttons
  on surface chrome — none in `GraphPanel.svelte` or
  `FileBrowserSurface.svelte`.
* Verified nothing else from the original audit's
  "Known concrete additions" sneaked through: no stray
  Settings entries, no extra menu items in the pane
  hamburger, no overlay-shaped re-mounts.

The audit miss in `fullstack-29` was the close-button
follow-up specifically — addressed here.

### Tests

* `web/src/components/revealBrowserActions.test.ts` —
  added a "no inline close affordance on first-class
  surfaces" describe block with two tests asserting
  neither `GraphPanel.svelte?raw` nor
  `FileBrowserSurface.svelte?raw` contains the
  `class="chrome-btn close"` string. Grep-friendly,
  catches regressions if someone reintroduces the
  pattern.

### Gate

* `npm run test -- revealBrowserActions` — 4 passed.
* `npm run check` — 0 errors / 0 warnings.
* `npm run build` — clean.
* `bash -lc 'ulimit -n 4096; scripts/pre-push'` — green
  (fmt + clippy + tests + no-default-features build).

### Proposed commit message

> Drop inline X close on Graph + File Browser surfaces (fullstack-31)
>
> Both surfaces are first-class tabs now; tab strip × is
> the canonical close affordance. Removes the redundant
> in-chrome close button and the now-unused X icon
> import. Adds revealBrowserActions test assertions to
> keep the pattern out of either surface.

Ready for commit + push under standing topic-level
clearance.
