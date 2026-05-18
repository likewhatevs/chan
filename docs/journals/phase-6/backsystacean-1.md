# @@Backsystacean-1

Owner: @@Backsystacean
Status: Ready for review

## Goal

Start the backend/systems/Rustacean track from [request.md](./request.md) with a focused terminal-session slice.

## Acceptance Criteria

- New terminal tabs use enumerated names (`Terminal-N`).
- Shift+Enter is passed through distinctly for terminal TUIs that support enhanced keyboard protocols.
- Shell EOF / process exit leaves the tab in an actionable state with close guidance.
- Changes have focused tests where practical.

## Progress Notes

- 2026-05-17: Started directly from Alex's chat assignment before @@Architect task files existed.
- 2026-05-17: Implemented enumerated terminal tab names, Shift+Enter enhanced-keyboard bytes, and Ctrl+D close handling after shell/process exit.

## Completion Notes

- Files changed:
  - `web/src/state/tabs.svelte.ts`
  - `web/src/state/tabs.test.ts`
  - `web/src/terminal/keymap.ts`
  - `web/src/terminal/keymap.test.ts`
  - `web/src/components/Pane.svelte`
  - `web/src/components/TerminalTab.svelte`
- Verification:
  - `npm test -- --run src/terminal/keymap.test.ts src/state/tabs.test.ts`
  - `npm run check`
- Known gap: `CHAN_TAB_NAME` can be set at PTY spawn, but the OS does not provide a general way for chan to mutate the environment of an already-running shell or child process after a tab rename. A complete fix needs explicit shell integration or an injected shell command, which should be a separate product decision.

## Commit Readiness

Ready for frontend/Rustacean review. Proposed subject:

`Improve terminal tab naming and key handling`
