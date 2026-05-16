# webdev-1

## Scope

Started on the frontend-only pre-release items from `request.md`:

- Search window arrow navigation should scroll the active result into view and recalibrate after pane/window resize.
- Assistant chat should stay pinned to the bottom with a bottom margin while new turns/streaming output arrive and after pane/window resize.
- Assistant chat bubbles should be allowed to stretch to the full chat width when text needs it.
- Assistant thinking state should keep only the orange-dot status badge, with the dot blinking like the file-tab assistant activity indicator.

## Changes

- `web/src/components/SearchPanel.svelte`
  - Added active-result scroll tracking for keyboard navigation.
  - Added `ResizeObserver` on the hits list so active selection is re-scrolled after panel/pane resize.

- `web/src/components/InlineAssist.svelte`
  - Replaced direct microtask scrolls with double-`requestAnimationFrame` bottom pinning and a 28px bottom margin.
  - Added `ResizeObserver` on the chat scroll container.
  - Removed the duplicate `thinking...` placeholder body when the status badge is already visible.
  - Let chat bubbles grow to `max-width: 100%`.
  - Added blinking animation to the stream status orange dot.

## Verification

- `cd web && npm run check`
  - Passes with 0 errors and 0 warnings.

## Notes

- No backend/API changes.
- No tests added; this is DOM layout behavior covered by Svelte type checking plus manual/browser verification.
