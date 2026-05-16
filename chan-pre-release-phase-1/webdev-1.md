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

## 2026-05-16 active-turn follow-up

Assistant-enabled CDP smoke later found one narrow-viewport layout miss:
completed assistant bubbles were still content-width at 390px. Updated
`InlineAssist.svelte` so assistant bubbles stretch across the chat column
and their body fills the available width.

The same smoke verifies active-turn pending badge, bottom pinning, and
wide assistant bubble behavior through the isolated fake-Codex fixture.

## Verification

- `cd web && npm run check`
  - Passes with 0 errors and 0 warnings.
- `cd web && npm test -- --run`
  - Passes: 6 files / 97 tests.
- `CHAN_WEB_URL=http://127.0.0.1:8793/ CHAN_WEBTEST_ONLY=assistant node chan-pre-release-phase-1/webtest-smoke.mjs`
  - Passes desktop and narrow active-turn assistant checks when run
    against the isolated fake-Codex fixture.

## Notes

- No backend/API changes.
- The active-turn DOM behavior is covered by the CDP browser smoke.
