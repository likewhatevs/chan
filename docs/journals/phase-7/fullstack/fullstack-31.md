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
